// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

// sce_forge_runtime — lookup
// Exact-match lookup tables. See SCE_FORGE.md §4.9.

#pragma once

#include <cstddef>
#include <optional>

namespace sce::forge {

/// Exact-match lookup over parallel key/value arrays. Returns the value at
/// the first index where `keys[i] == needle`, otherwise `std::nullopt`.
/// Keys must be unique (validated at parse time by the Forge compiler).
template <typename K, typename V, std::size_t N>
constexpr std::optional<V> lookup(const K (&keys)[N],
                                  const V (&values)[N],
                                  K needle) noexcept {
    for (std::size_t i = 0; i < N; ++i) {
        if (keys[i] == needle) return values[i];
    }
    return std::nullopt;
}

} // namespace sce::forge
