// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The keys of ADR 0109, D7, sealed (D8).
//!
//! A key is kept under the entry of its id, and its wrapped material is the record of the payload
//! `[id, label, wrapped]`; its label in the wrapped store is empty, and its use, algorithm,
//! creation and state are the wrapped store's. A generation is kept under the entry of its
//! identity, naming the entry of its key. The naming key is the record of its wrapped bytes, for
//! the one row of an empty entry.
//!
//! The sealing store's own keys ([`is_sealing_key_id`]) are kept as they are given: they open every
//! other row, so they cannot be sealed under themselves, and they are wrapped under the
//! key-encryption key the program holds apart from the store.

use sce_forge_runtime::cbor::{read_bytes, read_text, write_array_head, write_bytes, write_text};
use sce_rust_session_store::{Failure, KeyRow, KeyStore};
use zeroize::Zeroizing;

use super::{
    decoded, encoded, expect_items, is_sealing_key_id, Reseal, Resealed, SealError, Sealed,
    STORE_KEYS, STORE_KEY_NAMING,
};
use crate::cose::put;

/// The entry of the naming key's one row.
const NAMING_ENTRY: &[u8] = b"";

/// The payload of the key `id`, labelled `label`, whose wrapped material is `wrapped`.
fn key_payload(id: &str, label: &str, wrapped: &[u8]) -> Zeroizing<Vec<u8>> {
    encoded(id.len() + label.len() + wrapped.len() + 27, |w| {
        put(write_array_head(w, 3));
        put(write_text(w, id));
        put(write_text(w, label));
        put(write_bytes(w, wrapped));
    })
}

/// The id, the label and the wrapped material a key's payload holds.
fn key_of(plaintext: &[u8]) -> Result<(String, String, Vec<u8>), SealError> {
    decoded(plaintext, |c| {
        expect_items(c, 3)?;
        let id = read_text(c, None)?.to_string();
        let label = read_text(c, None)?.to_string();
        let wrapped = read_bytes(c, None)?.to_vec();
        Ok((id, label, wrapped))
    })
}

impl<S> Sealed<S> {
    /// The id the wrapped store keeps the key `id` under: the entry of `id`, or `id` itself for
    /// one of this store's own keys.
    fn stored_id(&self, id: &str) -> String {
        if is_sealing_key_id(id) {
            id.to_string()
        } else {
            self.digest_of(id)
        }
    }

    /// The key the wrapped store's row `row` holds, opened: its id, label and wrapped material
    /// read from its record, every other column as the row holds it. One of this store's own keys
    /// is the row itself.
    fn open_key(&self, row: KeyRow) -> Result<KeyRow, Failure> {
        if is_sealing_key_id(&row.id) {
            return Ok(row);
        }
        let plaintext = self.open(STORE_KEYS, row.id.as_bytes(), &row.wrapped)?;
        let (id, label, wrapped) = key_of(&plaintext)?;
        if self.digest_of(&id) != row.id {
            return Err(SealError::Refused.into());
        }
        Ok(KeyRow {
            id,
            label,
            wrapped,
            ..row
        })
    }

    /// The row the wrapped store keeps for `row`: under the entry of its id, its id, label and
    /// wrapped material sealed. One of this store's own keys is kept as given.
    fn sealed_key(&self, row: &KeyRow) -> Result<KeyRow, Failure> {
        if is_sealing_key_id(&row.id) {
            return Ok(row.clone());
        }
        let entry = self.digest_of(&row.id);
        let record = self.seal(
            STORE_KEYS,
            entry.as_bytes(),
            &key_payload(&row.id, &row.label, &row.wrapped),
        )?;
        Ok(KeyRow {
            id: entry,
            label: String::new(),
            wrapped: record,
            ..row.clone()
        })
    }
}

impl<S: KeyStore> Sealed<S> {
    /// Write every key, and the naming key, sealed under a key other than the active one again
    /// under the active one, through the wrapped store's replacement of a key's wrapped material:
    /// the write the rotation of the key-encryption key makes, which this pass is not to run
    /// beside. One that does not open is counted and left.
    pub fn reseal_keys(&self) -> Result<Resealed, Failure> {
        let mut resealed = Resealed::default();
        for id in self.inner.ids()? {
            if is_sealing_key_id(&id) {
                continue;
            }
            let Some(row) = self.inner.key(&id)? else {
                continue;
            };
            match self.reseal_record(STORE_KEYS, id.as_bytes(), &row.wrapped)? {
                Reseal::Current => {}
                Reseal::Refused => resealed.refused += 1,
                Reseal::Record(record) => resealed.count(self.inner.rewrap(&id, &record)?),
            }
        }
        if let Some(naming) = self.inner.naming_key()? {
            match self.reseal_record(STORE_KEY_NAMING, NAMING_ENTRY, &naming)? {
                Reseal::Current => {}
                Reseal::Refused => resealed.refused += 1,
                Reseal::Record(record) => {
                    resealed.count(self.inner.rewrap_naming_key(&record)?);
                }
            }
        }
        Ok(resealed)
    }
}

impl<S: KeyStore> KeyStore for Sealed<S> {
    fn key(&self, id: &str) -> Result<Option<KeyRow>, Failure> {
        let Some(row) = self.inner.key(&self.stored_id(id))? else {
            return Ok(None);
        };
        let key = self.open_key(row)?;
        if key.id != id {
            return Err(SealError::Refused.into());
        }
        Ok(Some(key))
    }

    fn ids(&self) -> Result<Vec<String>, Failure> {
        let mut ids = Vec::new();
        for stored in self.inner.ids()? {
            if is_sealing_key_id(&stored) {
                ids.push(stored);
            } else if let Some(row) = self.inner.key(&stored)? {
                ids.push(self.open_key(row)?.id);
            }
        }
        ids.sort();
        Ok(ids)
    }

    fn generated_by(&self, generation: &str) -> Result<Option<String>, Failure> {
        let Some(stored) = self.inner.generated_by(&self.digest_of(generation))? else {
            return Ok(None);
        };
        let Some(row) = self.inner.key(&stored)? else {
            return Ok(None);
        };
        Ok(Some(self.open_key(row)?.id))
    }

    fn insert(&self, row: &KeyRow, generation: Option<&str>) -> Result<KeyRow, Failure> {
        let sealed = self.sealed_key(row)?;
        let generation = generation.map(|generation| self.digest_of(generation));
        let kept = self.inner.insert(&sealed, generation.as_deref())?;
        self.open_key(kept)
    }

    fn retire(&self, id: &str) -> Result<Option<bool>, Failure> {
        self.inner.retire(&self.stored_id(id))
    }

    fn rewrap(&self, id: &str, wrapped: &[u8]) -> Result<bool, Failure> {
        let stored = self.stored_id(id);
        let Some(row) = self.inner.key(&stored)? else {
            return Ok(false);
        };
        let key = self.open_key(row)?;
        let replacement = self.sealed_key(&KeyRow {
            wrapped: wrapped.to_vec(),
            ..key
        })?;
        self.inner.rewrap(&stored, &replacement.wrapped)
    }

    fn naming_key(&self) -> Result<Option<Vec<u8>>, Failure> {
        let Some(record) = self.inner.naming_key()? else {
            return Ok(None);
        };
        Ok(Some(
            self.open(STORE_KEY_NAMING, NAMING_ENTRY, &record)?.to_vec(),
        ))
    }

    fn keep_naming_key(&self, wrapped: &[u8]) -> Result<Vec<u8>, Failure> {
        let record = self.seal(STORE_KEY_NAMING, NAMING_ENTRY, wrapped)?;
        let held = self.inner.keep_naming_key(&record)?;
        Ok(self.open(STORE_KEY_NAMING, NAMING_ENTRY, &held)?.to_vec())
    }

    fn rewrap_naming_key(&self, wrapped: &[u8]) -> Result<bool, Failure> {
        let record = self.seal(STORE_KEY_NAMING, NAMING_ENTRY, wrapped)?;
        self.inner.rewrap_naming_key(&record)
    }
}
