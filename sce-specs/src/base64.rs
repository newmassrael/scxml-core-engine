// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Standard base64 (RFC 4648 section 4), for the bytes of an original that travel as JSON.
//!
//! The command layer is JSON in and out, and a member of an original is bytes. The two small functions
//! here are all that is needed, so they are written and held by tests and no dependency is added for
//! them. `decode` is strict: the alphabet, the padding and the length are exactly the standard's, and
//! white space, a missing `=` and characters after the padding are refused, because a caller that sent
//! bytes with a damaged encoding has not sent the bytes it meant, and decoding what is readable of them
//! would make a different document under the same name.
//!
//! It is public because the application that keeps this crate's folder hands the same bytes on: a
//! picture of an original goes to an AI client as base64 too (ADR 0036), and one encoding of the
//! application's is one encoding to hold.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as base64 with padding.
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn value_of(byte: u8) -> Option<u32> {
    match byte {
        b'A'..=b'Z' => Some(u32::from(byte - b'A')),
        b'a'..=b'z' => Some(u32::from(byte - b'a') + 26),
        b'0'..=b'9' => Some(u32::from(byte - b'0') + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

/// The bytes `text` says, or what is wrong with it.
pub fn decode(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("its length is not a multiple of four".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    let groups = bytes.len() / 4;
    for (index, group) in bytes.chunks(4).enumerate() {
        let last = index + 1 == groups;
        let padding = group.iter().rev().take_while(|b| **b == b'=').count();
        if padding > 2 || (padding > 0 && !last) {
            return Err("padding is only at the end, and at most two characters".to_string());
        }
        let mut n = 0u32;
        for (place, byte) in group.iter().enumerate() {
            let value = if place >= 4 - padding {
                0
            } else {
                value_of(*byte).ok_or_else(|| {
                    format!(
                        "a character that is not in the base64 alphabet (at {})",
                        index * 4 + place
                    )
                })?
            };
            n = (n << 6) | value;
        }
        // The bits a padded group does not use are zero, as the standard writes them.
        if padding == 2 && n & 0xffff != 0 || padding == 1 && n & 0xff != 0 {
            return Err("the unused bits of the last group are not zero".to_string());
        }
        out.push((n >> 16) as u8);
        if padding < 2 {
            out.push((n >> 8) as u8);
        }
        if padding < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_standards_own_examples_encode_and_decode() {
        for (plain, coded) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(encode(plain.as_bytes()), coded);
            assert_eq!(decode(coded).unwrap(), plain.as_bytes());
        }
    }

    #[test]
    fn every_byte_value_survives_the_round_trip() {
        let all: Vec<u8> = (0..=255).collect();
        assert_eq!(decode(&encode(&all)).unwrap(), all);
    }

    #[test]
    fn a_damaged_encoding_is_refused_and_not_read_as_far_as_it_goes() {
        for bad in [
            "Zg=",       // short
            "Zg",        // no padding
            "Z g==",     // white space
            "Zg==\n",    // a trailing line break
            "Zm9v=Zm9v", // padding in the middle
            "Z===",      // three pads
            "Zm9*",      // outside the alphabet
            "Zh==",      // the unused bits are not zero
            "Zm9=",      // the unused bits are not zero
        ] {
            assert!(decode(bad).is_err(), "{bad:?} was accepted");
        }
    }
}
