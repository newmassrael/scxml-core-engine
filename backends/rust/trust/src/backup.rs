// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A device's secrets, kept under a passphrase.
//!
//! A person whose device is lost or broken has the list's recovery in an administrator's hands (a new
//! device, a removal at the last operation accepted); what they can keep for themselves is a copy of
//! the device's key, and of what the device held that cannot be had again (the MLS state, the keys of
//! the epochs it was given), sealed under a passphrase they know. This is that copy.
//!
//! The key is made from the passphrase by Argon2id (memory-hard, so that guessing is paid for in
//! memory as well as time), with a salt drawn here from the operating system, and the copy is sealed
//! with XChaCha20-Poly1305 under a nonce drawn the same way. The header (the cost, the salt) is
//! authenticated with the rest, and a copy that asks for more memory or time than any device offers
//! is refused before it is paid for, so that a backup file cannot be made a way to exhaust a host. A
//! wrong passphrase and a changed copy look the same, which is all a copy can say.
//!
//! What is kept is [`Contents`]: the list, the node, the seed that makes the device's key, and named
//! extras. The keys of an epoch the list had destroyed ([`crate::membership::Body::Shred`]) are not
//! kept, and are not put back.
//!
//! The same contents are what a device keeps of itself between runs, sealed under a key its host's
//! platform key store wraps rather than under a passphrase ([`seal_state`], [`open_state`]): envelope
//! encryption, the pattern of a key store whose keys are not exportable. A fresh [`DataKey`] seals
//! one state; the host wraps that key with its platform's key and keeps the wrapped key beside the
//! sealed state, and gives the key back unwrapped to open it. The cipher is the same
//! XChaCha20-Poly1305. Its key and the key the nonce is made with are drawn from the data key by
//! HKDF-Expand (RFC 5869 section 2.3; the data key is uniformly random, so it is the pseudorandom
//! key and needs no extraction), and the nonce is HMAC-SHA-256 under the second of what is sealed,
//! cut to 24 bytes: a synthetic nonce (the SIV construction of deterministic authenticated
//! encryption). So a sealed state is the same bytes for the same key and the same contents, which a
//! recording holds, and a key a host uses twice gives away only whether two states were the same,
//! never the keystream. The list, the node and the caller's name of what is sealed are associated
//! data: a state opens only for the device it was sealed for.

use core::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use hkdf::Hkdf;
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use zeroize::Zeroize;

use crate::encoding;
use crate::identity::Identity;
use crate::trust::Trust;

const DOMAIN: &str = "sce-trust/backup/1";
const MAGIC: &[u8; 5] = b"SCEB1";

/// What a sealed state is bound to beside the list, the node and the caller's name of it.
const STATE_DOMAIN: &str = "sce-trust/state/1";
/// The first bytes of a sealed state: what it is, and the version of its form.
const STATE_MAGIC: &[u8; 5] = b"SCES1";
/// The labels the cipher's key and the nonce's key are drawn from a data key under.
const STATE_SEAL_LABEL: &str = "sce-trust/state/1 seal";
const STATE_NONCE_LABEL: &str = "sce-trust/state/1 nonce";
/// The lengths of XChaCha20-Poly1305's nonce and tag.
const NONCE_LEN: usize = 24;
const TAG_LEN: usize = 16;

/// What a copy asks of whoever opens it: Argon2id's memory (KiB), passes and lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cost {
    pub memory_kib: u32,
    pub passes: u32,
    pub lanes: u32,
}

impl Cost {
    /// 64 MiB, 3 passes, 1 lane: what a phone or a laptop does in about a second, and what an
    /// attacker with a stolen copy pays for each guess.
    pub const fn standard() -> Self {
        Self {
            memory_kib: 64 * 1024,
            passes: 3,
            lanes: 1,
        }
    }

    /// The most a copy may ask: a device that opens one pays this at most.
    const MAX: Cost = Cost {
        memory_kib: 1024 * 1024,
        passes: 16,
        lanes: 16,
    };
}

/// Why a copy was not made or not opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackupError {
    /// It is not what [`seal`] or [`seal_state`] writes: another form, or bytes cut short of what
    /// any sealed text holds.
    Malformed,
    /// It asks for more memory, passes or lanes than a device offers, or for none.
    TooCostly,
    /// The passphrase or the key is not the one it was sealed under, it was changed, or (a sealed
    /// state) it was sealed for another list, node or form. A cipher cannot tell these apart.
    Corrupt,
    /// The operating system gave no randomness, and no weaker source is used in its place.
    NoRandomness,
}

/// What a device keeps of itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Contents {
    pub list: String,
    pub node: u64,
    /// The seed that makes the device's key (`Identity::from_seed`).
    pub seed: [u8; 32],
    extras: Vec<(String, Vec<u8>)>,
}

impl Contents {
    pub fn new(list: &str, node: u64, seed: [u8; 32]) -> Self {
        Self {
            list: list.to_string(),
            node,
            seed,
            extras: Vec::new(),
        }
    }

    /// The contents of the device of `node` in `list`, whose key is `identity`: its seed is read
    /// here, and leaves this crate only sealed.
    pub fn of_identity(list: &str, node: u64, identity: &Identity) -> Self {
        let mut seed = identity.seed();
        let contents = Self::new(list, node, seed);
        seed.zeroize();
        contents
    }

    /// The device's key, as its seed makes it.
    pub fn identity(&self) -> Identity {
        Identity::from_seed(self.seed)
    }

    /// Keep the key `identity` under `name` too: an administrator's key the device holds, say.
    pub fn with_identity(self, name: &str, identity: &Identity) -> Self {
        let mut seed = identity.seed();
        let contents = self.with(name, seed.to_vec());
        seed.zeroize();
        contents
    }

    /// The key kept under `name` ([`Self::with_identity`]); `None` when nothing is kept there, or
    /// what is kept is not a seed.
    pub fn identity_named(&self, name: &str) -> Option<Identity> {
        let mut seed: [u8; 32] = self.extra(name)?.try_into().ok()?;
        let identity = Identity::from_seed(seed);
        seed.zeroize();
        Some(identity)
    }

    /// Keep `bytes` under `name` too: the device's MLS state, say.
    pub fn with(mut self, name: &str, bytes: Vec<u8>) -> Self {
        self.extras.retain(|(held, _)| held != name);
        self.extras.push((name.to_string(), bytes));
        self
    }

    pub fn extra(&self, name: &str) -> Option<&[u8]> {
        self.extras
            .iter()
            .find(|(held, _)| held == name)
            .map(|(_, bytes)| bytes.as_slice())
    }

    /// Keep the keys of the epochs `trust` holds (and none of an epoch the list had destroyed, which
    /// it holds none of).
    pub fn with_epoch_keys(mut self, trust: &Trust) -> Self {
        for (epoch, key) in trust.keys_for_backup() {
            self = self.with(&format!("epoch-key/{epoch}"), key.to_vec());
        }
        self
    }

    /// Give `trust` back the keys of the epochs this copy holds, except those the list has had
    /// destroyed since. Returns how many it took.
    pub fn restore_epoch_keys(&self, trust: &Trust) -> usize {
        let mut taken = 0;
        for (name, bytes) in &self.extras {
            let Some(epoch) = name
                .strip_prefix("epoch-key/")
                .and_then(|epoch| epoch.parse::<u32>().ok())
            else {
                continue;
            };
            if let Ok(key) = <[u8; 32]>::try_from(bytes.as_slice()) {
                taken += usize::from(trust.set_epoch_key(epoch, key));
            }
        }
        taken
    }

    fn to_bytes(&self) -> Vec<u8> {
        // The length is counted first, so that the buffer is never grown: a buffer that grows leaves
        // a copy of the secrets it held where it was.
        let parts = 4 + 2 * self.extras.len();
        let length = MAGIC.len()
            + 8 * parts
            + self.list.len()
            + 8
            + self.seed.len()
            + 8
            + self
                .extras
                .iter()
                .map(|(name, bytes)| name.len() + bytes.len())
                .sum::<usize>();
        let mut out = Vec::with_capacity(length);
        out.extend_from_slice(MAGIC);
        let mut put = |bytes: &[u8]| {
            out.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
            out.extend_from_slice(bytes);
        };
        put(self.list.as_bytes());
        put(&self.node.to_be_bytes());
        put(&self.seed);
        put(&(self.extras.len() as u64).to_be_bytes());
        for (name, bytes) in &self.extras {
            put(name.as_bytes());
            put(bytes);
        }
        out
    }

    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        let mut rest = bytes.strip_prefix(MAGIC.as_slice())?;
        let mut take = || -> Option<Vec<u8>> {
            let length =
                usize::try_from(u64::from_be_bytes(rest.get(..8)?.try_into().ok()?)).ok()?;
            let part = rest.get(8..8usize.checked_add(length)?)?.to_vec();
            rest = &rest[8 + length..];
            Some(part)
        };
        let list = String::from_utf8(take()?).ok()?;
        let node = u64::from_be_bytes(take()?.try_into().ok()?);
        let seed: [u8; 32] = take()?.try_into().ok()?;
        let count = usize::try_from(u64::from_be_bytes(take()?.try_into().ok()?)).ok()?;
        let mut extras = Vec::new();
        for _ in 0..count {
            let name = String::from_utf8(take()?).ok()?;
            extras.push((name, take()?));
        }
        rest.is_empty().then_some(Self {
            list,
            node,
            seed,
            extras,
        })
    }
}

impl Drop for Contents {
    fn drop(&mut self) {
        self.seed.zeroize();
        for (_, bytes) in &mut self.extras {
            bytes.zeroize();
        }
    }
}

fn header(cost: Cost, salt: &[u8; 16]) -> String {
    format!(
        "{DOMAIN};argon2id;m={};t={};p={};salt={}",
        cost.memory_kib,
        cost.passes,
        cost.lanes,
        encoding::encode(salt)
    )
}

fn key_of(passphrase: &[u8], cost: Cost, salt: &[u8; 16]) -> Result<[u8; 32], BackupError> {
    if cost.memory_kib == 0
        || cost.passes == 0
        || cost.lanes == 0
        || cost.memory_kib > Cost::MAX.memory_kib
        || cost.passes > Cost::MAX.passes
        || cost.lanes > Cost::MAX.lanes
    {
        return Err(BackupError::TooCostly);
    }
    let params = Params::new(cost.memory_kib, cost.passes, cost.lanes, Some(32))
        .map_err(|_| BackupError::TooCostly)?;
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(passphrase, salt, &mut key)
        .map_err(|_| BackupError::TooCostly)?;
    Ok(key)
}

/// A copy of `contents` sealed under `passphrase`, with a salt and a nonce drawn from the operating
/// system.
pub fn seal(passphrase: &[u8], contents: &Contents, cost: Cost) -> Result<String, BackupError> {
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom::getrandom(&mut salt).map_err(|_| BackupError::NoRandomness)?;
    getrandom::getrandom(&mut nonce).map_err(|_| BackupError::NoRandomness)?;
    seal_with(passphrase, contents, cost, salt, nonce)
}

/// [`seal`] with the salt and nonce given: for the known-answer vector, and for a host that has its
/// own source of randomness. A salt or a nonce used twice with a passphrase weakens a copy.
pub fn seal_with(
    passphrase: &[u8],
    contents: &Contents,
    cost: Cost,
    salt: [u8; 16],
    nonce: [u8; 24],
) -> Result<String, BackupError> {
    let mut key = key_of(passphrase, cost, &salt)?;
    let mut plain = contents.to_bytes();
    let sealed = XChaCha20Poly1305::new(Key::from_slice(&key))
        .encrypt(
            XNonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: header(cost, &salt).as_bytes(),
            },
        )
        .map_err(|_| BackupError::Corrupt);
    key.zeroize();
    plain.zeroize();
    Ok(json!({
        "v": 1,
        "kdf": "argon2id",
        "m": cost.memory_kib,
        "t": cost.passes,
        "p": cost.lanes,
        "salt": encoding::encode(&salt),
        "nonce": encoding::encode(&nonce),
        "ct": encoding::encode(&sealed?),
    })
    .to_string())
}

/// The contents a copy holds, when `passphrase` is the one it was sealed under.
pub fn open(passphrase: &[u8], text: &str) -> Result<Contents, BackupError> {
    let value: Value = serde_json::from_str(text).map_err(|_| BackupError::Malformed)?;
    if value.get("v").and_then(Value::as_u64) != Some(1)
        || value.get("kdf").and_then(Value::as_str) != Some("argon2id")
    {
        return Err(BackupError::Malformed);
    }
    let number = |name: &str| -> Result<u32, BackupError> {
        u32::try_from(
            value
                .get(name)
                .and_then(Value::as_u64)
                .ok_or(BackupError::Malformed)?,
        )
        .map_err(|_| BackupError::TooCostly)
    };
    let cost = Cost {
        memory_kib: number("m")?,
        passes: number("t")?,
        lanes: number("p")?,
    };
    let bytes = |name: &str| -> Result<Vec<u8>, BackupError> {
        value
            .get(name)
            .and_then(Value::as_str)
            .and_then(encoding::decode)
            .ok_or(BackupError::Malformed)
    };
    let salt: [u8; 16] = bytes("salt")?
        .try_into()
        .map_err(|_| BackupError::Malformed)?;
    let nonce: [u8; 24] = bytes("nonce")?
        .try_into()
        .map_err(|_| BackupError::Malformed)?;
    let ciphertext = bytes("ct")?;
    let mut key = key_of(passphrase, cost, &salt)?;
    let opened = XChaCha20Poly1305::new(Key::from_slice(&key)).decrypt(
        XNonce::from_slice(&nonce),
        Payload {
            msg: &ciphertext,
            aad: header(cost, &salt).as_bytes(),
        },
    );
    key.zeroize();
    let mut plain = opened.map_err(|_| BackupError::Corrupt)?;
    let contents = Contents::from_bytes(&plain).ok_or(BackupError::Malformed);
    plain.zeroize();
    contents
}

/// The key one sealed state is sealed under ([`seal_state`]). The host wraps it with its platform's
/// key store and keeps the wrapped key beside the sealed state. It does not print itself, and is
/// zeroed when it is dropped.
pub struct DataKey([u8; 32]);

impl DataKey {
    /// A new key, drawn from the operating system's source of randomness, for one state.
    pub fn draw() -> Result<Self, BackupError> {
        let mut key = Self([0u8; 32]);
        getrandom::getrandom(&mut key.0).map_err(|_| BackupError::NoRandomness)?;
        Ok(key)
    }

    /// The key whose 32 bytes are `bytes`: the one a host unwrapped, or one a host with its own
    /// source of randomness drew.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The key's bytes, for the host to wrap.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// A key of its own drawn from this one under `label` (HKDF-Expand with SHA-256).
    fn derive(&self, label: &str) -> Result<[u8; 32], BackupError> {
        let expander = Hkdf::<Sha256>::from_prk(&self.0).map_err(|_| BackupError::Malformed)?;
        let mut derived = [0u8; 32];
        expander
            .expand(label.as_bytes(), &mut derived)
            .map_err(|_| BackupError::Malformed)?;
        Ok(derived)
    }
}

impl Drop for DataKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

impl fmt::Debug for DataKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DataKey(..)")
    }
}

/// What a sealed state is bound to: its domain, the list, the node and `context` (the caller's name
/// of what it seals and of its version), each written after its length, so that no two of them
/// write the same bytes.
fn state_header(list: &str, node: u64, context: &str) -> Vec<u8> {
    let node = node.to_be_bytes();
    let parts: [&[u8]; 4] = [
        STATE_DOMAIN.as_bytes(),
        list.as_bytes(),
        &node,
        context.as_bytes(),
    ];
    let mut header = Vec::new();
    for part in parts {
        header.extend_from_slice(&(part.len() as u64).to_be_bytes());
        header.extend_from_slice(part);
    }
    header
}

/// The nonce of `plain` sealed with `header`: HMAC-SHA-256 under `key` of both, cut to the
/// nonce's length.
fn synthetic_nonce(
    key: &[u8; 32],
    header: &[u8],
    plain: &[u8],
) -> Result<[u8; NONCE_LEN], BackupError> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).map_err(|_| BackupError::Malformed)?;
    mac.update(&(header.len() as u64).to_be_bytes());
    mac.update(header);
    mac.update(plain);
    let tag = mac.finalize().into_bytes();
    let mut nonce = [0u8; NONCE_LEN];
    nonce.copy_from_slice(&tag[..NONCE_LEN]);
    Ok(nonce)
}

/// `contents` sealed under `key` for the device of its node in its list, as what `context` names
/// (the caller's form, with its version): the bytes [`open_state`] takes back. The same key and
/// the same contents give the same bytes; the host draws a new key for each state
/// ([`DataKey::draw`]), wraps it, and keeps it beside them.
pub fn seal_state(
    key: &DataKey,
    context: &str,
    contents: &Contents,
) -> Result<Vec<u8>, BackupError> {
    let header = state_header(&contents.list, contents.node, context);
    let mut cipher_key = key.derive(STATE_SEAL_LABEL)?;
    let mut nonce_key = key.derive(STATE_NONCE_LABEL)?;
    let mut plain = contents.to_bytes();
    let sealed = synthetic_nonce(&nonce_key, &header, &plain).and_then(|nonce| {
        XChaCha20Poly1305::new(Key::from_slice(&cipher_key))
            .encrypt(
                XNonce::from_slice(&nonce),
                Payload {
                    msg: &plain,
                    aad: &header,
                },
            )
            .map(|ciphertext| (nonce, ciphertext))
            .map_err(|_| BackupError::Corrupt)
    });
    cipher_key.zeroize();
    nonce_key.zeroize();
    plain.zeroize();
    let (nonce, ciphertext) = sealed?;
    let mut out = Vec::with_capacity(STATE_MAGIC.len() + NONCE_LEN + ciphertext.len());
    out.extend_from_slice(STATE_MAGIC);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(out)
}

/// The contents [`seal_state`] sealed under `key` for the device of `node` in `list`, as what
/// `context` names. [`BackupError::Malformed`] when the bytes are not a sealed state, or are cut
/// short of the least one holds; [`BackupError::Corrupt`] when they do not open: another key, a
/// changed byte, a state cut inside what it seals, or one sealed for another list, node or
/// context.
pub fn open_state(
    key: &DataKey,
    list: &str,
    node: u64,
    context: &str,
    sealed: &[u8],
) -> Result<Contents, BackupError> {
    let rest = sealed
        .strip_prefix(STATE_MAGIC.as_slice())
        .ok_or(BackupError::Malformed)?;
    if rest.len() < NONCE_LEN + TAG_LEN {
        return Err(BackupError::Malformed);
    }
    let (nonce, ciphertext) = rest.split_at(NONCE_LEN);
    let header = state_header(list, node, context);
    let mut cipher_key = key.derive(STATE_SEAL_LABEL)?;
    let opened = XChaCha20Poly1305::new(Key::from_slice(&cipher_key)).decrypt(
        XNonce::from_slice(nonce),
        Payload {
            msg: ciphertext,
            aad: &header,
        },
    );
    cipher_key.zeroize();
    let mut plain = opened.map_err(|_| BackupError::Corrupt)?;
    // The list and the node are bound already; what is inside is held to them too, so that no
    // reader takes contents of one device as another's.
    let contents = Contents::from_bytes(&plain)
        .filter(|contents| contents.list == list && contents.node == node)
        .ok_or(BackupError::Malformed);
    plain.zeroize();
    contents
}

#[cfg(test)]
mod tests {
    use super::*;

    const WEAK: Cost = Cost {
        memory_kib: 64,
        passes: 1,
        lanes: 1,
    };

    fn contents() -> Contents {
        Contents::new("home", 4, [7; 32])
            .with("mls", vec![1, 2, 3])
            .with("epoch-key/2", vec![9; 32])
    }

    #[test]
    fn what_is_sealed_opens_under_the_passphrase_and_under_no_other() {
        let text = seal(b"correct horse", &contents(), WEAK).unwrap();
        assert_eq!(open(b"correct horse", &text), Ok(contents()));
        assert_eq!(open(b"correct horsf", &text), Err(BackupError::Corrupt));
        assert_eq!(open(b"", &text), Err(BackupError::Corrupt));
        assert!(
            !text.contains("correct"),
            "the passphrase is not in the copy"
        );
    }

    #[test]
    fn two_copies_of_one_thing_are_not_the_same_bytes() {
        let a = seal(b"pass", &contents(), WEAK).unwrap();
        let b = seal(b"pass", &contents(), WEAK).unwrap();
        assert_ne!(a, b, "a salt and a nonce of each");
    }

    #[test]
    fn a_changed_copy_does_not_open_whichever_part_was_changed() {
        let text = seal_with(b"pass", &contents(), WEAK, [1; 16], [2; 24]).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        for field in ["salt", "nonce", "ct"] {
            let mut changed = value.clone();
            let mut bytes = encoding::decode(changed[field].as_str().unwrap()).unwrap();
            bytes[0] ^= 1;
            changed[field] = Value::String(encoding::encode(&bytes));
            assert_eq!(
                open(b"pass", &changed.to_string()),
                Err(BackupError::Corrupt),
                "{field}"
            );
        }
        // The cost is part of what is authenticated, as well as of the key.
        let mut changed = value.clone();
        changed["t"] = json!(2);
        assert_eq!(
            open(b"pass", &changed.to_string()),
            Err(BackupError::Corrupt)
        );
    }

    #[test]
    fn a_copy_that_asks_for_more_than_a_device_offers_is_refused_before_it_is_paid_for() {
        let text = seal_with(b"pass", &contents(), WEAK, [1; 16], [2; 24]).unwrap();
        let value: Value = serde_json::from_str(&text).unwrap();
        for (field, cost) in [
            ("m", json!(4_000_000)),
            ("t", json!(1000)),
            ("p", json!(64)),
            ("m", json!(0)),
        ] {
            let mut asked = value.clone();
            asked[field] = cost;
            assert_eq!(
                open(b"pass", &asked.to_string()),
                Err(BackupError::TooCostly),
                "{field}"
            );
        }
        let mut huge = value.clone();
        huge["m"] = json!(u64::MAX);
        assert_eq!(
            open(b"pass", &huge.to_string()),
            Err(BackupError::TooCostly)
        );
        assert_eq!(
            seal(
                b"pass",
                &contents(),
                Cost {
                    memory_kib: 0,
                    passes: 1,
                    lanes: 1
                }
            ),
            Err(BackupError::TooCostly)
        );
    }

    #[test]
    fn what_is_not_a_copy_is_refused() {
        for text in [
            "",
            "not json",
            "{}",
            r#"{"v":2}"#,
            r#"{"v":1,"kdf":"scrypt"}"#,
        ] {
            assert_eq!(open(b"pass", text), Err(BackupError::Malformed), "{text}");
        }
    }

    #[test]
    fn a_copy_that_names_another_key_derivation_is_refused_though_it_is_whole() {
        let text = seal(b"pass", &contents(), WEAK).unwrap();
        let renamed = text.replacen("\"kdf\":\"argon2id\"", "\"kdf\":\"argon2i\"", 1);
        assert_ne!(
            renamed, text,
            "the copy names its derivation as written here"
        );
        assert_eq!(open(b"pass", &renamed), Err(BackupError::Malformed));
        assert!(open(b"pass", &text).is_ok());
    }

    #[test]
    fn contents_with_bytes_left_after_the_last_extra_are_not_read() {
        let bytes = contents().to_bytes();
        assert_eq!(Contents::from_bytes(&bytes), Some(contents()));
        let mut longer = bytes;
        longer.push(0);
        assert_eq!(Contents::from_bytes(&longer), None);
    }

    fn key(byte: u8) -> DataKey {
        DataKey::from_bytes([byte; 32])
    }

    #[test]
    fn a_state_sealed_under_a_key_opens_under_it_for_its_device_alone() {
        let sealed = seal_state(&key(5), "peer/3", &contents()).unwrap();
        assert_eq!(
            open_state(&key(5), "home", 4, "peer/3", &sealed),
            Ok(contents())
        );
        for (given, list, node, context, what) in [
            (key(6), "home", 4, "peer/3", "another key"),
            (key(5), "work", 4, "peer/3", "another list"),
            (key(5), "home", 5, "peer/3", "another node"),
            (key(5), "home", 4, "peer/4", "another form"),
        ] {
            assert_eq!(
                open_state(&given, list, node, context, &sealed),
                Err(BackupError::Corrupt),
                "{what}"
            );
        }
    }

    #[test]
    fn a_sealed_state_changed_or_cut_does_not_open() {
        let sealed = seal_state(&key(5), "peer/3", &contents()).unwrap();
        let open = |bytes: &[u8]| open_state(&key(5), "home", 4, "peer/3", bytes);
        // The nonce, the first byte sealed, the last byte of the tag.
        for at in [
            STATE_MAGIC.len(),
            STATE_MAGIC.len() + NONCE_LEN,
            sealed.len() - 1,
        ] {
            let mut changed = sealed.clone();
            changed[at] ^= 1;
            assert_eq!(open(&changed), Err(BackupError::Corrupt), "{at}");
        }
        assert_eq!(
            open(&sealed[..sealed.len() - 1]),
            Err(BackupError::Corrupt),
            "cut inside what it seals"
        );
        for length in [
            0,
            STATE_MAGIC.len(),
            STATE_MAGIC.len() + NONCE_LEN + TAG_LEN - 1,
        ] {
            assert_eq!(
                open(&sealed[..length]),
                Err(BackupError::Malformed),
                "{length}"
            );
        }
        let mut renamed = sealed.clone();
        renamed[0] = b'X';
        assert_eq!(open(&renamed), Err(BackupError::Malformed));
    }

    #[test]
    fn a_state_is_the_same_bytes_under_one_key_and_holds_no_secret_in_the_clear() {
        let sealed = seal_state(&key(5), "peer/3", &contents()).unwrap();
        assert_eq!(
            seal_state(&key(5), "peer/3", &contents()).unwrap(),
            sealed,
            "the same key and contents give the same bytes"
        );
        let nonce = STATE_MAGIC.len()..STATE_MAGIC.len() + NONCE_LEN;
        let other = seal_state(&key(5), "peer/3", &contents().with("mls", vec![4])).unwrap();
        assert_ne!(
            other[nonce.clone()],
            sealed[nonce],
            "another state under the same key has another nonce"
        );
        assert_ne!(seal_state(&key(6), "peer/3", &contents()).unwrap(), sealed);
        assert!(
            !sealed.windows(32).any(|window| window == [7u8; 32]),
            "the seed is not in the sealed state"
        );
        assert!(
            !sealed.windows(32).any(|window| window == [9u8; 32]),
            "nor the key of an epoch"
        );
    }

    #[test]
    fn a_data_key_drawn_is_new_each_time_and_prints_nothing() {
        let (a, b) = (DataKey::draw().unwrap(), DataKey::draw().unwrap());
        assert_ne!(a.as_bytes(), b.as_bytes());
        assert_eq!(format!("{a:?}"), "DataKey(..)");
    }

    #[test]
    fn a_key_kept_in_the_contents_is_the_same_key() {
        let person = Identity::from_seed([3; 32]);
        let administrator = Identity::from_seed([9; 32]);
        let contents = Contents::of_identity("home", 3, &person)
            .with_identity("administrator", &administrator);
        assert_eq!(contents.seed, [3; 32]);
        assert_eq!(contents.identity().public(), person.public());
        assert_eq!(
            contents
                .identity_named("administrator")
                .map(|key| key.public()),
            Some(administrator.public())
        );
        assert!(contents.identity_named("nothing").is_none());
        assert!(contents
            .with("short", vec![1])
            .identity_named("short")
            .is_none());
    }

    #[test]
    fn extras_are_named_and_a_name_is_kept_once() {
        let kept = contents().with("mls", vec![4, 5]);
        assert_eq!(kept.extra("mls"), Some([4u8, 5].as_slice()));
        assert_eq!(kept.extra("nothing"), None);
    }
}
