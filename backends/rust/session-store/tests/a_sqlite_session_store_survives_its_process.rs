// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! What the SQLite store keeps beyond the contract every store keeps: what it wrote is there when
//! the file is opened again, its numbers go on from where they stopped, a write that fails part of
//! the way through leaves nothing of itself, and a file that is not its own is not taken for one.

mod common;

use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};
use sce_rust_session_store::{
    AcquireError, Append, Batched, Committed, CredentialStore, Decided, Due, Held, KeyRow,
    KeyState, KeyStore, KeyUse, PinStore, SessionStore, SqliteStore, Stored, TokenRow, TokenStore,
    Write, WriteError, LATEST_MOMENT,
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

/// The layout this build writes, which a file of an earlier one is brought to when it is opened.
const LAYOUT: i64 = 7;

/// The tables layout 7 added (ADR 0109, D14), dropped from a file this build made so that it is
/// left as a build of layout 6 left it.
const WITHOUT_LAYOUT_7: &str = "DROP TABLE audit_identities; DROP TABLE audit_checkpoints; \
     DROP TABLE audit_rows; DROP TABLE audit_heads;";

/// The tables layouts 6 (ADR 0109, D13) and 7 (D14) added, dropped from a file this build made so
/// that it is left as a build of layout 5 left it: the later layout's first.
const WITHOUT_LAYOUT_6: &str = "DROP TABLE audit_identities; DROP TABLE audit_checkpoints; \
     DROP TABLE audit_rows; DROP TABLE audit_heads; \
     DROP TABLE kv_first_results; DROP TABLE kv_numbers; DROP TABLE kv_values;";

/// The tables layouts 5 (ADR 0109, D10) to 7 (D14) added, dropped from a file this build made so
/// that it is left as a build of layout 4 or earlier left it: the later layouts' first.
const WITHOUT_LAYOUT_5: &str = "DROP TABLE audit_identities; DROP TABLE audit_checkpoints; \
     DROP TABLE audit_rows; DROP TABLE audit_heads; \
     DROP TABLE kv_first_results; DROP TABLE kv_numbers; DROP TABLE kv_values; \
     DROP INDEX outbox_notes_by_session; DROP TABLE outbox_notes; \
     DROP TABLE outbox_entries; DROP TABLE outbox_heads;";

/// ADR 0109, Open question 1: a file of layout 1 is brought to this build's layout when it is
/// opened, its sessions and its counter kept, and the keys of layout 2, the credentials of
/// layouts 3 and 4 and the outboxes of layout 5 are then kept in it.
#[test]
fn a_file_of_layout_1_is_brought_to_layout_5_with_its_sessions() {
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
    assert_eq!(
        layout, LAYOUT,
        "the file was brought to this build's layout"
    );
    let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
    assert!(
        lease.fence > 1,
        "the counter goes on from the one layout 1 kept"
    );
    assert_eq!(
        store
            .outbox("phone")
            .expect("the outboxes of an upgraded file"),
        None
    );
    assert!(store
        .ids()
        .expect("the keys of an upgraded file")
        .is_empty());
    assert!(store
        .tokens()
        .expect("the tokens of an upgraded file")
        .is_empty());
}

/// ADR 0109, D3: a file of layout 2 is brought to this build's layout when it is opened, its keys
/// kept, and the credentials of layouts 3 and 4 are then kept in it.
#[test]
fn a_file_of_layout_2_is_brought_to_layout_5_with_its_keys() {
    let scratch = Scratch::new("layout-2");
    let kept = KeyRow {
        id: "generated:02".to_string(),
        key_use: KeyUse::Mac,
        algorithm: "HMAC-SHA-256".to_string(),
        wrapped: vec![0x5A; 72],
        label: "of layout 2".to_string(),
        created_ms: 1_000,
        state: KeyState::Active,
    };
    {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        store.insert(&kept, None).expect("keeping a key");
    }
    // What a build of layout 2 left: its tables and its number, and none of layout 3's or later.
    {
        let connection =
            Connection::open(scratch.database()).expect("a connection of a build of layout 2");
        connection
            .execute_batch(WITHOUT_LAYOUT_5)
            .expect("the file without layout 5's tables");
        connection
            .execute_batch(
                "DROP INDEX credential_tokens_by_name; DROP TABLE credential_tokens; \
                 DROP TABLE credential_pins; PRAGMA user_version = 2;",
            )
            .expect("the file as layout 2 left it");
    }
    let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 2");
    let layout: i64 = Connection::open(scratch.database())
        .expect("a second connection")
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("its layout");
    assert_eq!(
        layout, LAYOUT,
        "the file was brought to this build's layout"
    );
    assert_eq!(store.key("generated:02").expect("its key"), Some(kept));
    assert_eq!(
        store
            .pin("phone", &[1; 32])
            .expect("a pin in the upgraded file"),
        [1; 32]
    );
}

/// ADR 0109, D8: a file of layout 3 is brought to this build's layout when it is opened, its
/// tokens kept with no sealed bytes beside them, and a token's sealed bytes are then kept in it.
#[test]
fn a_file_of_layout_3_is_brought_to_layout_5_with_its_tokens() {
    let scratch = Scratch::new("layout-3");
    // What a build of layout 3 left: its tables, a token in them, and its number.
    {
        drop(SqliteStore::open(scratch.database()).expect("making the store"));
        let connection =
            Connection::open(scratch.database()).expect("a connection of a build of layout 3");
        connection
            .execute_batch(WITHOUT_LAYOUT_5)
            .expect("the file without layout 5's tables");
        connection
            .execute_batch(
                "DROP INDEX credential_tokens_by_name; DROP TABLE credential_tokens; \
                 CREATE TABLE credential_tokens ( \
                     digest BLOB PRIMARY KEY NOT NULL, \
                     name TEXT NOT NULL, \
                     issued_ms INTEGER NOT NULL, \
                     expires_ms INTEGER NOT NULL, \
                     revoked INTEGER NOT NULL \
                 ); \
                 CREATE INDEX credential_tokens_by_name ON credential_tokens (name); \
                 INSERT INTO credential_tokens (digest, name, issued_ms, expires_ms, revoked) \
                 VALUES (zeroblob(32), 'phone', 1000, 2000, 0); \
                 PRAGMA user_version = 3;",
            )
            .expect("the file as layout 3 left it");
    }
    let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 3");
    let layout: i64 = Connection::open(scratch.database())
        .expect("a second connection")
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("its layout");
    assert_eq!(
        layout, LAYOUT,
        "the file was brought to this build's layout"
    );
    let kept = TokenRow {
        digest: [0; 32],
        name: "phone".to_string(),
        issued_ms: 1_000,
        expires_ms: 2_000,
        revoked: false,
        sealed: Vec::new(),
    };
    assert_eq!(
        store.token(&[0; 32]).expect("its token"),
        Some(kept),
        "a token of layout 3 has no sealed bytes beside it"
    );
    assert!(store
        .reseal(&[0; 32], b"", b"sealed")
        .expect("sealed bytes kept beside it"));
    assert_eq!(
        store
            .token(&[0; 32])
            .expect("its token")
            .map(|row| row.sealed),
        Some(b"sealed".to_vec())
    );
}

/// ADR 0109, D10: a file of layout 4 is brought to layout 5 when it is opened, in place, its
/// sessions kept, and an outbox appended by a session's write is then kept in it.
#[test]
fn a_file_of_layout_4_is_brought_to_layout_5_and_keeps_an_outbox() {
    let scratch = Scratch::new("layout-4");
    let (lease, version) = {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
        let version = store
            .put("k", b"of layout 4", None, 0, lease.fence)
            .expect("a session");
        (lease, version)
    };
    {
        let connection =
            Connection::open(scratch.database()).expect("a connection of a build of layout 4");
        connection
            .execute_batch(WITHOUT_LAYOUT_5)
            .expect("the file without layout 5's tables");
        connection
            .pragma_update(None, "user_version", 4)
            .expect("the file as layout 4 left it");
    }
    let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 4");
    let layout: i64 = Connection::open(scratch.database())
        .expect("a second connection")
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("its layout");
    assert_eq!(
        layout, LAYOUT,
        "the file was brought to this build's layout"
    );
    assert_eq!(
        store.get("k").expect("its session").map(|held| held.state),
        Some(b"of layout 4".to_vec())
    );
    let written = store
        .write(
            "k",
            &Write {
                outbox: &[Append {
                    client: "phone",
                    incarnation: "0123456789abcdef0123456789abcdef",
                    sender: Some("k"),
                    envelope: b"reply",
                }],
                now_ms: 5_000,
                ..Write::put(b"after", None, version, lease.fence)
            },
        )
        .expect("a write that appends to an outbox in the upgraded file");
    assert_eq!(written.appended.len(), 1);
    assert_eq!(written.appended[0].number, 1);
}

/// ADR 0109, D10: an outbox is there when its file is opened again, its incarnation and its next
/// number with it, so a server that starts again numbers on where it stopped; so is a note an
/// expiry wrote, until the write of its session takes it.
#[test]
fn an_outbox_and_its_notes_are_there_when_the_file_is_opened_again() {
    let scratch = Scratch::new("outbox-reopen");
    let incarnation = "fedcba9876543210fedcba9876543210";
    let (lease, version) = {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
        let appends = [
            Append {
                client: "phone",
                incarnation,
                sender: Some("k"),
                envelope: b"one",
            },
            Append {
                client: "phone",
                incarnation,
                sender: Some("k"),
                envelope: b"two",
            },
        ];
        let written = store
            .write(
                "k",
                &Write {
                    outbox: &appends,
                    now_ms: 1_000,
                    ..Write::put(b"sent two", None, 0, lease.fence)
                },
            )
            .expect("a session that sent two");
        let expiry = store
            .expire("phone", 1_001, 1)
            .expect("the first entry expired");
        assert_eq!(expiry.expired.len(), 1);
        (lease, written.version)
    };
    let store = SqliteStore::open(scratch.database()).expect("opening the store again");
    let outbox = store
        .outbox("phone")
        .expect("the outbox")
        .expect("the outbox is kept");
    assert_eq!(outbox.incarnation, incarnation);
    assert_eq!(outbox.next, 3, "the next number is kept");
    assert_eq!(
        outbox
            .entries
            .iter()
            .map(|entry| (entry.number, entry.envelope.clone()))
            .collect::<Vec<_>>(),
        vec![(2, b"two".to_vec())]
    );
    let notes = store.notes("k").expect("the session's notes");
    assert_eq!(notes.len(), 1, "the expiry's note is kept");
    assert_eq!(notes[0].number, 1);
    assert_eq!(
        store
            .get("k")
            .expect("the session")
            .and_then(|held| held.earliest_due),
        Some(0),
        "the session the note is for is due at once"
    );
    let appended = store
        .append(
            &[Append {
                client: "phone",
                incarnation: "another incarnation, not taken",
                sender: None,
                envelope: b"three",
            }],
            2_000,
        )
        .expect("an append after the restart");
    assert_eq!(appended[0].number, 3, "numbering goes on after the restart");
    assert_eq!(appended[0].incarnation, incarnation);
    store
        .write(
            "k",
            &Write {
                notes_through: notes[0].id,
                ..Write::put(b"told", None, version, lease.fence)
            },
        )
        .expect("a write that takes the note");
    assert!(store.notes("k").expect("the notes").is_empty());
}

/// ADR 0109, D3: the pins and tokens kept in the file are there when it is opened again, as a
/// server process that stops and starts again finds them, revocations included; a second secret
/// is still refused and an expired token is still expired.
#[test]
fn the_credentials_a_store_kept_are_there_when_its_file_is_opened_again() {
    let scratch = Scratch::new("credentials-reopen");
    let live = TokenRow {
        digest: [0x11; 32],
        name: "phone".to_string(),
        issued_ms: 1_000,
        expires_ms: 61_000,
        revoked: false,
        sealed: Vec::new(),
    };
    let revoked = TokenRow {
        digest: [0x22; 32],
        name: "tablet".to_string(),
        ..live.clone()
    };
    {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        store.pin("phone", &[0xA1; 32]).expect("phone's pin");
        store.pin("tablet", &[0xB2; 32]).expect("tablet's pin");
        store.issue(&live).expect("phone's token");
        store.issue(&revoked).expect("tablet's token");
        assert_eq!(
            store.revoke("tablet").expect("revoking tablet").count(),
            2,
            "tablet's token and its pin"
        );
    }
    let store = SqliteStore::open(scratch.database()).expect("opening the store again");
    assert_eq!(
        store.pinned("phone").expect("phone's pin"),
        Some([0xA1; 32].to_vec())
    );
    assert_eq!(
        store.pin("phone", &[0xFF; 32]).expect("a second secret"),
        [0xA1; 32],
        "the pin kept before the restart refuses a second secret after it"
    );
    assert_eq!(store.pinned("tablet").expect("tablet's pin"), None);
    let held = store
        .token(&[0x11; 32])
        .expect("phone's token")
        .expect("phone's token is kept");
    assert_eq!(held, live);
    assert!(held.live_at(60_999) && !held.live_at(61_000));
    assert_eq!(
        store.token(&[0x22; 32]).expect("tablet's token"),
        Some(TokenRow {
            revoked: true,
            ..revoked
        }),
        "a revocation is kept across the restart"
    );
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

/// ADR 0109, D15: an item of a batch the database refuses part of the way through (after it took
/// its number from the counter) is answered failed and leaves nothing of itself, its number
/// included, while the items before and after it are kept; and what the batch kept is there when
/// the file is opened again, so the batch was durable when it answered. Red if the store rolls an
/// item back with the whole transaction (the others lost) or not at all (its number kept).
#[test]
fn a_batch_item_that_fails_partway_leaves_nothing_and_the_rest_is_kept() {
    let scratch = Scratch::new("batch-partway");
    let (before, after, kept_before, kept_after) = {
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        let before = store.acquire("before", "a", 1_000, 0).expect("a lease");
        let during = store.acquire("during", "a", 1_000, 0).expect("a lease");
        let after = store.acquire("after", "a", 1_000, 0).expect("a lease");

        // Another program makes the middle item's row fail after the item checked its lease and
        // its version and took its number from the counter.
        let raw = Connection::open(scratch.database()).expect("a connection of another program");
        raw.execute_batch(
            "CREATE TRIGGER refuse_thirteen AFTER INSERT ON sessions WHEN NEW.earliest_due = 13 \
             BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;",
        )
        .expect("the trigger");
        let decided = store
            .write_many(&[
                Batched::Write {
                    key: "before",
                    write: Write::put(b"before", Some(100), 0, before.fence),
                },
                Batched::Write {
                    key: "during",
                    write: Write::put(b"during", Some(13), 0, during.fence),
                },
                Batched::Write {
                    key: "after",
                    write: Write::put(b"after", Some(300), 0, after.fence),
                },
            ])
            .expect("a batch the store could make");
        let version = |decided: &Decided| match decided {
            Ok(Committed::Written(written)) => written.version,
            other => panic!("an item that should have been kept gave {other:?}"),
        };
        let kept_before = version(&decided[0]);
        assert!(
            matches!(decided[1], Err(WriteError::Failed(_))),
            "an item the database refused gave {:?}",
            decided[1]
        );
        let kept_after = version(&decided[2]);
        assert_eq!(
            kept_after,
            kept_before + 1,
            "the number the failed item took was not given back with the rest of it"
        );
        raw.execute_batch("DROP TRIGGER refuse_thirteen")
            .expect("dropping the trigger");
        (before, after, kept_before, kept_after)
    };

    let store = SqliteStore::open(scratch.database()).expect("opening the store again");
    assert_eq!(
        store.get("before").expect("a read"),
        Some(Stored {
            state: b"before".to_vec(),
            version: kept_before,
            earliest_due: Some(100),
        })
    );
    assert_eq!(store.get("during").expect("a read"), None);
    assert_eq!(
        store.get("after").expect("a read"),
        Some(Stored {
            state: b"after".to_vec(),
            version: kept_after,
            earliest_due: Some(300),
        })
    );
    assert_eq!(
        store.due_before(LATEST_MOMENT, 10).expect("a listing"),
        vec![
            Due {
                key: "before".to_string(),
                due: 100
            },
            Due {
                key: "after".to_string(),
                due: 300
            },
        ]
    );
    store
        .put("before", b"again", None, kept_before, before.fence)
        .expect("a write under the lease taken before the file was opened again");
    store
        .put("after", b"again", None, kept_after, after.fence)
        .expect("a write under the lease taken before the file was opened again");
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
        .pragma_update(None, "user_version", LAYOUT + 1)
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

/// The values of the key-value feature (ADR 0109, D13). The store keeps the session contract too,
/// whose `get`, `put` and `delete` share their names with this one's, so each call here names its
/// contract where both are in reach.
mod kv {
    use rusqlite::Connection;
    use sce_rust_session_store::{
        Identity, KvEntry, KvStore, Put, PutIf, SessionStore, SqliteStore,
    };

    use super::{LAYOUT, WITHOUT_LAYOUT_6};
    use crate::common::Scratch;

    /// The identity of the first start of the put-if `putif` in the session `s-1`.
    const FIRST: Identity<'static> = Identity {
        session: "s-1",
        invoke: "putif",
        start: 1,
    };

    /// ADR 0109, D13: a file of layout 5 is brought to this build's layout when it is opened, in
    /// place, its sessions kept, and the values of the key-value feature are then kept in it,
    /// numbered from their own counter.
    #[test]
    fn a_file_of_layout_5_is_brought_to_this_layout_and_keeps_values() {
        let scratch = Scratch::new("layout-5");
        {
            let store = SqliteStore::open(scratch.database()).expect("making the store");
            let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
            SessionStore::put(&store, "k", b"of layout 5", None, 0, lease.fence)
                .expect("a session");
        }
        {
            let connection =
                Connection::open(scratch.database()).expect("a connection of a build of layout 5");
            connection
                .execute_batch(WITHOUT_LAYOUT_6)
                .expect("the file without layout 6's tables");
            connection
                .pragma_update(None, "user_version", 5)
                .expect("the file as layout 5 left it");
        }
        let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 5");
        let layout: i64 = Connection::open(scratch.database())
            .expect("a second connection")
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("its layout");
        assert_eq!(
            layout, LAYOUT,
            "the file was brought to this build's layout"
        );
        assert_eq!(
            SessionStore::get(&store, "k")
                .expect("its session")
                .map(|held| held.state),
            Some(b"of layout 5".to_vec())
        );
        assert_eq!(
            KvStore::put(&store, "key", b"value", None).expect("a value in the upgraded file"),
            Put::Written {
                replaced: false,
                version: 1
            },
            "the feature's counter is its own, and starts at 0"
        );
    }

    /// ADR 0109, D13: what the store kept of the feature is there when its file is opened again,
    /// and its counter goes on from where it stopped.
    #[test]
    fn the_values_and_first_results_are_there_when_the_file_is_opened_again() {
        let scratch = Scratch::new("kv-reopen");
        {
            let store = SqliteStore::open(scratch.database()).expect("making the store");
            KvStore::put(&store, "a", b"1", None).expect("a value");
            store
                .put_if("b", b"2", Some(0), None, Some(&FIRST))
                .expect("a put-if under an identity");
            KvStore::delete(&store, "a").expect("a removal");
        }
        let store = SqliteStore::open(scratch.database()).expect("opening the store again");
        assert_eq!(KvStore::get(&store, "a").expect("a removed key"), None);
        assert_eq!(
            KvStore::get(&store, "b").expect("the put-if's value"),
            Some(KvEntry {
                value: b"2".to_vec(),
                version: 2
            })
        );
        assert_eq!(
            store.first_result(&FIRST).expect("its first result"),
            Some(PutIf::Written { version: 2 })
        );
        assert_eq!(
            KvStore::put(&store, "a", b"3", None).expect("written again"),
            Put::Written {
                replaced: false,
                version: 4
            },
            "the counter goes on past the removal the closed file took"
        );
    }

    /// ADR 0109, D13: a put-if whose process stopped after its write and before its completion
    /// reached the machine is answered, after the restart, with the result the write made, and
    /// writes nothing more; and the write and its result are one transaction, so a process that
    /// stopped inside it left neither.
    #[test]
    fn a_put_if_stopped_between_its_write_and_its_completion_is_answered_with_its_first_result() {
        let scratch = Scratch::new("kv-killed");
        // The first process: the write is made, and the process goes before its answer leaves.
        {
            let store = SqliteStore::open(scratch.database()).expect("making the store");
            assert_eq!(
                store
                    .put_if("k", b"first", Some(0), None, Some(&FIRST))
                    .expect("the write"),
                PutIf::Written { version: 1 }
            );
        }
        // The second process: the restore starts the invocation again under its identity.
        let store = SqliteStore::open(scratch.database()).expect("the store after the restart");
        assert_eq!(
            store
                .put_if("k", b"first", Some(0), None, Some(&FIRST))
                .expect("the start repeated"),
            PutIf::Written { version: 1 },
            "its first result, where the store alone would answer a lost race at version 1"
        );
        assert_eq!(
            KvStore::get(&store, "k").expect("the value"),
            Some(KvEntry {
                value: b"first".to_vec(),
                version: 1
            }),
            "written once"
        );

        // A put-if that fails between its write and the keeping of its result leaves neither:
        // another program makes the result's row fail after the value was written.
        let raw = Connection::open(scratch.database()).expect("a connection of another program");
        raw.execute_batch(
            "CREATE TRIGGER refuse_results AFTER INSERT ON kv_first_results \
             BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;",
        )
        .expect("the trigger");
        let second = Identity { start: 2, ..FIRST };
        assert!(
            store
                .put_if("k", b"second", Some(1), None, Some(&second))
                .is_err(),
            "the put-if failed with its result"
        );
        assert_eq!(
            KvStore::get(&store, "k").expect("the value"),
            Some(KvEntry {
                value: b"first".to_vec(),
                version: 1
            }),
            "the write went with its result"
        );
        raw.execute_batch("DROP TRIGGER refuse_results")
            .expect("dropping the trigger");
        assert_eq!(
            store
                .put_if("k", b"second", Some(1), None, Some(&second))
                .expect("the start judged again"),
            PutIf::Written { version: 2 },
            "a start that kept no result is judged again, and the number the failed one took \
             was given back with the rest of it"
        );
    }
}

/// The audit log (ADR 0109, D14).
mod audit {
    use rusqlite::Connection;
    use sce_rust_session_store::{
        AuditAppend, AuditHead, AuditRow, AuditStore, Identity, SessionStore, SqliteStore,
    };

    use super::{LAYOUT, WITHOUT_LAYOUT_7};
    use crate::common::Scratch;

    /// Row `sequence` of the stream `s`, its chain the sequence's bytes.
    fn row(sequence: u64) -> AuditRow {
        let mut chain = [0u8; 32];
        chain[24..].copy_from_slice(&sequence.to_be_bytes());
        AuditRow {
            stream: "s".to_string(),
            sequence,
            at_ms: sequence * 1_000,
            entry: format!("{{\"n\":{sequence}}}").into_bytes(),
            chain,
        }
    }

    /// ADR 0109, D14: a file of layout 6 is brought to this build's layout when it is opened, in
    /// place, its sessions kept, and an audit log is then kept in it.
    #[test]
    fn a_file_of_layout_6_is_brought_to_this_layout_and_keeps_an_audit_log() {
        let scratch = Scratch::new("layout-6");
        {
            let store = SqliteStore::open(scratch.database()).expect("making the store");
            let lease = store.acquire("k", "a", 1_000, 0).expect("a lease");
            SessionStore::put(&store, "k", b"of layout 6", None, 0, lease.fence)
                .expect("a session");
        }
        {
            let connection =
                Connection::open(scratch.database()).expect("a connection of a build of layout 6");
            connection
                .execute_batch(WITHOUT_LAYOUT_7)
                .expect("the file without layout 7's tables");
            connection
                .pragma_update(None, "user_version", 6)
                .expect("the file as layout 6 left it");
        }
        let store = SqliteStore::open(scratch.database()).expect("opening a file of layout 6");
        let layout: i64 = Connection::open(scratch.database())
            .expect("a second connection")
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .expect("its layout");
        assert_eq!(
            layout, LAYOUT,
            "the file was brought to this build's layout"
        );
        assert_eq!(
            SessionStore::get(&store, "k")
                .expect("its session")
                .map(|held| held.state),
            Some(b"of layout 6".to_vec())
        );
        assert_eq!(
            store
                .audit_append(&row(1), None, None)
                .expect("a row in the upgraded file"),
            AuditAppend::Appended
        );
    }

    /// ADR 0109, D14: what the store kept of the log is there when its file is opened again: the
    /// rows, the checkpoints, the identities and the head, which retention left.
    #[test]
    fn the_audit_log_is_there_when_the_file_is_opened_again() {
        let scratch = Scratch::new("audit-reopen");
        let start = Identity {
            session: "s-1",
            invoke: "audit",
            start: 1,
        };
        {
            let store = SqliteStore::open(scratch.database()).expect("making the store");
            store
                .audit_append(&row(1), None, Some(&start))
                .expect("a row under an identity");
            store
                .audit_append(&row(2), Some(b"signature"), None)
                .expect("a row with its checkpoint");
            store.audit_append(&row(3), None, None).expect("a row");
            store.audit_drop_through("s", 2).expect("a segment");
        }
        let store = SqliteStore::open(scratch.database()).expect("opening the store again");
        assert_eq!(
            store.audit_head("s").expect("the head"),
            AuditHead {
                sequence: 3,
                chain: row(3).chain
            }
        );
        assert_eq!(
            store.audit_rows("s", 0, 10).expect("the rows"),
            [row(3)],
            "the segment retention deleted is gone, the rest kept as written"
        );
        assert_eq!(
            store.audit_checkpoints("s").expect("the checkpoints").len(),
            1
        );
        assert_eq!(store.audit_first(&start).expect("its identity"), Some(1));
        assert_eq!(
            store
                .audit_append(&row(4), None, None)
                .expect("the next row"),
            AuditAppend::Appended,
            "numbered on from the head the closed file kept"
        );
    }

    /// ADR 0109, D14: an append, its checkpoint and its identity are one transaction, so a process
    /// that stopped inside it left none of them, and the head did not move.
    #[test]
    fn an_append_that_fails_part_of_the_way_through_leaves_nothing_of_itself() {
        let scratch = Scratch::new("audit-partway");
        let store = SqliteStore::open(scratch.database()).expect("making the store");
        store.audit_append(&row(1), None, None).expect("a row");
        let raw = Connection::open(scratch.database()).expect("a connection of another program");
        raw.execute_batch(
            "CREATE TRIGGER refuse_identities AFTER INSERT ON audit_identities \
             BEGIN SELECT RAISE(ABORT, 'refused by the test'); END;",
        )
        .expect("the trigger");
        let start = Identity {
            session: "s-1",
            invoke: "audit",
            start: 1,
        };
        assert!(
            store
                .audit_append(&row(2), Some(b"signature"), Some(&start))
                .is_err(),
            "the append failed with its identity"
        );
        assert_eq!(store.audit_head("s").expect("the head").sequence, 1);
        assert_eq!(store.audit_rows("s", 0, 10).expect("the rows").len(), 1);
        assert!(store.audit_checkpoints("s").expect("none").is_empty());
        raw.execute_batch("DROP TRIGGER refuse_identities")
            .expect("dropping the trigger");
        assert_eq!(
            store
                .audit_append(&row(2), Some(b"signature"), Some(&start))
                .expect("the append made again"),
            AuditAppend::Appended
        );
    }
}
