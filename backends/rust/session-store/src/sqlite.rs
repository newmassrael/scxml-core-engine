// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The reference session store: SQLite in WAL mode.

use std::path::Path;
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, Transaction, TransactionBehavior};

use crate::credentials::{
    CredentialStore, Digest, PinStore, Revoked, TokenRow, TokenStore, DIGEST_BYTES,
};
use crate::keys::{KeyRow, KeyState, KeyStore, KeyUse};
use crate::rules::{self, Holding};
use crate::{
    moment, AcquireError, Due, Failure, Lease, LeaseError, SessionStore, Stored, WriteError,
};

/// The layout this store writes, kept in the database's `user_version`; a database of a layout a
/// later build wrote is refused rather than read as this one, and one of an earlier layout is
/// brought to this one when it is opened, in one transaction.
const LAYOUT_VERSION: i64 = 3;

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

/// Make the tables in a database that has none, bring one of layout 1 or 2 to [`LAYOUT_VERSION`],
/// or check that the ones there are this store's. A database is changed in one transaction, so a
/// failed upgrade leaves it at the layout it had.
fn lay_out(connection: &mut Connection) -> Result<(), Failure> {
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
            transaction.execute_batch(LAYOUT_1)?;
            transaction.execute_batch(LAYOUT_2)?;
            transaction.execute_batch(LAYOUT_3)?;
        }
        1 => {
            transaction.execute_batch(LAYOUT_2)?;
            transaction.execute_batch(LAYOUT_3)?;
        }
        2 => transaction.execute_batch(LAYOUT_3)?,
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

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        let connection = self.connection();
        let limit = i64::try_from(limit).unwrap_or(i64::MAX);
        // A key is TEXT compared by the BINARY collation, which is the order of its UTF-8 bytes.
        let listed = match after {
            Some(after) => {
                let mut statement = connection.prepare_cached(
                    "SELECT session FROM sessions WHERE session > ?1 ORDER BY session LIMIT ?2",
                )?;
                let rows = statement.query_map((after, limit), |row| row.get(0))?;
                rows.collect::<rusqlite::Result<Vec<String>>>()?
            }
            None => {
                let mut statement = connection
                    .prepare_cached("SELECT session FROM sessions ORDER BY session LIMIT ?1")?;
                let rows = statement.query_map((limit,), |row| row.get(0))?;
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
type TokenColumns = (Vec<u8>, String, u64, u64, bool);

/// A token row from its columns.
fn token_row(
    (digest, name, issued_ms, expires_ms, revoked): TokenColumns,
) -> Result<TokenRow, Failure> {
    Ok(TokenRow {
        digest: digest_of(digest)?,
        name,
        issued_ms,
        expires_ms,
        revoked,
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
    ))
}

/// The token whose digest is `digest` in `connection`, if it holds one.
fn token_of(connection: &Connection, digest: &Digest) -> Result<Option<TokenRow>, Failure> {
    let columns = connection
        .prepare_cached(
            "SELECT digest, name, issued_ms, expires_ms, revoked FROM credential_tokens \
             WHERE digest = ?1",
        )?
        .query_row((&digest[..],), token_columns)
        .optional()?;
    columns.map(token_row).transpose()
}

/// The digest `name` is pinned to in `connection`, if it is pinned.
fn pin_of(connection: &Connection, name: &str) -> Result<Option<Digest>, Failure> {
    let column: Option<Vec<u8>> = connection
        .prepare_cached("SELECT digest FROM credential_pins WHERE name = ?1")?
        .query_row((name,), |row| row.get(0))
        .optional()?;
    column.map(digest_of).transpose()
}

impl PinStore for SqliteStore {
    fn pinned(&self, name: &str) -> Result<Option<Digest>, Failure> {
        pin_of(&self.connection(), name)
    }

    fn pin(&self, name: &str, digest: &Digest) -> Result<Digest, Failure> {
        let mut connection = self.connection();
        let transaction = begin(&mut connection)?;
        if let Some(held) = pin_of(&transaction, name)? {
            return Ok(held);
        }
        transaction
            .prepare_cached("INSERT INTO credential_pins (name, digest) VALUES (?1, ?2)")?
            .execute((name, &digest[..]))?;
        transaction.commit()?;
        Ok(*digest)
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
                "INSERT INTO credential_tokens (digest, name, issued_ms, expires_ms, revoked) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?
            .execute((
                &kept.digest[..],
                &kept.name,
                kept.issued_ms,
                kept.expires_ms,
                kept.revoked,
            ))?;
        transaction.commit()?;
        Ok(kept)
    }

    fn tokens(&self) -> Result<Vec<TokenRow>, Failure> {
        let connection = self.connection();
        let mut statement = connection.prepare_cached(
            "SELECT digest, name, issued_ms, expires_ms, revoked FROM credential_tokens \
             ORDER BY digest",
        )?;
        let columns = statement
            .query_map([], token_columns)?
            .collect::<rusqlite::Result<Vec<TokenColumns>>>()?;
        columns.into_iter().map(token_row).collect()
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
