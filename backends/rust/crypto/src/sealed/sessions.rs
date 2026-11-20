// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The sessions of ADR 0107 sealed (ADR 0109, D8): a session's row is kept under the entry of its
//! key, and its value is the record of the payload `[key, state]`, a CBOR array of the key's text
//! and the saved state's bytes. Its version, earliest due and lease are the wrapped store's, which
//! compares and orders them. What its write appends to an outbox is sealed as module `outbox`
//! says, in the same write.

use sce_forge_runtime::cbor::{read_bytes, read_text, write_array_head, write_bytes, write_text};
use sce_rust_session_store::{
    AcquireError, Append, Appended, Batched, Change, Committed, Decided, Due, Expiry, Failure,
    Lease, LeaseError, Note, Outbox, SessionStore, Stored, Write, WriteError, Written,
};
use zeroize::Zeroizing;

use super::outbox::SealedAppend;
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

/// A write as the wrapped store keeps it: the entry of its session's key, the session's record
/// when it keeps one, its appends sealed (module `outbox`), and the entries of the sessions a
/// removal ends, as the wrapped store keeps their identities (module `kv`).
struct SealedWrite {
    entry: String,
    record: Option<Vec<u8>>,
    appends: Vec<SealedAppend>,
    ended: Vec<String>,
}

impl SealedWrite {
    /// The appends the wrapped store is given, each in the incarnation `write`'s append names.
    fn appends<'a>(&'a self, write: &Write<'a>) -> Vec<Append<'a>> {
        self.appends
            .iter()
            .zip(write.outbox)
            .map(|(held, append)| held.append(append.incarnation))
            .collect()
    }

    /// The entries of the sessions a removal ends.
    fn ended(&self) -> Vec<&str> {
        self.ended.iter().map(String::as_str).collect()
    }

    /// The write the wrapped store is given: `write`, with the session's record, `appends` and
    /// `ended` in place of what it names in the clear.
    fn given<'a>(
        &'a self,
        write: &Write<'a>,
        appends: &'a [Append<'a>],
        ended: &'a [&'a str],
    ) -> Write<'a> {
        let change = match (write.change, &self.record) {
            (Change::Keep { earliest_due, .. }, Some(record)) => Change::Keep {
                state: record,
                earliest_due,
            },
            _ => Change::Remove,
        };
        Write {
            change,
            outbox: appends,
            sessions: ended,
            ..*write
        }
    }
}

/// An item of a batch as the wrapped store keeps it, with what it names in the clear.
enum SealedItem<'a> {
    Write {
        held: SealedWrite,
        write: Write<'a>,
    },
    /// An acknowledgement of the outbox of the client whose entry is `client`.
    Acknowledge {
        client: String,
        incarnation: &'a str,
        through: u64,
        now_ms: u64,
    },
}

impl<S: SessionStore> Sealed<S> {
    /// `write` to the session `key` names, as the wrapped store keeps it.
    fn seal_write(&self, key: &str, write: &Write<'_>) -> Result<SealedWrite, Failure> {
        let entry = self.digest_of(key);
        let record = match write.change {
            Change::Keep { state, .. } => {
                Some(self.seal(STORE_SESSIONS, entry.as_bytes(), &payload(key, state))?)
            }
            Change::Remove => None,
        };
        Ok(SealedWrite {
            appends: self.seal_appends(write.outbox)?,
            ended: write
                .sessions
                .iter()
                .map(|session| self.digest_of(session))
                .collect(),
            entry,
            record,
        })
    }

    /// `item` of a batch, as the wrapped store keeps it.
    fn seal_item<'a>(&self, item: &Batched<'a>) -> Result<SealedItem<'a>, Failure> {
        Ok(match *item {
            Batched::Write { key, write } => SealedItem::Write {
                held: self.seal_write(key, &write)?,
                write,
            },
            Batched::Acknowledge {
                client,
                incarnation,
                through,
                now_ms,
            } => SealedItem::Acknowledge {
                client: self.digest_of(client),
                incarnation,
                through,
                now_ms,
            },
        })
    }

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
        let sealed = self.seal_write(key, write)?;
        let appends = sealed.appends(write);
        let ended = sealed.ended();
        let written = self
            .inner
            .write(&sealed.entry, &sealed.given(write, &appends, &ended))?;
        Ok(Written {
            version: written.version,
            appended: Self::named(written.appended, write.outbox),
        })
    }

    /// Every item sealed as [`Self::write`] and [`Self::acknowledge`] seal it, handed to the
    /// wrapped store as one batch (ADR 0109, D15), which decides each item; a write's appends
    /// are answered under their clients' names.
    fn write_many(&self, batch: &[Batched<'_>]) -> Result<Vec<Decided>, Failure> {
        let sealed = batch
            .iter()
            .map(|item| self.seal_item(item))
            .collect::<Result<Vec<SealedItem<'_>>, Failure>>()?;
        let appends: Vec<Vec<Append<'_>>> = sealed
            .iter()
            .map(|item| match item {
                SealedItem::Write { held, write } => held.appends(write),
                SealedItem::Acknowledge { .. } => Vec::new(),
            })
            .collect();
        let ended: Vec<Vec<&str>> = sealed
            .iter()
            .map(|item| match item {
                SealedItem::Write { held, .. } => held.ended(),
                SealedItem::Acknowledge { .. } => Vec::new(),
            })
            .collect();
        let given: Vec<Batched<'_>> = sealed
            .iter()
            .zip(appends.iter().zip(&ended))
            .map(|(item, (appends, ended))| match item {
                SealedItem::Write { held, write } => Batched::Write {
                    key: &held.entry,
                    write: held.given(write, appends, ended),
                },
                SealedItem::Acknowledge {
                    client,
                    incarnation,
                    through,
                    now_ms,
                } => Batched::Acknowledge {
                    client,
                    incarnation,
                    through: *through,
                    now_ms: *now_ms,
                },
            })
            .collect();
        let decided = self.inner.write_many(&given)?;
        if decided.len() != batch.len() {
            return Err(Failure::new(format!(
                "the wrapped store answered {} items of a batch of {}",
                decided.len(),
                batch.len()
            )));
        }
        Ok(decided
            .into_iter()
            .zip(batch)
            .map(|(decided, item)| match (decided, item) {
                (Ok(Committed::Written(written)), Batched::Write { write, .. }) => {
                    Ok(Committed::Written(Written {
                        version: written.version,
                        appended: Self::named(written.appended, write.outbox),
                    }))
                }
                (decided, _) => decided,
            })
            .collect())
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

    /// The outboxes as the wrapped store keeps them: the entries of their clients' names, which
    /// `SessionStore::indexed` answers for a name.
    fn outbox_clients_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<String>, Failure> {
        self.inner.outbox_clients_after(after, limit)
    }

    fn reseal_entry(
        &self,
        client: &str,
        number: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        self.reseal_entry_sealed(client, number, held, envelope)
    }

    fn reseal_note(
        &self,
        key: &str,
        id: u64,
        held: &[u8],
        envelope: &[u8],
    ) -> Result<bool, Failure> {
        self.reseal_note_sealed(key, id, held, envelope)
    }
}
