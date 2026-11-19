// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! A keyed digest: HMAC-SHA-256 (RFC 2104, FIPS 198-1), the whole 32-byte tag
//! (docs/SCE_HOST_FEATURES.md section 12; ADR 0109, D6).

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::equal::same_secret;

/// The bytes of a tag: the whole output of SHA-256.
pub const TAG_BYTES: usize = 32;

/// The HMAC-SHA-256 of `message` under `key`, a key of any length.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; TAG_BYTES] {
    let mut mac =
        <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC takes a key of any length");
    mac.update(message);
    let digest = mac.finalize().into_bytes();
    let mut tag = [0u8; TAG_BYTES];
    tag.copy_from_slice(&digest);
    tag
}

/// Whether `tag` is the HMAC-SHA-256 of `message` under `key`, compared in constant time
/// ([`same_secret`]). A tag that is not 32 bytes long is no tag of this digest.
pub fn verify_hmac_sha256(key: &[u8], message: &[u8], tag: &[u8]) -> bool {
    same_secret(&hmac_sha256(key, message), tag)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytes_of_hex;

    fn hex(text: &str) -> Vec<u8> {
        bytes_of_hex(text).expect("hexadecimal")
    }

    /// RFC 4231 section 4: test cases 1 to 4, 6 and 7, each a key, a message and the
    /// HMAC-SHA-256 in hexadecimal. Case 5 truncates its tag to 128 bits, which this digest never
    /// does.
    fn rfc_4231() -> Vec<(Vec<u8>, Vec<u8>, &'static str)> {
        let large_key = vec![0xaa; 131];
        vec![
            (
                vec![0x0b; 20],
                b"Hi There".to_vec(),
                "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            ),
            (
                b"Jefe".to_vec(),
                b"what do ya want for nothing?".to_vec(),
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            ),
            (
                vec![0xaa; 20],
                vec![0xdd; 50],
                "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
            ),
            (
                (1..=25).collect(),
                vec![0xcd; 50],
                "82558a389a443c0ea4cc819899f2083a85f0faa3e578f8077a2e3ff46729665b",
            ),
            (
                large_key.clone(),
                b"Test Using Larger Than Block-Size Key - Hash Key First".to_vec(),
                "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
            ),
            (
                large_key,
                b"This is a test using a larger than block-size key and a larger than \
                  block-size data. The key needs to be hashed before being used by the HMAC \
                  algorithm."
                    .to_vec(),
                "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
            ),
        ]
    }

    #[test]
    fn the_tag_is_the_hmac_sha_256_rfc_4231_gives() {
        let vectors = rfc_4231();
        assert_eq!(vectors.len(), 6, "cases 1 to 4, 6 and 7");
        for (n, (key, message, tag)) in vectors.iter().enumerate() {
            assert_eq!(
                hmac_sha256(key, message).to_vec(),
                hex(tag),
                "RFC 4231 vector {n}"
            );
            assert!(verify_hmac_sha256(key, message, &hex(tag)));
        }
    }

    #[test]
    fn a_tag_of_another_key_message_or_length_does_not_verify() {
        let (key, message, tag) = rfc_4231().swap_remove(1);
        let tag = hex(tag);
        let mut flipped = tag.clone();
        flipped[31] ^= 0x80;
        assert!(!verify_hmac_sha256(&key, &message, &flipped));
        assert!(!verify_hmac_sha256(&key, b"another message", &tag));
        assert!(!verify_hmac_sha256(b"another key", &message, &tag));
        assert!(
            !verify_hmac_sha256(&key, &message, &tag[..16]),
            "a truncated tag is no tag of this digest"
        );
    }
}
