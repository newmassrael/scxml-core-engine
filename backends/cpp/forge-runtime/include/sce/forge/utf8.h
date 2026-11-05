// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — utf8
// Whether bytes are well-formed UTF-8 (RFC 3629): what a codec's text holds,
// which a decode refuses otherwise, and what a text held to its bound
// (`Text<MAX>`, sce/forge/bounded.h) is made of.

#pragma once

#include <cstddef>
#include <cstdint>

namespace SCE::Forge {

/// Whether the `n` bytes at `p` are well-formed UTF-8, each read as the byte
/// it holds: the bytes of a byte string (`std::uint8_t`) or of a text
/// (`char`) alike, so a text is checked as it is held, with no cast of its
/// pointer.
template <typename Byte> [[nodiscard]] constexpr bool is_well_formed_utf8(const Byte *p, std::size_t n) noexcept {
    static_assert(sizeof(Byte) == 1, "UTF-8 is read a byte at a time");
    const auto at = [p](std::size_t index) { return static_cast<std::uint8_t>(p[index]); };
    std::size_t i = 0;
    while (i < n) {
        const std::uint8_t b0 = at(i);
        if (b0 <= 0x7F) {
            i += 1;
        } else if (b0 >= 0xC2 && b0 <= 0xDF) {
            if (i + 1 >= n) {
                return false;
            }
            const std::uint8_t b1 = at(i + 1);
            if (b1 < 0x80 || b1 > 0xBF) {
                return false;
            }
            i += 2;
        } else if (b0 >= 0xE0 && b0 <= 0xEF) {
            if (i + 2 >= n) {
                return false;
            }
            const std::uint8_t b1 = at(i + 1);
            const std::uint8_t b2 = at(i + 2);
            // Per RFC 3629 §4: E0 disallows overlong (b1 < 0xA0); ED
            // disallows surrogate (b1 > 0x9F = 0xA0..0xBF excluded).
            const std::uint8_t b1_min = (b0 == 0xE0) ? 0xA0 : 0x80;
            const std::uint8_t b1_max = (b0 == 0xED) ? 0x9F : 0xBF;
            if (b1 < b1_min || b1 > b1_max) {
                return false;
            }
            if (b2 < 0x80 || b2 > 0xBF) {
                return false;
            }
            i += 3;
        } else if (b0 >= 0xF0 && b0 <= 0xF4) {
            if (i + 3 >= n) {
                return false;
            }
            const std::uint8_t b1 = at(i + 1);
            const std::uint8_t b2 = at(i + 2);
            const std::uint8_t b3 = at(i + 3);
            // F0 disallows overlong (b1 < 0x90); F4 caps at U+10FFFF
            // (b1 > 0x8F = 0x90..0xBF excluded).
            const std::uint8_t b1_min = (b0 == 0xF0) ? 0x90 : 0x80;
            const std::uint8_t b1_max = (b0 == 0xF4) ? 0x8F : 0xBF;
            if (b1 < b1_min || b1 > b1_max) {
                return false;
            }
            if (b2 < 0x80 || b2 > 0xBF) {
                return false;
            }
            if (b3 < 0x80 || b3 > 0xBF) {
                return false;
            }
            i += 4;
        } else {
            // Lead bytes 0x80..0xC1 (continuation in lead position +
            // overlong 1-byte) and 0xF5..0xFF (above U+10FFFF) reject.
            return false;
        }
    }
    return true;
}

/// Whether the `n` bytes at `p` are well-formed UTF-8.
[[nodiscard]] inline bool is_valid_utf8(const std::uint8_t *p, std::size_t n) noexcept {
    return is_well_formed_utf8(p, n);
}

}  // namespace SCE::Forge
