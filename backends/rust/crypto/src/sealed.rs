// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Everything a server stores, sealed at rest, and every key it finds a row by kept as a keyed
//! digest (ADR 0109, D8; the record is docs/SCE_SEALED_RECORD.md).
//!
//! SCE_MESH.md §mesh-10.11 asks a host to store a checkpoint encrypted at rest under its own key,
//! since a saved state holds a document's values in the clear. [`Sealed`] does that for each store
//! contract a server keeps, over any implementation of it: the sessions of ADR 0107
//! (`SessionStore`) with the outboxes of D10 their writes append to, the credentials of D3
//! (`PinStore`, `TokenStore`, `CredentialStore`), the keys of D7 (`KeyStore`) and the values of
//! the key-value feature of D13 (`KvStore`), with the first results of its put-ifs.
//!
//! An outbox's envelopes are not re-sealed by a pass: each lives until its client acknowledges
//! it or the binding's `retain_ms` expires it, so once that long has passed after a rotation, a
//! retired key opens no envelope a store still holds.
//!
//! - **Every value is a sealed record** ([`seal_record`]): `SCE-SEALED-1`, the id of the key that
//!   sealed it, a 192-bit random nonce and the XChaCha20-Poly1305 ciphertext with its tag. The
//!   associated data is the record's header, the store's name and the row's entry, so a record
//!   moved to another row or another store does not open, nor one changed by a byte, nor one opened
//!   under another key. A random 192-bit nonce has no bound on the records one key seals that a
//!   server writing a checkpoint per input could reach (NIST SP 800-38D section 8.3 bounds a 96-bit
//!   one at 2^32).
//! - **Every key is an entry** ([`Sealed::digest_of`]): the HMAC-SHA-256 of the key under the
//!   store's index key, in lowercase hexadecimal; a token's digest is its keyed digest's 32 bytes.
//!   The key itself is inside the sealed value, so a key that names a person or is a bearer value
//!   is never in the clear on the medium, and a store that lists rows (`due_before`) lists entries,
//!   whose keys `SessionStore::key_of` reads from the rows.
//! - **Rotation.** Each record names its sealing key, and a write seals under the active one. A
//!   re-seal pass ([`Sealed::reseal_sessions`], [`Sealed::reseal_credentials`],
//!   [`Sealed::reseal_keys`]) writes the rows sealed under any other key again through the store's
//!   ordinary write: a session's versioned and fenced put, under a lease the pass takes; a pin's,
//!   a token's or a value's replacement from the bytes it read ([`Sealed::reseal_values`]). Once
//!   a pass leaves nothing, a retired key opens nothing and may be destroyed.
//! - **The keys** ([`SealingKeys`]) are keys of a key ring ([`Keyring`]) kept in the store's own key
//!   store, wrapped under the key-encryption key the program holds apart from the store: the index
//!   key [`INDEX_KEY_ID`] and the sealing keys `sealing:<hex>`. Those rows are the ones a sealing
//!   key store keeps as they are given ([`is_sealing_key_id`]): a key cannot be sealed under
//!   itself, and the key-encryption key protects them.
//!
//! What it does not give: freshness against someone who can write the medium. A row replaced by an
//! earlier sealing of the same row opens; the lease's fence and the version stop a process that is
//! wrong, not a medium written by hand. And what a store compares or sets stays as the store needs
//! it: a session's version, earliest due and lease, a token's revocation, a key's use, algorithm,
//! creation and state.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::{PoisonError, RwLock, RwLockReadGuard};

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use sce_forge_runtime::cbor::read_array_len;
use sce_forge_runtime::codec::{CodecError, SceCursor, VecSink};
use sce_rust_session_store::{Failure, KeyRow};
use zeroize::Zeroizing;

use crate::cose::KEY_BYTES;
use crate::hex;
use crate::keyring::{algorithm_of, KeyError, KeyState, KeyUse, Keyring, Purpose, Reserved};
use crate::mac::{hmac_sha256, TAG_BYTES as DIGEST_BYTES};
use crate::random::{RandomSource, SystemRandom};

mod credentials;
mod keys;
mod kv;
mod outbox;
mod sessions;

pub use kv::STORE_KV_VALUES;
pub use outbox::STORE_OUTBOX;

/// What every sealed record begins with: the version of its form.
pub const RECORD_MAGIC: &[u8; 12] = b"SCE-SEALED-1";
/// The bytes of a record's nonce: XChaCha20-Poly1305's 192 bits.
pub const NONCE_BYTES: usize = 24;
/// The bytes of a record's tag: Poly1305's 128 bits.
pub const TAG_BYTES: usize = 16;
/// The most bytes a sealing key's id or a store's name has: each is written after its length in
/// one byte.
pub const NAME_MAX: usize = 255;

/// The name the sessions of a `SessionStore` are sealed for.
pub const STORE_SESSIONS: &str = "sessions";
/// The name the pins of a `PinStore` are sealed for.
pub const STORE_PINS: &str = "credential_pins";
/// The name the tokens of a `TokenStore` are sealed for.
pub const STORE_TOKENS: &str = "credential_tokens";
/// The name the keys of a `KeyStore` are sealed for.
pub const STORE_KEYS: &str = "keys";
/// The name a `KeyStore`'s naming key is sealed for; its one row's entry is empty.
pub const STORE_KEY_NAMING: &str = "key_naming";

/// The id of a sealing store's index key, a MAC key of the key ring.
pub const INDEX_KEY_ID: &str = "index:sealed";
/// What the id of a sealing key begins with; 16 random bytes in hexadecimal follow.
pub const SEALING_PREFIX: &str = "sealing:";
/// The random bytes a sealing key's id is spelled from.
const SEALING_ID_RANDOM_BYTES: usize = 16;
/// The algorithm a sealing key is for, as its key store names it.
const SEALING_ALGORITHM: &str = "XChaCha20-Poly1305";

/// The index key, as the key ring keeps it.
const INDEX_KEY: Reserved<'static> = Reserved {
    id: INDEX_KEY_ID,
    key_use: KeyUse::Mac,
    algorithm: algorithm_of(KeyUse::Mac),
    label: "the key a sealing store digests the keys of its rows under",
};

/// What a sealing key is labelled, for the operator who lists the store's keys.
const SEALING_LABEL: &str = "a key a sealing store seals its rows under";

/// Whether `id` names one of a sealing store's own keys, which its key store keeps as given.
pub fn is_sealing_key_id(id: &str) -> bool {
    id == INDEX_KEY_ID || id.starts_with(SEALING_PREFIX)
}

/// Why a record was not made or not opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SealError {
    /// The bytes are not a sealed record: no `SCE-SEALED-1`, an empty key id, or too few bytes
    /// for a nonce and a tag.
    Malformed,
    /// The record names a sealing key the store was not given.
    UnknownKey(String),
    /// The record does not open for this store and row under the key its id names: it was sealed
    /// for another row or store, changed since, or sealed under another key of the same id.
    Refused,
    /// What the record opened to is not the payload of a row of its store.
    Payload,
    /// A key id or a store name that is empty or longer than [`NAME_MAX`] bytes, which no record
    /// names.
    Unnameable,
    /// A plaintext longer than the cipher seals.
    TooLong,
    /// The operating system gave no nonce.
    NoNonce,
}

impl fmt::Display for SealError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed => f.write_str("the bytes are not a sealed record"),
            Self::UnknownKey(id) => {
                write!(
                    f,
                    "the record is sealed under '{id}', a key the store does not hold"
                )
            }
            Self::Refused => f.write_str(
                "the record does not open for this row: it was moved from another row, changed, \
                 or sealed under another key",
            ),
            Self::Payload => f.write_str("the record opened to bytes that are no row of its store"),
            Self::Unnameable => {
                f.write_str("a key id or a store name is empty or longer than 255 bytes")
            }
            Self::TooLong => f.write_str("the plaintext is longer than the cipher seals"),
            Self::NoNonce => f.write_str("the operating system gave no nonce"),
        }
    }
}

impl std::error::Error for SealError {}

impl From<SealError> for Failure {
    fn from(error: SealError) -> Self {
        Failure::new(error)
    }
}

/// The header of a record sealed under `key_id`: the magic, the id's length in one byte, the id.
fn header(key_id: &str) -> Result<Vec<u8>, SealError> {
    let id = key_id.as_bytes();
    let length = u8::try_from(id.len())
        .ok()
        .filter(|length| *length != 0)
        .ok_or(SealError::Unnameable)?;
    let mut header = Vec::with_capacity(RECORD_MAGIC.len() + 1 + id.len());
    header.extend_from_slice(RECORD_MAGIC);
    header.push(length);
    header.extend_from_slice(id);
    Ok(header)
}

/// What a record authenticates beside its ciphertext: its header, the store's name after its
/// length in one byte, and the row's entry, which runs to the end.
fn associated(header: &[u8], store: &str, indexed: &[u8]) -> Result<Vec<u8>, SealError> {
    let name = store.as_bytes();
    let length = u8::try_from(name.len()).map_err(|_| SealError::Unnameable)?;
    let mut data = Vec::with_capacity(header.len() + 1 + name.len() + indexed.len());
    data.extend_from_slice(header);
    data.push(length);
    data.extend_from_slice(name);
    data.extend_from_slice(indexed);
    Ok(data)
}

fn cipher(key: &[u8; KEY_BYTES]) -> XChaCha20Poly1305 {
    XChaCha20Poly1305::new(Key::from_slice(&key[..]))
}

/// The record of `plaintext` sealed under the key `key`, whose id is `key_id`, with the nonce
/// `nonce`, for the row of the store `store` whose entry is `indexed`. A store draws a fresh nonce
/// from the operating system for every record; a test that holds a sealing to its bytes gives one.
pub fn seal_record(
    key_id: &str,
    key: &[u8; KEY_BYTES],
    nonce: &[u8; NONCE_BYTES],
    store: &str,
    indexed: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, SealError> {
    let header = header(key_id)?;
    let aad = associated(&header, store, indexed)?;
    let sealed = cipher(key)
        .encrypt(
            XNonce::from_slice(&nonce[..]),
            Payload {
                msg: plaintext,
                aad: &aad,
            },
        )
        .map_err(|_| SealError::TooLong)?;
    let mut record = header;
    record.reserve(NONCE_BYTES + sealed.len());
    record.extend_from_slice(&nonce[..]);
    record.extend_from_slice(&sealed);
    Ok(record)
}

/// A record read apart.
struct Parts<'a> {
    header: &'a [u8],
    key_id: &'a str,
    nonce: &'a [u8],
    sealed: &'a [u8],
}

fn parts(record: &[u8]) -> Result<Parts<'_>, SealError> {
    let rest = record
        .strip_prefix(RECORD_MAGIC.as_slice())
        .ok_or(SealError::Malformed)?;
    let (&length, rest) = rest.split_first().ok_or(SealError::Malformed)?;
    let length = usize::from(length);
    if length == 0 || rest.len() < length + NONCE_BYTES + TAG_BYTES {
        return Err(SealError::Malformed);
    }
    let (id, rest) = rest.split_at(length);
    let key_id = std::str::from_utf8(id).map_err(|_| SealError::Malformed)?;
    let (nonce, sealed) = rest.split_at(NONCE_BYTES);
    Ok(Parts {
        header: &record[..RECORD_MAGIC.len() + 1 + length],
        key_id,
        nonce,
        sealed,
    })
}

/// The id of the key `record` names as the one that sealed it.
pub fn record_key_id(record: &[u8]) -> Result<&str, SealError> {
    parts(record).map(|parts| parts.key_id)
}

fn open_parts(
    key: &[u8; KEY_BYTES],
    parts: &Parts<'_>,
    store: &str,
    indexed: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SealError> {
    let aad = associated(parts.header, store, indexed)?;
    cipher(key)
        .decrypt(
            XNonce::from_slice(parts.nonce),
            Payload {
                msg: parts.sealed,
                aad: &aad,
            },
        )
        .map(Zeroizing::new)
        .map_err(|_| SealError::Refused)
}

/// The plaintext `record` holds, opened under `key` for the row of the store `store` whose entry
/// is `indexed`: refused when it was sealed for another row or store, under another key, or
/// changed since. The key is the one [`record_key_id`] names.
pub fn open_record(
    key: &[u8; KEY_BYTES],
    store: &str,
    indexed: &[u8],
    record: &[u8],
) -> Result<Zeroizing<Vec<u8>>, SealError> {
    open_parts(key, &parts(record)?, store, indexed)
}

/// The keyed digest a sealing store keeps `bytes` as: their HMAC-SHA-256 under `index_key`.
pub fn keyed_digest(index_key: &[u8; KEY_BYTES], bytes: &[u8]) -> [u8; DIGEST_BYTES] {
    hmac_sha256(&index_key[..], bytes)
}

/// The keys a sealing store holds: the index key it digests the keys of its rows under, the
/// sealing keys it opens records with, and which of them seals what it writes. Never printed.
#[derive(Clone)]
pub struct SealingKeys {
    index: Zeroizing<[u8; KEY_BYTES]>,
    active: String,
    keys: BTreeMap<String, Zeroizing<[u8; KEY_BYTES]>>,
}

/// The ids, never the keys.
impl fmt::Debug for SealingKeys {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SealingKeys")
            .field("active", &self.active)
            .field("held", &self.keys.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl SealingKeys {
    /// The keys `keys` names, of which `active` seals what is written, and the index key `index`:
    /// `None` when `active` is none of them, or an id is empty or longer than [`NAME_MAX`] bytes.
    pub fn new(
        index: [u8; KEY_BYTES],
        active: &str,
        keys: impl IntoIterator<Item = (String, [u8; KEY_BYTES])>,
    ) -> Option<Self> {
        let keys: BTreeMap<String, Zeroizing<[u8; KEY_BYTES]>> = keys
            .into_iter()
            .map(|(id, key)| (id, Zeroizing::new(key)))
            .collect();
        let nameable = keys.keys().all(|id| !id.is_empty() && id.len() <= NAME_MAX);
        (nameable && keys.contains_key(active)).then(|| Self {
            index: Zeroizing::new(index),
            active: active.to_string(),
            keys,
        })
    }

    /// The keys a sealing store keeps in the key store of `keyring`, wrapped under its
    /// key-encryption key: the index key, and every sealing key, active or retired. At the
    /// store's first call the index key and a first sealing key are drawn from `random`, made at
    /// `now_ms`, and kept; every process that shares the store then holds the same ones. Of the
    /// active sealing keys the latest made seals what is written, the one of the greatest id among
    /// two made at one moment, so that every process picks the same.
    pub fn from_keyring(
        keyring: &Keyring,
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<Self, KeyError> {
        keyring.keep_drawn(&INDEX_KEY, now_ms, random)?;
        let mut held = sealing_rows(keyring)?;
        if !held.iter().any(|row| row.state == KeyState::Active) {
            Self::rotate(keyring, now_ms, random)?;
            held = sealing_rows(keyring)?;
        }
        let active = held
            .iter()
            .filter(|row| row.state == KeyState::Active)
            .max_by(|a, b| {
                a.created_ms
                    .cmp(&b.created_ms)
                    .then_with(|| a.id.cmp(&b.id))
            })
            .map(|row| row.id.clone())
            .ok_or_else(|| {
                KeyError::Unavailable("the store keeps no active sealing key".to_string())
            })?;
        let mut keys = BTreeMap::new();
        for row in &held {
            keys.insert(row.id.clone(), keyring.key(&row.id, Purpose::Open)?);
        }
        Ok(Self {
            index: keyring.key(INDEX_KEY_ID, Purpose::Verify)?,
            active,
            keys,
        })
    }

    /// Make a new sealing key in the key store of `keyring`, its id and its material drawn from
    /// `random`, made at `now_ms`, and retire every other active sealing key: the rotation of the
    /// sealing key. A retired key still opens what it sealed. Answers the new key's id; a store
    /// seals under it once it is given keys read again ([`SealingKeys::from_keyring`],
    /// [`Sealed::set_keys`]), and a re-seal pass then moves every row to it.
    pub fn rotate(
        keyring: &Keyring,
        now_ms: u64,
        random: &mut dyn RandomSource,
    ) -> Result<String, KeyError> {
        let mut drawn = [0u8; SEALING_ID_RANDOM_BYTES];
        if !random.fill(&mut drawn) {
            return Err(KeyError::Unavailable(
                "the random source gave no id for a sealing key".to_string(),
            ));
        }
        let id = format!("{SEALING_PREFIX}{}", hex(&drawn));
        keyring.keep_drawn(
            &Reserved {
                id: &id,
                key_use: KeyUse::Seal,
                algorithm: SEALING_ALGORITHM,
                label: SEALING_LABEL,
            },
            now_ms,
            random,
        )?;
        for row in sealing_rows(keyring)? {
            if row.id != id && row.state == KeyState::Active {
                keyring.retire(&row.id)?;
            }
        }
        Ok(id)
    }

    /// The id of the key that seals what is written.
    pub fn active(&self) -> &str {
        &self.active
    }

    /// The ids of every sealing key held, in their order.
    pub fn ids(&self) -> impl Iterator<Item = &str> + '_ {
        self.keys.keys().map(String::as_str)
    }

    fn key(&self, id: &str) -> Option<&[u8; KEY_BYTES]> {
        self.keys.get(id).map(|key| &**key)
    }
}

/// Every sealing key the key store of `keyring` holds.
fn sealing_rows(keyring: &Keyring) -> Result<Vec<KeyRow>, KeyError> {
    let store = keyring.store();
    let mut rows = Vec::new();
    for id in store.ids()? {
        if id.starts_with(SEALING_PREFIX) {
            if let Some(row) = store.key(&id)? {
                rows.push(row);
            }
        }
    }
    Ok(rows)
}

/// What a re-seal pass did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Resealed {
    /// The rows written again under the active key.
    pub resealed: u64,
    /// The rows left under another key: another process held the session's lease, or the row
    /// changed between the pass's read and its write. A later pass takes them.
    pub left: u64,
    /// The rows that did not open: changed on the medium, or sealed under a key the store was not
    /// given. No pass writes them; they are the operator's.
    pub refused: u64,
}

/// A store whose every value is sealed and whose every key is kept as its keyed digest, over the
/// store `S` (see the module documentation). It implements each store contract `S` implements.
pub struct Sealed<S> {
    inner: S,
    keys: RwLock<SealingKeys>,
}

impl<S> Sealed<S> {
    /// The store `inner`, sealed under `keys`.
    pub fn new(inner: S, keys: SealingKeys) -> Self {
        Self {
            inner,
            keys: RwLock::new(keys),
        }
    }

    /// The store this one wraps, which holds the sealed records under the entries.
    pub fn inner(&self) -> &S {
        &self.inner
    }

    /// Seal and open under `keys` from here on: the keys of a rotation, read again from the key
    /// ring. Its index key must be the one the store's entries were made under.
    pub fn set_keys(&self, keys: SealingKeys) {
        *self.keys.write().unwrap_or_else(PoisonError::into_inner) = keys;
    }

    fn keys(&self) -> RwLockReadGuard<'_, SealingKeys> {
        self.keys.read().unwrap_or_else(PoisonError::into_inner)
    }

    /// The entry the store keeps the row of `key` under: the lowercase hexadecimal of the
    /// HMAC-SHA-256 of its UTF-8 bytes under the index key. What an SQL read of the medium shows
    /// in place of the key, and how an operator finds the row of one.
    pub fn digest_of(&self, key: &str) -> String {
        hex(&self.keyed(key.as_bytes()))
    }

    /// The keyed digest of `bytes`.
    fn keyed(&self, bytes: &[u8]) -> [u8; DIGEST_BYTES] {
        keyed_digest(&self.keys().index, bytes)
    }

    /// `plaintext` sealed under the active key, with a nonce drawn from the operating system, for
    /// the row of `store` whose entry is `indexed`.
    fn seal(&self, store: &str, indexed: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Failure> {
        let keys = self.keys();
        let key = keys
            .key(&keys.active)
            .ok_or_else(|| SealError::UnknownKey(keys.active.clone()))?;
        let mut nonce = [0u8; NONCE_BYTES];
        if !SystemRandom.fill(&mut nonce) {
            return Err(SealError::NoNonce.into());
        }
        seal_record(&keys.active, key, &nonce, store, indexed, plaintext).map_err(Failure::from)
    }

    /// The plaintext `record` holds for the row of `store` whose entry is `indexed`, opened under
    /// the key it names.
    fn open(
        &self,
        store: &str,
        indexed: &[u8],
        record: &[u8],
    ) -> Result<Zeroizing<Vec<u8>>, Failure> {
        let split = parts(record)?;
        let keys = self.keys();
        let key = keys
            .key(split.key_id)
            .ok_or_else(|| SealError::UnknownKey(split.key_id.to_string()))?;
        open_parts(key, &split, store, indexed).map_err(Failure::from)
    }

    /// What a re-seal pass makes of `record`, the record of the row of `store` whose entry is
    /// `indexed`.
    fn reseal_record(&self, store: &str, indexed: &[u8], record: &[u8]) -> Result<Reseal, Failure> {
        if self.is_current(record) {
            return Ok(Reseal::Current);
        }
        let Ok(plaintext) = self.open(store, indexed, record) else {
            return Ok(Reseal::Refused);
        };
        Ok(Reseal::Record(self.seal(store, indexed, &plaintext)?))
    }

    /// Whether `record` is sealed under the active key.
    fn is_current(&self, record: &[u8]) -> bool {
        record_key_id(record).is_ok_and(|id| id == self.keys().active)
    }
}

/// A row's payload in the deterministic CBOR the codecs write (RFC 8949 section 4.2.1), held in
/// memory that is wiped when it is dropped.
fn encoded(capacity: usize, write: impl FnOnce(&mut VecSink<'_>)) -> Zeroizing<Vec<u8>> {
    let mut bytes = Zeroizing::new(Vec::with_capacity(capacity));
    write(&mut VecSink::new(&mut bytes));
    bytes
}

/// The row `read` reads from the payload `plaintext`, which it must read to its end.
fn decoded<T>(
    plaintext: &[u8],
    read: impl FnOnce(&mut SceCursor<'_>) -> Result<T, CodecError>,
) -> Result<T, SealError> {
    let mut cursor = SceCursor::new(plaintext);
    let value = read(&mut cursor).map_err(|_| SealError::Payload)?;
    if cursor.remaining() == 0 {
        Ok(value)
    } else {
        Err(SealError::Payload)
    }
}

/// Read the head of an array of `items` items.
fn expect_items(cursor: &mut SceCursor<'_>, items: u64) -> Result<(), CodecError> {
    if read_array_len(cursor)? == items {
        Ok(())
    } else {
        Err(CodecError::CborWrongLength)
    }
}

/// What a re-seal pass makes of one record.
enum Reseal {
    /// It is sealed under the active key already.
    Current,
    /// It does not open: no pass writes it.
    Refused,
    /// The record that holds the same plaintext under the active key.
    Record(Vec<u8>),
}

impl Resealed {
    /// Count a row a pass wrote again (`true`) or left (`false`).
    fn count(&mut self, written: bool) {
        if written {
            self.resealed += 1;
        } else {
            self.left += 1;
        }
    }
}
