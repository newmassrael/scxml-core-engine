// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What a session of the pool says into the event journal (docs/adr/0114, D1 and D2): each entry
//! is appended by the write that keeps the state that said it, to the session's own stream, and
//! exists exactly when that state does.

use super::*;
use sce_rust_session_store::{JournalStore, StreamName};

/// The schema of what a teller says.
pub(super) const SAID: &str = "teller.said";

/// The description of the record of [`SAID`] (`sce_build::forge::payload_record::describe`):
/// the fact, and the room whose session said it.
pub(super) const SAID_RECORD: &str =
    r#"{"record":[["fact",false,{"text":32},null],["room",false,{"text":16},null]]}"#;

/// A machine that says a fact for each `say` it is told, the fact the text of its data's `fact`,
/// and keeps how many it said; `close` ends its session. Written by hand in the shape a generated
/// one has to the pool, as the rooms are.
pub(super) struct Teller {
    said: u32,
    fresh: Vec<JournalEntry>,
    closed: bool,
}

impl SessionMachine for Teller {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            stepped = true;
            match event.name.as_str() {
                "say" => {
                    let data =
                        sce_rust_runtime::record_cbor::encode(&event.metadata.data, SAID_RECORD)
                            .unwrap_or_else(|refusal| panic!("a fact of its record: {refusal:?}"));
                    self.said += 1;
                    self.fresh.push(JournalEntry {
                        schema: SAID,
                        revision: 1,
                        data,
                        valid_ms: None,
                        principal: event.principal.clone(),
                    });
                }
                "close" => self.closed = true,
                _ => {}
            }
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "teller".to_string(),
            configuration: vec!["telling".to_string()],
            current: "telling".to_string(),
            variables: vec![("said".to_string(), Value::Number(self.said.to_string()))],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "teller".to_string(),
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
        self.closed
    }

    fn take_journal(&mut self) -> Vec<JournalEntry> {
        core::mem::take(&mut self.fresh)
    }
}

/// How many facts the saved teller `state` said.
fn said_of(state: &SavedState) -> Result<u32, String> {
    match state.variable("said") {
        Ok(Value::Number(digits)) => digits.parse().map_err(|_| "not a count".to_string()),
        other => Err(alloc::format!("its count is {other:?}")),
    }
}

/// The factory of tellers.
pub(super) struct Tellers;

impl SessionFactory<Wire, Clock> for Tellers {
    type Machine = Teller;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Teller, String> {
        Ok(Teller {
            said: 0,
            fresh: Vec::new(),
            closed: false,
        })
    }

    fn restore(
        &self,
        _key: &str,
        state: &SavedState,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Teller, String> {
        Ok(Teller {
            said: said_of(state)?,
            fresh: Vec::new(),
            closed: false,
        })
    }
}

/// The tellers: one session per room, keyed by the `room` field of what a client sends.
const TELLERS: Machine = Machine {
    name: "tellers",
    ..ROOMS
};

/// A pool of [`TELLERS`] kept in `store`, its links up.
pub(super) fn tellers(store: Arc<dyn SessionStore>, clock: &Clock) -> Pool<Tellers, Wire, Clock> {
    let pool = Pool::new(
        TELLERS,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Tellers,
        store,
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    pool.peer_ready("client");
    pool
}

/// The `n`th thing the client tells the room `room`: `event`, saying `fact`.
pub(super) fn told(room: &str, event: &str, fact: &str, n: u32) -> Vec<u8> {
    from_client(
        event,
        &alloc::format!(r#"{{"room":"{room}","fact":"{fact}"}}"#),
        n,
        false,
    )
}

/// The fact the data of an entry of [`SAID`] holds.
pub(super) fn fact_of(data: &[u8]) -> String {
    let record = sce_rust_runtime::record_cbor::decode(data, SAID_RECORD)
        .unwrap_or_else(|refusal| panic!("a record of {SAID}: {refusal:?}"));
    match json::parse(&record)
        .ok()
        .and_then(|record| record.member("fact").cloned())
    {
        Some(Value::Text(fact)) => fact,
        other => panic!("a fact of text: {other:?}"),
    }
}

/// The stream and the fact of every entry `store`'s journal holds, in the order of their
/// positions.
fn entries(store: &impl JournalStore) -> Vec<(StreamName, String)> {
    store
        .after(0, usize::MAX)
        .expect("a read of the journal")
        .entries
        .into_iter()
        .map(|entry| (entry.stream, fact_of(&entry.data)))
        .collect()
}

/// The stream of the teller of `room`.
fn stream(room: &str) -> StreamName {
    StreamName::Session {
        machine: "tellers".to_string(),
        key: room.to_string(),
    }
}

/// D1: what a run said is appended by the write that keeps the state it left, to the stream of
/// the session that said it, the machine's and the key's (D2). Red if the pool's write of a
/// checkpoint carries no journal.
#[test]
fn a_fact_is_appended_by_the_write_that_keeps_the_state_that_said_it() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    let pool = tellers(Arc::new(store.clone()), &clock);
    for (n, (room, fact)) in [("r1", "opened"), ("r2", "booked"), ("r1", "closed")]
        .into_iter()
        .enumerate()
    {
        let key = pool
            .receive("client", told(room, "say", fact, n as u32 + 1))
            .expect("placed in a session");
        pool.run(&key);
    }
    assert_eq!(
        entries(&store),
        [
            (stream("r1"), "opened".to_string()),
            (stream("r2"), "booked".to_string()),
            (stream("r1"), "closed".to_string()),
        ]
    );
    let loaded = checkpoint::load(&store, "r1")
        .expect("read")
        .expect("saved");
    assert_eq!(said_of(&loaded.checkpoint.state), Ok(2));
}

/// D1: a fact whose write the store failed is not lost and not said twice: the session holds it
/// with its state, and the next write that keeps the state appends it, before what came after.
/// Red if the pool lets a fact go when its write fails, or appends it again after one landed.
#[test]
fn a_fact_whose_write_failed_is_appended_once_by_the_next() {
    let clock = Clock::new();
    let store = Arc::new(Flaky {
        inner: MemoryStore::new(),
        failing: AtomicBool::new(true),
    });
    let pool = tellers(Arc::clone(&store) as Arc<dyn SessionStore>, &clock);
    let key = pool
        .receive("client", told("r1", "say", "first", 1))
        .expect("placed");
    pool.run(&key);
    assert!(
        entries(&store.inner).is_empty(),
        "a write that failed kept a fact"
    );
    store.failing.store(false, Ordering::SeqCst);
    let key = pool
        .receive("client", told("r1", "say", "second", 2))
        .expect("placed");
    pool.run(&key);
    let key = pool
        .receive("client", told("r1", "say", "third", 3))
        .expect("placed");
    pool.run(&key);
    assert_eq!(
        entries(&store.inner),
        [
            (stream("r1"), "first".to_string()),
            (stream("r1"), "second".to_string()),
            (stream("r1"), "third".to_string()),
        ]
    );
}

/// D1: a run that ends its session appends what it said by the write that removes it, and so does
/// one of a key the store never held, which would otherwise cost the store nothing. Red if the
/// pool skips the write of a session that ends in its first run though it said a fact.
#[test]
fn a_fact_said_by_a_run_that_ends_the_session_is_appended_by_its_removal() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    let pool = tellers(Arc::new(store.clone()), &clock);
    pool.receive("client", told("r1", "say", "last words", 1))
        .expect("placed");
    let key = pool
        .receive("client", told("r1", "close", "", 2))
        .expect("placed");
    pool.run(&key);
    assert_eq!(store.get("r1").expect("read"), None, "the session ended");
    assert_eq!(entries(&store), [(stream("r1"), "last words".to_string())]);
}

/// D2: a pool keeps its sessions through a handle of its own machine, or one of the store's one
/// machine, never through another machine's. Red if a pool is built over a store that keeps
/// another machine's sessions.
#[test]
fn a_pool_refuses_a_store_that_keeps_another_machines_sessions() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    let refused = Pool::new(
        TELLERS,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Tellers,
        Arc::new(store.of_machine("rooms")),
        Wire::default(),
        clock.clone(),
    );
    assert!(
        matches!(
            refused,
            Err(PoolConfigError::StoreOfAnotherMachine { ref machine, ref store })
                if machine == "tellers" && store == "rooms"
        ),
        "a pool over another machine's sessions"
    );
    let pool = tellers(Arc::new(store.of_machine("tellers")), &clock);
    let key = pool
        .receive("client", told("r1", "say", "kept apart", 1))
        .expect("placed");
    pool.run(&key);
    assert!(
        store.get("r1").expect("read").is_none(),
        "kept as the one machine's"
    );
    assert!(store
        .of_machine("tellers")
        .get("r1")
        .expect("read")
        .is_some());
}

/// docs/adr/0116, D7: a fact the engine recorded as said for a person is given to the write as
/// said for them, the instance and the whole person, and one said for nobody as said for nobody.
/// Red if the pool's write leaves who a fact was said for out.
#[test]
fn a_fact_said_for_a_person_is_written_as_said_for_them() {
    use sce_rust_session_store::{ActorName, Journal, Person};

    let alice = sce_rust_runtime::Principal {
        issuer: "https://id.example".to_string(),
        subject: "alice".to_string(),
        proved_ms: 1_700_000_000_000,
        method: "pwd".to_string(),
        instance: "a1".to_string(),
    };
    let said = |principal: Option<sce_rust_runtime::Principal>| JournalEntry {
        schema: SAID,
        revision: 1,
        data: b"fact".to_vec(),
        valid_ms: None,
        principal: principal.map(Box::new),
    };
    let entries = [said(Some(alice)), said(None)].map(|entry| Said {
        entry,
        read: Vec::new(),
    });
    let store = MemoryStore::new();
    let lease = store
        .acquire("r1", "p1", LEASE_TTL_MS, 0)
        .expect("a lease on a key no one holds");
    let facts = facts_of(&entries);
    store
        .write(
            "r1",
            &Write {
                journal: Some(Journal {
                    machine: "tellers",
                    facts: &facts,
                }),
                now_ms: 1_000,
                ..Write::put(b"s", None, 0, lease.fence)
            },
        )
        .expect("a write that appends");
    let actors: Vec<Option<ActorName>> = store
        .after(0, 10)
        .expect("a read of the journal")
        .entries
        .into_iter()
        .map(|entry| entry.actor)
        .collect();
    assert_eq!(
        actors,
        [
            Some(ActorName {
                instance: "a1".to_string(),
                person: Person {
                    issuer: "https://id.example".to_string(),
                    subject: "alice".to_string(),
                    proved_ms: 1_700_000_000_000,
                    method: "pwd".to_string(),
                },
            }),
            None,
        ]
    );
}

/// The tellers, bound to the instances of the open machine [`PHONE`] as well as to the client.
const OPEN_TELLERS: Machine = Machine {
    name: "tellers",
    open_peers: &[PHONE],
    ..ROOMS
};

/// What the instance `instance` of `phone` tells the room `room`, numbered `number` in its
/// incarnation, under the envelope id `id(0x55, n)`: `say`, saying `fact`.
fn said_by(instance: &str, room: &str, fact: &str, n: u32, number: u32) -> Vec<u8> {
    let data = alloc::format!(r#"{{"room":"{room}","fact":"{fact}"}}"#);
    Envelope {
        id: (&id(0x55, n)).into(),
        source: within("phone").unwrap(),
        instance: Some(within(instance).unwrap()),
        event_type: within("say").unwrap(),
        pattern: PatternKind::FireForget,
        datacontenttype: PayloadCodec::Json,
        data: within(data.as_bytes()).unwrap(),
        sequence_no: Some(u64::from(number)),
        ..Envelope::new()
    }
    .encode_to_vec()
}

/// docs/adr/0113, D5, with docs/adr/0114, D1: a run made under an incarnation that ended keeps
/// nothing, the facts it said among it. The session is let go and loaded again from what the
/// store keeps, whose state said none of them, so the next write appends only what the next run
/// said (`Pool::let_go`). Red if a fenced session keeps what its refused write carried: the
/// fact of the ended incarnation is then appended by the next write.
#[test]
fn a_fact_said_under_an_ended_incarnation_is_dropped_with_its_run() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    let pool = Pool::with_members(
        OPEN_TELLERS,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Tellers,
        Arc::new(store.clone()),
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    let first = joined(&store, "phone/a1", "n1", &clock);
    let under_first = Standing {
        incarnation: first,
        mark: 1,
    };
    let (key, kept) = pool.receive_member(
        "phone/a1",
        first,
        under_first,
        said_by("a1", "r1", "kept", 1, 1),
    );
    pool.run(key.as_deref().expect("placed"));
    assert_eq!(kept.fate(), Fate::Settled, "{:?}", pool.take_errors());

    // The instance joins again, ending its first incarnation, while what it said under the first
    // waits for its session.
    let second = joined(&store, "phone/a1", "n2", &clock);
    let (_, dropped) = pool.receive_member(
        "phone/a1",
        first,
        under_first,
        said_by("a1", "r1", "dropped", 2, 2),
    );
    pool.run("r1");
    assert_eq!(dropped.fate(), Fate::Lost);
    assert!(pool
        .take_errors()
        .iter()
        .any(|error| matches!(error, PoolError::Fenced { key, .. } if key == "r1")));

    let (_, after) = pool.receive_member(
        "phone/a1",
        second,
        Standing {
            incarnation: second,
            mark: 1,
        },
        said_by("a1", "r1", "after", 3, 1),
    );
    pool.run("r1");
    assert_eq!(after.fate(), Fate::Settled, "{:?}", pool.take_errors());
    assert_eq!(
        entries(&store),
        [
            (stream("r1"), "kept".to_string()),
            (stream("r1"), "after".to_string()),
        ]
    );
    let loaded = checkpoint::load(&store, "r1")
        .expect("read")
        .expect("saved");
    assert_eq!(
        said_of(&loaded.checkpoint.state),
        Ok(2),
        "the state that said them"
    );
}

/// docs/adr/0113, D4, with docs/adr/0114, D2: a pool of a machine bound to an open machine keeps
/// its sessions and its members through a handle of its own machine, as any pool does, and
/// refuses one that keeps another machine's. Red if `Pool::with_members` skips the check of the
/// handle's machine that `Pool::new` makes.
#[test]
fn a_pool_with_members_refuses_a_store_that_keeps_another_machines_sessions() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    let config = || PoolConfig {
        owner: "p1".to_string(),
        lease_ttl_ms: LEASE_TTL_MS,
        renew_every_ms: 3_000,
    };
    let refused = Pool::with_members(
        OPEN_TELLERS,
        config(),
        Tellers,
        Arc::new(store.of_machine("rooms")),
        Wire::default(),
        clock.clone(),
    );
    assert!(
        matches!(
            refused,
            Err(PoolConfigError::StoreOfAnotherMachine { ref machine, ref store })
                if machine == "tellers" && store == "rooms"
        ),
        "a pool with members over another machine's sessions"
    );
    assert!(Pool::with_members(
        OPEN_TELLERS,
        config(),
        Tellers,
        Arc::new(store.of_machine("tellers")),
        Wire::default(),
        clock,
    )
    .is_ok());
}
