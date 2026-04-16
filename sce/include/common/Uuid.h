// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Uuid — RFC 9562 UUID v7 / v4 helpers.
//
// v7 (preferred): 48-bit Unix millisecond timestamp prefix + 74 random bits
// + version/variant. Time-ordered for log/correlation use; sufficient
// uniqueness for distributed peers (MeshEnvelope correlation_id, mesh peer
// identity). See SCE_MESH.md Section 13 Phase 3.5.
//
// v4 (fallback): 122 fully random bits + version/variant. Use when time
// source is unavailable or monotonic ordering is not required.
//
// Returns raw 16-byte arrays (big-endian wire form per RFC 9562 §4). Byte
// form is preferred on hot paths (envelope codec, correlation tables) to
// avoid allocations; `to_string` / `from_string` bridge to the RFC 4122
// canonical 36-character textual form when the value crosses the engine
// metadata boundary (`_event.invokeid`, logs, deploy manifests).

#pragma once

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>

namespace SCE::uuid {

using Bytes = std::array<uint8_t, 16>;

/// Generate a UUID v7 (RFC 9562 §5.7). Big-endian wire bytes.
/// Thread-safe; uses a thread-local PRNG seeded from std::random_device.
[[nodiscard]] Bytes v7();

/// Generate a UUID v4 (RFC 9562 §5.4). Big-endian wire bytes.
/// Thread-safe; uses the same thread-local PRNG as v7().
[[nodiscard]] Bytes v4();

namespace detail {

/// RFC 4122 §3 canonical text form is exactly 36 characters.
inline constexpr std::size_t kCanonicalTextLen = 36;

/// RFC 4122 §3 dash positions — the only single source of truth for where
/// the `-` separators fall in `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
/// Every consumer (to_string, from_string) goes through this array; a
/// future spec amendment (or regression) changes one place.
inline constexpr std::array<std::size_t, 4> kCanonicalDashPositions = {8, 13, 18, 23};

[[nodiscard]] inline constexpr bool is_dash_position(std::size_t i) {
    for (auto p : kCanonicalDashPositions) {
        if (i == p) return true;
    }
    return false;
}

[[nodiscard]] inline std::optional<std::uint8_t> parse_hex_nibble(char c) {
    if (c >= '0' && c <= '9') return static_cast<std::uint8_t>(c - '0');
    if (c >= 'a' && c <= 'f') return static_cast<std::uint8_t>(c - 'a' + 10);
    if (c >= 'A' && c <= 'F') return static_cast<std::uint8_t>(c - 'A' + 10);
    return std::nullopt;
}

}  // namespace detail

/// RFC 4122 §3 canonical textual form: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`
/// (36 chars, lowercase hex, dashes at positions 8/13/18/23). Inverse of
/// `from_string` on valid input. Dash positions and length are sourced from
/// `detail::kCanonicalDashPositions` / `detail::kCanonicalTextLen` so the
/// spec constants have one authoritative definition.
[[nodiscard]] inline std::string to_string(const Bytes &uuid) {
    constexpr char kHex[] = "0123456789abcdef";
    std::string out(detail::kCanonicalTextLen, '-');
    std::size_t byte = 0;
    for (std::size_t i = 0; i < detail::kCanonicalTextLen; ++i) {
        if (detail::is_dash_position(i)) continue;
        const std::uint8_t b = uuid[byte / 2];
        const bool hi_nibble = (byte % 2 == 0);
        out[i] = kHex[hi_nibble ? (b >> 4) : (b & 0x0F)];
        ++byte;
    }
    return out;
}

/// Parse an RFC 4122 §3 canonical 36-character UUID string. Accepts both
/// upper- and lower-case hex. Returns `std::nullopt` on any length,
/// separator, or hex-digit mismatch. No allocations on the failure path.
[[nodiscard]] inline std::optional<Bytes> from_string(std::string_view text) {
    if (text.size() != detail::kCanonicalTextLen) return std::nullopt;
    Bytes out{};
    std::size_t byte = 0;
    for (std::size_t i = 0; i < detail::kCanonicalTextLen; ) {
        if (detail::is_dash_position(i)) {
            if (text[i] != '-') return std::nullopt;
            ++i;
            continue;
        }
        auto hi = detail::parse_hex_nibble(text[i]);
        auto lo = detail::parse_hex_nibble(text[i + 1]);
        if (!hi || !lo) return std::nullopt;
        out[byte] = static_cast<std::uint8_t>((*hi << 4) | *lo);
        ++byte;
        i += 2;
    }
    return out;
}

}  // namespace SCE::uuid
