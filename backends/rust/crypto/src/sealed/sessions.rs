// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The sessions of ADR 0107 sealed (ADR 0109, D8): a session's row is kept under the entry of its
//! key, and its value is the record of the payload `[key, state]`, a CBOR array of the key's text
//! and the saved state's bytes. Its version, earliest due and lease are the wrapped store's, which
//! compares and orders them. What its write appends to an outbox is sealed as module `outbox`
//! says, in the same write.

use sce_forge_runtime::cbor::{read_bytes, read_text, write_array_head, write_bytes, write_text};
use sce_rust_session_store::{
    AcquireError, Append, Appended, Change, Due, Expiry, Failure, Lease, LeaseError, Note, Outbox,
    SessionStore, Stored, Write, WriteError, Written,
};
use zeroize::Zeroizing;

use super::{decoded, encoded, expect_items, Reseal, Resealed, SealError, Sealed, STORE_SESSIONS};
use crate::cose::put;

/// How many sessions one page of a re-seal pass lists.
const RESEAL_PAGE: usize = 256;

/// The payload of the session `key` whose saved state is `state`.
fn payload(key: &str, state: &[u8]) -> Zeroizing<Vec<u8>> {
    encoded(key.len() + state.len() + 18, |w| {
        put(write_array_head(w, 2));
        put(write_text(w, key));
        put(write_bytes(w, state));
    })
}

/// The key and the saved state a session's payload holds.
fn session_of(plaintext: &[u8]) -> Result<(String, Vec<u8>), SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 2)?;
        let key = read_text(c, None)?.to_string();
        let state = read_bytes(c, None)?.to_vec();
        Ok((key, state))
    })
}

impl<S: SessionStore> Sealed<S> {
    /// The key and the saved state the record `record` of the entry `entry` holds.
    fn open_session(&self, entry: &str, record: &[u8]) -> Result<(String, Vec<u8>), Failure> {
        let plaintext = self.open(STORE_SESSIONS, entry.as_bytes(), record)?;
        session_of(&plaintext).map_err(Failure::from)
    }

    /// Write every session sealed under a key other than the active one again under the active
    /// one, each through the wrapped store's put: under a lease on its key that the pass takes as
    /// `owner` for `lease_ttl_ms` from `now_ms` and gives back, from the version it read. A
    /// session another process holds is left for a later pass, as is one written between the
    /// pass's read and its put; the lease the pass takes makes a process that served the key
    /// load it again. A session that does not open is counted and left.
    pub fn reseal_sessions(
        &self,
        owner: &str,
        lease_ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        let mut after: Option<String> = None;
        loop {
            let page = self.inner.indexed_after(after.as_deref(), RESEAL_PAGE)?;
            let Some(last) = page.last().cloned() else {
                break;
            };
            for listed in &page {
                // The entry as the wrapped store keys it: the digest this store made.
                let Some(entry) = self.inner.key_of(listed)? else {
                    continue;
                };
                let Some(stored) = self.inner.get(&entry)? else {
                    continue;
                };
                if self.is_current(&stored.state) {
                    continue;
                }
                let lease = match self.inner.acquire(&entry, owner, lease_ttl_ms, now_ms) {
                    Ok(lease) => lease,
                    Err(AcquireError::Held(_)) => {
                        resealed.count(false);
                        continue;
                    }
                    Err(AcquireError::Failed(failure)) => return Err(failure),
                };
                let outcome = self.reseal_session(&entry, lease.fence);
                let released = self.inner.release(&entry, lease.fence);
                match outcome? {
                    Some(written) => resealed.count(written),
                    None => resealed.refused += 1,
                }
                if let Err(LeaseError::Failed(failure)) = released {
                    return Err(failure);
                }
            }
            after = Some(last);
        }
        Ok(resealed)
    }

    /// Write the session of `entry` again under the active key, under the lease of fence `fence`:
    /// whether it was written, `Some(false)` when it changed or went since it was listed, and
    /// `None` when it does not open.
    fn reseal_session(&self, entry: &str, fence: u64) -> Result<Option<bool>, Failure> {
        let Some(stored) = self.inner.get(entry)? else {
            return Ok(Some(false));
        };
        let record = match self.reseal_record(STORE_SESSIONS, entry.as_bytes(), &stored.state)? {
            Reseal::Current => return Ok(Some(false)),
            Reseal::Refused => return Ok(None),
            Reseal::Record(record) => record,
        };
        match self
            .inner
            .put(entry, &record, stored.earliest_due, stored.version, fence)
        {
            Ok(_) => Ok(Some(true)),
            Err(WriteError::Conflict(_)) => Ok(Some(false)),
            Err(WriteError::Failed(failure)) => Err(failure),
        }
    }
}

impl<S: SessionStore> SessionStore for Sealed<S> {
    fn get(&self, key: &str) -> Result<Option<Stored>, Failure> {
        let entry = self.digest_of(key);
        let Some(stored) = self.inner.get(&entry)? else {
            return Ok(None);
        };
        let (held, state) = self.open_session(&entry, &stored.state)?;
        if held != key {
            return Err(SealError::Refused.into());
        }
        Ok(Some(Stored {
            state,
            version: stored.version,
            earliest_due: stored.earliest_due,
        }))
    }

    /// The session sealed under the entry of its key, and each append sealed for its client's
    /// entry (module `outbox`), in one write of the wrapped store.
    fn write(&self, key: &str, write: &Write<'_>) -> Result<Written, WriteError> {
        let entry = self.digest_of(key);
        let record = match write.change {
            Change::Keep { state, .. } => {
                Some(self.seal(STORE_SESSIONS, entry.as_bytes(), &payload(key, state))?)
            }
            Change::Remove => None,
        };
        let change = match (write.change, &record) {
            (Change::Keep { earliest_due, .. }, Some(record)) => Change::Keep {
                state: record,
                earliest_due,
            },
            _ => Change::Remove,
        };
        let sealed = self.seal_appends(write.outbox)?;
        let appends: Vec<Append<'_>> = sealed
            .iter()
            .zip(write.outbox)
            .map(|(held, append)| held.append(append.incarnation))
            .collect();
        // The sessions whose first results go with a removal, as the wrapped store keeps their
        // identities: each the entry of its id (module `kv`).
        let ended: Vec<String> = write
            .sessions
            .iter()
            .map(|session| self.digest_of(session))
            .collect();
        let ended: Vec<&str> = ended.iter().map(String::as_str).collect();
        let written = self.inner.write(
            &entry,
            &Write {
                change,
                outbox: &appends,
                sessions: &ended,
                ..*write
            },
        )?;
        Ok(Written {
            version: written.version,
            appended: Self::named(written.appended, write.outbox),
        })
    }

    fn due_before(&self, ms: u64, limit: usize) -> Result<Vec<Due>, Failure> {
        self.inner.due_before(ms, limit)
    }

    fn indexed(&self, key: &str) -> String {
        self.inner.indexed(&self.digest_of(key))
    }

    fn key_of(&self, indexed: &str) -> Result<Option<String>, Failure> {
        let Some(entry) = self.inner.key_of(indexed)? else {
            return Ok(None);
        };
        let Some(stored) = self.inner.get(&entry)? else {
            return Ok(None);
        };
        let (key, _) = self.open_session(&entry, &stored.state)?;
        Ok(Some(key))
    }

    fn indexed_after(&self, after: Option<&str>, limit: usize) -> Result<Vec<String>, Failure> {
        self.inner.indexed_after(after, limit)
    }

    fn acquire(
        &self,
        key: &str,
        owner: &str,
        ttl_ms: u64,
        now_ms: u64,
    ) -> Result<Lease, AcquireError> {
        self.inner
            .acquire(&self.digest_of(key), owner, ttl_ms, now_ms)
    }

    fn renew(&self, key: &str, fence: u64, ttl_ms: u64, now_ms: u64) -> Result<Lease, LeaseError> {
        self.inner
            .renew(&self.digest_of(key), fence, ttl_ms, now_ms)
    }

    fn release(&self, key: &str, fence: u64) -> Result<(), LeaseError> {
        self.inner.release(&self.digest_of(key), fence)
    }

    fn append(&self, appends: &[Append<'_>], now_ms: u64) -> Result<Vec<Appended>, Failure> {
        self.append_sealed(appends, now_ms)
    }

    fn outbox(&self, client: &str) -> Result<Option<Outbox>, Failure> {
        self.outbox_sealed(client)
    }

    fn open_outbox(&self, client: &str, incarnation: &str, now_ms: u64) -> Result<Outbox, Failure> {
        self.open_outbox_sealed(client, incarnation, now_ms)
    }

    fn acknowledge(
        &self,
        client: &str,
        incarnation: &str,
        through: u64,
        now_ms: u64,
    ) -> Result<u64, Failure> {
        self.inner
            .acknowledge(&self.digest_of(client), incarnation, through, now_ms)
    }

    fn expire(&self, client: &str, before_ms: u64, limit: usize) -> Result<Expiry, Failure> {
        let expiry = self
            .inner
            .expire(&self.digest_of(client), before_ms, limit)?;
        Ok(Expiry {
            expired: expiry
                .expired
                .into_iter()
                .map(|expired| self.opened_expired(client, expired))
                .collect(),
            closed: expiry.closed,
        })
    }

    fn notes(&self, key: &str) -> Result<Vec<Note>, Failure> {
        self.inner
            .notes(&self.digest_of(key))?
            .into_iter()
            .map(|note| self.opened_note(note))
            .collect()
    }
}
