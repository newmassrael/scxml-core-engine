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
