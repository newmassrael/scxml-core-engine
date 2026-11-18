// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the SQLite store keeps beyond the contract every store keeps: what it wrote is there when
//! the file is opened again, its numbers go on from where they stopped, a write that fails part of
//! the way through leaves nothing of itself, and a file that is not its own is not taken for one.

mod common;

use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};
use sce_rust_session_store::{
    AcquireError, Due, Held, SessionStore, SqliteStore, Stored, WriteError, LATEST_MOMENT,
};

use common::Scratch;

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
        .pragma_update(None, "user_version", 2)
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
