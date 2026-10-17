// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Keeping the content of a write from the hub.
//!
//! A list has a key for each epoch, held by its members and by no one else, and a write that
//! leaves a member's host has its number and its text sealed under the key of the latest epoch:
//! what the hub holds and relays is the stamp of the write (which to-do, which field, when, by
//! whom) and a text it cannot read. The stamp stays in the clear, because the hub's machine
//! orders, merges, forgets and compares digests by it; what a hub learns is that a member changed
//! a field of a to-do at a time, and not what it was changed to.
//!
//! The cipher is XChaCha20-Poly1305. Its nonce is made from the stamp, which every write has
//! of its own, so that a host needs no source of randomness and one write is sealed one way
//! (a host that sends a write again sends the same bytes); the stamp is also the cipher's
//! associated data with the list's name and the epoch, so a sealed text moved to another write,
//! another list or another epoch does not open.
//!
//! A key is given to a member sealed under what their key and the giver's agree on
//! ([`crate::Identity::agree`]), so that it travels over any channel and the host that carries it
//! learns nothing.

use std::collections::BTreeMap;

use chacha20poly1305::aead::{Aead, KeyInit, Payload};
use chacha20poly1305::{Key, XChaCha20Poly1305, XNonce};
use sha2::{Digest, Sha256};

use crate::encoding;
use crate::identity::PublicKey;

/// A sealed text begins with this, so that a host that holds keys can tell it from a text
/// that was written in the clear, which it does not take.
pub const SEALED_PREFIX: &str = "~";

const NONCE_DOMAIN: &str = "sce-trust/nonce/1";
const AAD_DOMAIN: &str = "sce-trust/content/1";
const WRAP_DOMAIN: &str = "sce-trust/wrap/1";

/// A key of one epoch of a list.
pub type EpochKey = [u8; 32];

/// A new key, drawn from the operating system's source of randomness. A host that has no such
/// source (it cannot run this crate's host) fails here, and never falls back to a weaker one.
pub fn generate_key() -> Result<EpochKey, getrandom::Error> {
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key)?;
    Ok(key)
}

/// Why a sealed text did not open.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenError {
    /// It is not what [`seal`] writes.
    NotSealed,
    /// The host holds no key of the epoch it was sealed under.
    NoKey { epoch: u32 },
    /// The key does not open it: it was sealed under another key, moved from another write or
    /// list, or changed on the way.
    Corrupt,
}

fn digest(parts: &[&[u8]]) -> [u8; 32] {
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    hash.finalize().into()
}

fn nonce_of(calendar: &str, epoch: u32, stamp: &[u8]) -> [u8; 24] {
    let digest = digest(&[
        NONCE_DOMAIN.as_bytes(),
        calendar.as_bytes(),
        &epoch.to_be_bytes(),
        stamp,
    ]);
    let mut nonce = [0u8; 24];
    nonce.copy_from_slice(&digest[..24]);
    nonce
}

fn aad_of(calendar: &str, epoch: u32, stamp: &[u8]) -> Vec<u8> {
    let mut aad = Vec::new();
    for part in [
        AAD_DOMAIN.as_bytes(),
        calendar.as_bytes(),
        &epoch.to_be_bytes(),
        stamp,
    ] {
        aad.extend_from_slice(&(part.len() as u64).to_be_bytes());
        aad.extend_from_slice(part);
    }
    aad
}

/// The least a text takes in what is sealed, and the step by which a longer one grows: a sealed
/// write is the same length whether it holds a number, a word or a line of the longest a person
/// types (32 bytes), so what a relay sees of its size says nothing of what it holds.
pub const TEXT_CLASS: usize = 32;

/// `value` and `text` of the write whose stamp is `stamp`, sealed under `key` of `epoch`, as
/// the text that is put in the write's `text`: [`SEALED_PREFIX`] and the epoch and the
/// ciphertext in the URL alphabet, 84 characters for any text of up to [`TEXT_CLASS`] bytes.
/// What is sealed is the number, the length of the text, the text, and zeros up to the class.
pub fn seal(
    calendar: &str,
    epoch: u32,
    key: &EpochKey,
    stamp: &[u8],
    value: i64,
    text: &str,
) -> String {
    let mut plain = value.to_be_bytes().to_vec();
    plain.extend_from_slice(&(text.len() as u16).to_be_bytes());
    plain.extend_from_slice(text.as_bytes());
    let padded = 10 + text.len().div_ceil(TEXT_CLASS).max(1) * TEXT_CLASS;
    plain.resize(padded, 0);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let sealed = cipher
        .encrypt(
            XNonce::from_slice(&nonce_of(calendar, epoch, stamp)),
            Payload {
                msg: &plain,
                aad: &aad_of(calendar, epoch, stamp),
            },
        )
        .expect("a message this short is sealed");
    let mut bytes = epoch.to_be_bytes().to_vec();
    bytes.extend_from_slice(&sealed);
    format!("{SEALED_PREFIX}{}", encoding::encode(&bytes))
}

/// The epoch a sealed text was sealed under, when it is one.
pub fn epoch_of(sealed: &str) -> Option<u32> {
    let bytes = encoding::decode(sealed.strip_prefix(SEALED_PREFIX)?)?;
    Some(u32::from_be_bytes(bytes.get(..4)?.try_into().ok()?))
}

/// What [`seal`] sealed, given back by the key of its epoch.
pub fn open(
    calendar: &str,
    keys: &BTreeMap<u32, EpochKey>,
    stamp: &[u8],
    sealed: &str,
) -> Result<(i64, String), OpenError> {
    let bytes = encoding::decode(
        sealed
            .strip_prefix(SEALED_PREFIX)
            .ok_or(OpenError::NotSealed)?,
    )
    .ok_or(OpenError::NotSealed)?;
    let epoch = u32::from_be_bytes(
        bytes
            .get(..4)
            .ok_or(OpenError::NotSealed)?
            .try_into()
            .map_err(|_| OpenError::NotSealed)?,
    );
    let key = keys.get(&epoch).ok_or(OpenError::NoKey { epoch })?;
    let cipher = XChaCha20Poly1305::new(Key::from_slice(key));
    let plain = cipher
        .decrypt(
            XNonce::from_slice(&nonce_of(calendar, epoch, stamp)),
            Payload {
                msg: &bytes[4..],
                aad: &aad_of(calendar, epoch, stamp),
            },
        )
        .map_err(|_| OpenError::Corrupt)?;
    if plain.len() < 10 + TEXT_CLASS || (plain.len() - 10) % TEXT_CLASS != 0 {
        return Err(OpenError::Corrupt);
    }
    let value = i64::from_be_bytes(plain[..8].try_into().map_err(|_| OpenError::Corrupt)?);
    let length = usize::from(u16::from_be_bytes([plain[8], plain[9]]));
    let body = plain.get(10..10 + length).ok_or(OpenError::Corrupt)?;
    // Padding is zeros and no more than the class takes: one write has one sealed form.
    if plain[10 + length..].iter().any(|byte| *byte != 0)
        || plain.len() != 10 + length.div_ceil(TEXT_CLASS).max(1) * TEXT_CLASS
    {
        return Err(OpenError::Corrupt);
    }
    let text = String::from_utf8(body.to_vec()).map_err(|_| OpenError::Corrupt)?;
    Ok((value, text))
}

fn wrapping_key(shared: &[u8; 32], from: &PublicKey, to: &PublicKey) -> [u8; 32] {
    digest(&[
        WRAP_DOMAIN.as_bytes(),
        shared,
        &from.to_bytes(),
        &to.to_bytes(),
    ])
}

/// `key` of `epoch` of `calendar`, sealed so that only the holder of `to`'s key opens it:
/// what `shared` is (the secret `from` and `to` agree on) is what it is sealed under.
pub fn wrap(
    calendar: &str,
    shared: &[u8; 32],
    from: &PublicKey,
    to: &PublicKey,
    epoch: u32,
    key: &EpochKey,
) -> String {
    let wrapping = wrapping_key(shared, from, to);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&wrapping));
    let nonce = digest(&[
        WRAP_DOMAIN.as_bytes(),
        calendar.as_bytes(),
        &epoch.to_be_bytes(),
    ]);
    let sealed = cipher
        .encrypt(
            XNonce::from_slice(&nonce[..24]),
            Payload {
                msg: key,
                aad: calendar.as_bytes(),
            },
        )
        .expect("a key is sealed");
    encoding::encode(&sealed)
}

/// What [`wrap`] sealed, given back, when `shared` is the secret it was sealed under.
pub fn unwrap(
    calendar: &str,
    shared: &[u8; 32],
    from: &PublicKey,
    to: &PublicKey,
    epoch: u32,
    wrapped: &str,
) -> Option<EpochKey> {
    let wrapping = wrapping_key(shared, from, to);
    let cipher = XChaCha20Poly1305::new(Key::from_slice(&wrapping));
    let nonce = digest(&[
        WRAP_DOMAIN.as_bytes(),
        calendar.as_bytes(),
        &epoch.to_be_bytes(),
    ]);
    let plain = cipher
        .decrypt(
            XNonce::from_slice(&nonce[..24]),
            Payload {
                msg: &encoding::decode(wrapped)?,
                aad: calendar.as_bytes(),
            },
        )
        .ok()?;
    plain.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Identity;

    fn keys(epoch: u32, key: EpochKey) -> BTreeMap<u32, EpochKey> {
        BTreeMap::from([(epoch, key)])
    }

    #[test]
    fn what_is_sealed_opens_with_its_key_and_the_stamp_it_was_sealed_for() {
        let key = [7u8; 32];
        let sealed = seal("home", 3, &key, b"stamp", -42, "buy milk");
        assert!(sealed.starts_with(SEALED_PREFIX));
        assert_eq!(epoch_of(&sealed), Some(3));
        assert_eq!(
            open("home", &keys(3, key), b"stamp", &sealed),
            Ok((-42, "buy milk".to_string()))
        );
        assert_eq!(
            open("home", &keys(3, key), b"another", &sealed),
            Err(OpenError::Corrupt),
            "moved to another write"
        );
        assert_eq!(
            open("work", &keys(3, key), b"stamp", &sealed),
            Err(OpenError::Corrupt),
            "moved to another list"
        );
        assert_eq!(
            open("home", &keys(3, [8; 32]), b"stamp", &sealed),
            Err(OpenError::Corrupt)
        );
        assert_eq!(
            open("home", &keys(4, key), b"stamp", &sealed),
            Err(OpenError::NoKey { epoch: 3 })
        );
        assert_eq!(
            open("home", &keys(3, key), b"stamp", "buy milk"),
            Err(OpenError::NotSealed)
        );
    }

    #[test]
    fn a_generated_key_is_new_each_time() {
        let a = generate_key().unwrap();
        let b = generate_key().unwrap();
        assert_ne!(a, b);
        assert_ne!(a, [0u8; 32]);
    }

    #[test]
    fn a_text_of_the_longest_a_person_writes_fits_the_room_a_write_has() {
        let sealed = seal("home", u32::MAX, &[1; 32], b"s", i64::MIN, &"x".repeat(32));
        assert!(sealed.len() <= 96, "{}", sealed.len());
        assert_eq!(sealed.len(), 84);
    }

    /// What a relay sees of the size says nothing of the text, up to what a person types; a longer
    /// one grows by a class.
    #[test]
    fn a_sealed_write_is_one_length_for_every_text_up_to_a_class() {
        let key = [7u8; 32];
        let sizes: Vec<usize> = ["", "a", "buy milk", &"x".repeat(32), &"y".repeat(33)]
            .iter()
            .map(|text| seal("home", 1, &key, b"s", 5, text).len())
            .collect();
        assert_eq!(sizes[..4], [84, 84, 84, 84]);
        assert!(sizes[4] > sizes[0]);
        for text in ["", "a", "buy milk", &"x".repeat(32), &"y".repeat(33)] {
            let sealed = seal("home", 1, &key, b"s", 5, text);
            assert_eq!(
                open("home", &keys(1, key), b"s", &sealed),
                Ok((5, text.to_string()))
            );
        }
    }

    #[test]
    fn one_write_is_sealed_one_way_and_a_flipped_bit_does_not_open() {
        let key = [7u8; 32];
        let a = seal("home", 1, &key, b"stamp", 1, "x");
        assert_eq!(a, seal("home", 1, &key, b"stamp", 1, "x"));
        assert_ne!(a, seal("home", 1, &key, b"other", 1, "x"));
        let mut bytes = encoding::decode(&a[1..]).unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        let flipped = format!("{SEALED_PREFIX}{}", encoding::encode(&bytes));
        assert_eq!(
            open("home", &keys(1, key), b"stamp", &flipped),
            Err(OpenError::Corrupt)
        );
    }

    #[test]
    fn a_key_given_to_a_member_opens_for_that_member_alone() {
        let giver = Identity::from_seed([1; 32]);
        let member = Identity::from_seed([2; 32]);
        let other = Identity::from_seed([3; 32]);
        let key = [9u8; 32];
        let shared = giver.agree(&member.public()).unwrap();
        assert_eq!(Some(shared), member.agree(&giver.public()));
        let wrapped = wrap("home", &shared, &giver.public(), &member.public(), 2, &key);
        assert_eq!(
            unwrap(
                "home",
                &shared,
                &giver.public(),
                &member.public(),
                2,
                &wrapped
            ),
            Some(key)
        );
        let stranger = other.agree(&giver.public()).unwrap();
        assert_eq!(
            unwrap(
                "home",
                &stranger,
                &giver.public(),
                &member.public(),
                2,
                &wrapped
            ),
            None
        );
        assert_eq!(
            unwrap(
                "home",
                &shared,
                &giver.public(),
                &member.public(),
                3,
                &wrapped
            ),
            None,
            "another epoch"
        );
        assert_eq!(
            unwrap(
                "work",
                &shared,
                &giver.public(),
                &member.public(),
                2,
                &wrapped
            ),
            None,
            "another list"
        );
    }
}
