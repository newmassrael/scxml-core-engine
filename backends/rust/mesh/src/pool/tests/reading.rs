// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a decision of a pool that follows journals had read (docs/adr/0116, D5): a fact its
//! session says records how far the session had read each journal it follows.

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

/// The deciders, keyed by the `room` field of what they are told and of the entries they follow.
fn deciders_machine() -> Machine {
    Machine {
        name: "deciders",
        sessions: Some(Sessions {
            follows: FOLLOWS,
            ..ROOMS_SESSIONS
        }),
        ..ROOMS
    }
}

/// A pool of the deciders, its sessions kept through the deciders' handle on `store`, reading the
/// journal of `store`.
fn deciders(store: &MemoryStore, clock: &Clock) -> Pool<Deciders, Wire, Clock> {
    let pool = Pool::new(
        deciders_machine(),
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Deciders,
        Arc::new(store.of_machine("deciders")),
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"))
    .follow_from(Arc::new(store.clone()));
    pool.peer_ready("client");
    pool
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
    let pool = deciders(&store, &clock);
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
