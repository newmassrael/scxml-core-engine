// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A pool that follows a journal (docs/adr/0114, D4): each entry reaches the session its key
//! names, in order, once, whatever is handed again; a reader that cannot hand an entry holds it.

use super::journal::{fact_of, tellers, told, SAID, SAID_RECORD};
use super::*;
use crate::peers::{FollowedEntry, Follows};
use sce_rust_session_store::{Fact, FactData, JournalStore};

/// A view of the tellers: for each room, the facts it took in, in the order taken, and the origin
/// of the first.
struct Viewer {
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
struct Viewers {
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
fn views(
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
fn say(store: &MemoryStore, clock: &Clock, said: &[(&str, &str)]) {
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
fn view_of(store: &MemoryStore, room: &str) -> (Vec<String>, String, Option<u64>) {
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
