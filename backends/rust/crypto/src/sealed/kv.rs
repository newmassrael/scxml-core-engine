// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The values of the key-value feature (ADR 0109, D13) sealed (D8).
//!
//! - A value is kept under the entry of its key, and its value is the record of the payload
//!   `[key, value]`, a CBOR array of the key's text and the value's bytes. Its version is the
//!   wrapped store's, which numbers it and compares it.
//! - A first result is kept under the identity whose session and invoke are each the entry of
//!   their text, so neither is in the clear on the medium. Its outcome is the wrapped store's: a
//!   version and whether it wrote, which the store decides in the transaction that writes, and
//!   which hold no value of a document.
//! - The wrapped store orders its keys by their entries, which say nothing of the keys' order, so
//!   a walk (`next_key`) and a listing (`entries_after`) open every value the store holds and
//!   order the keys they hold: their cost grows with the store. A store that needs a fast walk over
//!   keys that are not secret keeps them unsealed under an encryption of its medium.

use std::collections::BTreeMap;

use sce_forge_runtime::cbor::{read_bytes, read_text, write_array_head, write_bytes, write_text};
use sce_rust_session_store::kv::rules;
use sce_rust_session_store::{Failure, Identity, KvEntry, KvStore, Put, PutIf};
use zeroize::Zeroizing;

use super::{decoded, encoded, expect_items, Reseal, Resealed, SealError, Sealed};
use crate::cose::put;

/// The name the values of a `KvStore` are sealed for.
pub const STORE_KV_VALUES: &str = "kv_values";

/// How many values one page of a pass over the wrapped store lists.
const PAGE: usize = 256;

/// The payload of the value `value` of `key`.
fn payload(key: &str, value: &[u8]) -> Zeroizing<Vec<u8>> {
    encoded(key.len() + value.len() + 18, |w| {
        put(write_array_head(w, 2));
        put(write_text(w, key));
        put(write_bytes(w, value));
    })
}

/// The key and the value a value's payload holds.
fn value_of(plaintext: &[u8]) -> Result<(String, Vec<u8>), SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 2)?;
        let key = read_text(c, None)?.to_string();
        let value = read_bytes(c, None)?.to_vec();
        Ok((key, value))
    })
}

/// An identity as the wrapped store keeps it: its session and its invoke each as its entry.
struct Indexed {
    session: String,
    invoke: String,
    start: u64,
}

impl Indexed {
    fn identity(&self) -> Identity<'_> {
        Identity {
            session: &self.session,
            invoke: &self.invoke,
            start: self.start,
        }
    }
}

impl<S> Sealed<S> {
    /// The identity the wrapped store keeps `identity`'s first result under.
    fn indexed_identity(&self, identity: &Identity<'_>) -> Indexed {
        Indexed {
            session: self.digest_of(identity.session),
            invoke: self.digest_of(identity.invoke),
            start: identity.start,
        }
    }

    /// The key and the value the record `record` of the entry `entry` holds.
    fn open_value(&self, entry: &str, record: &[u8]) -> Result<(String, Vec<u8>), Failure> {
        let plaintext = self.open(STORE_KV_VALUES, entry.as_bytes(), record)?;
        value_of(&plaintext).map_err(Failure::from)
    }
}

impl<S: KvStore> Sealed<S> {
    /// Every value the wrapped store holds, opened, by its key.
    fn opened_values(&self) -> Result<BTreeMap<String, KvEntry>, Failure> {
        let mut values = BTreeMap::new();
        let mut after: Option<String> = None;
        loop {
            let page = self.inner.entries_after(after.as_deref(), PAGE)?;
            let Some((last, _)) = page.last() else {
                break;
            };
            let last = last.clone();
            for (entry, held) in page {
                let (key, value) = self.open_value(&entry, &held.value)?;
                if self.digest_of(&key) != entry {
                    return Err(SealError::Refused.into());
                }
                values.insert(
                    key,
                    KvEntry {
                        value,
                        version: held.version,
                    },
                );
            }
            after = Some(last);
        }
        Ok(values)
    }

    /// Write every value sealed under a key other than the active one again under the active one,
    /// each by the wrapped store's replacement from the bytes the pass read, its version kept: one
    /// another process changed in between is left for a later pass. One that does not open is
    /// counted and left.
    pub fn reseal_values(&self) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        let mut after: Option<String> = None;
        loop {
            let page = self.inner.entries_after(after.as_deref(), PAGE)?;
            let Some((last, _)) = page.last() else {
                break;
            };
            let last = last.clone();
            for (entry, held) in page {
                match self.reseal_record(STORE_KV_VALUES, entry.as_bytes(), &held.value)? {
                    Reseal::Current => {}
                    Reseal::Refused => resealed.refused += 1,
                    Reseal::Record(record) => {
                        resealed.count(self.inner.reseal(&entry, &held.value, &record)?);
                    }
                }
            }
            after = Some(last);
        }
        Ok(resealed)
    }
}

impl<S: KvStore> KvStore for Sealed<S> {
    fn get(&self, key: &str) -> Result<Option<KvEntry>, Failure> {
        let entry = self.digest_of(key);
        let Some(held) = self.inner.get(&entry)? else {
            return Ok(None);
        };
        let (opened, value) = self.open_value(&entry, &held.value)?;
        if opened != key {
            return Err(SealError::Refused.into());
        }
        Ok(Some(KvEntry {
            value,
            version: held.version,
        }))
    }

    fn put(&self, key: &str, value: &[u8], quota: Option<u64>) -> Result<Put, Failure> {
        let entry = self.digest_of(key);
        let record = self.seal(STORE_KV_VALUES, entry.as_bytes(), &payload(key, value))?;
        self.inner.put(&entry, &record, quota)
    }

    fn put_if(
        &self,
        key: &str,
        value: &[u8],
        expected: Option<u64>,
        quota: Option<u64>,
        identity: Option<&Identity<'_>>,
    ) -> Result<PutIf, Failure> {
        let entry = self.digest_of(key);
        let record = self.seal(STORE_KV_VALUES, entry.as_bytes(), &payload(key, value))?;
        let indexed = identity.map(|identity| self.indexed_identity(identity));
        let identity = indexed.as_ref().map(Indexed::identity);
        self.inner
            .put_if(&entry, &record, expected, quota, identity.as_ref())
    }

    fn delete(&self, key: &str) -> Result<bool, Failure> {
        self.inner.delete(&self.digest_of(key))
    }

    fn next_key(&self, after: &str, prefix: &str) -> Result<Option<String>, Failure> {
        Ok(self
            .opened_values()?
            .into_keys()
            .find(|key| rules::walks_to(key, after, prefix)))
    }

    fn entries_after(
        &self,
        after: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, KvEntry)>, Failure> {
        Ok(self
            .opened_values()?
            .into_iter()
            .filter(|(key, _)| after.is_none_or(|after| key.as_str() > after))
            .take(limit)
            .collect())
    }

    fn first_result(&self, identity: &Identity<'_>) -> Result<Option<PutIf>, Failure> {
        self.inner
            .first_result(&self.indexed_identity(identity).identity())
    }

    fn keep_first(&self, identity: &Identity<'_>, result: &PutIf) -> Result<PutIf, Failure> {
        self.inner
            .keep_first(&self.indexed_identity(identity).identity(), result)
    }

    fn reseal(&self, key: &str, held: &[u8], value: &[u8]) -> Result<bool, Failure> {
        let entry = self.digest_of(key);
        let Some(row) = self.inner.get(&entry)? else {
            return Ok(false);
        };
        let (opened, kept) = self.open_value(&entry, &row.value)?;
        if opened != key || kept != held {
            return Ok(false);
        }
        let replacement = self.seal(STORE_KV_VALUES, entry.as_bytes(), &payload(key, value))?;
        self.inner.reseal(&entry, &row.value, &replacement)
    }
}
