// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The table read of the `lookup` kind (SCE_FORGE.md): exact match over parallel key and
// value lists. Siblings: `com.sce.forge.runtime.lookup` (Kotlin), `sce_forge_runtime::lookup`
// (Rust) and `scealgorithm.Lookup` (Go).

/// The reads of a lookup table.
public enum Lookup {
    /// `values[i]` at the first index where `keys[i] == needle`, or `nil` on a miss.
    ///
    /// The keys are unique (the build refuses a table that repeats one), so the first
    /// match is the only one. A miss answers `nil` and no value of its own: what a
    /// miss means, an error or a default, is the table's declaration and not this
    /// function's.
    public static func find<K: Equatable, V>(_ keys: [K], _ values: [V], _ needle: K) -> V? {
        for (index, key) in keys.enumerated() where key == needle {
            return values[index]
        }
        return nil
    }
}
