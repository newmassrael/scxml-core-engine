// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The name of a list a device creates at a relay that takes a creation from a connection with no
//! token, and the work that creation costs (ADR 0080 section 3).
//!
//! **The name certifies its genesis.** Such a list is named `n1.<salt>.<digest>`: the salt in the
//! URL alphabet (from 1 to 32 bytes, which the device chooses or draws), and the first 16 bytes of
//! SHA-256 of the parts `sce-relay/list-name/1`, the salt and the first administrator key the
//! genesis names (each in the URL alphabet), in the trust layer's framing, in the URL alphabet too
//! ([`list_name`]). A relay takes a creation of such a name only with a genesis whose first
//! administrator key the name certifies ([`certifies`]), so that no one without that key takes a
//! name another device made, before the list lapses or after; and every device that holds the log
//! can create the list again under the same name, since the genesis is in its log.
//!
//! **The work.** A creation carries a counter such that SHA-256 of the parts `sce-relay/work/1`,
//! the relay's nonce in the URL alphabet, the hash of the genesis in the URL alphabet and the
//! counter in decimal, in the trust layer's framing, begins with at least as many zero bits as the
//! relay asked ([`work_holds`]). The device finds it by trying counters from 0 ([`find_work`]),
//! which costs it about two to the power of the bits asked; a device does no more than
//! [`MOST_WORK_BITS`], whatever a relay asks.

use sha2::{Digest, Sha256};

use crate::encoding;
use crate::identity::PublicKey;
use crate::membership::{Body, Entry, Hash};
use crate::message::part;

/// The domain the digest of a list's name is made under.
pub const NAME_DOMAIN: &str = "sce-relay/list-name/1";
/// The domain the work of a creation is made under.
pub const WORK_DOMAIN: &str = "sce-relay/work/1";
/// The first part of a name that certifies its genesis, and its version.
pub const NAME_PREFIX: &str = "n1";
/// How many bytes of the digest a name carries.
pub const DIGEST_BYTES: usize = 16;
/// How many bytes of salt [`draw_salt`] draws.
pub const SALT_BYTES: usize = 16;
/// The longest salt a name carries.
pub const MOST_SALT_BYTES: usize = 32;
/// The most bits of work a device does for a creation, and a relay asks: about sixteen million
/// hashes, seconds on a phone.
pub const MOST_WORK_BITS: u32 = 24;

/// A salt drawn from the operating system's source of randomness, the only source it is drawn
/// from.
pub fn draw_salt() -> Result<[u8; SALT_BYTES], getrandom::Error> {
    let mut salt = [0u8; SALT_BYTES];
    getrandom::getrandom(&mut salt)?;
    Ok(salt)
}

/// The digest a name carries, of `salt` and the genesis's first administrator key.
fn digest(salt: &[u8], administrator: &PublicKey) -> [u8; DIGEST_BYTES] {
    let mut bytes = Vec::new();
    part(&mut bytes, NAME_DOMAIN);
    part(&mut bytes, &encoding::encode(salt));
    part(&mut bytes, &administrator.to_text());
    let hash = Sha256::digest(&bytes);
    let mut digest = [0u8; DIGEST_BYTES];
    digest.copy_from_slice(&hash[..DIGEST_BYTES]);
    digest
}

/// The name that certifies a genesis whose first administrator key is `administrator`, under
/// `salt`; `None` for a salt that is empty or longer than [`MOST_SALT_BYTES`].
pub fn list_name(salt: &[u8], administrator: &PublicKey) -> Option<String> {
    if salt.is_empty() || salt.len() > MOST_SALT_BYTES {
        return None;
    }
    Some(format!(
        "{NAME_PREFIX}.{}.{}",
        encoding::encode(salt),
        encoding::encode(&digest(salt, administrator))
    ))
}

/// Whether `list` is the name [`list_name`] makes for `administrator` under the salt it carries.
pub fn certifies(list: &str, administrator: &PublicKey) -> bool {
    let mut parts = list.split('.');
    let (Some(NAME_PREFIX), Some(salt), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return false;
    };
    encoding::decode(salt)
        .and_then(|salt| list_name(&salt, administrator))
        .is_some_and(|made| made == list)
}

/// The first administrator key `genesis` names, when it is the genesis of a log.
pub fn first_administrator(genesis: &Entry) -> Option<PublicKey> {
    match &genesis.body {
        Body::Genesis { administrators, .. } if genesis.index == 0 => {
            administrators.first().copied()
        }
        _ => None,
    }
}

/// The hash the work of `counter` comes to, against the relay's `nonce` and the hash of the
/// genesis `genesis`.
fn work_hash(nonce: &str, genesis: &str, counter: u64) -> [u8; 32] {
    let mut bytes = Vec::new();
    part(&mut bytes, WORK_DOMAIN);
    part(&mut bytes, nonce);
    part(&mut bytes, genesis);
    part(&mut bytes, &counter.to_string());
    Sha256::digest(&bytes).into()
}

/// How many zero bits `bytes` begins with.
fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut bits = 0;
    for byte in bytes {
        if *byte == 0 {
            bits += 8;
        } else {
            return bits + byte.leading_zeros();
        }
    }
    bits
}

/// Whether `counter` is work of at least `bits` against the relay's `nonce` and the hash of the
/// genesis `genesis`.
pub fn work_holds(nonce: &[u8], genesis: &Hash, counter: u64, bits: u32) -> bool {
    let hash = work_hash(
        &encoding::encode(nonce),
        &encoding::encode(genesis),
        counter,
    );
    leading_zero_bits(&hash) >= bits
}

/// The first counter, from 0, that is work of at least `bits` against `nonce` and `genesis`;
/// `None` when `bits` is past [`MOST_WORK_BITS`], which a device does not do.
pub fn find_work(nonce: &[u8], genesis: &Hash, bits: u32) -> Option<u64> {
    if bits > MOST_WORK_BITS {
        return None;
    }
    let (nonce, genesis) = (encoding::encode(nonce), encoding::encode(genesis));
    (0..=u64::MAX).find(|counter| leading_zero_bits(&work_hash(&nonce, &genesis, *counter)) >= bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::membership::{Log, Profile};
    use crate::Identity;

    fn key(seed: u8) -> PublicKey {
        Identity::from_seed([seed; 32]).public()
    }

    /// The parts of a name's digest and of the work, in the trust layer's framing: a relay in
    /// another language makes these bytes.
    #[test]
    fn the_bytes_are_the_parts_in_the_trust_layers_framing() {
        let framed = |parts: &[&str]| {
            let mut bytes = Vec::new();
            for part in parts {
                bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
                bytes.extend_from_slice(part.as_bytes());
            }
            bytes
        };
        let administrator = key(9);
        let expected = Sha256::digest(framed(&[
            "sce-relay/list-name/1",
            "AQID",
            &administrator.to_text(),
        ]));
        assert_eq!(
            list_name(&[1, 2, 3], &administrator),
            Some(format!(
                "n1.AQID.{}",
                encoding::encode(&expected[..DIGEST_BYTES])
            ))
        );
        let expected: [u8; 32] = Sha256::digest(framed(&[
            "sce-relay/work/1",
            "BQUF",
            &encoding::encode(&[7; 32]),
            "42",
        ]))
        .into();
        assert_eq!(work_hash("BQUF", &encoding::encode(&[7; 32]), 42), expected);
    }

    #[test]
    fn a_name_certifies_the_key_it_was_made_for_and_no_other() {
        let name = list_name(b"household", &key(9)).unwrap();
        assert!(name.starts_with("n1."), "{name}");
        assert!(certifies(&name, &key(9)));
        assert!(!certifies(&name, &key(8)), "another key");
        let other_salt = list_name(b"householc", &key(9)).unwrap();
        assert_ne!(name, other_salt);
        for wrong in [
            "home",
            "n1",
            "n1.aG91c2Vob2xk",
            &format!("n2{}", &name[2..]),
            &format!("{name}.x"),
            &name[..name.len() - 1],
        ] {
            assert!(!certifies(wrong, &key(9)), "{wrong}");
        }
        assert_eq!(list_name(&[], &key(9)), None, "no salt");
        assert_eq!(list_name(&[0; 33], &key(9)), None, "a salt too long");
        assert!(list_name(&[0; 32], &key(9)).is_some());
        assert_eq!(draw_salt().unwrap().len(), SALT_BYTES);
    }

    #[test]
    fn the_first_administrator_is_read_from_a_genesis_alone() {
        let admin = Identity::from_seed([9; 32]);
        let genesis = Log::genesis_draft(
            "home",
            Body::Genesis {
                administrators: vec![admin.public(), key(8)],
                quorum: 1,
                profile: Profile::Untrusted,
            },
        )
        .sign(&admin)
        .finish();
        assert_eq!(first_administrator(&genesis), Some(admin.public()));
        let moved = Entry {
            index: 1,
            ..genesis
        };
        assert_eq!(first_administrator(&moved), None, "not at the log's start");
    }

    #[test]
    fn the_work_found_is_the_first_counter_that_holds_at_its_bits() {
        let (nonce, genesis) = ([5u8; 32], [7u8; 32]);
        let counter = find_work(&nonce, &genesis, 12).unwrap();
        assert!(work_holds(&nonce, &genesis, counter, 12));
        assert!(work_holds(&nonce, &genesis, counter, 0));
        // The first counter that holds is the one found: every one before it falls short.
        assert!((0..counter).all(|before| !work_holds(&nonce, &genesis, before, 12)));
        assert_eq!(find_work(&nonce, &genesis, 0), Some(0));
        assert_eq!(find_work(&nonce, &genesis, MOST_WORK_BITS + 1), None);
        assert_eq!(leading_zero_bits(&[0, 0b0001_0000, 0xff]), 11);
        assert_eq!(leading_zero_bits(&[0, 0]), 16);
    }
}
