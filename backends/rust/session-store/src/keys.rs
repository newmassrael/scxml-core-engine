// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Where a host keeps the keys it generates and derives (ADR 0109, decision D7).
//!
//! A key has an id, one use, its algorithm, a label, the moment it was made and a state; its
//! material is kept **wrapped** under a key-encryption key that is never in the store (envelope
//! encryption, NIST SP 800-57 Part 1): the store holds bytes it cannot read, and whoever reads the
//! medium without the key-encryption key reads no key. The wrapping and the meaning of each use are
//! the caller's (`sce_rust_crypto::keyring`); this contract keeps rows.
//!
//! Beside the keys, a store keeps two things a host needs to agree with every other process that
//! shares it:
//!
//! * the **generations**: which key the generation of one invocation made, written in the
//!   transaction that keeps the key, so that a generation repeated under one identity (a restore
//!   that starts an invocation again, docs/SCE_HOST_FEATURES.md section 1) answers the key it made
//!   the first time instead of making a second;
//! * the **naming key**, wrapped as a key is, under which a derived key's id is computed from what
//!   derived it, so that the same derivation is one id in every process that shares the store.
//!
//! * [`MemoryKeys`] keeps them in the process.
//! * `SqliteStore` (feature `sqlite`) keeps them in the session store's file, beside the sessions
//!   (layout 2).
//! * [`crate::key_conformance`] is what every implementation is held to.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crate::Failure;

/// What a key is for. A key a store generates has one use (key separation, NIST SP 800-57 Part 1
/// section 5.2); a derived key is the symmetric key a derivation has always made
/// (docs/SCE_HOST_FEATURES.md section 10): it seals, opens and derives.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeyUse {
    /// An Ed25519 signing key.
    Sign,
    /// A ChaCha20/Poly1305 key that seals and opens.
    Seal,
    /// An HMAC-SHA-256 key that tags and verifies.
    Mac,
    /// Input keying material of HKDF-SHA-256.
    Derive,
    /// A key a derivation made.
    Derived,
}

impl KeyUse {
    /// Every use, in the order of the enum.
    pub const ALL: [KeyUse; 5] = [
        KeyUse::Sign,
        KeyUse::Seal,
        KeyUse::Mac,
        KeyUse::Derive,
        KeyUse::Derived,
    ];

    /// The name a store writes the use as.
    pub fn name(self) -> &'static str {
        match self {
            Self::Sign => "sign",
            Self::Seal => "seal",
            Self::Mac => "mac",
            Self::Derive => "derive",
            Self::Derived => "derived",
        }
    }

    /// The use `name` names, if it names one.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key_use| key_use.name() == name)
    }
}

/// Where a key is in its life. A retired key still verifies and opens what it made, and makes
/// nothing new; destroying it is the operator's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyState {
    Active,
    Retired,
}

impl KeyState {
    /// The name a store writes the state as.
    pub fn name(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Retired => "retired",
        }
    }

    /// The state `name` names, if it names one.
    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Active, Self::Retired]
            .into_iter()
            .find(|state| state.name() == name)
    }
}

/// A key as a store keeps it.
#[derive(Clone, PartialEq, Eq)]
pub struct KeyRow {
    /// Its id, by which a statechart names it.
    pub id: String,
    pub key_use: KeyUse,
    /// The algorithm its material is for, by name.
    pub algorithm: String,
    /// Its material, wrapped under a key-encryption key the store does not hold: bytes the store
    /// keeps and never reads.
    pub wrapped: Vec<u8>,
    /// What the document that generated it called it, for the operator; empty for a derived key.
    pub label: String,
    /// When it was made, in milliseconds since the Unix epoch.
    pub created_ms: u64,
    pub state: KeyState,
}

/// Every column but the wrapped material, of which only the length is shown.
impl fmt::Debug for KeyRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KeyRow")
            .field("id", &self.id)
            .field("key_use", &self.key_use)
            .field("algorithm", &self.algorithm)
            .field("wrapped_bytes", &self.wrapped.len())
            .field("label", &self.label)
            .field("created_ms", &self.created_ms)
            .field("state", &self.state)
            .finish()
    }
}

/// A store of keys. Every method is one transaction: it reads and writes what it names, or, when
/// it returns an error, writes nothing. A handle may be shared between threads; whether handles on
/// one store may live in several processes is the implementation's to say.
pub trait KeyStore: Send + Sync {
    /// The key `id` names, or `None`.
    fn key(&self, id: &str) -> Result<Option<KeyRow>, Failure>;

    /// The ids of every key held, in the order of their UTF-8 bytes.
    fn ids(&self) -> Result<Vec<String>, Failure>;

    /// The id of the key the generation `generation` made, if one did.
    fn generated_by(&self, generation: &str) -> Result<Option<String>, Failure>;

    /// Keep `row`, and with it, when `generation` is given, that this generation made it; answer
    /// the row the store holds afterwards. Nothing is written, and the row held before is answered,
    /// when `generation` already made a key, or when the store holds a key of `row`'s id: a key is
    /// never replaced, and a generation never makes two.
    fn insert(&self, row: &KeyRow, generation: Option<&str>) -> Result<KeyRow, Failure>;

    /// Retire the key `id` names: `None` when the store holds none, `Some(true)` when it was
    /// active and is retired now, `Some(false)` when it was retired already.
    fn retire(&self, id: &str) -> Result<Option<bool>, Failure>;

    /// Replace the wrapped material of the key `id` names, every other column kept, as the
    /// rotation of the key-encryption key does: `false`, and nothing written, when the store holds
    /// no key of the id.
    fn rewrap(&self, id: &str, wrapped: &[u8]) -> Result<bool, Failure>;

    /// The store's naming key, wrapped, if it holds one.
    fn naming_key(&self) -> Result<Option<Vec<u8>>, Failure>;

    /// Keep `wrapped` as the store's naming key unless it holds one already, and answer the one it
    /// holds afterwards: two processes that each drew one agree on the first kept.
    fn keep_naming_key(&self, wrapped: &[u8]) -> Result<Vec<u8>, Failure>;

    /// Replace the wrapped naming key, as the rotation of the key-encryption key does: `false`, and
    /// nothing written, when the store holds none.
    fn rewrap_naming_key(&self, wrapped: &[u8]) -> Result<bool, Failure>;
}

/// A key store in memory: what a host keeps when it is given no durable one. A clone is another
/// handle on the same keys, as a second connection is on a database. One lock is a transaction.
#[derive(Clone, Default)]
pub struct MemoryKeys {
    shared: Arc<Mutex<KeyRecords>>,
}

#[derive(Default)]
struct KeyRecords {
    keys: BTreeMap<String, KeyRow>,
    generations: BTreeMap<String, String>,
    naming: Option<Vec<u8>>,
}

impl MemoryKeys {
    pub fn new() -> Self {
        Self::default()
    }

    /// The records, whatever a thread that panicked while it held them left: nothing, since every
    /// call changes them only after its checks, in steps that do not panic.
    fn records(&self) -> MutexGuard<'_, KeyRecords> {
        self.shared.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl KeyStore for MemoryKeys {
    fn key(&self, id: &str) -> Result<Option<KeyRow>, Failure> {
        Ok(self.records().keys.get(id).cloned())
    }

    fn ids(&self) -> Result<Vec<String>, Failure> {
        Ok(self.records().keys.keys().cloned().collect())
    }

    fn generated_by(&self, generation: &str) -> Result<Option<String>, Failure> {
        Ok(self.records().generations.get(generation).cloned())
    }

    fn insert(&self, row: &KeyRow, generation: Option<&str>) -> Result<KeyRow, Failure> {
        let mut records = self.records();
        if let Some(made) = generation.and_then(|generation| records.generations.get(generation)) {
            if let Some(held) = records.keys.get(made) {
                return Ok(held.clone());
            }
        }
        if let Some(held) = records.keys.get(&row.id) {
            return Ok(held.clone());
        }
        records.keys.insert(row.id.clone(), row.clone());
        if let Some(generation) = generation {
            records
                .generations
                .insert(generation.to_string(), row.id.clone());
        }
        Ok(row.clone())
    }

    fn retire(&self, id: &str) -> Result<Option<bool>, Failure> {
        let mut records = self.records();
        Ok(records.keys.get_mut(id).map(|row| {
            let active = row.state == KeyState::Active;
            row.state = KeyState::Retired;
            active
        }))
    }

    fn rewrap(&self, id: &str, wrapped: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.keys.get_mut(id) {
            Some(row) => {
                row.wrapped = wrapped.to_vec();
                true
            }
            None => false,
        })
    }

    fn naming_key(&self) -> Result<Option<Vec<u8>>, Failure> {
        Ok(self.records().naming.clone())
    }

    fn keep_naming_key(&self, wrapped: &[u8]) -> Result<Vec<u8>, Failure> {
        Ok(self
            .records()
            .naming
            .get_or_insert_with(|| wrapped.to_vec())
            .clone())
    }

    fn rewrap_naming_key(&self, wrapped: &[u8]) -> Result<bool, Failure> {
        let mut records = self.records();
        Ok(match records.naming.as_mut() {
            Some(naming) => {
                *naming = wrapped.to_vec();
                true
            }
            None => false,
        })
    }
}
