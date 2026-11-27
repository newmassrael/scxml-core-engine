// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference session store: SQLite in WAL mode.

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::audit::{
    rules as audit_rules, AuditAppend, AuditCheckpoint, AuditHead, AuditRow, AuditStore, Chain,
    NoCheckpointAt, CHAIN_BYTES,
};
use crate::batch::{Batched, Committed, Decided};
use crate::credentials::{
    CredentialStore, Digest, PinRow, PinStore, Revoked, TokenRow, TokenStore, DIGEST_BYTES,
};
use crate::journal::{
    holds_from, recorded, AsOf, AsOfCursor, AsOfError, Fact, Journal, JournalEntry, JournalPage,
    JournalStore, Observation, ObservedStream, PastHorizon, SessionsStream, Stream, StreamName,
};
use crate::keys::{KeyRow, KeyState, KeyStore, KeyUse};
use crate::kv::{rules as kv_rules, Identity, KvEntry, KvStore, Put, PutIf};
use crate::membership::judge::{self, Judged};
use crate::membership::{
    Admission, Delegation, Ended, Joining, Member, Membership, MembershipStore, Offer, Offered,
    Person, PersonKey, Renewal, Revocation, Terms,
};
use crate::outbox::{
    Append, Appended, Change, Expired, Expiry, Note, Outbox, OutboxEntry, Write, Written,
};
use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// The layout this store writes, kept in the database's `user_version`; a database of a layout a
/// later build wrote is refused rather than read as this one, and one of an earlier layout is
/// brought to this one when it is opened, in one transaction.
const LAYOUT_VERSION: i64 = 12;

/// The tables of layout 1. A session's state, version and earliest due are one row, so
/// that the index of dues, which SQLite keeps in the transaction that writes the row, cannot
/// disagree with the states. A lease is a row of its own, outliving a deleted session; the numbers
/// handed out come from the one row of `numbers`, so that a version or a fence is never given
/// twice, even for a key whose session or lease was removed and made again.
const LAYOUT_1: &str = "
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
    INSERT INTO numbers (id, issued) VALUES (0, 0);
";

/// What layout 2 adds to layout 1: the keys of [`KeyStore`] (ADR 0109, D7), each row its material
/// wrapped under a key-encryption key the file never holds; which key each generation made, so a
/// generation repeated under its identity answers its first key; and the one wrapped naming key
/// a derived key's id is computed under.
const LAYOUT_2: &str = "
    CREATE TABLE keys (
        id TEXT PRIMARY KEY NOT NULL,
        key_use TEXT NOT NULL,
        algorithm TEXT NOT NULL,
        wrapped BLOB NOT NULL,
        label TEXT NOT NULL,
        created_ms INTEGER NOT NULL,
        state TEXT NOT NULL
    );
    CREATE TABLE key_generations (
        generation TEXT PRIMARY KEY NOT NULL,
        key TEXT NOT NULL REFERENCES keys (id)
    );
    CREATE TABLE key_naming (
        id INTEGER PRIMARY KEY CHECK (id = 0),
        wrapped BLOB NOT NULL
    );
";

/// What layout 3 adds to layout 2: the credentials of [`CredentialStore`] (ADR 0109, D3), each the
/// SHA-256 of its secret and never the secret. A pin is a name to the digest of its installation
/// secret; a token is found by the digest of its text, and the tokens of a name by the index on
/// it, which revocation reads.
const LAYOUT_3: &str = "
    CREATE TABLE credential_pins (
        name TEXT PRIMARY KEY NOT NULL,
        digest BLOB NOT NULL
    );
    CREATE TABLE credential_tokens (
        digest BLOB PRIMARY KEY NOT NULL,
        name TEXT NOT NULL,
        issued_ms INTEGER NOT NULL,
        expires_ms INTEGER NOT NULL,
        revoked INTEGER NOT NULL
    );
    CREATE INDEX credential_tokens_by_name ON credential_tokens (name);
";

/// What layout 4 adds to layout 3: beside each token, the bytes a store that wraps this one keeps
/// the token's sealed record in (ADR 0109, D8; [`TokenRow::sealed`]), empty for a token its issuer
/// wrote. A pin's `digest` column already holds any bytes: the digest, or its sealing.
const LAYOUT_4: &str = "
    ALTER TABLE credential_tokens ADD COLUMN sealed BLOB NOT NULL DEFAULT x'';
";

/// What layout 5 adds to layout 4: the outboxes of the clients of a durable `https` binding (ADR
/// 0109, D10). A head per client keeps the incarnation its outbox numbers in and the number its
/// next entry takes, so both outlive the process; an entry is found by its client and number,
/// and an expiry's note by the session it is for. A note's id is a number of the store's counter.
const LAYOUT_5: &str = "
    CREATE TABLE outbox_heads (
        client TEXT PRIMARY KEY NOT NULL,
        incarnation TEXT NOT NULL,
        next INTEGER NOT NULL,
        touched_ms INTEGER NOT NULL
    );
    CREATE TABLE outbox_entries (
        client TEXT NOT NULL,
        number INTEGER NOT NULL,
        at_ms INTEGER NOT NULL,
        sender TEXT,
        envelope BLOB NOT NULL,
        PRIMARY KEY (client, number)
    );
    CREATE TABLE outbox_notes (
        id INTEGER PRIMARY KEY NOT NULL,
        session TEXT NOT NULL,
        client TEXT NOT NULL,
        number INTEGER NOT NULL,
        at_ms INTEGER NOT NULL,
        envelope BLOB NOT NULL
    );
    CREATE INDEX outbox_notes_by_session ON outbox_notes (session, id);
";

/// What layout 6 adds to layout 5: the values of the key-value feature (ADR 0109, D13), each at
/// the version a number of the feature's own counter gave it, so that its versions do not share
/// the sessions' counter; and the first result of each put-if by the identity of its invocation,
/// written in the transaction that judged it, which a removal of a session deletes by its session.
const LAYOUT_6: &str = "
    CREATE TABLE kv_values (
        key TEXT PRIMARY KEY NOT NULL,
        value BLOB NOT NULL,
        version INTEGER NOT NULL
    );
    CREATE TABLE kv_numbers (
        id INTEGER PRIMARY KEY CHECK (id = 0),
        issued INTEGER NOT NULL
    );
    INSERT INTO kv_numbers (id, issued) VALUES (0, 0);
    CREATE TABLE kv_first_results (
        session TEXT NOT NULL,
        invoke TEXT NOT NULL,
        start INTEGER NOT NULL,
        outcome TEXT NOT NULL,
        version INTEGER NOT NULL,
        PRIMARY KEY (session, invoke, start)
    );
";

/// What layout 7 adds to layout 6: the audit log (ADR 0109, D14). A stream's head outlives the
/// rows retention deletes, so the next row is numbered and chained on; a row and a checkpoint are
/// found by their stream and sequence; and the identity an append was made under names the
/// sequence it was appended at, which a removal of its session deletes.
const LAYOUT_7: &str = "
    CREATE TABLE audit_heads (
        stream TEXT PRIMARY KEY NOT NULL,
        sequence INTEGER NOT NULL,
        chain BLOB NOT NULL
    );
    CREATE TABLE audit_rows (
        stream TEXT NOT NULL,
        sequence INTEGER NOT NULL,
        at_ms INTEGER NOT NULL,
        entry BLOB NOT NULL,
        chain BLOB NOT NULL,
        PRIMARY KEY (stream, sequence)
    );
    CREATE TABLE audit_checkpoints (
        stream TEXT NOT NULL,
        sequence INTEGER NOT NULL,
        chain BLOB NOT NULL,
        signature BLOB NOT NULL,
        PRIMARY KEY (stream, sequence)
    );
    CREATE TABLE audit_identities (
        session TEXT NOT NULL,
        invoke TEXT NOT NULL,
        start INTEGER NOT NULL,
        sequence INTEGER NOT NULL,
        PRIMARY KEY (session, invoke, start)
    );
";

/// What layout 8 adds to layout 7: the journal (ADR 0114). The one row of its head is the last
/// position given and the last moment recorded, which the write that gives the next moves, so a
/// position is never given twice and a moment never goes back, after a restart either. An entry
/// is of a session's stream (the machine and the key) or of an observed stream's (its name, with
/// the id the other system gave), never of both; `holds_ms` is its valid time, or its recorded
/// moment where it has none, which an answer as of a moment is ordered by. Every index ends with
/// the position, which is the table's row id.
const LAYOUT_8: &str = "
    CREATE TABLE journal_head (
        id INTEGER PRIMARY KEY CHECK (id = 0),
        issued INTEGER NOT NULL,
        recorded_ms INTEGER NOT NULL
    );
    INSERT INTO journal_head (id, issued, recorded_ms) VALUES (0, 0, 0);
    CREATE TABLE journal_entries (
        position INTEGER PRIMARY KEY NOT NULL,
        recorded_ms INTEGER NOT NULL,
        name TEXT NOT NULL,
        key TEXT,
        origin TEXT,
        schema TEXT NOT NULL,
        revision INTEGER NOT NULL,
        valid_ms INTEGER,
        holds_ms INTEGER NOT NULL,
        data BLOB NOT NULL,
        CHECK ((key IS NULL) <> (origin IS NULL))
    );
    CREATE INDEX journal_by_stream ON journal_entries (name, key);
    CREATE INDEX journal_by_valid ON journal_entries (name, key, holds_ms);
    CREATE INDEX journal_by_moment ON journal_entries (recorded_ms);
    CREATE UNIQUE INDEX journal_by_origin ON journal_entries (name, origin)
        WHERE origin IS NOT NULL;
";

/// What layout 9 adds to layout 8: the members of open machines (ADR 0113, D4 and D5). A member is
/// one row by its name, its incarnation a number of the store's counter, so no two rows share one;
/// an offer that waits is a row of its own beside it, which its confirmation deletes. Both are
/// found by when they run out, which a sweep of what ended reads.
const LAYOUT_9: &str = "
    CREATE TABLE members (
        member TEXT PRIMARY KEY NOT NULL,
        incarnation INTEGER NOT NULL,
        mark INTEGER NOT NULL,
        expires_ms INTEGER NOT NULL,
        kept INTEGER NOT NULL,
        lease_ms INTEGER NOT NULL
    );
    CREATE UNIQUE INDEX members_by_incarnation ON members (incarnation);
    CREATE INDEX members_by_expiry ON members (expires_ms, member);
    CREATE TABLE member_offers (
        member TEXT PRIMARY KEY NOT NULL,
        incarnation INTEGER NOT NULL,
        nonce TEXT NOT NULL,
        expires_ms INTEGER NOT NULL
    );
    CREATE INDEX member_offers_by_expiry ON member_offers (expires_ms, member);
";

/// What layout 10 adds to layout 9: the person a member's incarnation acts for (ADR 0116, D1 and
/// D6), in the member's row, empty for one that acts for nobody: the person's issuer and subject,
/// by which a revocation finds every incarnation that acts for them, the moment and method of the
/// proof, the delegation's `until`, and the bytes a store that wraps this one seals the person in.
/// A member ends at its lease or at its `until`, whichever comes first: that moment is kept as
/// `ends_ms`, by which the members that stand are counted and those that ran out are swept, so
/// the sweep stays one range of an index. A member of layout 9 acts for nobody and ends at its
/// lease.
const LAYOUT_10: &str = "
    ALTER TABLE members ADD COLUMN person_issuer TEXT;
    ALTER TABLE members ADD COLUMN person_subject TEXT;
    ALTER TABLE members ADD COLUMN proved_ms INTEGER;
    ALTER TABLE members ADD COLUMN method TEXT;
    ALTER TABLE members ADD COLUMN until_ms INTEGER;
    ALTER TABLE members ADD COLUMN person_sealed BLOB;
    ALTER TABLE members ADD COLUMN ends_ms INTEGER NOT NULL DEFAULT 0;
    UPDATE members SET ends_ms = expires_ms;
    DROP INDEX members_by_expiry;
    CREATE INDEX members_by_end ON members (ends_ms, member);
    CREATE INDEX members_by_person ON members (person_issuer, person_subject);
";

/// What layout 11 adds to layout 10: whether an offer waits for a proof of a person (ADR 0116,
/// D2), `1` when it does. An offer of layout 10 waits for an exchange.
const LAYOUT_11: &str = "
    ALTER TABLE member_offers ADD COLUMN proof INTEGER NOT NULL DEFAULT 0;
";

/// What layout 12 changes of layout 11: a session is its machine and its key (ADR 0114, D2), so the
/// rows of a session, its lease and the notes an expiry wrote for it are each found by both, and an
/// outbox entry keeps the machine of the session that sent it, which its note is written for. The
/// rows a file of an earlier layout holds are the machine the empty name stands for, the one
/// machine of a store that kept one. Beside them, how far each reader of the journal has taken
/// in every entry it reads (D4), which only rises.
const LAYOUT_12: &str = "
    CREATE TABLE sessions_of_machines (
        machine TEXT NOT NULL,
        session TEXT NOT NULL,
        state BLOB NOT NULL,
        version INTEGER NOT NULL,
        earliest_due INTEGER,
        PRIMARY KEY (machine, session)
    );
    INSERT INTO sessions_of_machines (machine, session, state, version, earliest_due)
        SELECT '', session, state, version, earliest_due FROM sessions;
    DROP TABLE sessions;
    ALTER TABLE sessions_of_machines RENAME TO sessions;
    CREATE INDEX sessions_by_due ON sessions (machine, earliest_due, session)
        WHERE earliest_due IS NOT NULL;
    CREATE TABLE leases_of_machines (
        machine TEXT NOT NULL,
        session TEXT NOT NULL,
        owner TEXT NOT NULL,
        fence INTEGER NOT NULL,
        expires_ms INTEGER NOT NULL,
        PRIMARY KEY (machine, session)
    );
    INSERT INTO leases_of_machines (machine, session, owner, fence, expires_ms)
        SELECT '', session, owner, fence, expires_ms FROM leases;
    DROP TABLE leases;
    ALTER TABLE leases_of_machines RENAME TO leases;
    ALTER TABLE outbox_notes ADD COLUMN machine TEXT NOT NULL DEFAULT '';
    DROP INDEX outbox_notes_by_session;
    CREATE INDEX outbox_notes_by_session ON outbox_notes (machine, session, id);
    ALTER TABLE outbox_entries ADD COLUMN machine TEXT NOT NULL DEFAULT '';
    CREATE TABLE journal_readers (
        reader TEXT PRIMARY KEY NOT NULL,
        position INTEGER NOT NULL
    );
";

/// A session store in one SQLite database file.
///
/// The database is in WAL mode, so that reads go on while a write is made, with
/// `synchronous=FULL`, so that the log is synced to the disk before a commit returns: a put that
/// returned survives a power loss, which is what lets the host core release a reply only once the
/// checkpoint that produced it is written (ADR 0107, D5).
///
/// A handle is one connection. Several handles may be open on one file, in one process or in
/// several, which is how two server processes share a store; SQLite's locks keep their
/// transactions apart, and the leases' fences keep their sessions apart.
///
/// A handle keeps the sessions of one machine ([`SessionStore::machine`]): the one
/// [`Self::open`] gives is of the machine the empty name stands for, and [`Self::of_machine`]
/// gives one of another machine on the same connection, so the machines of a deployment keep
/// their sessions in one file and an equal key of two of them names two sessions (ADR 0114, D2).
/// Every write is an
/// `IMMEDIATE` transaction: it takes the database's write lock before it reads the lease and the
/// version it checks, so that what it read is still what is there when it writes. A deferred
/// transaction would read first and, when another connection committed in between, fail at its
/// first write in a way the busy timeout cannot wait out.
pub struct SqliteStore {
    connection: Arc<Mutex<Connection>>,
    /// The machine whose sessions this handle keeps.
    machine: String,
}

impl SqliteStore {
    /// How long a call waits for another connection's write to end before it fails, unless the
    /// store was opened with another bound.
    pub const BUSY_TIMEOUT: Duration = Duration::from_secs(5);

    /// Open the store in the file at `path`, making it if it does not exist, with
    /// [`Self::BUSY_TIMEOUT`].
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Failure> {
        Self::open_with_busy_timeout(path, Self::BUSY_TIMEOUT)
    }

    /// Open the store in the file at `path`, making it if it does not exist; a call that finds the
    /// database locked by another connection's write waits `busy_timeout` for it to end before it
    /// fails.
    ///
    /// Refused when the file cannot be put in WAL mode (a database in memory, or a file system
    /// without the shared memory WAL needs), when it holds a layout this build does not know, and
    /// when it holds tables this store did not make.
    pub fn open_with_busy_timeout(
        path: impl AsRef<Path>,
        busy_timeout: Duration,
    ) -> Result<Self, Failure> {
        let mut connection = Connection::open(path)?;
        // Before anything else, so that a store opened while another connection writes waits for
        // it, the change of journal mode included.
        connection.busy_timeout(busy_timeout)?;
        // Before the journal mode is changed, so that a file this store refuses is left as it was
        // found.
        lay_out(&mut connection)?;
        let journal: String =
            connection.pragma_update_and_check(None, "journal_mode", "WAL", |row| row.get(0))?;
        if !journal.eq_ignore_ascii_case("wal") {
            return Err(Failure::new(format!(
                "the database could not be put in WAL mode; it is in '{journal}' mode"
            )));
        }
        connection.pragma_update(None, "synchronous", "FULL")?;
        // FULL is 2. A build of SQLite that took the setting differently would make every put
        // a promise it does not keep, so the store does not open.
        let synchronous: i64 =
            connection.pragma_query_value(None, "synchronous", |row| row.get(0))?;
        if synchronous != 2 {
            return Err(Failure::new(format!(
                "the database did not take synchronous=FULL; it is at {synchronous}"
            )));
        }
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
            machine: String::new(),
        })
    }

    /// A handle on the same connection that keeps the sessions of `machine` (ADR 0114, D2): every
    /// call that names a key names the session of `machine` and that key, and what the handles
    /// share (the journal, the outboxes, the counter that numbers versions and fences) is one.
    pub fn of_machine(&self, machine: &str) -> Self {
        Self {
            connection: Arc::clone(&self.connection),
            machine: machine.to_string(),
        }
    }

    /// The connection, whatever a thread that panicked while it held it left: nothing, since a
    /// transaction that is dropped unfinished, as unwinding drops it, is rolled back.
    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Make the tables in a database that has none, bring one of an earlier layout to
/// [`LAYOUT_VERSION`], or check that the ones there are this store's. A database is changed in one
/// transaction, so a failed upgrade leaves it at the layout it had.
fn lay_out(connection: &mut Connection) -> Result<(), Failure> {
    // Each layout's additions, in order: layout `n` is the first `n` of them.
    const ADDITIONS: [&str; 12] = [
        LAYOUT_1, LAYOUT_2, LAYOUT_3, LAYOUT_4, LAYOUT_5, LAYOUT_6, LAYOUT_7, LAYOUT_8, LAYOUT_9,
        LAYOUT_10, LAYOUT_11, LAYOUT_12,
    ];
    let transaction = begin(connection)?;
    let layout: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    match layout {
        LAYOUT_VERSION => return Ok(()),
        0 => {
            let tables: i64 =
                transaction
                    .query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
            if tables != 0 {
                return Err(Failure::new(
                    "the database holds tables a session store did not make",
                ));
            }
            for addition in ADDITIONS {
                transaction.execute_batch(addition)?;
            }
        }
        1..LAYOUT_VERSION => {
            let held = usize::try_from(layout).unwrap_or(ADDITIONS.len());
            for addition in &ADDITIONS[held..] {
                transaction.execute_batch(addition)?;
            }
        }
        _ => {
            return Err(Failure::new(format!(
                "the database is laid out as session store {layout}, which this build does not know"
            )))
        }
    }
    transaction.pragma_update(None, "user_version", LAYOUT_VERSION)?;
    transaction.commit()?;
    Ok(())
}

/// A transaction that holds the write lock from its start (see [`SqliteStore`]).
fn begin(connection: &mut Connection) -> rusqlite::Result<Transaction<'_>> {
    connection.transaction_with_behavior(TransactionBehavior::Immediate)
}

/// The lease held on the session of `machine` that `key` names, if one is.
fn holding(connection: &Connection, machine: &str, key: &str) -> rusqlite::Result<Option<Holding>> {
    connection
        .prepare_cached(
            "SELECT owner, fence, expires_ms FROM leases WHERE machine = ?1 AND session = ?2",
        )?
        .query_row((machine, key), |row| {
            Ok(Holding {
                owner: row.get(0)?,
                fence: row.get(1)?,
                expires_ms: row.get(2)?,
            })
        })
        .optional()
}

/// The version of the session of `machine` that `key` names, `0` when there is none.
fn version_of(connection: &Connection, machine: &str, key: &str) -> rusqlite::Result<u64> {
    let version = connection
        .prepare_cached("SELECT version FROM sessions WHERE machine = ?1 AND session = ?2")?
        .query_row((machine, key), |row| row.get(0))
        .optional()?;
    Ok(version.unwrap_or(0))
}

/// The next number of the store's counter, a version or a fence.
fn next_number(connection: &Connection) -> rusqlite::Result<u64> {
    connection
        .prepare_cached("UPDATE numbers SET issued = issued + 1 WHERE id = 0")?
        .execute([])?;
    connection
        .prepare_cached("SELECT issued FROM numbers WHERE id = 0")?
        .query_row([], |row| row.get(0))
}

impl SessionStore for SqliteStore {
    fn machine(&self) -> &str {
        &self.machine
    }

    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        let connection = self.connection();
        let stored = connection
            .prepare_cached(
                "SELECT state, version, earliest_due FROM sessions \
                 WHERE machine = ?1 AND session = ?2",
            )?
            .query_row((self.machine.as_str(), key), |row| {
                Ok(Stored {
                    state: row.get(0)?,
                    version: row.get(1)?,
                    earliest_due: row.get(2)?,
                })
            })
            .optional()?;
        Ok(stored)
    }

    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let written = write_in(&transaction, &self.machine, key, write)?;
        transaction.commit()?;
        Ok(written)
    }

    /// One `IMMEDIATE` transaction, one savepoint per item: an item refused or failed is rolled
    /// back to its savepoint, and the others are kept. The commit syncs the log once
    /// (`synchronous=FULL`), so the batch is durable as a whole when this returns.
    fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
        let mut connection = self.connection();
        let mut transaction = begin(&mut connection)?;
        let mut decided = Vec::with_capacity(batch.len());
        for item in batch {
            let savepoint = transaction.savepoint()?;
            let done = match *item {
                Batched::Write { key, write } => {
                    write_in(&savepoint, &self.machine, key, &write).map(Committed::Written)
                }
                Batched::Acknowledge {
                    client,
                    incarnation,
                    through,
                    now_ms,
                } => acknowledge_in(&savepoint, client, incarnation, through, now_ms)
                    .map(Committed::Acknowledged)
                    .map_err(WriteError::from),
            };
            // A savepoint that cannot be released or rolled back leaves the transaction in a
            // state no item can be judged against: the batch fails whole, and keeps nothing.
            if done.is_ok() {
                savepoint.commit()?;
            } else {
                savepoint.finish()?;
            }
            decided.push(done);
        }
        transaction.commit()?;
        Ok(decided)
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT session, earliest_due FROM sessions \
             WHERE machine = ?1 AND earliest_due IS NOT NULL AND earliest_due <= ?2 \
             ORDER BY earliest_due, session LIMIT ?3",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let listed = statement
            .query_map((self.machine.as_str(), moment(ms), limit), |row| {
                Ok(Due {
                    key: row.get(0)?,
                    due: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<Due>>>()?;
        Ok(listed)
    }

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A key is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes.
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT session FROM sessions WHERE machine = ?1 AND session > ?2 \
                     ORDER BY session LIMIT ?3",
                )?;
                let rows =
                    statement.query_map((self.machine.as_str(), after, limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            None => {
                let mut statement = connection.prepare_cached(
                    "SELECT session FROM sessions WHERE machine = ?1 ORDER BY session LIMIT ?2",
                )?;
                let rows = statement.query_map((self.machine.as_str(), limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
        };
        Ok(listed)
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_acquire(
            holding(&transaction, &self.machine, key)?.as_ref(),
            owner,
            now_ms,
        )?;
        let lease = Lease {
            fence: next_number(&transaction)?,
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO leases (machine, session, owner, fence, expires_ms) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?
            .execute((
                self.machine.as_str(),
                key,
                owner,
                lease.fence,
                lease.expires_ms,
            ))?;
        transaction.commit()?;
        Ok(lease)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_renew(
            holding(&transaction, &self.machine, key)?.as_ref(),
            fence,
            now_ms,
        )?;
        let expires_ms = rules::expiry(now_ms, ttl_ms);
        transaction
            .prepare_cached(
                "UPDATE leases SET expires_ms = ?3 WHERE machine = ?1 AND session = ?2",
            )?
            .execute((self.machine.as_str(), key, expires_ms))?;
        transaction.commit()?;
        Ok(Lease { fence, expires_ms })
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_release(holding(&transaction, &self.machine, key)?.as_ref(), fence)?;
        transaction
            .prepare_cached("DELETE FROM leases WHERE machine = ?1 AND session = ?2")?
            .execute((self.machine.as_str(), key))?;
        transaction.commit()?;
        Ok(())
    }

    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let appended = append_in(&transaction, &self.machine, appends, now_ms)?;
        transaction.commit()?;
        Ok(appended)
    }

    fn outbox(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        Ok(outbox_of(&self.connection(), client)?)
    }

    fn open_outbox(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<Outbox, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        transaction
            .prepare_cached(
                "INSERT OR IGNORE INTO outbox_heads (client, incarnation, next, touched_ms) \
                 VALUES (?1, ?2, 1, ?3)",
            )?
            .execute((client, incarnation, moment(now_ms)))?;
        let opened = outbox_of(&transaction, client)?;
        transaction.commit()?;
        opened.ok_or_else(|| Failure::new("the outbox opened in this transaction is not held"))
    }

    fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let removed = acknowledge_in(&transaction, client, incarnation, through, now_ms)?;
        transaction.commit()?;
        Ok(removed)
    }

    fn expire(&self, client: &str, before_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let old = {
            let mut statement = transaction.prepare_cached(
                "SELECT number, at_ms, sender, envelope, machine FROM outbox_entries \
                 WHERE client = ?1 AND at_ms < ?2 ORDER BY number LIMIT ?3",
            )?;
            let rows = statement.query_map((client, moment(before_ms), limit), |row| {
                Ok((entry_row(row)?, row.get::<_, String>(4)?))
            })?;
            rows.collect::<rusqlite::Result<Vec<(OutboxEntry, String)>>>()?
        };
        let mut expiry = Expiry::default();
        for (entry, machine) in old {
            transaction
                .prepare_cached("DELETE FROM outbox_entries WHERE client = ?1 AND number = ?2")?
                .execute((client, entry.number))?;
            // A note is written for a session the store holds, of the machine that sent the
            // entry: one that ended has no next run.
            let noted = match &entry.sender {
                Some(sender) if version_of(&transaction, &machine, sender)? != 0 => Some(sender),
                _ => None,
            };
            if let Some(sender) = noted {
                let id = next_number(&transaction)?;
                transaction
                    .prepare_cached(
                        "INSERT INTO outbox_notes \
                         (id, machine, session, client, number, at_ms, envelope) \
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    )?
                    .execute((
                        id,
                        machine.as_str(),
                        sender,
                        client,
                        entry.number,
                        entry.at_ms,
                        &entry.envelope,
                    ))?;
                transaction
                    .prepare_cached(
                        "UPDATE sessions SET earliest_due = ?3 WHERE machine = ?1 AND session = ?2",
                    )?
                    .execute((machine.as_str(), sender, rules::DUE_AT_ONCE))?;
            }
            expiry.expired.push(Expired {
                client: client.to_string(),
                entry,
            });
        }
        let left: i64 = transaction
            .prepare_cached("SELECT count(*) FROM outbox_entries WHERE client = ?1")?
            .query_row((client,), |row| row.get(0))?;
        if left == 0 {
            let closed = transaction
                .prepare_cached("DELETE FROM outbox_heads WHERE client = ?1 AND touched_ms < ?2")?
                .execute((client, moment(before_ms)))?;
            expiry.closed = closed == 1;
        }
        transaction.commit()?;
        Ok(expiry)
    }

    fn notes(&self, key: &str) -> Result<Vec<Note>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT id, client, number, at_ms, envelope FROM outbox_notes \
             WHERE machine = ?1 AND session = ?2 ORDER BY id",
        )?;
        let notes = statement
            .query_map((self.machine.as_str(), key), |row| {
                Ok(Note {
                    id: row.get(0)?,
                    client: row.get(1)?,
                    number: row.get(2)?,
                    at_ms: row.get(3)?,
                    envelope: row.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<Note>>>()?;
        Ok(notes)
    }

    fn outbox_clients_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A client is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes.
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT client FROM outbox_heads WHERE client > ?1 ORDER BY client LIMIT ?2",
                )?;
                let rows = statement.query_map((after, limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            None => {
                let mut statement = connection
                    .prepare_cached("SELECT client FROM outbox_heads ORDER BY client LIMIT ?1")?;
                let rows = statement.query_map((limit,), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
        };
        Ok(listed)
    }

    fn reseal_entry(
        &self,
        client: &str,
        number: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE outbox_entries SET envelope = ?4 \
                 WHERE client = ?1 AND number = ?2 AND envelope = ?3",
            )?
            .execute((client, number, held, envelope))?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    fn reseal_note(
        &self,
        key: &str,
        id: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE outbox_notes SET envelope = ?4 \
                 WHERE id = ?1 AND machine = ?5 AND session = ?2 AND envelope = ?3",
            )?
            .execute((id, key, held, envelope, self.machine.as_str()))?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

/// What [`SessionStore::write`] does with `write` to the session of `machine` that `key` names,
/// in the transaction (or the savepoint of one) `connection` holds: refused before it writes
/// anything unless the rules admit it. A statement that fails after that leaves what the ones
/// before it wrote, which the caller rolls back with the rest of its transaction or savepoint.
fn write_in(
    connection: &Connection,
    machine: &str,
    key: &str,
    write: &Write<'_>,
) -> Result<Written, WriteError> {
    rules::admit_write(
        holding(connection, machine, key)?.as_ref(),
        write.fence,
        version_of(connection, machine, key)?,
        write.expected_version,
    )?;
    for under in write.under {
        judge::admit_under(
            member_of(connection, under.member)?.as_ref(),
            under.incarnation,
            write.now_ms,
        )?;
    }
    let version = match write.change {
        Change::Keep {
            state,
            earliest_due,
        } => {
            let version = next_number(connection)?;
            connection
                .prepare_cached(
                    "INSERT OR REPLACE INTO sessions \
                     (machine, session, state, version, earliest_due) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )?
                .execute((machine, key, state, version, earliest_due.map(moment)))?;
            connection
                .prepare_cached(
                    "DELETE FROM outbox_notes WHERE machine = ?1 AND session = ?2 AND id <= ?3",
                )?
                .execute((machine, key, write.notes_through))?;
            version
        }
        Change::Remove => {
            connection
                .prepare_cached("DELETE FROM sessions WHERE machine = ?1 AND session = ?2")?
                .execute((machine, key))?;
            connection
                .prepare_cached("DELETE FROM outbox_notes WHERE machine = ?1 AND session = ?2")?
                .execute((machine, key))?;
            for &session in write.sessions {
                connection
                    .prepare_cached("DELETE FROM kv_first_results WHERE session = ?1")?
                    .execute((session,))?;
                connection
                    .prepare_cached("DELETE FROM audit_identities WHERE session = ?1")?
                    .execute((session,))?;
            }
            0
        }
    };
    let appended = append_in(connection, machine, write.outbox, write.now_ms)?;
    if let Some(journal) = &write.journal {
        journal_in(connection, key, journal, write.now_ms)?;
    }
    for under in write.under {
        if let Some(held) = member_of(connection, under.member)? {
            put_member(connection, under.member, &judge::kept(&held, write.now_ms))?;
        }
    }
    Ok(Written { version, appended })
}

/// The member `member` names, whether or not it still stands.
fn member_of(connection: &Connection, member: &str) -> rusqlite::Result<Option<Member>> {
    connection
        .prepare_cached(
            "SELECT incarnation, mark, expires_ms, kept, lease_ms, person_issuer, person_subject, \
             proved_ms, method, until_ms, person_sealed FROM members WHERE member = ?1",
        )?
        .query_row((member,), |row| {
            // A member that acts for nobody has no subject, and none of the person's columns.
            let subject: Option<String> = row.get(6)?;
            let delegation = subject
                .map(|subject| {
                    Ok::<_, rusqlite::Error>(Delegation {
                        person: Person {
                            issuer: row.get(5)?,
                            subject,
                            proved_ms: row.get(7)?,
                            method: row.get(8)?,
                        },
                        until_ms: row.get(9)?,
                        sealed: row.get(10)?,
                    })
                })
                .transpose()?;
            Ok(Member {
                incarnation: row.get(0)?,
                mark: row.get(1)?,
                expires_ms: row.get(2)?,
                kept: row.get(3)?,
                lease_ms: row.get(4)?,
                delegation,
            })
        })
        .optional()
}

/// The offer that waits for `member`, whether or not it still stands.
fn offer_of(connection: &Connection, member: &str) -> rusqlite::Result<Option<Offer>> {
    connection
        .prepare_cached(
            "SELECT incarnation, nonce, expires_ms, proof FROM member_offers WHERE member = ?1",
        )?
        .query_row((member,), |row| {
            let proof: bool = row.get(3)?;
            Ok(Offer {
                incarnation: row.get(0)?,
                nonce: row.get(1)?,
                expires_ms: row.get(2)?,
                joining: if proof {
                    Joining::Proof
                } else {
                    Joining::Exchange
                },
            })
        })
        .optional()
}

/// Keep `held` as the member `member` names, in place of the one before, with the moment it ends
/// ([`Member::ends_ms`]) beside it.
fn put_member(connection: &Connection, member: &str, held: &Member) -> rusqlite::Result<()> {
    let delegation = held.delegation.as_ref();
    connection
        .prepare_cached(
            "INSERT OR REPLACE INTO members \
             (member, incarnation, mark, expires_ms, kept, lease_ms, person_issuer, \
              person_subject, proved_ms, method, until_ms, person_sealed, ends_ms) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        )?
        .execute(rusqlite::params![
            member,
            held.incarnation,
            held.mark,
            held.expires_ms,
            held.kept,
            held.lease_ms,
            delegation.map(|delegation| delegation.person.issuer.as_str()),
            delegation.map(|delegation| delegation.person.subject.as_str()),
            delegation.map(|delegation| delegation.person.proved_ms),
            delegation.map(|delegation| delegation.person.method.as_str()),
            delegation.map(|delegation| delegation.until_ms),
            delegation.map(|delegation| delegation.sealed.as_slice()),
            held.ends_ms(),
        ])?;
    Ok(())
}

/// What [`SessionStore::acknowledge`] does, in the transaction (or the savepoint of one)
/// `connection` holds: how many entries it deleted.
fn acknowledge_in(
    connection: &Connection,
    client: &str,
    incarnation: &str,
    through: u64,
    now_ms: u64,
) -> rusqlite::Result<u64> {
    let held: Option<String> = connection
        .prepare_cached("SELECT incarnation FROM outbox_heads WHERE client = ?1")?
        .query_row((client,), |row| row.get(0))
        .optional()?;
    if !rules::acknowledges(held.as_deref(), incarnation) {
        return Ok(0);
    }
    let removed = connection
        .prepare_cached("DELETE FROM outbox_entries WHERE client = ?1 AND number <= ?2")?
        .execute((client, through))?;
    if removed > 0 {
        connection
            .prepare_cached("UPDATE outbox_heads SET touched_ms = ?2 WHERE client = ?1")?
            .execute((client, moment(now_ms)))?;
    }
    Ok(u64::try_from(removed).unwrap_or(u64::MAX))
}

/// Append `appends` at `now_ms` in the transaction `connection` holds, each to its client's
/// outbox, opened in the append's incarnation when the store holds none for the client, each sent
/// by a session of `machine`.
fn append_in(
    connection: &Connection,
    machine: &str,
    appends: &[Append<'_>],
    now_ms: u64,
) -> rusqlite::Result<Vec<Appended>> {
    let at_ms = moment(now_ms);
    let mut appended = Vec::with_capacity(appends.len());
    for append in appends {
        connection
            .prepare_cached(
                "INSERT OR IGNORE INTO outbox_heads (client, incarnation, next, touched_ms) \
                 VALUES (?1, ?2, 1, ?3)",
            )?
            .execute((append.client, append.incarnation, at_ms))?;
        let (incarnation, number): (String, u64) = connection
            .prepare_cached("SELECT incarnation, next FROM outbox_heads WHERE client = ?1")?
            .query_row((append.client,), |row| Ok((row.get(0)?, row.get(1)?)))?;
        connection
            .prepare_cached(
                "UPDATE outbox_heads SET next = next + 1, touched_ms = ?2 WHERE client = ?1",
            )?
            .execute((append.client, at_ms))?;
        connection
            .prepare_cached(
                "INSERT INTO outbox_entries (client, number, at_ms, sender, envelope, machine) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?
            .execute((
                append.client,
                number,
                at_ms,
                append.sender,
                append.envelope,
                machine,
            ))?;
        appended.push(Appended {
            client: append.client.to_string(),
            incarnation,
            number,
        });
    }
    Ok(appended)
}

/// An outbox entry from its columns, read in the order `number, at_ms, sender, envelope`.
fn entry_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<OutboxEntry> {
    Ok(OutboxEntry {
        number: row.get(0)?,
        at_ms: row.get(1)?,
        sender: row.get(2)?,
        envelope: row.get(3)?,
    })
}

/// The outbox of `client` in `connection`, if it holds one.
fn outbox_of(connection: &Connection, client: &str) -> rusqlite::Result<Option<Outbox>> {
    let head: Option<(String, u64)> = connection
        .prepare_cached("SELECT incarnation, next FROM outbox_heads WHERE client = ?1")?
        .query_row((client,), |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    let Some((incarnation, next)) = head else {
        return Ok(None);
    };
    let mut statement = connection.prepare_cached(
        "SELECT number, at_ms, sender, envelope FROM outbox_entries \
         WHERE client = ?1 ORDER BY number",
    )?;
    let entries = statement
        .query_map((client,), entry_row)?
        .collect::<rusqlite::Result<Vec<OutboxEntry>>>()?;
    Ok(Some(Outbox {
        incarnation,
        next,
        entries,
    }))
}

/// The columns of a key row as SQLite gives them, before their names are read.
type KeyColumns = (String, String, String, Vec<u8>, String, u64, String);

/// The key `id` names in `connection`, if it holds one.
fn key_of(connection: &Connection, id: &str) -> Result<Option<KeyRow>, Failure> {
    let columns: Option<KeyColumns> = connection
        .prepare_cached(
            "SELECT id, key_use, algorithm, wrapped, label, created_ms, state FROM keys \
             WHERE id = ?1",
        )?
        .query_row((id,), |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .optional()?;
    columns.map(key_row).transpose()
}

/// A key row from its columns, refused when a name in it is none this build writes.
fn key_row(
    (id, key_use, algorithm, wrapped, label, created_ms, state): KeyColumns,
) -> Result<KeyRow, Failure> {
    let key_use = KeyUse::from_name(&key_use).ok_or_else(|| {
        Failure::new(format!(
            "the key {id} is of the use '{key_use}', which this build does not know"
        ))
    })?;
    let state = KeyState::from_name(&state).ok_or_else(|| {
        Failure::new(format!(
            "the key {id} is in the state '{state}', which this build does not know"
        ))
    })?;
    Ok(KeyRow {
        id,
        key_use,
        algorithm,
        wrapped,
        label,
        created_ms,
        state,
    })
}

/// The wrapped naming key `connection` holds, if it holds one.
fn naming_key_of(connection: &Connection) -> rusqlite::Result<Option<Vec<u8>>> {
    connection
        .prepare_cached("SELECT wrapped FROM key_naming WHERE id = 0")?
        .query_row([], |row| row.get(0))
        .optional()
}

impl KeyStore for SqliteStore {
    fn key(&self, id: &str) -> Result<Option<KeyRow>, Failure> {
        key_of(&self.connection(), id)
    }

    fn ids(&self) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached("SELECT id FROM keys ORDER BY id")?;
        let ids = statement
            .query_map([], |row| row.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(ids)
    }

    fn generated_by(&self, generation: &str) -> Result<Option<String>, Failure> {
        let connection = self.connection();
        let made = connection
            .prepare_cached("SELECT key FROM key_generations WHERE generation = ?1")?
            .query_row((generation,), |row| row.get(0))
            .optional()?;
        Ok(made)
    }

    fn insert(&self, row: &KeyRow, generation: Option<&str>) -> Result<KeyRow, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(generation) = generation {
            let made: Option<String> = transaction
                .prepare_cached("SELECT key FROM key_generations WHERE generation = ?1")?
                .query_row((generation,), |row| row.get(0))
                .optional()?;
            if let Some(made) = made {
                if let Some(held) = key_of(&transaction, &made)? {
                    return Ok(held);
                }
            }
        }
        if let Some(held) = key_of(&transaction, &row.id)? {
            return Ok(held);
        }
        transaction
            .prepare_cached(
                "INSERT INTO keys (id, key_use, algorithm, wrapped, label, created_ms, state) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?
            .execute((
                &row.id,
                row.key_use.name(),
                &row.algorithm,
                &row.wrapped,
                &row.label,
                moment(row.created_ms),
                row.state.name(),
            ))?;
        if let Some(generation) = generation {
            transaction
                .prepare_cached("INSERT INTO key_generations (generation, key) VALUES (?1, ?2)")?
                .execute((generation, &row.id))?;
        }
        transaction.commit()?;
        Ok(row.clone())
    }

    fn retire(&self, id: &str) -> Result<Option<bool>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let Some(held) = key_of(&transaction, id)? else {
            return Ok(None);
        };
        transaction
            .prepare_cached("UPDATE keys SET state = ?2 WHERE id = ?1")?
            .execute((id, KeyState::Retired.name()))?;
        transaction.commit()?;
        Ok(Some(held.state == KeyState::Active))
    }

    fn rewrap(&self, id: &str, wrapped: &[u8]) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached("UPDATE keys SET wrapped = ?2 WHERE id = ?1")?
            .execute((id, wrapped))?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    fn naming_key(&self) -> Result<Option<Vec<u8>>, Failure> {
        Ok(naming_key_of(&self.connection())?)
    }

    fn keep_naming_key(&self, wrapped: &[u8]) -> Result<Vec<u8>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(held) = naming_key_of(&transaction)? {
            return Ok(held);
        }
        transaction
            .prepare_cached("INSERT INTO key_naming (id, wrapped) VALUES (0, ?1)")?
            .execute((wrapped,))?;
        transaction.commit()?;
        Ok(wrapped.to_vec())
    }

    fn rewrap_naming_key(&self, wrapped: &[u8]) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached("UPDATE key_naming SET wrapped = ?1 WHERE id = 0")?
            .execute((wrapped,))?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

/// A digest as a column holds it, refused when it is not [`DIGEST_BYTES`] long: a row this build
/// did not write.
fn digest_of(column: Vec<u8>) -> Result<Digest, Failure> {
    Digest::try_from(column.as_slice()).map_err(|_| {
        Failure::new(format!(
            "a credential's digest is {} bytes, not {DIGEST_BYTES}",
            column.len()
        ))
    })
}

/// The columns of a token row as SQLite gives them, before its digest is read.
type TokenColumns = (Vec<u8>, String, u64, u64, bool, Vec<u8>);

/// A token row from its columns.
fn token_row(
    (digest, name, issued_ms, expires_ms, revoked, sealed): TokenColumns,
) -> Result<TokenRow, Failure> {
    Ok(TokenRow {
        digest: digest_of(digest)?,
        name,
        issued_ms,
        expires_ms,
        revoked,
        sealed,
    })
}

/// The columns of a token row, read in the order [`TokenColumns`] names.
fn token_columns(row: &rusqlite::Row<'_>) -> rusqlite::Result<TokenColumns> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

/// The token whose digest is `digest` in `connection`, if it holds one.
fn token_of(connection: &Connection, digest: &Digest) -> Result<Option<TokenRow>, Failure> {
    let columns = connection
        .prepare_cached(
            "SELECT digest, name, issued_ms, expires_ms, revoked, sealed FROM credential_tokens \
             WHERE digest = ?1",
        )?
        .query_row((&digest[..],), token_columns)
        .optional()?;
    columns.map(token_row).transpose()
}

/// The bytes `name` is pinned to in `connection`, if it is pinned.
fn pin_of(connection: &Connection, name: &str) -> Result<Option<Vec<u8>>, Failure> {
    Ok(connection
        .prepare_cached("SELECT digest FROM credential_pins WHERE name = ?1")?
        .query_row((name,), |row| row.get(0))
        .optional()?)
}

impl PinStore for SqliteStore {
    fn pinned(&self, name: &str) -> Result<Option<Vec<u8>>, Failure> {
        pin_of(&self.connection(), name)
    }

    fn pin(&self, name: &str, pin: &[u8]) -> Result<Vec<u8>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(held) = pin_of(&transaction, name)? {
            return Ok(held);
        }
        transaction
            .prepare_cached("INSERT INTO credential_pins (name, digest) VALUES (?1, ?2)")?
            .execute((name, pin))?;
        transaction.commit()?;
        Ok(pin.to_vec())
    }

    fn pins(&self) -> Result<Vec<PinRow>, Failure> {
        let connection = self.connection();
        let mut statement =
            connection.prepare_cached("SELECT name, digest FROM credential_pins ORDER BY name")?;
        let rows = statement.query_map([], |row| {
            Ok(PinRow {
                name: row.get(0)?,
                pin: row.get(1)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<PinRow>>>()?)
    }

    fn repin(&self, name: &str, held: &[u8], pin: &[u8]) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE credential_pins SET digest = ?3 WHERE name = ?1 AND digest = ?2",
            )?
            .execute((name, held, pin))?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    fn unpin(&self, name: &str) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let removed = transaction
            .prepare_cached("DELETE FROM credential_pins WHERE name = ?1")?
            .execute((name,))?;
        transaction.commit()?;
        Ok(removed == 1)
    }
}

impl TokenStore for SqliteStore {
    fn token(&self, digest: &Digest) -> Result<Option<TokenRow>, Failure> {
        token_of(&self.connection(), digest)
    }

    fn issue(&self, row: &TokenRow) -> Result<TokenRow, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(held) = token_of(&transaction, &row.digest)? {
            return Ok(held);
        }
        let kept = TokenRow {
            issued_ms: moment(row.issued_ms),
            expires_ms: moment(row.expires_ms),
            ..row.clone()
        };
        transaction
            .prepare_cached(
                "INSERT INTO credential_tokens \
                 (digest, name, issued_ms, expires_ms, revoked, sealed) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )?
            .execute((
                &kept.digest[..],
                &kept.name,
                kept.issued_ms,
                kept.expires_ms,
                kept.revoked,
                &kept.sealed,
            ))?;
        transaction.commit()?;
        Ok(kept)
    }

    fn tokens(&self) -> Result<Vec<TokenRow>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT digest, name, issued_ms, expires_ms, revoked, sealed FROM credential_tokens \
             ORDER BY digest",
        )?;
        let columns = statement
            .query_map([], token_columns)?
            .collect::<rusqlite::Result<Vec<TokenColumns>>>()?;
        columns.into_iter().map(token_row).collect()
    }

    fn reseal(&self, digest: &Digest, held: &[u8], sealed: &[u8]) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE credential_tokens SET sealed = ?3 WHERE digest = ?1 AND sealed = ?2",
            )?
            .execute((&digest[..], held, sealed))?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

impl CredentialStore for SqliteStore {
    fn revoke(&self, name: &str) -> Result<Revoked, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let tokens = transaction
            .prepare_cached(
                "UPDATE credential_tokens SET revoked = 1 WHERE name = ?1 AND revoked = 0",
            )?
            .execute((name,))?;
        let pins = transaction
            .prepare_cached("DELETE FROM credential_pins WHERE name = ?1")?
            .execute((name,))?;
        transaction.commit()?;
        Ok(Revoked {
            tokens: u32::try_from(tokens).unwrap_or(u32::MAX),
            pin: pins == 1,
        })
    }
}

/// The value `key` names in `connection`, if it holds one.
fn value_of(connection: &Connection, key: &str) -> rusqlite::Result<Option<KvEntry>> {
    connection
        .prepare_cached("SELECT value, version FROM kv_values WHERE key = ?1")?
        .query_row((key,), |row| {
            Ok(KvEntry {
                value: row.get(0)?,
                version: row.get(1)?,
            })
        })
        .optional()
}

/// The next number of the key-value feature's counter.
fn next_value_number(connection: &Connection) -> rusqlite::Result<u64> {
    connection
        .prepare_cached("UPDATE kv_numbers SET issued = issued + 1 WHERE id = 0")?
        .execute([])?;
    connection
        .prepare_cached("SELECT issued FROM kv_numbers WHERE id = 0")?
        .query_row([], |row| row.get(0))
}

/// Whether writing a key would hold more values than `quota` allows, in `connection`: `held` is
/// whether the store holds a value of the key, which a write replaces.
fn values_over_quota(
    connection: &Connection,
    held: bool,
    quota: Option<u64>,
) -> rusqlite::Result<bool> {
    if held || quota.is_none() {
        return Ok(false);
    }
    let count: u64 = connection
        .prepare_cached("SELECT count(*) FROM kv_values")?
        .query_row([], |row| row.get(0))?;
    Ok(kv_rules::over_quota(true, count, quota))
}

/// Keep `value` under `key` at the counter's next number in `connection`; that number.
fn write_value(connection: &Connection, key: &str, value: &[u8]) -> rusqlite::Result<u64> {
    let version = next_value_number(connection)?;
    connection
        .prepare_cached(
            "INSERT OR REPLACE INTO kv_values (key, value, version) VALUES (?1, ?2, ?3)",
        )?
        .execute((key, value, version))?;
    Ok(version)
}

/// The first result `connection` keeps under `identity`, if it keeps one.
fn first_result_of(
    connection: &Connection,
    identity: &Identity<'_>,
) -> Result<Option<PutIf>, Failure> {
    let kept: Option<(String, u64)> = connection
        .prepare_cached(
            "SELECT outcome, version FROM kv_first_results \
             WHERE session = ?1 AND invoke = ?2 AND start = ?3",
        )?
        .query_row((identity.session, identity.invoke, identity.start), |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()?;
    kept.map(|(outcome, version)| {
        PutIf::from_kept(&outcome, version).ok_or_else(|| {
            Failure::new(format!(
                "a first result is kept as '{outcome}', which this build does not know"
            ))
        })
    })
    .transpose()
}

/// Keep `result` under `identity` in `connection`; the caller found none kept there.
fn keep_first_in(
    connection: &Connection,
    identity: &Identity<'_>,
    result: &PutIf,
) -> rusqlite::Result<()> {
    connection
        .prepare_cached(
            "INSERT INTO kv_first_results (session, invoke, start, outcome, version) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
        )?
        .execute((
            identity.session,
            identity.invoke,
            identity.start,
            result.outcome(),
            result.version(),
        ))?;
    Ok(())
}

/// The members, in the same file as the sessions, so that a write made under an incarnation is
/// judged against them in its own transaction.
impl MembershipStore for SqliteStore {
    fn offer_as(
        &self,
        member: &str,
        nonce: &str,
        joining: Joining,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Offered, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(offer) = offer_of(&transaction, member)?
            .filter(|offer| judge::offers_again(Some(offer), nonce, joining, now_ms))
        {
            return Ok(Offered {
                incarnation: offer.incarnation,
                expires_ms: offer.expires_ms,
            });
        }
        let offered = Offered {
            incarnation: next_number(&transaction)?,
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO member_offers \
                 (member, incarnation, nonce, expires_ms, proof) VALUES (?1, ?2, ?3, ?4, ?5)",
            )?
            .execute((
                member,
                offered.incarnation,
                nonce,
                offered.expires_ms,
                joining == Joining::Proof,
            ))?;
        transaction.commit()?;
        Ok(offered)
    }

    fn admit_as(
        &self,
        member: &str,
        incarnation: u64,
        mark: u64,
        delegation: Option<&Delegation>,
        terms: Terms,
        now_ms: u64,
    ) -> Result<Admission, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let held = member_of(&transaction, member)?;
        let offered = offer_of(&transaction, member)?;
        let others: i64 = transaction
            .prepare_cached("SELECT count(*) FROM members WHERE ends_ms > ?1 AND member <> ?2")?
            .query_row((moment(now_ms), member), |row| row.get(0))?;
        let judged = judge::admit(
            held.as_ref(),
            offered.as_ref(),
            u64::try_from(others).unwrap_or(0),
            judge::Exchange {
                incarnation,
                mark,
                delegation,
            },
            terms,
            now_ms,
        );
        match &judged {
            Judged::Renew { member: renewed } => put_member(&transaction, member, renewed)?,
            Judged::Confirm {
                member: confirmed, ..
            } => {
                put_member(&transaction, member, confirmed)?;
                transaction
                    .prepare_cached("DELETE FROM member_offers WHERE member = ?1")?
                    .execute((member,))?;
            }
            Judged::Refuse(_) => {}
        }
        transaction.commit()?;
        Ok(judged.admission())
    }

    fn membership(&self, member: &str) -> Result<Membership, Failure> {
        let connection = self.connection();
        Ok(Membership {
            member: member_of(&connection, member)?,
            offer: offer_of(&connection, member)?,
        })
    }

    fn end_expired(&self, before_ms: u64, limit: usize) -> Result<Vec<Ended>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let ended = {
            let mut statement = transaction.prepare_cached(
                "SELECT member, incarnation FROM members WHERE ends_ms <= ?1 \
                 ORDER BY ends_ms, member LIMIT ?2",
            )?;
            let rows = statement.query_map((moment(before_ms), limit), |row| {
                Ok(Ended {
                    member: row.get(0)?,
                    incarnation: row.get(1)?,
                })
            })?;
            rows.collect::<rusqlite::Result<Vec<Ended>>>()?
        };
        for ended in &ended {
            transaction
                .prepare_cached("DELETE FROM members WHERE member = ?1")?
                .execute((&ended.member,))?;
        }
        transaction
            .prepare_cached(
                "DELETE FROM member_offers WHERE member IN (SELECT member FROM member_offers \
                 WHERE expires_ms <= ?1 ORDER BY expires_ms, member LIMIT ?2)",
            )?
            .execute((moment(before_ms), limit))?;
        transaction.commit()?;
        Ok(ended)
    }

    fn end_revoked(&self, revocation: Revocation<'_>) -> Result<Vec<Ended>, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        // The names a revocation may end: every member of the person, found by the index of
        // persons, or the one member it names. Each is then judged by the rule every store reads.
        let named: Vec<String> = match revocation {
            Revocation::Person(person) => {
                let mut statement = transaction.prepare_cached(
                    "SELECT member FROM members WHERE person_issuer = ?1 AND person_subject = ?2 \
                     ORDER BY member",
                )?;
                let rows =
                    statement.query_map((person.issuer, person.subject), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            Revocation::Instance(member) | Revocation::Incarnation { member, .. } => {
                vec![member.to_string()]
            }
        };
        let mut ended = Vec::new();
        for name in named {
            let Some(held) = member_of(&transaction, &name)? else {
                continue;
            };
            if judge::revokes(&revocation, &name, &held) {
                transaction
                    .prepare_cached("DELETE FROM members WHERE member = ?1")?
                    .execute((&name,))?;
                ended.push(Ended {
                    member: name,
                    incarnation: held.incarnation,
                });
            }
        }
        transaction.commit()?;
        Ok(ended)
    }

    fn renew_delegation(
        &self,
        member: &str,
        incarnation: u64,
        person: PersonKey<'_>,
        until_ms: u64,
        now_ms: u64,
    ) -> Result<Renewal, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let (renewal, renewed) = judge::renew(
            member_of(&transaction, member)?.as_ref(),
            incarnation,
            person,
            until_ms,
            now_ms,
        );
        if let Some(renewed) = renewed {
            put_member(&transaction, member, &renewed)?;
        }
        transaction.commit()?;
        Ok(renewal)
    }

    fn delegated_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A name is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes.
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT member FROM members WHERE person_subject IS NOT NULL AND member > ?1 \
                     ORDER BY member LIMIT ?2",
                )?;
                let rows = statement.query_map((after, limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            None => {
                let mut statement = connection.prepare_cached(
                    "SELECT member FROM members WHERE person_subject IS NOT NULL \
                     ORDER BY member LIMIT ?1",
                )?;
                let rows = statement.query_map((limit,), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
        };
        Ok(listed)
    }

    fn reseal_delegation(
        &self,
        member: &str,
        incarnation: u64,
        held: &[u8],
        sealed: &[u8],
    ) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let replaced = transaction
            .prepare_cached(
                "UPDATE members SET person_sealed = ?4 WHERE member = ?1 AND incarnation = ?2 \
                 AND person_subject IS NOT NULL AND person_sealed = ?3",
            )?
            .execute((member, incarnation, held, sealed))?;
        transaction.commit()?;
        Ok(replaced == 1)
    }
}

impl KvStore for SqliteStore {
    fn get(&self, key: &str) -> Result<Option<KvEntry>, Failure> {
        Ok(value_of(&self.connection(), key)?)
    }

    fn put(&self, key: &str, value: &[u8], quota: Option<u64>) -> Result<Put, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let replaced = value_of(&transaction, key)?.is_some();
        if values_over_quota(&transaction, replaced, quota)? {
            return Ok(Put::OverQuota);
        }
        let version = write_value(&transaction, key, value)?;
        transaction.commit()?;
        Ok(Put::Written { replaced, version })
    }

    fn put_if(
        &self,
        key: &str,
        value: &[u8],
        expected: Option<u64>,
        quota: Option<u64>,
        identity: Option<&Identity<'_>>,
    ) -> Result<PutIf, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(identity) = identity {
            if let Some(first) = first_result_of(&transaction, identity)? {
                return Ok(first);
            }
        }
        let held = value_of(&transaction, key)?;
        let current = held.as_ref().map_or(0, |entry| entry.version);
        let result = if !kv_rules::matches(expected, current) {
            PutIf::Unwritten { version: current }
        } else if values_over_quota(&transaction, held.is_some(), quota)? {
            PutIf::OverQuota
        } else {
            PutIf::Written {
                version: write_value(&transaction, key, value)?,
            }
        };
        if let Some(identity) = identity {
            keep_first_in(&transaction, identity, &result)?;
        }
        transaction.commit()?;
        Ok(result)
    }

    fn delete(&self, key: &str) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let removed = transaction
            .prepare_cached("DELETE FROM kv_values WHERE key = ?1")?
            .execute((key,))?;
        if removed == 1 {
            next_value_number(&transaction)?;
        }
        transaction.commit()?;
        Ok(removed == 1)
    }

    fn next_key(&self, after: &str, prefix: &str) -> Result<Option<String>, Failure> {
        // A key is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes;
        // the keys with the prefix are contiguous in it, so the walk starts at the later of the
        // prefix and `after` and the first key it meets answers or ends it.
        let connection = self.connection();
        let found: Option<String> = if after.is_empty() || after < prefix {
            connection
                .prepare_cached("SELECT key FROM kv_values WHERE key >= ?1 ORDER BY key LIMIT 1")?
                .query_row((prefix,), |row| row.get(0))
                .optional()?
        } else {
            connection
                .prepare_cached("SELECT key FROM kv_values WHERE key > ?1 ORDER BY key LIMIT 1")?
                .query_row((after,), |row| row.get(0))
                .optional()?
        };
        Ok(found.filter(|key| kv_rules::walks_to(key, after, prefix)))
    }

    fn entries_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, KvEntry)>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let entry = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(String, KvEntry)> {
            Ok((
                row.get(0)?,
                KvEntry {
                    value: row.get(1)?,
                    version: row.get(2)?,
                },
            ))
        };
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT key, value, version FROM kv_values WHERE key > ?1 \
                     ORDER BY key LIMIT ?2",
                )?;
                let rows = statement.query_map((after, limit), entry)?;
                rows.collect::<rusqlite::Result<Vec<(String, KvEntry)>>>()?
            }
            None => {
                let mut statement = connection.prepare_cached(
                    "SELECT key, value, version FROM kv_values ORDER BY key LIMIT ?1",
                )?;
                let rows = statement.query_map((limit,), entry)?;
                rows.collect::<rusqlite::Result<Vec<(String, KvEntry)>>>()?
            }
        };
        Ok(listed)
    }

    fn first_result(&self, identity: &Identity<'_>) -> Result<Option<PutIf>, Failure> {
        first_result_of(&self.connection(), identity)
    }

    fn keep_first(&self, identity: &Identity<'_>, result: &PutIf) -> Result<PutIf, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(first) = first_result_of(&transaction, identity)? {
            return Ok(first);
        }
        keep_first_in(&transaction, identity, result)?;
        transaction.commit()?;
        Ok(result.clone())
    }

    fn reseal(&self, key: &str, held: &[u8], value: &[u8]) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached("UPDATE kv_values SET value = ?3 WHERE key = ?1 AND value = ?2")?
            .execute((key, held, value))?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

/// A chain as a column holds it, refused when it is not [`CHAIN_BYTES`] long: a row this build did
/// not write.
fn chain_of(column: Vec<u8>) -> Result<Chain, Failure> {
    Chain::try_from(column.as_slice()).map_err(|_| {
        Failure::new(format!(
            "an audit chain is {} bytes, not {CHAIN_BYTES}",
            column.len()
        ))
    })
}

/// The head of `stream` in `connection`.
fn audit_head_of(connection: &Connection, stream: &str) -> Result<AuditHead, Failure> {
    let held: Option<(u64, Vec<u8>)> = connection
        .prepare_cached("SELECT sequence, chain FROM audit_heads WHERE stream = ?1")?
        .query_row((stream,), |row| Ok((row.get(0)?, row.get(1)?)))
        .optional()?;
    match held {
        None => Ok(AuditHead::EMPTY),
        Some((sequence, chain)) => Ok(AuditHead {
            sequence,
            chain: chain_of(chain)?,
        }),
    }
}

/// The sequence `connection` keeps for `identity`, if it keeps one.
fn audit_first_of(
    connection: &Connection,
    identity: &Identity<'_>,
) -> rusqlite::Result<Option<u64>> {
    connection
        .prepare_cached(
            "SELECT sequence FROM audit_identities \
             WHERE session = ?1 AND invoke = ?2 AND start = ?3",
        )?
        .query_row((identity.session, identity.invoke, identity.start), |row| {
            row.get(0)
        })
        .optional()
}

/// The columns of an audit row as SQLite gives them, before its chain is read.
type AuditColumns = (String, u64, u64, Vec<u8>, Vec<u8>);

/// The columns of a checkpoint as SQLite gives them, before its chain is read.
type CheckpointColumns = (String, u64, Vec<u8>, Vec<u8>);

impl AuditStore for SqliteStore {
    fn audit_head(&self, stream: &str) -> Result<AuditHead, Failure> {
        audit_head_of(&self.connection(), stream)
    }

    fn audit_append(
        &self,
        row: &AuditRow,
        checkpoint: Option<&[u8]>,
        identity: Option<&Identity<'_>>,
    ) -> Result<AuditAppend, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(identity) = identity {
            if let Some(sequence) = audit_first_of(&transaction, identity)? {
                return Ok(AuditAppend::Repeated { sequence });
            }
        }
        let head = audit_head_of(&transaction, &row.stream)?;
        if let Err(moved) = audit_rules::follows(&head, row) {
            return Ok(moved);
        }
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO audit_heads (stream, sequence, chain) VALUES (?1, ?2, ?3)",
            )?
            .execute((&row.stream, row.sequence, &row.chain[..]))?;
        transaction
            .prepare_cached(
                "INSERT INTO audit_rows (stream, sequence, at_ms, entry, chain) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?
            .execute((
                &row.stream,
                row.sequence,
                moment(row.at_ms),
                &row.entry,
                &row.chain[..],
            ))?;
        if let Some(signature) = checkpoint {
            transaction
                .prepare_cached(
                    "INSERT INTO audit_checkpoints (stream, sequence, chain, signature) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?
                .execute((&row.stream, row.sequence, &row.chain[..], signature))?;
        }
        if let Some(identity) = identity {
            transaction
                .prepare_cached(
                    "INSERT INTO audit_identities (session, invoke, start, sequence) \
                     VALUES (?1, ?2, ?3, ?4)",
                )?
                .execute((
                    identity.session,
                    identity.invoke,
                    identity.start,
                    row.sequence,
                ))?;
        }
        transaction.commit()?;
        Ok(AuditAppend::Appended)
    }

    fn audit_checkpoint(&self, checkpoint: &AuditCheckpoint) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let kept = transaction
            .prepare_cached(
                "INSERT OR IGNORE INTO audit_checkpoints (stream, sequence, chain, signature) \
                 VALUES (?1, ?2, ?3, ?4)",
            )?
            .execute((
                &checkpoint.stream,
                checkpoint.sequence,
                &checkpoint.chain[..],
                &checkpoint.signature,
            ))?;
        transaction.commit()?;
        Ok(kept == 1)
    }

    fn audit_rows(&self, stream: &str, after: u64, limit: usize) -> Result<Vec<AuditRow>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A sequence past the largest signed 64-bit number is none SQLite holds.
        let Ok(after) = i64::try_from(after) else {
            return Ok(Vec::new());
        };
        let mut statement = connection.prepare_cached(
            "SELECT stream, sequence, at_ms, entry, chain FROM audit_rows \
             WHERE stream = ?1 AND sequence > ?2 ORDER BY sequence LIMIT ?3",
        )?;
        let columns = statement
            .query_map((stream, after, limit), |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<AuditColumns>>>()?;
        columns
            .into_iter()
            .map(|(stream, sequence, at_ms, entry, chain)| {
                Ok(AuditRow {
                    stream,
                    sequence,
                    at_ms,
                    entry,
                    chain: chain_of(chain)?,
                })
            })
            .collect()
    }

    fn audit_checkpoints(&self, stream: &str) -> Result<Vec<AuditCheckpoint>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT stream, sequence, chain, signature FROM audit_checkpoints \
             WHERE stream = ?1 ORDER BY sequence",
        )?;
        let columns = statement
            .query_map((stream,), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?
            .collect::<rusqlite::Result<Vec<CheckpointColumns>>>()?;
        columns
            .into_iter()
            .map(|(stream, sequence, chain, signature)| {
                Ok(AuditCheckpoint {
                    stream,
                    sequence,
                    chain: chain_of(chain)?,
                    signature,
                })
            })
            .collect()
    }

    fn audit_drop_through(&self, stream: &str, through: u64) -> Result<u64, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let anchored: i64 = transaction
            .prepare_cached(
                "SELECT count(*) FROM audit_checkpoints WHERE stream = ?1 AND sequence = ?2",
            )?
            .query_row((stream, through), |row| row.get(0))?;
        if anchored == 0 {
            return Err(Failure::new(NoCheckpointAt {
                stream: stream.to_string(),
                sequence: through,
            }));
        }
        let removed = transaction
            .prepare_cached("DELETE FROM audit_rows WHERE stream = ?1 AND sequence <= ?2")?
            .execute((stream, through))?;
        transaction
            .prepare_cached("DELETE FROM audit_checkpoints WHERE stream = ?1 AND sequence < ?2")?
            .execute((stream, through))?;
        transaction.commit()?;
        Ok(u64::try_from(removed).unwrap_or(u64::MAX))
    }

    fn audit_first(&self, identity: &Identity<'_>) -> Result<Option<u64>, Failure> {
        Ok(audit_first_of(&self.connection(), identity)?)
    }

    fn audit_streams_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A stream is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes.
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT stream FROM audit_heads WHERE stream > ?1 ORDER BY stream LIMIT ?2",
                )?;
                let rows = statement.query_map((after, limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            None => {
                let mut statement = connection
                    .prepare_cached("SELECT stream FROM audit_heads ORDER BY stream LIMIT ?1")?;
                let rows = statement.query_map((limit,), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
        };
        Ok(listed)
    }

    fn audit_reseal_row(
        &self,
        stream: &str,
        sequence: u64,
        held: &[u8],
        entry: &[u8],
    ) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE audit_rows SET entry = ?4 \
                 WHERE stream = ?1 AND sequence = ?2 AND entry = ?3",
            )?
            .execute((stream, sequence, held, entry))?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    fn audit_reseal_checkpoint(
        &self,
        stream: &str,
        sequence: u64,
        held: &[u8],
        signature: &[u8],
    ) -> Result<bool, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE audit_checkpoints SET signature = ?4 \
                 WHERE stream = ?1 AND sequence = ?2 AND signature = ?3",
            )?
            .execute((stream, sequence, held, signature))?;
        transaction.commit()?;
        Ok(changed == 1)
    }
}

/// Append the facts of `journal` to the stream of the session `key` at `now_ms`, in the transaction
/// (or the savepoint of one) `connection` holds.
fn journal_in(
    connection: &Connection,
    key: &str,
    journal: &Journal<'_>,
    now_ms: u64,
) -> Result<(), Failure> {
    if journal.facts.is_empty() {
        return Ok(());
    }
    let observed: bool = connection
        .prepare_cached(
            "SELECT EXISTS (SELECT 1 FROM journal_entries WHERE name = ?1 AND key IS NULL)",
        )?
        .query_row((journal.machine,), |row| row.get(0))?;
    if observed {
        return Err(Failure::new(ObservedStream {
            name: journal.machine.to_string(),
        }));
    }
    let stream = Stream::Session {
        machine: journal.machine,
        key,
    };
    for fact in journal.facts {
        fact_in(connection, stream, None, fact, now_ms)?;
    }
    Ok(())
}

/// Append `fact` to `stream`, observed under `origin` when it is an observed one, at `now_ms`, in
/// the transaction `connection` holds; the position it was given.
///
/// The position is given inside that transaction, which holds the database's one write lock from
/// its start to its end ([`begin`]): no other transaction gives one until this one has committed
/// or rolled back, and one that rolls back gives back what it took, which no reader saw. So
/// positions are committed in their own order and become visible in it, which is why the horizon
/// is the last one committed ([`journal_horizon_of`]).
fn fact_in(
    connection: &Connection,
    stream: Stream<'_>,
    origin: Option<&str>,
    fact: &Fact<'_>,
    now_ms: u64,
) -> Result<u64, Failure> {
    let (issued, last_ms): (u64, u64) = connection
        .prepare_cached("SELECT issued, recorded_ms FROM journal_head WHERE id = 0")?
        .query_row([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    let position = issued + 1;
    let recorded_ms = recorded(last_ms, now_ms);
    connection
        .prepare_cached("UPDATE journal_head SET issued = ?1, recorded_ms = ?2 WHERE id = 0")?
        .execute((position, recorded_ms))?;
    let data = fact.data.at(position)?;
    let (name, key) = stream_columns(stream);
    connection
        .prepare_cached(
            "INSERT INTO journal_entries (position, recorded_ms, name, key, origin, schema, \
             revision, valid_ms, holds_ms, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )?
        .execute((
            position,
            recorded_ms,
            name,
            key,
            origin,
            fact.schema,
            fact.revision,
            fact.valid_ms,
            holds_from(fact.valid_ms, recorded_ms),
            &*data,
        ))?;
    Ok(position)
}

/// The journal's horizon in `connection`: the last position a committed transaction gave, since
/// this store writes one transaction at a time and gives a position inside it ([`fact_in`]), so
/// nothing can still appear at or below it. A read asks for it before it reads entries, and reads
/// none above it, so that one committed in between, by another connection, is left to the next.
fn journal_horizon_of(connection: &Connection) -> rusqlite::Result<u64> {
    connection
        .prepare_cached("SELECT issued FROM journal_head WHERE id = 0")?
        .query_row([], |row| row.get(0))
}

/// The columns `name` and `key` of the entries of `stream`.
fn stream_columns(stream: Stream<'_>) -> (&str, Option<&str>) {
    match stream {
        Stream::Session { machine, key } => (machine, Some(key)),
        Stream::Observed { name } => (name, None),
    }
}

/// An entry of the journal from its columns, read in the order `position, recorded_ms, name, key,
/// origin, schema, revision, valid_ms, data`.
fn journal_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<JournalEntry> {
    let name: String = row.get(2)?;
    let key: Option<String> = row.get(3)?;
    Ok(JournalEntry {
        position: row.get(0)?,
        recorded_ms: row.get(1)?,
        stream: match key {
            Some(key) => StreamName::Session { machine: name, key },
            None => StreamName::Observed { name },
        },
        origin: row.get(4)?,
        schema: row.get(5)?,
        revision: row.get(6)?,
        valid_ms: row.get(7)?,
        data: row.get(8)?,
    })
}

impl JournalStore for SqliteStore {
    fn after(&self, position: u64, limit: usize) -> Result<JournalPage, Failure> {
        let connection = self.connection();
        let horizon = journal_horizon_of(&connection)?;
        // A position past the horizon has nothing after it to read, and one past the largest
        // signed 64-bit number is none SQLite holds.
        if position >= horizon {
            return Ok(JournalPage {
                entries: Vec::new(),
                horizon,
            });
        }
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut statement = connection.prepare_cached(
            "SELECT position, recorded_ms, name, key, origin, schema, revision, valid_ms, data \
             FROM journal_entries WHERE position > ?1 AND position <= ?2 \
             ORDER BY position LIMIT ?3",
        )?;
        let entries = statement
            .query_map((position, horizon, limit), journal_row)?
            .collect::<rusqlite::Result<Vec<JournalEntry>>>()?;
        Ok(JournalPage { entries, horizon })
    }

    fn stream(&self, stream: Stream<'_>, after: u64, limit: usize) -> Result<JournalPage, Failure> {
        let connection = self.connection();
        let horizon = journal_horizon_of(&connection)?;
        if after >= horizon {
            return Ok(JournalPage {
                entries: Vec::new(),
                horizon,
            });
        }
        let (name, key) = stream_columns(stream);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut statement = connection.prepare_cached(
            "SELECT position, recorded_ms, name, key, origin, schema, revision, valid_ms, data \
             FROM journal_entries \
             WHERE name = ?1 AND key IS ?2 AND position > ?3 AND position <= ?4 \
             ORDER BY position LIMIT ?5",
        )?;
        let entries = statement
            .query_map((name, key, after, horizon, limit), journal_row)?
            .collect::<rusqlite::Result<Vec<JournalEntry>>>()?;
        Ok(JournalPage { entries, horizon })
    }

    fn position_at(&self, ms: u64) -> Result<u64, Failure> {
        let connection = self.connection();
        let horizon = journal_horizon_of(&connection)?;
        // The moments rise with the positions, since both are given in the order of the commits:
        // the latest moment's largest position is the largest of all.
        let position = connection
            .prepare_cached(
                "SELECT position FROM journal_entries \
                 WHERE recorded_ms <= ?1 AND position <= ?2 \
                 ORDER BY recorded_ms DESC, position DESC LIMIT 1",
            )?
            .query_row((moment(ms), horizon), |row| row.get(0))
            .optional()?;
        Ok(position.unwrap_or(0))
    }

    fn as_of(
        &self,
        stream: Stream<'_>,
        as_of: &AsOf,
        after: AsOfCursor,
        limit: usize,
    ) -> Result<JournalPage, AsOfError> {
        let connection = self.connection();
        let horizon = journal_horizon_of(&connection)?;
        as_of.admitted(horizon)?;
        let (name, key) = stream_columns(stream);
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let mut statement = connection.prepare_cached(
            "SELECT position, recorded_ms, name, key, origin, schema, revision, valid_ms, data \
             FROM journal_entries \
             WHERE name = ?1 AND key IS ?2 AND position <= ?3 \
             AND holds_ms >= ?5 AND holds_ms <= ?4 AND (holds_ms > ?5 OR position > ?6) \
             ORDER BY holds_ms, position LIMIT ?7",
        )?;
        let entries = statement
            .query_map(
                (
                    name,
                    key,
                    as_of.position,
                    as_of.valid_ms,
                    after.valid_ms,
                    // Past the largest signed 64-bit number there is no position to come after.
                    i64::try_from(after.position).unwrap_or(i64::MAX),
                    limit,
                ),
                journal_row,
            )?
            .collect::<rusqlite::Result<Vec<JournalEntry>>>()?;
        Ok(JournalPage { entries, horizon })
    }

    fn observe(
        &self,
        name: &str,
        origin: &str,
        fact: &Fact<'_>,
        now_ms: u64,
    ) -> Result<Observation, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let held: Option<u64> = transaction
            .prepare_cached("SELECT position FROM journal_entries WHERE name = ?1 AND origin = ?2")?
            .query_row((name, origin), |row| row.get(0))
            .optional()?;
        if let Some(position) = held {
            return Ok(Observation::Repeated { position });
        }
        let sessions: bool = transaction
            .prepare_cached(
                "SELECT EXISTS (SELECT 1 FROM journal_entries \
                 WHERE name = ?1 AND key IS NOT NULL)",
            )?
            .query_row((name,), |row| row.get(0))?;
        if sessions {
            return Err(Failure::new(SessionsStream {
                name: name.to_string(),
            }));
        }
        let position = fact_in(
            &transaction,
            Stream::Observed { name },
            Some(origin),
            fact,
            now_ms,
        )?;
        transaction.commit()?;
        Ok(Observation::Kept { position })
    }

    fn reseal_journaled(&self, position: u64, held: &[u8], data: &[u8]) -> Result<bool, Failure> {
        // A position past the largest signed 64-bit number is none SQLite holds.
        let Ok(position) = i64::try_from(position) else {
            return Ok(false);
        };
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let changed = transaction
            .prepare_cached(
                "UPDATE journal_entries SET data = ?3 WHERE position = ?1 AND data = ?2",
            )?
            .execute((position, held, data))?;
        transaction.commit()?;
        Ok(changed == 1)
    }

    fn read_through(&self, reader: &str) -> Result<u64, Failure> {
        let position: Option<u64> = self
            .connection()
            .prepare_cached("SELECT position FROM journal_readers WHERE reader = ?1")?
            .query_row((reader,), |row| row.get(0))
            .optional()?;
        Ok(position.unwrap_or(0))
    }

    fn keep_read(&self, reader: &str, position: u64) -> Result<u64, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        let horizon = journal_horizon_of(&transaction)?;
        if position > horizon {
            return Err(Failure::new(PastHorizon {
                asked: position,
                horizon,
            }));
        }
        let held: Option<u64> = transaction
            .prepare_cached("SELECT position FROM journal_readers WHERE reader = ?1")?
            .query_row((reader,), |row| row.get(0))
            .optional()?;
        let kept = held.map_or(position, |held| held.max(position));
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO journal_readers (reader, position) VALUES (?1, ?2)",
            )?
            .execute((reader, kept))?;
        transaction.commit()?;
        Ok(kept)
    }
}

impl From<rusqlite::Error> for Failure {
    fn from(error: rusqlite::Error) -> Self {
        Self::new(error)
    }
}

impl From<rusqlite::Error> for WriteError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Failed(error.into())
    }
}

impl From<rusqlite::Error> for AcquireError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Failed(error.into())
    }
}

impl From<rusqlite::Error> for LeaseError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Failed(error.into())
    }
}

impl From<rusqlite::Error> for AsOfError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Failed(error.into())
    }
}
