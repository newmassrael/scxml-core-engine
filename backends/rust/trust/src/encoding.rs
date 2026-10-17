// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! Base64 with the URL alphabet and no padding: how a key and a signature are written into
//! text (an event's data, a certificate), 43 bytes for a key and 86 for a signature.

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity((bytes.len() * 4).div_ceil(3));
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..=chunk.len() {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * i)) & 63) as usize]));
        }
    }
    out
}

/// The bytes `text` writes, or `None` when it is not what [`encode`] writes (a character
/// outside the alphabet, a length no byte count gives, or bits past the last byte set).
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.as_bytes().chunks(4) {
        if chunk.len() == 1 {
            return None;
        }
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let sextet = ALPHABET.iter().position(|a| a == c)? as u32;
            n |= sextet << (18 - 6 * i);
        }
        let bytes = n.to_be_bytes();
        let count = chunk.len() - 1;
        if bytes[1 + count..4].iter().any(|b| *b != 0) {
            return None;
        }
        out.extend_from_slice(&bytes[1..=count]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn what_is_written_reads_back_at_every_length() {
        for len in 0..=70usize {
            let bytes: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
            let text = encode(&bytes);
            assert_eq!(decode(&text).as_deref(), Some(bytes.as_slice()), "{len}");
        }
        assert_eq!(encode(&[0u8; 64]).len(), 86, "a signature is 86 characters");
        assert_eq!(encode(&[0u8; 32]).len(), 43, "a key is 43 characters");
    }

    #[test]
    fn what_is_not_written_by_encode_is_refused() {
        assert_eq!(decode("ab+/"), None, "the other alphabet");
        assert_eq!(decode("a"), None, "a length no byte count gives");
        assert_eq!(decode("AB"), None, "bits past the last byte");
        assert_eq!(decode("AA=="), None, "padding");
    }
}
