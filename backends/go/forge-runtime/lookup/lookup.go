// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// Package lookup provides exact-match lookup tables. See SCE_FORGE.md
// Section 4.9.
//
// A lookup is a finite discrete mapping — unlike interpolation there is no
// continuity assumption. Lookup[K, V] returns (value, true) iff needle equals
// one of the keys exactly; otherwise (zero, false). Find answers the same
// reading as the forge runtime's optional, which is what a lookup whose miss
// is an error answers (docs/adr/0055 section 5): a present value on a hit and
// the absent value on a miss.
package lookup

import "github.com/newmassrael/sce-forge-runtime/optional"

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

// Find is Lookup answered as an optional: the value paired with needle,
// present, or the absent value where no key equals it.
func Find[K comparable, V any](keys []K, values []V, needle K) optional.Of[V] {
	if v, ok := Lookup(keys, values, needle); ok {
		return optional.Some(v)
	}
	return optional.None[V]()
}
