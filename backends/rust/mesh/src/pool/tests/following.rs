// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A pool that follows a journal (docs/adr/0114, D4): each entry reaches the session its key
//! names, in order, once, whatever is handed again; a reader that cannot hand an entry holds it.

use super::journal::{fact_of, tellers, told, SAID, SAID_RECORD};
use super::*;
use crate::peers::{FollowedEntry, Follows, KeyFrom};
use sce_rust_session_store::{Fact, FactData, JournalStore};

/// A view of the tellers: for each room, the facts it took in, in the order taken, and the origin
/// of the first.
pub(super) struct Viewer {
    taken: Vec<String>,
    first_origin: String,
}

impl SessionMachine for Viewer {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if event.name != SAID {
                continue;
            }
            stepped = true;
            let fact = match json::parse(&event.metadata.data)
                .ok()
                .and_then(|data| data.member("fact").cloned())
            {
                Some(Value::Text(fact)) => fact,
                other => panic!("an entry's data holds its fact: {other:?}"),
            };
            if self.taken.is_empty() {
                self.first_origin = event.metadata.origin_address().to_string();
            }
            self.taken.push(fact);
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "viewer".to_string(),
            configuration: vec!["viewing".to_string()],
            current: "viewing".to_string(),
            variables: vec![
                (
                    "taken".to_string(),
                    Value::Array(self.taken.iter().cloned().map(Value::Text).collect()),
                ),
                (
                    "first_origin".to_string(),
                    Value::Text(self.first_origin.clone()),
                ),
            ],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "viewer".to_string(),
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
}

/// What a saved viewer took in, and the origin of the first.
fn viewed(state: &SavedState) -> Result<(Vec<String>, String), String> {
    let taken = match state.variable("taken") {
        Ok(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Text(fact) => Ok(fact.clone()),
                other => Err(alloc::format!("a fact that is {other:?}")),
            })
            .collect::<Result<Vec<_>, _>>()?,
        other => return Err(alloc::format!("what it took is {other:?}")),
    };
    let first_origin = match state.variable("first_origin") {
        Ok(Value::Text(origin)) => origin.clone(),
        other => return Err(alloc::format!("its first origin is {other:?}")),
    };
    Ok((taken, first_origin))
}

/// The factory of viewers, counting the machines it builds.
#[derive(Default)]
pub(super) struct Viewers {
    built: AtomicU64,
}

impl SessionFactory<Wire, Clock> for Viewers {
    type Machine = Viewer;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Viewer, String> {
        self.built.fetch_add(1, Ordering::SeqCst);
        Ok(Viewer {
            taken: Vec::new(),
            first_origin: String::new(),
        })
    }

    fn restore(
        &self,
        _key: &str,
        state: &SavedState,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Viewer, String> {
        self.built.fetch_add(1, Ordering::SeqCst);
        let (taken, first_origin) = viewed(state)?;
        Ok(Viewer {
            taken,
            first_origin,
        })
    }
}

/// What the views follow: the tellers' facts, and the observed stream `directory`.
const FOLLOWS: Follows = Follows {
    journals: &["tellers", "directory"],
    entries: &[FollowedEntry {
        event: SAID,
        record: SAID_RECORD,
    }],
};

/// The views, keyed by the `room` field of the entries they follow, holding at most `max_inbox`
/// inputs in one session's inbox.
fn views_machine(max_inbox: u32) -> Machine {
    Machine {
        name: "views",
        sessions: Some(Sessions {
            max_inbox,
            follows: FOLLOWS,
            ..ROOMS_SESSIONS
        }),
        ..ROOMS
    }
}

/// A pool of the views as the process `owner` serves it, its sessions kept through the views'
/// handle on `store`, reading the journal of `store`.
pub(super) fn views(
    owner: &str,
    store: &MemoryStore,
    clock: &Clock,
    max_inbox: u32,
) -> Pool<Viewers, Wire, Clock> {
    Pool::new(
        views_machine(max_inbox),
        PoolConfig {
            owner: owner.to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Viewers::default(),
        Arc::new(store.of_machine("views")),
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"))
    .follow_from(Arc::new(store.clone()))
}

/// Make the tellers of `store` say `said`, each a room and a fact, in order.
pub(super) fn say(store: &MemoryStore, clock: &Clock, said: &[(&str, &str)]) {
    let pool = tellers(Arc::new(store.of_machine("tellers")), clock);
    for (n, (room, fact)) in said.iter().enumerate() {
        let key = pool
            .receive("client", told(room, "say", fact, n as u32 + 1))
            .expect("placed in a session");
        pool.run(&key);
    }
    assert!(pool.take_errors().is_empty());
}

/// Follow until the reader hands nothing more, running each session it names.
fn follow_all(pool: &Pool<Viewers, Wire, Clock>) {
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

/// What the durable checkpoint of the view of `room` holds: the facts it took, the origin of the
/// first, and the position of the tellers' journal it took in.
pub(super) fn view_of(store: &MemoryStore, room: &str) -> (Vec<String>, String, Option<u64>) {
    let loaded = checkpoint::load(&store.of_machine("views"), room)
        .expect("read")
        .expect("a view saved");
    let (taken, origin) = viewed(&loaded.checkpoint.state).expect("a view");
    (
        taken,
        origin,
        loaded.checkpoint.followed.get("tellers").copied(),
    )
}

/// D4: each entry of the followed journal reaches the session its key field names, in the order
/// of the positions, as the event of its schema with the record's data and the origin
/// `journal://<machine>/<key>/<position>`, and the position is written with what it did; once
/// every entry was taken in, the reader's mark is the last one. Red if an entry reaches another
/// session, out of order, or the position is not kept.
#[test]
fn a_follower_takes_each_entry_of_its_key_in_order() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r2", "b"), ("r1", "c")]);
    let pool = views("p1", &store, &clock, 64);
    follow_all(&pool);
    assert_eq!(
        view_of(&store, "r1"),
        (
            vec!["a".to_string(), "c".to_string()],
            "journal://tellers/r1/1".to_string(),
            Some(3)
        )
    );
    assert_eq!(
        view_of(&store, "r2"),
        (
            vec!["b".to_string()],
            "journal://tellers/r2/2".to_string(),
            Some(2)
        )
    );
    assert_eq!(store.read_through("views").expect("the mark"), 3);
    assert!(pool.take_errors().is_empty());
}

/// D4: a pool that starts again, its mark not yet kept, hands every entry again, and each session
/// drops what its position says it took: nothing takes effect twice, and nothing is missed. Red if
/// a session takes in an entry at or below its position.
#[test]
fn a_follower_that_starts_again_takes_nothing_twice() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r2", "b"), ("r1", "c")]);
    {
        let pool = views("p1", &store, &clock, 64);
        for key in pool.follow() {
            pool.run(&key);
        }
        // Stopped here, as a killed process stops: the mark was not kept.
    }
    assert_eq!(store.read_through("views").expect("the mark"), 0);
    say(&store, &clock, &[("r2", "d")]);
    let pool = views("p1", &store, &clock, 64);
    follow_all(&pool);
    assert_eq!(view_of(&store, "r1").0, ["a", "c"]);
    assert_eq!(view_of(&store, "r2").0, ["b", "d"]);
    assert_eq!(store.read_through("views").expect("the mark"), 4);
}

/// D4: an entry handed to a session that is dropped before a durable checkpoint holds it (here,
/// its key leased to another process) is handed again, before any later entry of its key, once
/// the key is this process's to serve; the entries of other keys go on meanwhile, and the mark
/// stays below it. Red if a lost entry is passed, or a later one of its key overtakes it.
#[test]
fn an_entry_lost_with_its_session_is_handed_again_first() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r2", "b"), ("r1", "c")]);
    let held = store
        .of_machine("views")
        .acquire("r1", "another", 1_000, clock.at())
        .expect("another process's lease");
    let pool = views("p1", &store, &clock, 64);
    for key in pool.follow() {
        pool.run(&key);
    }
    assert!(pool.follow().is_empty(), "r1 is held by another process");
    assert_eq!(view_of(&store, "r2").0, ["b"]);
    assert!(
        checkpoint::load(&store.of_machine("views"), "r1")
            .expect("read")
            .is_none(),
        "r1 was served here"
    );
    assert_eq!(store.read_through("views").expect("the mark"), 0);
    store
        .of_machine("views")
        .release("r1", held.fence)
        .expect("the other process lets it go");
    clock.advance(2_000);
    follow_all(&pool);
    assert_eq!(view_of(&store, "r1").0, ["a", "c"]);
    assert_eq!(store.read_through("views").expect("the mark"), 3);
}

/// D4: an entry of a key another process serves is that process's to hand: once its stored
/// checkpoint holds the entry's position, this one passes it, and its mark rises past it, though
/// it never served the key. Red if a reader waits for an entry another process took in.
#[test]
fn an_entry_another_process_took_in_is_passed() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r1", "c")]);
    let pool = views("p1", &store, &clock, 64);
    let handed = pool.follow();
    assert_eq!(handed, ["r1"]);
    // Another process serves r1 first, and keeps no mark of its own yet.
    let other = views("p2", &store, &clock, 64);
    for key in other.follow() {
        other.run(&key);
    }
    assert_eq!(view_of(&store, "r1").0, ["a", "c"]);
    assert_eq!(store.read_through("views").expect("the mark"), 0);
    for key in handed {
        pool.run(&key);
    }
    assert!(!pool.is_live("r1"), "the other process serves r1");
    assert!(pool.follow().is_empty(), "nothing is handed again");
    assert_eq!(
        store.read_through("views").expect("the mark"),
        2,
        "the reader passed what the other process took in"
    );
    assert_eq!(view_of(&store, "r1").0, ["a", "c"]);
}

/// D4: a session whose inbox holds `sessions.max_inbox` inputs holds the reader at its next
/// entry; the entries reach it as it takes them, all of them, in order. Red if an entry past the
/// bound is dropped.
#[test]
fn a_follower_at_its_inbox_bound_holds_the_reader_and_drops_nothing() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r1", "b"), ("r1", "c")]);
    let pool = views("p1", &store, &clock, 1);
    assert_eq!(pool.follow(), ["r1"]);
    assert_eq!(
        store.read_through("views").expect("the mark"),
        0,
        "nothing was taken in yet"
    );
    pool.run("r1");
    follow_all(&pool);
    assert_eq!(view_of(&store, "r1").0, ["a", "b", "c"]);
    assert_eq!(store.read_through("views").expect("the mark"), 3);
}

/// D4: an entry no session can be handed (here, an observed entry of a schema the views follow
/// no kind of) is told to the host and passed; the entries after it go on. Red if the reader stops
/// at it, or hands it.
#[test]
fn an_entry_no_session_can_take_is_told_and_passed() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    store
        .observe(
            "directory",
            "tx-1",
            &Fact {
                schema: "directory.moved",
                revision: 1,
                data: FactData::Bytes(b"elsewhere"),
                valid_ms: None,
                actor: None,
                read: &[],
            },
            clock.at(),
        )
        .expect("an observed entry");
    say(&store, &clock, &[("r1", "a")]);
    let pool = views("p1", &store, &clock, 64);
    follow_all(&pool);
    assert_eq!(view_of(&store, "r1").0, ["a"]);
    let errors = pool.take_errors();
    assert!(
        matches!(
            errors.as_slice(),
            [PoolError::Unfollowed { journal, position: 1, .. }] if journal == "directory"
        ),
        "{errors:?}"
    );
    assert_eq!(store.read_through("views").expect("the mark"), 2);
    assert_eq!(
        fact_of(&store.after(1, 1).expect("read").entries[0].data),
        "a"
    );
}

/// The schema of what a clerk notes.
const NOTED: &str = "clerk.noted";

/// The description of the record of [`NOTED`]: the note, a field no teller's fact has.
const NOTED_RECORD: &str = r#"{"record":[["note",false,{"text":32},null]]}"#;

/// The schema of an entry of the observed stream `directory` the ledger follows.
const MOVED: &str = "directory.moved";

/// The description of the record of [`MOVED`]: the place, a field neither a teller's fact nor a
/// clerk's note has.
const MOVED_RECORD: &str = r#"{"record":[["place",false,{"text":32},null]]}"#;

/// A clerk: one session for the whole machine, which notes a fact for each `note` it is told, the
/// fact the text of its data's `note`. Written by hand in the shape a generated machine has to the
/// pool, as the rooms are.
struct Clerk {
    fresh: Vec<JournalEntry>,
}

impl SessionMachine for Clerk {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if event.name != "note" {
                continue;
            }
            stepped = true;
            let data = sce_rust_runtime::record_cbor::encode(&event.metadata.data, NOTED_RECORD)
                .unwrap_or_else(|refusal| panic!("a note of its record: {refusal:?}"));
            self.fresh.push(JournalEntry {
                schema: NOTED,
                revision: 1,
                data,
                valid_ms: None,
                principal: event.principal.clone(),
            });
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "clerk".to_string(),
            configuration: vec!["noting".to_string()],
            current: "noting".to_string(),
            variables: Vec::new(),
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "clerk".to_string(),
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

/// The factory of clerks.
struct Clerks;

impl SessionFactory<Wire, Clock> for Clerks {
    type Machine = Clerk;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Clerk, String> {
        Ok(Clerk { fresh: Vec::new() })
    }

    fn restore(
        &self,
        _key: &str,
        _state: &SavedState,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Clerk, String> {
        Ok(Clerk { fresh: Vec::new() })
    }
}

/// How a pool that keeps one session is pooled (deploy.yaml `sessions: { key_from: none }`,
/// docs/adr/0114, D4): no key is read, and its one key, the empty key, holds no byte.
const ONE_SESSION: Sessions = Sessions {
    key_from: KeyFrom::One,
    max_key_bytes: 0,
    ..ROOMS_SESSIONS
};

/// The clerks: one session for the whole machine, which keeps a journal.
const CLERKS: Machine = Machine {
    name: "clerks",
    sessions: Some(ONE_SESSION),
    ..ROOMS
};

/// Make the clerks of `store` note `noted`, each the number of the envelope that carries it and the
/// note, in order: the clerks' one session holds the envelopes it took, so a number is never
/// carried twice.
fn note(store: &MemoryStore, clock: &Clock, noted: &[(u32, &str)]) {
    let pool = Pool::new(
        CLERKS,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Clerks,
        Arc::new(store.of_machine("clerks")),
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"));
    pool.peer_ready("client");
    for (n, text) in noted {
        let key = pool
            .receive(
                "client",
                from_client("note", &alloc::format!(r#"{{"note":"{text}"}}"#), *n, false),
            )
            .expect("placed in the one session");
        assert_eq!(key, "", "the one session's key is the empty key");
        pool.run(&key);
    }
    assert!(pool.take_errors().is_empty());
}

/// Make the other system say, into the observed stream `directory` under its id `origin`, that
/// something moved to `place`.
fn moved(store: &MemoryStore, clock: &Clock, origin: &str, place: &str) {
    let data = sce_rust_runtime::record_cbor::encode(
        &alloc::format!(r#"{{"place":"{place}"}}"#),
        MOVED_RECORD,
    )
    .unwrap_or_else(|refusal| panic!("a move of its record: {refusal:?}"));
    store
        .observe(
            "directory",
            origin,
            &Fact {
                schema: MOVED,
                revision: 1,
                data: FactData::Bytes(&data),
                valid_ms: None,
                actor: None,
                read: &[],
            },
            clock.at(),
        )
        .expect("an observed entry");
}

/// A ledger of the whole deployment: one session that takes in every entry it follows and every
/// envelope a client sends, and keeps what it took in the order taken, each entry by its event and
/// origin and each envelope by its event.
struct Overview {
    taken: Vec<String>,
}

impl SessionMachine for Overview {
    fn run(&mut self, calls: Vec<EngineCall>, _wall_now_ms: u64) -> bool {
        let mut stepped = false;
        for call in calls {
            let EngineCall::Raise(event) = call else {
                continue;
            };
            if ![SAID, NOTED, MOVED, "ask"].contains(&event.name.as_str()) {
                continue;
            }
            stepped = true;
            let taken = match &event.metadata.origin {
                Some(origin) if origin.processor == crate::pool::JOURNAL_ORIGIN_TYPE => {
                    alloc::format!("{} {}", event.name, origin.address)
                }
                _ => event.name.clone(),
            };
            self.taken.push(taken);
        }
        stepped
    }

    fn save(&self, _wall_now_ms: u64) -> Result<SavedState, String> {
        Ok(SavedState {
            shape: "overview".to_string(),
            configuration: vec!["overseeing".to_string()],
            current: "overseeing".to_string(),
            variables: vec![(
                "taken".to_string(),
                Value::Array(self.taken.iter().cloned().map(Value::Text).collect()),
            )],
            history: Vec::new(),
            pending: Vec::new(),
            invokes: SavedInvokes::Resumed(Vec::new()),
            session_id: "overview".to_string(),
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
}

/// What a saved overview took, in the order taken.
fn overseen(state: &SavedState) -> Result<Vec<String>, String> {
    match state.variable("taken") {
        Ok(Value::Array(items)) => items
            .iter()
            .map(|item| match item {
                Value::Text(taken) => Ok(taken.clone()),
                other => Err(alloc::format!("a taken input that is {other:?}")),
            })
            .collect(),
        other => Err(alloc::format!("what it took is {other:?}")),
    }
}

/// The factory of overviews.
struct Overviews;

impl SessionFactory<Wire, Clock> for Overviews {
    type Machine = Overview;

    fn start(
        &self,
        _key: &str,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Overview, String> {
        Ok(Overview { taken: Vec::new() })
    }

    fn restore(
        &self,
        _key: &str,
        state: &SavedState,
        _endpoint: &SessionEndpoint<Wire, Clock>,
        _wall_now_ms: u64,
    ) -> Result<Overview, String> {
        Ok(Overview {
            taken: overseen(state)?,
        })
    }
}

/// What the ledger follows: the tellers, the clerks and the observed stream `directory`, whose
/// entries share no field.
const LEDGER_FOLLOWS: Follows = Follows {
    journals: &["tellers", "clerks", "directory"],
    entries: &[
        FollowedEntry {
            event: SAID,
            record: SAID_RECORD,
        },
        FollowedEntry {
            event: NOTED,
            record: NOTED_RECORD,
        },
        FollowedEntry {
            event: MOVED,
            record: MOVED_RECORD,
        },
    ],
};

/// The ledger: one session (`key_from: none`) that follows [`LEDGER_FOLLOWS`].
const LEDGER: Machine = Machine {
    name: "ledger",
    sessions: Some(Sessions {
        follows: LEDGER_FOLLOWS,
        ..ONE_SESSION
    }),
    ..ROOMS
};

/// A pool of the [`LEDGER`] as the process `p1` serves it, its session kept through the ledger's
/// handle on `store`, reading the journal of `store`.
fn ledger(store: &MemoryStore, clock: &Clock) -> Pool<Overviews, Wire, Clock> {
    let pool = Pool::new(
        LEDGER,
        PoolConfig {
            owner: "p1".to_string(),
            lease_ttl_ms: LEASE_TTL_MS,
            renew_every_ms: 3_000,
        },
        Overviews,
        Arc::new(store.of_machine("ledger")),
        Wire::default(),
        clock.clone(),
    )
    .unwrap_or_else(|error| panic!("a pooled machine: {error:?}"))
    .follow_from(Arc::new(store.clone()));
    pool.peer_ready("client");
    pool
}

/// Follow until the ledger's reader hands nothing more, running the one session it names.
fn follow_ledger(pool: &Pool<Overviews, Wire, Clock>) {
    for _ in 0..64 {
        let keys = pool.follow();
        if keys.is_empty() {
            return;
        }
        assert_eq!(
            keys,
            [""],
            "the one session is the only one handed an entry"
        );
        pool.run("");
    }
    panic!("the reader never stopped handing entries");
}

/// What the durable checkpoint of the ledger's one session holds: what it took, in order, and the
/// position it took in of each journal it follows.
fn ledger_of(store: &MemoryStore) -> (Vec<String>, BTreeMap<String, u64>) {
    let loaded = checkpoint::load(&store.of_machine("ledger"), "")
        .expect("read")
        .expect("the one session saved under the empty key");
    (
        overseen(&loaded.checkpoint.state).expect("an overview"),
        loaded.checkpoint.followed,
    )
}

/// docs/adr/0114, D4 (amended 2026-11-28): a pool that keeps one session (`key_from: none`) hands
/// every entry of each journal and observed stream it follows to that one session, whose key is
/// the empty key, reading no field of the entries' data (here they share none): each once, in the
/// order of the positions, so in each journal's own order, with the origin
/// `journal://<machine>//<position>` for an entry of another machine's one session; it keeps one
/// position for each journal, and a pool that starts again, its mark not kept, takes nothing
/// twice. What a client sends reaches the same session. Red if an entry is keyed by a field of its
/// data (none of them has `room` in common, so the clerks' and the directory's would be told to
/// the host and passed), if the empty key is refused as a key read from an envelope is, or if the
/// one session takes an entry again after the restart.
#[test]
fn a_follower_that_keeps_one_session_takes_every_entry_once_in_order() {
    let (store, clock) = (MemoryStore::new(), Clock::new());
    say(&store, &clock, &[("r1", "a"), ("r2", "b")]);
    note(&store, &clock, &[(1, "x")]);
    moved(&store, &clock, "m-1", "north");
    say(&store, &clock, &[("r3", "c")]);
    {
        let pool = ledger(&store, &clock);
        for key in pool.follow() {
            pool.run(&key);
        }
        assert!(pool.take_errors().is_empty());
        // Stopped here, as a killed process stops: the mark was not kept.
    }
    assert_eq!(store.read_through("ledger").expect("the mark"), 0);
    let before: Vec<String> = [
        "teller.said journal://tellers/r1/1",
        "teller.said journal://tellers/r2/2",
        "clerk.noted journal://clerks//3",
        "directory.moved journal://directory/4",
        "teller.said journal://tellers/r3/5",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    assert_eq!(
        ledger_of(&store),
        (
            before.clone(),
            BTreeMap::from([
                ("clerks".to_string(), 3),
                ("directory".to_string(), 4),
                ("tellers".to_string(), 5),
            ])
        )
    );

    note(&store, &clock, &[(2, "y")]);
    moved(&store, &clock, "m-2", "south");
    say(&store, &clock, &[("r4", "d")]);
    let pool = ledger(&store, &clock);
    follow_ledger(&pool);
    let mut after = before;
    after.extend(
        [
            "clerk.noted journal://clerks//6",
            "directory.moved journal://directory/7",
            "teller.said journal://tellers/r4/8",
        ]
        .into_iter()
        .map(String::from),
    );
    assert_eq!(
        ledger_of(&store),
        (
            after.clone(),
            BTreeMap::from([
                ("clerks".to_string(), 6),
                ("directory".to_string(), 7),
                ("tellers".to_string(), 8),
            ])
        )
    );
    assert_eq!(store.read_through("ledger").expect("the mark"), 8);

    // What a client sends carries no key, and reaches the one session that took the entries.
    let key = pool
        .receive("client", from_client("ask", r#"{"what":"all"}"#, 1, false))
        .expect("placed in the one session");
    assert_eq!(key, "");
    pool.run(&key);
    after.push("ask".to_string());
    assert_eq!(ledger_of(&store).0, after);
    assert!(pool.take_errors().is_empty());
}
