// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

//! The one comparison of a secret (ADR 0109, D3 and D6).
//!
//! An ordinary comparison stops at the first byte that differs, so the time it takes tells an
//! attacker how much of a guessed tag or token was right. Every comparison of a tag, a digest, a
//! token or any other secret in SCE's Rust code calls [`same_secret`], so that there is one place
//! that keeps this, and it keeps it with `subtle`'s `ConstantTimeEq`, written so the compiler does
//! not shorten it.

use subtle::ConstantTimeEq;

/// Whether `a` and `b` are the same bytes, in time that depends on their lengths alone. Two byte
/// strings of different lengths are not the same; a length is not a secret.
pub fn same_secret(a: &[u8], b: &[u8]) -> bool {
    bool::from(a.ct_eq(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_bytes_are_the_same_and_any_difference_is_not() {
        let tag = [0x5au8; 32];
        assert!(same_secret(&tag, &tag.clone()));
        for at in [0, 15, 31] {
            let mut other = tag;
            other[at] ^= 0x01;
            assert!(!same_secret(&tag, &other), "a difference at byte {at}");
        }
        assert!(!same_secret(&tag, &tag[..31]), "a shorter string");
        assert!(same_secret(&[], &[]), "two empty strings");
    }
}
