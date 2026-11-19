// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the SQLite store keeps beyond the contract every store keeps: what it wrote is there when
//! the file is opened again, its numbers go on from where they stopped, a write that fails part of
//! the way through leaves nothing of itself, and a file that is not its own is not taken for one.

mod common;

use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};
use sce_rust_session_store::{
    AcquireError, Due, Held, KeyRow, KeyState, KeyStore, KeyUse, SessionStore, SqliteStore, Stored,
    WriteError, LATEST_MOMENT,
};

use common::Scratch;

/// The tables a build of layout 1 made, as it made them, with one session in them.
const LAYOUT_1_WITH_A_SESSION: &str = "
    CREATE TABLE sessions (
        session TEXT PRIMARY KEY NOT NULL,
        state BLOB NOT NULL,
        version INTEGER NOT NULL,
        earliest_due INTEGER
    );
    CREATE INDEX sessions_by_due ON sessions (earliest_due, session)
        WHERE earliest_due IS NOT NULL;
    CREATE TABLE leases (
        session TEXT PRIMARY KEY NOT NULL,
        owner TEXT NOT NULL,
        fence INTEGER NOT NULL,
        expires_ms INTEGER NOT NULL
    );
    CREATE TABLE numbers (
        id INTEGER PRIMARY KEY CHECK (id = 0),
        issued INTEGER NOT NULL
    );
    INSERT INTO numbers (id, issued) VALUES (0, 1);
    INSERT INTO sessions (session, state, version, earliest_due) VALUES ('k', X'6f6c64', 1, NULL);
    PRAGMA user_version = 1;
";

/// ADR 0109, Open question 1: a file of layout 1 is brought to layout 2 when it is opened, its
/// sessions and its counter kept, and the keys of layout 2 are then kept in it.
#[test]
fn a_file_of_layout_1_is_brought_to_layout_2_with_its_sessions() {
    let scratch = Scratch::new("layout-1");
    Connection::open(scratch.database())
        .expect("a database of a build of layout 1")
        .execute_batch(LAYOUT_1_WITH_A_SESSION)
        .expect("its tables");
    let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 1");
    assert_eq!(
        store.get("k").expect("its session"),
        Some(Stored {
            state: b"old".to_vec(),
            version: 1,
            earliest_due: None,
        })
    );
    let layout: i64 = Connection::open(scratch.database())
        .expect("a second connection")
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("its layout");
    assert_eq!(layout, 2, "the file was brought to layout 2");
    let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
    assert!(
        lease.fence > 1,
        "the counter goes on from the one layout 1 kept"
    );
    assert!(store
        .ids()
        .expect("the keys of an upgraded file")
        .is_empty());
}

/// The keys kept in the file are there when it is opened again, their generations and the naming
/// key with them (ADR 0109, D7).
#[test]
fn the_keys_a_store_kept_are_there_when_its_file_is_opened_again() {
    let scratch = Scratch::new("keys-reopen");
    let kept = KeyRow {
        id: "generated:00".to_string(),
        key_use: KeyUse::Sign,
        algorithm: "EdDSA".to_string(),
        wrapped: vec![0xAB; 57],
        label: "device".to_string(),
        created_ms: 1_700_000_000_000,
        state: KeyState::Active,
    };
    {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        store
            .insert(&kept, Some("generation"))
            .expect("keeping a key");
        store.keep_naming_key(&[0xCD; 57]).expect("a naming key");
        store.retire("generated:00").expect("retiring it");
    }
    let store = SqliteStore::open(scratch.database()).expect("opening the store again");
    assert_eq!(
        store.key("generated:00").expect("the key"),
        Some(KeyRow {
            state: KeyState::Retired,
            ..kept
        })
    );
    assert_eq!(
        store.generated_by("generation").expect("its generation"),
        Some("generated:00".to_string())
    );
    assert_eq!(
        store.naming_key().expect("the naming key"),
        Some(vec![0xCD; 57])
    );
}

#[test]
fn what_a_store_wrote_is_there_when_its_file_is_opened_again() {
    let scratch = Scratch::new("reopen");
    let (lease, version) = {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        let lease = store
            .acquire("k", "a", 1_000, 0)
            .expect("a lease on a new key");
        let version = store
            .put("k", b"saved", Some(500), 0, lease.fence)
            .expect("a first put");
        (lease, version)
    };

    let store = SqliteStore::open(scratch.database()).expect("opening the store again");
    assert_eq!(
        store.get("k").expect("a read"),
        Some(Stored {
            state: b"saved".to_vec(),
            version,
            earliest_due: Some(500),
        })
    );
    assert_eq!(
        store.due_before(LATEST_MOMENT, 10).expect("a listing"),
        vec![Due {
            key: "k".to_string(),
            due: 500
        }]
    );
    match store.acquire("k", "b", 1_000, 10) {
        Err(AcquireError::Held(held)) => assert_eq!(
            held,
            Held {
                owner: "a".to_string(),
                expires_ms: 1_000
            }
        ),
        other => panic!("the lease taken before the store was closed is not held: {other:?}"),
    }
    let next = store
        .put("k", b"again", None, version, lease.fence)
        .expect("a put under the lease taken before the store was closed");
    assert!(
        next > version && next > lease.fence,
        "the store's numbers started again: {next} after version {version} and fence {}",
        lease.fence
    );

    // WAL mode is kept in the file, so a connection that did not ask for it is in it too.
    let raw = Connection::open(scratch.database()).expect("a connection of another program");
    let mode: String = raw
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("the journal mode");
    assert_eq!(mode, "wal");
}

#[test]
fn a_write_that_fails_part_of_the_way_through_leaves_nothing_of_itself() {
    let scratch = Scratch::new("partway");
    let store = SqliteStore::open(scratch.database()).expect("making the store");
    let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
    let version = store
        .put("k", b"before", Some(100), 0, lease.fence)
        .expect("a first put");
    let before = store.get("k").expect("a read");

    // Another program makes the row's write fail after the put has checked the lease and the
    // version and taken its number from the counter: the counter's step must go with the rest.
    let raw = Connection::open(scratch.database()).expect("a connection of another program");
    raw.execute_batch(
        "CREATE TRIGGER refuse_thirteen AFTER INSERT ON sessions WHEN NEW.earliest_due = 13 \
         BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;",
    )
    .expect("the trigger");
    match store.put("k", b"during", Some(13), version, lease.fence) {
        Err(WriteError::Failed(_)) => {}
        other => panic!("a put the database refused gave {other:?}"),
    }
    assert_eq!(store.get("k").expect("a read"), before);
    assert_eq!(
        store.due_before(LATEST_MOMENT, 10).expect("a listing"),
        vec![Due {
            key: "k".to_string(),
            due: 100
        }]
    );

    raw.execute_batch("DROP TRIGGER refuse_thirteen")
        .expect("dropping the trigger");
    let after = store
        .put("k", b"after", Some(13), version, lease.fence)
        .expect("the same put once nothing refuses it");
    assert_eq!(
        after,
        version + 1,
        "the number the failed put took was not given back with the rest of it"
    );
}

#[test]
fn a_write_that_cannot_take_the_write_lock_writes_nothing() {
    let scratch = Scratch::new("locked");
    let store = SqliteStore::open_with_busy_timeout(scratch.database(), Duration::ZERO)
        .expect("making the store");
    let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
    let version = store
        .put("k", b"before", Some(100), 0, lease.fence)
        .expect("a first put");

    let mut raw = Connection::open(scratch.database()).expect("a connection of another program");
    let holding = raw
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .expect("the write lock");
    match store.put("k", b"during", Some(5), version, lease.fence) {
        Err(WriteError::Failed(_)) => {}
        other => panic!("a put while another holds the write lock gave {other:?}"),
    }
    match store.acquire("other", "a", 1_000, 0) {
        Err(AcquireError::Failed(_)) => {}
        other => panic!("a lease while another holds the write lock gave {other:?}"),
    }
    holding.rollback().expect("letting the write lock go");

    assert_eq!(
        store.get("k").expect("a read"),
        Some(Stored {
            state: b"before".to_vec(),
            version,
            earliest_due: Some(100),
        })
    );
    store
        .put("k", b"after", Some(5), version, lease.fence)
        .expect("the same put once the lock is free");
}

#[test]
fn a_file_that_is_not_a_session_store_is_not_taken_for_one() {
    let foreign = Scratch::new("foreign");
    Connection::open(foreign.database())
        .expect("a database of another program")
        .execute_batch("CREATE TABLE notes (text TEXT);")
        .expect("its table");
    assert!(
        SqliteStore::open(foreign.database()).is_err(),
        "a database with tables of another program was taken for a session store"
    );

    let later = Scratch::new("later");
    drop(SqliteStore::open(later.database()).expect("making the store"));
    Connection::open(later.database())
        .expect("a connection of a later build")
        .pragma_update(None, "user_version", 3)
        .expect("a layout of a later build");
    assert!(
        SqliteStore::open(later.database()).is_err(),
        "a database a later build laid out was read as this build's"
    );

    assert!(
        SqliteStore::open(":memory:").is_err(),
        "a database with no file, which cannot be in WAL mode, was taken for a durable store"
    );
}
