// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference session store: SQLite in WAL mode.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// The layout this store writes, kept in the database's `user_version`; a database of a layout a
/// later build wrote is refused rather than read as this one.
const LAYOUT_VERSION: i64 = 1;

/// The tables of [`LAYOUT_VERSION`]. A session's state, version and earliest due are one row, so
/// that the index of dues, which SQLite keeps in the transaction that writes the row, cannot
/// disagree with the states. A lease is a row of its own, outliving a deleted session; the numbers
/// handed out come from the one row of `numbers`, so that a version or a fence is never given
/// twice, even for a key whose session or lease was removed and made again.
const LAYOUT: &str = "
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

/// A session store in one SQLite database file.
///
/// The database is in WAL mode, so that reads go on while a write is made, with
/// `synchronous=FULL`, so that the log is synced to the disk before a commit returns: a put that
/// returned survives a power loss, which is what lets the host core release a reply only once the
/// checkpoint that produced it is written (ADR 0107, D5).
///
/// A handle is one connection. Several handles may be open on one file, in one process or in
/// several, which is how two server processes share a store; SQLite's locks keep their
/// transactions apart, and the leases' fences keep their sessions apart. Every write is an
/// `IMMEDIATE` transaction: it takes the database's write lock before it reads the lease and the
/// version it checks, so that what it read is still what is there when it writes. A deferred
/// transaction would read first and, when another connection committed in between, fail at its
/// first write in a way the busy timeout cannot wait out.
pub struct SqliteStore {
    connection: Mutex<Connection>,
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
            connection: Mutex::new(connection),
        })
    }

    /// The connection, whatever a thread that panicked while it held it left: nothing, since a
    /// transaction that is dropped unfinished, as unwinding drops it, is rolled back.
    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// Make the tables in a database that has none, or check that the ones there are this store's.
fn lay_out(connection: &mut Connection) -> Result<(), Failure> {
    let transaction = begin(connection)?;
    let layout: i64 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if layout == LAYOUT_VERSION {
        return Ok(());
    }
    if layout != 0 {
        return Err(Failure::new(format!(
            "the database is laid out as session store {layout}, which this build does not know"
        )));
    }
    let tables: i64 =
        transaction.query_row("SELECT count(*) FROM sqlite_master", [], |row| row.get(0))?;
    if tables != 0 {
        return Err(Failure::new(
            "the database holds tables a session store did not make",
        ));
    }
    transaction.execute_batch(LAYOUT)?;
    transaction.pragma_update(None, "user_version", LAYOUT_VERSION)?;
    transaction.commit()?;
    Ok(())
}

/// A transaction that holds the write lock from its start (see [`SqliteStore`]).
fn begin(connection: &mut Connection) -> rusqlite::Result<Transaction<'_>> {
    connection.transaction_with_behavior(TransactionBehavior::Immediate)
}

/// The lease held on `key`, if one is.
fn holding(connection: &Connection, key: &str) -> rusqlite::Result<Option<Holding>> {
    connection
        .prepare_cached("SELECT owner, fence, expires_ms FROM leases WHERE session = ?1")?
        .query_row((key,), |row| {
            Ok(Holding {
                owner: row.get(0)?,
                fence: row.get(1)?,
                expires_ms: row.get(2)?,
            })
        })
        .optional()
}

/// The version of the session `key` names, `0` when there is none.
fn version_of(connection: &Connection, key: &str) -> rusqlite::Result<u64> {
    let version = connection
        .prepare_cached("SELECT version FROM sessions WHERE session = ?1")?
        .query_row((key,), |row| row.get(0))
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
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        let connection = self.connection();
        let stored = connection
            .prepare_cached("SELECT state, version, earliest_due FROM sessions WHERE session = ?1")?
            .query_row((key,), |row| {
                Ok(Stored {
                    state: row.get(0)?,
                    version: row.get(1)?,
                    earliest_due: row.get(2)?,
                })
            })
            .optional()?;
        Ok(stored)
    }

    fn put(
        &self,
        key: &str,
        state: &[u8],
        earliest_due: Option<u64>,
        expected_version: u64,
        fence: u64,
    ) -> Result<u64, WriteError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_write(
            holding(&transaction, key)?.as_ref(),
            fence,
            version_of(&transaction, key)?,
            expected_version,
        )?;
        let version = next_number(&transaction)?;
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO sessions (session, state, version, earliest_due) \
                 VALUES (?1, ?2, ?3, ?4)",
            )?
            .execute((key, state, version, earliest_due.map(moment)))?;
        transaction.commit()?;
        Ok(version)
    }

    fn delete(&self, key: &str, expected_version: u64, fence: u64) -> Result<(), WriteError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_write(
            holding(&transaction, key)?.as_ref(),
            fence,
            version_of(&transaction, key)?,
            expected_version,
        )?;
        transaction
            .prepare_cached("DELETE FROM sessions WHERE session = ?1")?
            .execute((key,))?;
        transaction.commit()?;
        Ok(())
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT session, earliest_due FROM sessions \
             WHERE earliest_due IS NOT NULL AND earliest_due <= ?1 \
             ORDER BY earliest_due, session LIMIT ?2",
        )?;
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        let listed = statement
            .query_map((moment(ms), limit), |row| {
                Ok(Due {
                    key: row.get(0)?,
                    due: row.get(1)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<Due>>>()?;
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
        rules::admit_acquire(holding(&transaction, key)?.as_ref(), owner, now_ms)?;
        let lease = Lease {
            fence: next_number(&transaction)?,
            expires_ms: rules::expiry(now_ms, ttl_ms),
        };
        transaction
            .prepare_cached(
                "INSERT OR REPLACE INTO leases (session, owner, fence, expires_ms) \
                 VALUES (?1, ?2, ?3, ?4)",
            )?
            .execute((key, owner, lease.fence, lease.expires_ms))?;
        transaction.commit()?;
        Ok(lease)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_renew(holding(&transaction, key)?.as_ref(), fence, now_ms)?;
        let expires_ms = rules::expiry(now_ms, ttl_ms);
        transaction
            .prepare_cached("UPDATE leases SET expires_ms = ?2 WHERE session = ?1")?
            .execute((key, expires_ms))?;
        transaction.commit()?;
        Ok(Lease { fence, expires_ms })
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        rules::admit_release(holding(&transaction, key)?.as_ref(), fence)?;
        transaction
            .prepare_cached("DELETE FROM leases WHERE session = ?1")?
            .execute((key,))?;
        transaction.commit()?;
        Ok(())
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
