// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a decision of a pool that follows journals had read (docs/adr/0116, D5): a fact its
//! session says records how far the session had read each journal it follows, and a request that
//! carries a position of those journals waits in its session's inbox until the pool's reader has
//! passed it.

use super::journal::{tellers, told, SAID};
use super::*;
use crate::peers::{FollowedEntry, Follows};
use sce_rust_session_store::{JournalStore, Read, Stream};

/// The schema of what a decider says.
const DECIDED: &str = "decider.decided";

/// A decider of a room: it takes in the tellers' facts of its room, and to each `ask` it is told
/// it says a fact, of how many it had taken in, and keeps that count. Written by hand in the shape
/// a generated machine has to the pool, as the rooms are.
struct Decider {
    taken: u32,
    decided: Vec<u32>,
    fresh: Vec<JournalEntry>,
}

impl SessionMachine for Decider {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if event.name == SAID {
                stepped = true;
                self.taken += 1;
            } else if event.name == "ask" {
                stepped = true;
                self.decided.push(self.taken);
                self.fresh.push(JournalEntry {
                    schema: DECIDED,
                    revision: 1,
                    data: self.taken.to_string().into_bytes(),
                    valid_ms: None,
                    principal: None,
                });
            }
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "decider".to_string(),
            configuration: vec!["deciding".to_string()],
            current: "deciding".to_string(),
            variables: vec![
                ("taken".to_string(), Value::Number(self.taken.to_string())),
                (
                    "decided".to_string(),
                    Value::Array(
                        self.decided
                            .iter()
                            .map(|count| Value::Number(count.to_string()))
                            .collect(),
                    ),
                ),
            ],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "decider".to_string(),
            invoke_starts: Vec::new(),
            host_invokes: Vec::new(),
            host_invoke_starts: Vec::new(),
            host_invoke_token: 0,
            auto_send_seq: 0,
            external: Vec::new(),
            revisions: Vec::new(),
            features: None,
        })
    }

    fn finished(&self) -> bool {
        false
    }

    fn take_journal(&mut self) -> Vec<JournalEntry> {
        core::mem::take(&mut self.fresh)
    }
}

/// A count of a saved decider.
fn count(value: &Value) -> Result<u32, String> {
    match value {
        Value::Number(digits) => digits.parse().map_err(|_| "not a count".to_string()),
        other => Err(alloc::format!("a count that is {other:?}")),
    }
}

/// What a saved decider took in, and the count each of its decisions saw.
fn decider_of(state: &SavedState) -> Result<(u32, Vec<u32>), String> {
    let taken = count(
        state
            .variable("taken")
            .map_err(|error| alloc::format!("{error:?}"))?,
    )?;
    let decided = match state.variable("decided") {
        Ok(Value::Array(items)) => items.iter().map(count).collect::<Result<Vec<_>, _>>()?,
        other => return Err(alloc::format!("its decisions are {other:?}")),
    };
    Ok((taken, decided))
}

/// The factory of deciders.
struct Deciders;

impl SessionFactory<Wire, Clock> for Deciders {
    type Machine = Decider;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Decider, String> {
        Ok(Decider {
            taken: 0,
            decided: Vec::new(),
            fresh: Vec::new(),
        })
    }

    fn restore(
        &self,
        _key: &str,
        state: &SavedState,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Decider, String> {
        let (taken, decided) = decider_of(state)?;
        Ok(Decider {
            taken,
            decided,
            fresh: Vec::new(),
        })
    }
}

/// What the deciders follow: the tellers' facts.
const FOLLOWS: Follows = Follows {
    journals: &["tellers"],
    entries: &[FollowedEntry {
        event: SAID,
        record: super::journal::SAID_RECORD,
    }],
};

/// The deciders, keyed by the `room` field of what they are told and of the entries they follow,
/// a request's position of the tellers' journal read from `position_from`, holding at most
/// `max_inbox` envelopes in one session's inbox.
fn deciders_machine(position_from: Option<&'static str>, max_inbox: u32) -> Machine {
    Machine {
        name: "deciders",
        sessions: Some(Sessions {
            follows: FOLLOWS,
            position_from,
            max_inbox,
            ..ROOMS_SESSIONS
        }),
        ..ROOMS
    }
}

/// A pool of `machine`, a deployment of the deciders, its sessions kept through the deciders'
/// handle on `store`, reading the journal of `store`, and the transport it sends through.
fn deciders(
    machine: Machine,
    store: &MemoryStore,
    clock: &Clock,
) -> (Pool<Deciders, Wire, Clock>, Wire) {
    let wire = Wire::default();
    let pool = Pool::new(
        machine,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Deciders,
        Arc::new(store.of_machine("deciders")),
        wire.clone(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"))
    .follow_from(Arc::new(store.clone()));
    pool.peer_ready("client");
    (pool, wire)
}

/// Make the tellers of `store` say `said`, each a room and a fact, in order, the client's
/// envelopes numbered from `first`.
fn say(store: &MemoryStore, clock: &Clock, first: u32, said: &[(&str, &str)]) {
    let pool = tellers(Arc::new(store.of_machine("tellers")), clock);
    for (n, (room, fact)) in (first..).zip(said) {
        let key = pool
            .receive("client", told(room, "say", fact, n))
            .expect("placed in a session");
        pool.run(&key);
    }
    assert!(pool.take_errors().is_empty());
}

/// Follow until the reader hands nothing more, running each session it names.
fn follow_all(pool: &Pool<Deciders, Wire, Clock>) {
    for _ in 0..64 {
        let keys = pool.follow();
        if keys.is_empty() {
            return;
        }
        for key in keys {
            pool.run(&key);
        }
    }
    panic!("the reader never stopped handing entries");
}

/// The client's `n`th `ask` of the decider of `room`, its data `data`.
fn ask(data: &str, n: u32) -> Vec<u8> {
    from_client("ask", data, n, false)
}

/// What the deciders' journal holds of `room`: each decision's count and what it was decided
/// having read.
fn decisions(store: &MemoryStore, room: &str) -> Vec<(String, Vec<Read>)> {
    store
        .stream(
            Stream::Session {
                machine: "deciders",
                key: room,
            },
            0,
            64,
        )
        .expect("the deciders' stream")
        .entries
        .into_iter()
        .map(|entry| (String::from_utf8(entry.data).expect("a count"), entry.read))
        .collect()
}

/// How far a decision had read the tellers' journal.
fn tellers_through(through: u64) -> Vec<Read> {
    vec![Read {
        journal: "tellers".to_string(),
        through,
    }]
}

/// D5: a fact a follower's session says records how far the session had read each journal it
/// follows when its run said it, the position its checkpoint holds of each, and a session that
/// took nothing in of a journal records none of it. Red if the pool's write leaves what a decision
/// had read out, or records what the session read after it decided.
#[test]
fn a_fact_a_follower_says_records_how_far_it_had_read() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, 1, &[("r1", "a"), ("r1", "b"), ("r2", "c")]);
    let (pool, _) = deciders(deciders_machine(None, 64), &store, &clock);
    follow_all(&pool);
    for (n, room) in (1..).zip(["r1", "r2", "r3"]) {
        let key = pool
            .receive("client", ask(&alloc::format!(r#"{{"room":"{room}"}}"#), n))
            .expect("placed in a session");
        pool.run(&key);
    }
    // The tellers' journal goes on past the decisions at 4 to 6.
    say(&store, &clock, 4, &[("r1", "d")]);
    follow_all(&pool);
    let key = pool
        .receive("client", ask(r#"{"room":"r1"}"#, 4))
        .expect("placed in a session");
    pool.run(&key);
    assert!(pool.take_errors().is_empty());
    assert_eq!(
        decisions(&store, "r1"),
        [
            ("2".to_string(), tellers_through(2)),
            ("3".to_string(), tellers_through(7)),
        ]
    );
    assert_eq!(
        decisions(&store, "r2"),
        [("1".to_string(), tellers_through(3))]
    );
    assert_eq!(decisions(&store, "r3"), [("0".to_string(), Vec::new())]);
}

/// What the decider of `room` decided, as its durable checkpoint holds it: the count of the
/// tellers' facts each decision had taken in.
fn decided(store: &MemoryStore, room: &str) -> Vec<u32> {
    let loaded = checkpoint::load(&store.of_machine("deciders"), room)
        .expect("read")
        .expect("a decider saved");
    decider_of(&loaded.checkpoint.state).expect("a decider").1
}

/// D5: a request that carries a position of the journal the deciders follow waits in its
/// session's inbox, its receipt pending, while the session takes in the entries the reader hands
/// it past it; once the mark the store keeps of the reader has passed the position,
/// [`Pool::follow`] answers the session and the request is decided on every entry at or below
/// it. A request whose position the mark has passed, or that carries none, is taken at once. Red
/// if the pool hands the session a request before the reader has passed its position: the first
/// decision is then made on none of the room's facts.
#[test]
fn a_request_waits_until_the_reader_has_passed_its_position() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, 1, &[("r1", "a"), ("r1", "b"), ("r2", "c")]);
    let (pool, _) = deciders(deciders_machine(Some("after"), 64), &store, &clock);
    let (key, waits) = pool.receive_with_receipt("client", ask(r#"{"room":"r1","after":2}"#, 1));
    let key = key.expect("placed in a session");
    pool.run(&key);
    assert!(pool.is_live("r1"), "its session is loaded and waits");
    assert_eq!(waits.fate(), Fate::Pending, "it waits in its inbox");
    assert!(
        decisions(&store, "r1").is_empty(),
        "decided before the reader passed it"
    );
    follow_all(&pool);
    assert_eq!(waits.fate(), Fate::Settled);
    assert_eq!(
        decisions(&store, "r1"),
        [("2".to_string(), tellers_through(2))]
    );
    // Past the decision, at 4, which the reader reads and follows no stream of.
    assert_eq!(store.read_through("deciders").expect("the mark"), 4);
    for (n, data) in [(2, r#"{"room":"r2","after":3}"#), (3, r#"{"room":"r2"}"#)] {
        let key = pool
            .receive("client", ask(data, n))
            .expect("placed in a session");
        pool.run(&key);
    }
    assert_eq!(decided(&store, "r2"), [1, 1], "taken at once");
    assert!(pool.take_errors().is_empty());
}

/// D5: an entry of the followed journal reaches a session whose inbox is at its bound with a
/// request that waits for the reader, and the session takes it past the request, so the mark
/// the request waits for rises. Red if the reader holds the entry at the bound as it holds one
/// for a session that cannot keep up: the request then waits for an entry that waits for it.
#[test]
fn an_entry_reaches_a_session_whose_request_waits_at_its_bound() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, 1, &[("r1", "a"), ("r1", "b")]);
    let (pool, _) = deciders(deciders_machine(Some("after"), 1), &store, &clock);
    let key = pool
        .receive("client", ask(r#"{"room":"r1","after":2}"#, 1))
        .expect("placed in a session");
    pool.run(&key);
    follow_all(&pool);
    assert_eq!(decided(&store, "r1"), [2]);
    assert_eq!(store.read_through("deciders").expect("the mark"), 3);
    assert!(pool.take_errors().is_empty());
}

/// D5: a position above the journal's horizon was told by no answer, and the reader would never
/// pass it, so it is refused where it is received: a request is answered `InvalidArgument`, and
/// anything else is dropped and told to the host. So is a field that holds no position. Red if
/// such a request is held: it then waits for ever.
#[test]
fn a_position_above_the_horizon_or_no_position_is_refused() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, 1, &[("r1", "a")]);
    let (pool, wire) = deciders(deciders_machine(Some("after"), 64), &store, &clock);
    for (n, data) in [
        (1, r#"{"room":"r1","after":2}"#),
        (2, r#"{"room":"r1","after":-1}"#),
        (3, r#"{"room":"r1","after":"one"}"#),
    ] {
        assert_eq!(
            pool.receive("client", from_client("ask", data, n, true)),
            None,
            "{data}"
        );
        let replies = wire.take();
        assert_eq!(replies.len(), 1, "{data}: {replies:?}");
        assert_eq!(
            replies[0].status,
            Some(RpcStatus::InvalidArgument),
            "{data}"
        );
        assert_eq!(replies[0].wire_id, Some(id(0x22, n)));
    }
    assert_eq!(
        pool.receive("client", ask(r#"{"room":"r1","after":2}"#, 4)),
        None
    );
    let errors = pool.take_errors();
    assert!(
        matches!(
            errors.as_slice(),
            [PoolError::Unpositioned { key, event, reason }]
                if key == "r1" && event == "ask" && reason.contains("above the journal's horizon, 1")
        ),
        "{errors:?}"
    );
    assert!(
        !pool.is_live("r1"),
        "no session was loaded for what was refused"
    );
}

/// D5: a stop answers a request that waits for a reader the stopping pool no longer runs,
/// whether its session was loaded or never was, and does not wait for it until its deadline;
/// the sender carries it again. Red if the stop runs a session whose inbox holds only what waits
/// until its deadline, or leaves a session that was never loaded holding its request unanswered.
#[test]
fn a_stop_answers_a_request_that_waits_for_the_reader() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, 1, &[("r1", "a"), ("r2", "b")]);
    let (pool, wire) = deciders(deciders_machine(Some("after"), 64), &store, &clock);
    let loaded = pool
        .receive(
            "client",
            from_client("ask", r#"{"room":"r1","after":2}"#, 1, true),
        )
        .expect("placed in a session");
    pool.run(&loaded);
    assert!(pool.is_live("r1"));
    assert_eq!(
        pool.receive(
            "client",
            from_client("ask", r#"{"room":"r2","after":2}"#, 2, true)
        ),
        Some("r2".to_string()),
        "placed in a session never run"
    );
    let patience = Duration::from_secs(5);
    let began = Instant::now();
    let report = pool.stop(began + patience);
    assert!(
        began.elapsed() < patience,
        "the stop waited for what waits for the reader until its deadline"
    );
    assert_eq!(report.saved, ["r1"], "{report:?}");
    let mut answered: Vec<(Option<[u8; 16]>, Option<RpcStatus>)> = wire
        .take()
        .into_iter()
        .filter(|sent| sent.pattern == PatternKind::RpcReply)
        .map(|sent| (sent.wire_id, sent.status))
        .collect();
    answered.sort_by_key(|(wire_id, _)| *wire_id);
    assert_eq!(
        answered,
        [
            (Some(id(0x22, 1)), Some(RpcStatus::Unavailable)),
            (Some(id(0x22, 2)), Some(RpcStatus::Unavailable)),
        ]
    );
    assert!(decisions(&store, "r1").is_empty() && decisions(&store, "r2").is_empty());
}
