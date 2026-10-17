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

use argon2::{Algorithm, Argon2, Params, Version};
use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use serde_json::{json, Value};
use zeroize::Zeroize;

use crate::encoding;
use crate::trust::Trust;

const DOMAIN: &str = "sce-trust/backup/1";
const MAGIC: &[u8; 5] = b"SCEB1";

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
    /// It is not what [`seal`] writes.
    Malformed,
    /// It asks for more memory, passes or lanes than a device offers, or for none.
    TooCostly,
    /// The passphrase is not the one it was sealed under, or it was changed.
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
        let mut out = MAGIC.to_vec();
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

    #[test]
    fn extras_are_named_and_a_name_is_kept_once() {
        let kept = contents().with("mls", vec![4, 5]);
        assert_eq!(kept.extra("mls"), Some([4u8, 5].as_slice()));
        assert_eq!(kept.extra("nothing"), None);
    }
}
