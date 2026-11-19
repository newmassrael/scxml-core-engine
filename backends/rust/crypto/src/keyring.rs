// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The life of a key a host generates or derives (ADR 0109, D7; docs/SCE_HOST_FEATURES.md
//! sections 10 and 13), over a key store ([`KeyStore`]) and a key-encryption key ([`KeyWrap`]).
//!
//! - **Generated** keys have one use each ([`KeyUse::Sign`], [`KeyUse::Seal`], [`KeyUse::Mac`],
//!   [`KeyUse::Derive`]; key separation, NIST SP 800-57 Part 1). The id is `generated:` and 16
//!   random bytes in hexadecimal, and the material 32 random bytes, drawn in that order, in one
//!   draw, from the host's random source. A generation repeated under one identity answers the key
//!   it made the first time: the store keeps the identity in the transaction that keeps the key.
//! - **Derived** keys are named by what derived them: `derived:` and the first 16 bytes, in
//!   hexadecimal, of the HMAC-SHA-256 under the store's naming key of the deterministic CBOR array
//!   `[input, salt, info]`, the input the text of the input key's id or the byte string of the
//!   SHA-256 of the secret. The same derivation is then one id in every process that shares the
//!   store, and a `derived:` id a restored session holds names the key it named before. The naming
//!   key is 32 bytes the store keeps wrapped, drawn from the host's random source at the store's
//!   first derivation.
//! - **The issuing key** ([`ISSUING_KEY_ID`]) is the MAC key a host issues its bearer tokens
//!   under (ADR 0109, D3; `crate::credential`): 32 bytes drawn from the host's random source at the
//!   store's first issue and kept as a key of the ring, so that every process that shares the store
//!   issues one token for one identity.
//! - **Retired** keys still verify, open and export, and no longer sign, seal, tag or derive.
//! - **Material** is wrapped under the key-encryption key, bound to the key's id and use, so the
//!   store holds no key in the clear and a row's material moved to another row does not unwrap.
//!   It is unwrapped for each use into memory that is wiped when it is dropped.

use std::fmt;

use sce_forge_runtime::cbor::{write_array_head, write_bytes, write_text};
use sce_forge_runtime::codec::VecSink;
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

pub use sce_rust_session_store::{Failure, KeyRow, KeyState, KeyStore, KeyUse, MemoryKeys};

use crate::cose::{derive_key, public_key_of, put, KEY_BYTES};
use crate::hex;
use crate::mac::{hmac_sha256, TAG_BYTES};
use crate::random::RandomSource;
use crate::wrap::{KeyEncryptionKey, KeyWrap, NoKeyEncryptionKey, WrapError};

/// What the id of a generated key begins with.
pub const GENERATED_PREFIX: &str = "generated:";
/// What the id of a derived key begins with.
pub const DERIVED_PREFIX: &str = "derived:";
/// The random bytes a generated key's id is spelled from.
pub const KEY_ID_RANDOM_BYTES: usize = 16;
/// The bytes of a derived key's naming digest its id is spelled from.
const DERIVED_ID_BYTES: usize = 16;
/// The context the naming key is wrapped for.
const NAMING_CONTEXT: &[u8] = b"sce-key-naming";
/// The id of the key a host issues its credentials under (ADR 0109, D3): a MAC key of the key
/// ring, kept as any other is, which no statechart names, since a statechart reaches a key of the
/// ring only by a `generated:` or `derived:` id.
pub const ISSUING_KEY_ID: &str = "issuing:credentials";
/// What the issuing key is labelled, for the operator who lists the store's keys.
const ISSUING_LABEL: &str = "the key bearer tokens are issued under";

/// Whether `id` names a key of a key ring rather than one an embedding configured: a configured
/// key never has such an id.
pub fn is_key_ring_id(id: &str) -> bool {
    id.starts_with(GENERATED_PREFIX) || id.starts_with(DERIVED_PREFIX)
}

/// The algorithm a key of `key_use` is for, as a store names it.
pub const fn algorithm_of(key_use: KeyUse) -> &'static str {
    match key_use {
        KeyUse::Sign => "EdDSA-Ed25519",
        KeyUse::Seal | KeyUse::Derived => "ChaCha20-Poly1305",
        KeyUse::Mac => "HMAC-SHA-256",
        KeyUse::Derive => "HKDF-SHA-256",
    }
}

/// The use a generation request names: `sign`, `seal`, `mac` or `derive`. A derived key is made by
/// a derivation, never by a generation.
pub fn generated_use(name: &str) -> Option<KeyUse> {
    KeyUse::from_name(name).filter(|key_use| *key_use != KeyUse::Derived)
}

/// What an operation does with a key, which decides the uses it takes and whether a retired key
/// still serves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Purpose {
    Sign,
    Seal,
    Open,
    Tag,
    Verify,
    /// The input keying material of a derivation.
    DeriveFrom,
    /// The public key of a signing key.
    Export,
}

impl Purpose {
    /// Whether a key of `key_use` serves this purpose.
    pub fn serves(self, key_use: KeyUse) -> bool {
        match self {
            Self::Sign | Self::Export => key_use == KeyUse::Sign,
            Self::Seal | Self::Open => matches!(key_use, KeyUse::Seal | KeyUse::Derived),
            Self::Tag | Self::Verify => key_use == KeyUse::Mac,
            Self::DeriveFrom => matches!(key_use, KeyUse::Derive | KeyUse::Derived),
        }
    }

    /// Whether it makes something new under the key, which a retired key no longer does.
    pub fn protects(self) -> bool {
        matches!(self, Self::Sign | Self::Seal | Self::Tag | Self::DeriveFrom)
    }
}

/// Why a key ring did not do what it was asked, each the failure code of the operation that asked
/// (docs/SCE_HOST_FEATURES.md section 13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyError {
    /// The store holds no key of the id, or one of a use the operation does not take:
    /// `unknown_key`.
    Unknown,
    /// The key is retired, and the operation would make something new under it: `key_retired`.
    Retired,
    /// The store, the random source or the key-encryption key failed: `unavailable`.
    Unavailable(String),
}

impl KeyError {
    /// The failure code of the operation that met this.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unknown => "unknown_key",
            Self::Retired => "key_retired",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

impl fmt::Display for KeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => f.write_str("no key of that id serves the operation"),
            Self::Retired => f.write_str("the key is retired"),
            Self::Unavailable(why) => write!(f, "the key ring is unavailable: {why}"),
        }
    }
}

impl std::error::Error for KeyError {}

impl From<Failure> for KeyError {
    fn from(failure: Failure) -> Self {
        Self::Unavailable(failure.to_string())
    }
}

impl From<WrapError> for KeyError {
    fn from(error: WrapError) -> Self {
        Self::Unavailable(error.to_string())
    }
}

/// What a derived key's id names its input by. Not `Debug`: a secret is never printed.
#[derive(Clone, Copy)]
pub enum DerivationInput<'a> {
    /// A key of the host, by its id.
    Key(&'a str),
    /// Bytes the statechart gave, which the id names by their SHA-256.
    Secret(&'a [u8]),
}

/// A key a host keeps for itself under an id no statechart names, drawn at its first use
/// ([`Keyring::keep_drawn`]).
pub(crate) struct Reserved<'a> {
    pub(crate) id: &'a str,
    pub(crate) key_use: KeyUse,
    pub(crate) algorithm: &'a str,
    /// What the operator who lists the store's keys reads it as.
    pub(crate) label: &'a str,
}

/// The context a key's material is wrapped for: its id and its use.
fn context(id: &str, key_use: KeyUse) -> Vec<u8> {
    format!("{id}\0{}", key_use.name()).into_bytes()
}

/// The id of the key derived from `input` with `salt` and `info` under the naming key
/// `naming_key` (see the module documentation).
pub fn derived_id(
    naming_key: &[u8],
    input: DerivationInput<'_>,
    salt: &[u8],
    info: &[u8],
) -> String {
    let mut named = Vec::new();
    let mut w = VecSink::new(&mut named);
    put(write_array_head(&mut w, 3));
    match input {
        DerivationInput::Key(id) => put(write_text(&mut w, id)),
        DerivationInput::Secret(secret) => put(write_bytes(&mut w, &Sha256::digest(secret))),
    }
    put(write_bytes(&mut w, salt));
    put(write_bytes(&mut w, info));
    let digest = hmac_sha256(naming_key, &named);
    format!("{DERIVED_PREFIX}{}", hex(&digest[..DERIVED_ID_BYTES]))
}

/// The keys a host generates and derives, kept in a key store under a key-encryption key.
pub struct Keyring {
    store: Box<dyn KeyStore>,
    wrap: Box<dyn KeyWrap>,
}

impl Keyring {
    /// Keys kept in `store`, their material wrapped by `wrap`.
    pub fn new(store: Box<dyn KeyStore>, wrap: Box<dyn KeyWrap>) -> Self {
        Self { store, wrap }
    }

    /// Keys kept in memory for the life of the process, wrapped under a key-encryption key drawn
    /// from the operating system: what a host keeps when it is given no store. When the system
    /// gives no key, every operation that needs one fails `unavailable`.
    pub fn in_memory() -> Self {
        let wrap: Box<dyn KeyWrap> = match KeyEncryptionKey::drawn() {
            Some(key) => Box::new(key),
            None => Box::new(NoKeyEncryptionKey),
        };
        Self::new(Box::new(MemoryKeys::new()), wrap)
    }

    /// The store the keys are kept in.
    pub fn store(&self) -> &dyn KeyStore {
        self.store.as_ref()
    }

    /// Generate a key of `key_use` labelled `label` for the generation `generation` (the identity
    /// of the invocation that asked), made at `now_ms`, and answer its id; a generation repeated
    /// answers the id of the key it made the first time, and draws nothing.
    pub fn generate(
        &self,
        key_use: KeyUse,
        label: &str,
        generation: &str,
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<String, KeyError> {
        if let Some(id) = self.store.generated_by(generation)? {
            return Ok(id);
        }
        let mut drawn = Zeroizing::new([0u8; KEY_ID_RANDOM_BYTES + KEY_BYTES]);
        if !random.fill(&mut drawn[..]) {
            return Err(KeyError::Unavailable(
                "the random source gave no bytes".to_string(),
            ));
        }
        let id = format!("{GENERATED_PREFIX}{}", hex(&drawn[..KEY_ID_RANDOM_BYTES]));
        let row = KeyRow {
            wrapped: self
                .wrap
                .wrap(&context(&id, key_use), &drawn[KEY_ID_RANDOM_BYTES..])?,
            id,
            key_use,
            algorithm: algorithm_of(key_use).to_string(),
            label: label.to_string(),
            created_ms: now_ms,
            state: KeyState::Active,
        };
        Ok(self.store.insert(&row, Some(generation))?.id)
    }

    /// Derive the key HKDF-SHA-256 gives of `material` with `salt` and `info`, keep it under the
    /// id `input`, `salt` and `info` name (see the module documentation), made at `now_ms`, and
    /// answer the id. A derivation whose id the store holds already answers it and keeps what the
    /// store holds.
    pub fn derive(
        &self,
        input: DerivationInput<'_>,
        material: &[u8],
        salt: &[u8],
        info: &[u8],
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<String, KeyError> {
        let naming = self.naming_key(random)?;
        let id = derived_id(&naming, input, salt, info);
        if self.store.key(&id)?.is_some() {
            return Ok(id);
        }
        let key = Zeroizing::new(derive_key(material, salt, info));
        let row = KeyRow {
            wrapped: self.wrap.wrap(&context(&id, KeyUse::Derived), &key[..])?,
            id,
            key_use: KeyUse::Derived,
            algorithm: algorithm_of(KeyUse::Derived).to_string(),
            label: String::new(),
            created_ms: now_ms,
            state: KeyState::Active,
        };
        Ok(self.store.insert(&row, None)?.id)
    }

    /// The store's naming key, unwrapped; drawn from `random` and kept when the store holds none.
    fn naming_key(&self, random: &mut dyn RandomSource) -> Result<Zeroizing<Vec<u8>>, KeyError> {
        let wrapped = match self.store.naming_key()? {
            Some(wrapped) => wrapped,
            None => {
                let mut fresh = Zeroizing::new([0u8; KEY_BYTES]);
                if !random.fill(&mut fresh[..]) {
                    return Err(KeyError::Unavailable(
                        "the random source gave no naming key".to_string(),
                    ));
                }
                let wrapped = self.wrap.wrap(NAMING_CONTEXT, &fresh[..])?;
                self.store.keep_naming_key(&wrapped)?
            }
        };
        Ok(self.wrap.unwrap(NAMING_CONTEXT, &wrapped)?)
    }

    /// The material of the key `id` names, for `purpose`: [`KeyError::Unknown`] when the store
    /// holds none, or one of a use the purpose does not take, and [`KeyError::Retired`] when it is
    /// retired and the purpose would make something new under it.
    pub fn material(&self, id: &str, purpose: Purpose) -> Result<Zeroizing<Vec<u8>>, KeyError> {
        let row = self.store.key(id)?.ok_or(KeyError::Unknown)?;
        if !purpose.serves(row.key_use) {
            return Err(KeyError::Unknown);
        }
        if row.state == KeyState::Retired && purpose.protects() {
            return Err(KeyError::Retired);
        }
        Ok(self
            .wrap
            .unwrap(&context(&row.id, row.key_use), &row.wrapped)?)
    }

    /// [`Keyring::material`] as the 32 bytes every key of a key ring is.
    pub fn key(&self, id: &str, purpose: Purpose) -> Result<Zeroizing<[u8; KEY_BYTES]>, KeyError> {
        let material = self.material(id, purpose)?;
        if material.len() != KEY_BYTES {
            return Err(KeyError::Unavailable(format!(
                "the material of {id} is not {KEY_BYTES} bytes"
            )));
        }
        let mut key = Zeroizing::new([0u8; KEY_BYTES]);
        key.copy_from_slice(&material);
        Ok(key)
    }

    /// The HMAC-SHA-256 of `message` under the issuing key ([`ISSUING_KEY_ID`]), the key a host
    /// issues its bearer tokens under (`crate::credential`). The key is drawn from `random` at the
    /// store's first issue, made at `now_ms`, and kept as a MAC key of the ring: wrapped under the
    /// key-encryption key, the same in every process that shares the store (of two processes that
    /// each drew one, the first kept is the key), and rewrapped by [`Keyring::rewrap_all`]. It never
    /// leaves the ring: an issue asks for a tag under it. Retired by the operator, it issues
    /// nothing more ([`KeyError::Retired`]).
    pub fn issuing_tag(
        &self,
        message: &[u8],
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<[u8; TAG_BYTES], KeyError> {
        self.keep_drawn(
            &Reserved {
                id: ISSUING_KEY_ID,
                key_use: KeyUse::Mac,
                algorithm: algorithm_of(KeyUse::Mac),
                label: ISSUING_LABEL,
            },
            now_ms,
            random,
        )?;
        let key = self.material(ISSUING_KEY_ID, Purpose::Tag)?;
        Ok(hmac_sha256(&key[..], message))
    }

    /// Keep the key `reserved` names, 32 bytes drawn from `random`, made at `now_ms` and wrapped
    /// as every key of the ring is, unless the store holds a key of its id: of two processes that
    /// each drew one, the first kept is the key. A key a host keeps for itself under an id no
    /// statechart names (the issuing key, the keys of a sealing store) is made this way.
    pub(crate) fn keep_drawn(
        &self,
        reserved: &Reserved<'_>,
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<(), KeyError> {
        if self.store.key(reserved.id)?.is_some() {
            return Ok(());
        }
        let mut drawn = Zeroizing::new([0u8; KEY_BYTES]);
        if !random.fill(&mut drawn[..]) {
            return Err(KeyError::Unavailable(format!(
                "the random source gave no key for {}",
                reserved.id
            )));
        }
        let row = KeyRow {
            id: reserved.id.to_string(),
            key_use: reserved.key_use,
            algorithm: reserved.algorithm.to_string(),
            wrapped: self
                .wrap
                .wrap(&context(reserved.id, reserved.key_use), &drawn[..])?,
            label: reserved.label.to_string(),
            created_ms: now_ms,
            state: KeyState::Active,
        };
        self.store.insert(&row, None)?;
        Ok(())
    }

    /// The Ed25519 public key of the signing key `id` names, retired or not.
    pub fn public_key(&self, id: &str) -> Result<[u8; KEY_BYTES], KeyError> {
        Ok(public_key_of(&*self.key(id, Purpose::Export)?))
    }

    /// Retire the key `id` names: `true` when it was active, `false` when it was retired already.
    pub fn retire(&self, id: &str) -> Result<bool, KeyError> {
        self.store.retire(id)?.ok_or(KeyError::Unknown)
    }

    /// Wrap every key, and the naming key, under `wrap` from here on: the rotation of the
    /// key-encryption key. Each key is rewrapped in a write of its own, and a key that already
    /// unwraps under `wrap` is left as it is, so a rotation that stopped part of the way is
    /// finished by running it again with the same two keys.
    pub fn rewrap_all(&mut self, wrap: Box<dyn KeyWrap>) -> Result<(), KeyError> {
        for id in self.store.ids()? {
            let Some(row) = self.store.key(&id)? else {
                continue;
            };
            let context = context(&row.id, row.key_use);
            if wrap.unwrap(&context, &row.wrapped).is_ok() {
                continue;
            }
            let material = self.wrap.unwrap(&context, &row.wrapped)?;
            self.store.rewrap(&id, &wrap.wrap(&context, &material)?)?;
        }
        if let Some(wrapped) = self.store.naming_key()? {
            if wrap.unwrap(NAMING_CONTEXT, &wrapped).is_err() {
                let naming = self.wrap.unwrap(NAMING_CONTEXT, &wrapped)?;
                self.store
                    .rewrap_naming_key(&wrap.wrap(NAMING_CONTEXT, &naming)?)?;
            }
        }
        self.wrap = wrap;
        Ok(())
    }
}
