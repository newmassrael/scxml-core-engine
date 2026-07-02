// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

package com.sce.forge.runtime

/**
 * Exact-match lookup tables. See SCE_FORGE.md Section 4.9.
 *
 * A lookup is a finite discrete mapping — unlike interpolation there is no
 * continuity assumption. [lookup] returns the matching value iff `needle`
 * equals one of the keys exactly; otherwise `null`.
 */

/**
 * Exact-match lookup over parallel key/value lists. Returns `values[i]` at
 * the first index where `keys[i] == needle`, or `null` on miss. Keys must be
 * unique (validated at parse time by the Forge compiler).
 */
public fun <K, V> lookup(keys: List<K>, values: List<V>, needle: K): V? {
    for (i in keys.indices) {
        if (keys[i] == needle) return values[i]
    }
    return null
}
