// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Latin1Bytes — how a byte string rides in a text.
//
// JSON has no byte string, so a `bytes` value crosses a wire, a `<param>` and a saved
// state as its byte-exact Latin-1 text: each byte is the character of that code
// point, so every one of the 256 values is one character and back (docs/adr/0005,
// decision 2; docs/SCE_ACCEPTED_SUBSET.md, "`bytes` on the wire"). The text itself is
// UTF-8 like every text here, which makes a byte above 0x7F the two bytes of its
// character: 0xFF is `C3 BF`. Printable ASCII, which is what a bytes guard compares,
// is the same bytes either way.
//
// Header-only and dependency-free because its callers are header-only helpers that a
// generated machine compiles against `sce/include` alone, with no runtime library.

#pragma once

#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::Latin1Bytes {

/// The text `bytes` spells: each byte the UTF-8 of the character of its code point.
inline std::string textOf(const std::vector<uint8_t> &bytes) {
    std::string text;
    text.reserve(bytes.size());
    for (const uint8_t byte : bytes) {
        if (byte < 0x80u) {
            text += static_cast<char>(byte);
        } else {
            text += static_cast<char>(0xC0u | (byte >> 6));
            text += static_cast<char>(0x80u | (byte & 0x3Fu));
        }
    }
    return text;
}

/// The bytes `text` spells, written to `out`, which is replaced. An empty string when
/// `text` is a byte string, else why it is not — a clause the caller names the field
/// before, as `'frame' carries a character above U+00FF, which no single byte spells`.
/// A character past U+00FF is no byte and is refused, never cut or wrapped.
inline std::string bytesOf(std::string_view text, std::vector<uint8_t> &out) {
    std::vector<uint8_t> bytes;
    bytes.reserve(text.size());
    for (std::size_t i = 0; i < text.size();) {
        const auto lead = static_cast<unsigned char>(text[i]);
        if (lead < 0x80u) {
            bytes.push_back(lead);
            i += 1;
        } else if ((lead == 0xC2u || lead == 0xC3u) && i + 1 < text.size() &&
                   (static_cast<unsigned char>(text[i + 1]) & 0xC0u) == 0x80u) {
            bytes.push_back(
                static_cast<uint8_t>(((lead & 0x03u) << 6) | (static_cast<unsigned char>(text[i + 1]) & 0x3Fu)));
            i += 2;
        } else if (lead >= 0xC4u && lead <= 0xF4u) {
            // The lead of a character past U+00FF, which no single byte spells.
            return "carries a character above U+00FF, which no single byte spells";
        } else {
            return "is not valid UTF-8 text";
        }
    }
    out = std::move(bytes);
    return {};
}

}  // namespace SCE::Latin1Bytes
