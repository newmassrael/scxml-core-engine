// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The length a template's length restriction holds a text to
// (docs/SCE_TEST_PROFILE.md §6.1, §11.4): ES 201 873-1 B.1.4.1 restricts the
// length of a `universal charstring` in characters, and SCE holds a text as
// UTF-8 bytes, so the two counts differ wherever a character takes more than
// one byte.

#pragma once

#include <cstddef>
#include <cstdint>
#include <string_view>

namespace SCE::Test {

/**
 * @brief The TTCN-3 length of a text: how many characters it holds
 *
 * ES 201 873-1 B.1.4.1 counts the length of a `universal charstring` in
 * characters, each one Unicode code point. A generated template's
 * `length-min` and `length-max` on a `string[N]` position compare this, and
 * not the text's size in bytes, which is what bounds it in SCE (docs/SCE_TEST_PROFILE.md §6.3).
 *
 * Precondition: @p text is well-formed UTF-8 (RFC 3629). Every SCE string
 * value is, by construction: a codec refuses to decode one that is not, and a
 * text the document writes is read as UTF-8. Each character then begins with
 * exactly one byte that is not a continuation byte (`10xxxxxx`), so the
 * characters are counted as those bytes.
 */
[[nodiscard]] constexpr std::size_t textLength(std::string_view text) noexcept {
    std::size_t characters = 0;
    for (const char byte : text) {
        if ((static_cast<std::uint8_t>(byte) & 0xC0u) != 0x80u) {
            ++characters;
        }
    }
    return characters;
}

}  // namespace SCE::Test
