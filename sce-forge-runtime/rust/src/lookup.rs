// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

//! Exact-match lookup tables. See SCE_FORGE.md Section 4.9.
//!
//! A lookup is a finite discrete mapping — unlike interpolation there is no
//! continuity assumption. `lookup(needle)` returns `Some(value)` iff `needle`
//! equals one of the keys exactly; otherwise `None`. The caller is responsible
//! for handling the miss case.

/// Exact-match lookup over parallel key/value arrays. Returns `Some(value)`
/// when `needle == keys[i]` for some `i`, otherwise `None`. Keys must be
/// unique (duplicates cause the earliest match to win, but the generated
/// code validates uniqueness at parse time).
pub fn lookup<K, V, const N: usize>(keys: &[K; N], values: &[V; N], needle: K) -> Option<V>
where
    K: PartialEq + Copy,
    V: Copy,
{
    let mut i = 0;
    while i < N {
        if keys[i] == needle {
            return Some(values[i]);
        }
        i += 1;
    }
    None
}
