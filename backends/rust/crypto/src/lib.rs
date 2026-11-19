// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The cryptography a host of SCE holds for its machines (ADR 0109, D5 to D7; D9 builds on it).
//!
//! A statechart holds no key and compares no secret: it names a key by its id and asks its host
//! (docs/SCE_HOST_FEATURES.md, sections 10 to 13). What the host does with it is here, once:
//!
//! - [`equal`]: the one comparison of a secret or a digest, in time that does not depend on where
//!   two byte strings differ;
//! - [`credential`]: the bearer tokens a host issues, kept as their digests, and the check of a
//!   server that reads them (ADR 0109, D3);
//! - [`mac`]: HMAC-SHA-256 (RFC 2104) and its verification through that comparison;
//! - [`random`]: where random bytes come from, the operating system's source unless a test gives
//!   another;
//! - [`cose`]: COSE objects (RFC 9052) signed with Ed25519 and sealed with ChaCha20/Poly1305, keys
//!   derived with HKDF-SHA-256, and a public key exported as a `COSE_Key`;
//! - [`wrap`]: a key's material wrapped under a key-encryption key the key store never holds;
//! - [`keyring`]: the life of a key a host generates or derives, over a key store
//!   (`sce_rust_session_store::KeyStore`);
//! - [`sealed`]: every store a server keeps sealed at rest, each key it finds a row by kept as a
//!   keyed digest (ADR 0109, D8).
//!
//! Key material is held in [`zeroize::Zeroizing`] wherever this crate holds it, and no type here
//! prints a key through `Debug`.

#![forbid(unsafe_code)]

pub mod cose;
pub mod credential;
pub mod equal;
pub mod keyring;
pub mod mac;
pub mod random;
pub mod sealed;
pub mod wrap;

pub use zeroize::Zeroizing;

/// The lowercase hexadecimal spelling of `bytes`, two digits a byte.
pub fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push(char::from(DIGITS[usize::from(byte >> 4)]));
        text.push(char::from(DIGITS[usize::from(byte & 0x0f)]));
    }
    text
}

/// The bytes `text` spells in hexadecimal, two digits a byte, either case; `None` for a text that
/// is not that.
pub fn bytes_of_hex(text: &str) -> Option<Vec<u8>> {
    let digits = text.as_bytes();
    if !digits.len().is_multiple_of(2) {
        return None;
    }
    let value = |digit: u8| char::from(digit).to_digit(16);
    digits
        .chunks(2)
        .map(|pair| Some((value(pair[0])? * 16 + value(pair[1])?) as u8))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_spells_each_byte_in_two_lowercase_digits_and_reads_back() {
        assert_eq!(hex(&[0x00, 0x0f, 0xa0, 0xff]), "000fa0ff");
        assert_eq!(bytes_of_hex("000FA0ff"), Some(vec![0x00, 0x0f, 0xa0, 0xff]));
        assert_eq!(bytes_of_hex("0"), None, "an odd count of digits");
        assert_eq!(bytes_of_hex("0g"), None, "a character that is no digit");
    }
}
