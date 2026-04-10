// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package lookup provides exact-match lookup tables. See SCE_FORGE.md
// Section 4.9.
//
// A lookup is a finite discrete mapping — unlike interpolation there is no
// continuity assumption. Lookup[K, V] returns (value, true) iff needle equals
// one of the keys exactly; otherwise (zero, false).
package lookup

// Lookup performs exact-match lookup over parallel key/value slices. It
// returns (values[i], true) at the first index where keys[i] == needle, and
// (zero V, false) otherwise. Keys must be unique (validated at parse time by
// the Forge compiler).
func Lookup[K comparable, V any](keys []K, values []V, needle K) (V, bool) {
	for i, k := range keys {
		if k == needle {
			return values[i], true
		}
	}
	var zero V
	return zero, false
}
