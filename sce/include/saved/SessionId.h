// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Saved::SessionId — the id of a root session, the value of the system variable
// `_sessionid` (docs/adr/0045, "Questions the owner answered" 2).
//
// `_sessionid` is bound when a session is loaded and kept until it ends, and it is an XML
// Nmtoken: another session addresses this one as `#_scxml_<id>`, and every child's id is
// its parent's and the id the child was started under, `<parent>.<invokeid>`. So a root's
// id is never empty. The host may give it when it creates the machine (refusalOf tells it
// when what it gives is not an Nmtoken, rather than having it replaced), and where it
// gives none the runtime draws one (draw), a UUID version 4 in lowercase hexadecimal with
// hyphens, the one spelling every backend draws. A restore binds the saved id instead of
// either, since the session carries on.
//
// Cross-language siblings: backends/go/runtime/session_id.go,
// backends/python/runtime/sce_runtime/session_id.py, com.sce.runtime.SessionId (Kotlin)
// and sce_rust_runtime::helpers::session_id (Rust).

#pragma once

#include <array>
#include <cstdint>
#include <cstdio>
#include <optional>
#include <random>
#include <string>
#include <string_view>

namespace SCE::Saved::SessionId {

namespace detail {

/// XML 1.0 `NameStartChar`.
inline bool isNameStartChar(std::uint32_t c) {
    return c == 0x3A || (c >= 0x41 && c <= 0x5A) || c == 0x5F || (c >= 0x61 && c <= 0x7A) || (c >= 0xC0 && c <= 0xD6) ||
           (c >= 0xD8 && c <= 0xF6) || (c >= 0xF8 && c <= 0x2FF) || (c >= 0x370 && c <= 0x37D) ||
           (c >= 0x37F && c <= 0x1FFF) || (c >= 0x200C && c <= 0x200D) || (c >= 0x2070 && c <= 0x218F) ||
           (c >= 0x2C00 && c <= 0x2FEF) || (c >= 0x3001 && c <= 0xD7FF) || (c >= 0xF900 && c <= 0xFDCF) ||
           (c >= 0xFDF0 && c <= 0xFFFD) || (c >= 0x10000 && c <= 0xEFFFF);
}

/// XML 1.0 `NameChar`.
inline bool isNameChar(std::uint32_t c) {
    return isNameStartChar(c) || c == 0x2D || c == 0x2E || (c >= 0x30 && c <= 0x39) || c == 0xB7 ||
           (c >= 0x300 && c <= 0x36F) || (c >= 0x203F && c <= 0x2040);
}

/// The code point of the UTF-8 sequence at `text[at]`, advancing `at` past it; U+FFFD for
/// a byte that starts no well-formed sequence, one byte consumed, as the other backends
/// read a malformed text.
inline std::uint32_t nextCodePoint(std::string_view text, std::size_t &at) {
    const auto byte = [&](std::size_t i) { return static_cast<std::uint32_t>(static_cast<unsigned char>(text[i])); };
    const std::uint32_t lead = byte(at);
    std::size_t length = 0;
    std::uint32_t point = 0;
    if (lead < 0x80) {
        ++at;
        return lead;
    }
    if (lead >= 0xC2 && lead <= 0xDF) {
        length = 2;
        point = lead & 0x1F;
    } else if (lead >= 0xE0 && lead <= 0xEF) {
        length = 3;
        point = lead & 0x0F;
    } else if (lead >= 0xF0 && lead <= 0xF4) {
        length = 4;
        point = lead & 0x07;
    } else {
        ++at;
        return 0xFFFD;
    }
    if (at + length > text.size()) {
        ++at;
        return 0xFFFD;
    }
    for (std::size_t i = 1; i < length; ++i) {
        const std::uint32_t continuation = byte(at + i);
        if ((continuation & 0xC0) != 0x80) {
            ++at;
            return 0xFFFD;
        }
        point = (point << 6) | (continuation & 0x3F);
    }
    const bool overlong = (length == 3 && point < 0x800) || (length == 4 && point < 0x10000);
    if (overlong || point > 0x10FFFF || (point >= 0xD800 && point <= 0xDFFF)) {
        ++at;
        return 0xFFFD;
    }
    at += length;
    return point;
}

}  // namespace detail

/// Why `sessionId` is not an id a session may have, or nullopt when it is one: an XML 1.0
/// Nmtoken, one name character or more (production `Nmtoken`, fifth edition).
inline std::optional<std::string> refusalOf(std::string_view sessionId) {
    if (sessionId.empty()) {
        return "the session id is empty, and a session id is one name character or more (an XML Nmtoken)";
    }
    std::size_t at = 0;
    while (at < sessionId.size()) {
        const std::size_t before = at;
        const std::uint32_t point = detail::nextCodePoint(sessionId, at);
        // A byte that starts no well-formed sequence reads as U+FFFD, one byte wide, and is no
        // name character however the replacement character itself is classed.
        const bool malformed = point == 0xFFFD && at - before == 1;
        if (malformed || !detail::isNameChar(point)) {
            char hex[16];
            std::snprintf(hex, sizeof hex, "%04X", static_cast<unsigned>(point));
            return std::string("the session id holds U+") + hex +
                   ", which is not a name character: a session id is an XML Nmtoken (letters, digits, '.', '-', '_', "
                   "':')";
        }
    }
    return std::nullopt;
}

/// Whether `sessionId` is an XML 1.0 Nmtoken.
inline bool isNmtoken(std::string_view sessionId) {
    return !refusalOf(sessionId).has_value();
}

/// A session id drawn from the platform's randomness, a UUID version 4 (RFC 9562, section
/// 5.4) in lowercase hexadecimal in groups of 8, 4, 4, 4 and 12, for a session whose host
/// gave none.
inline std::string draw() {
    static thread_local std::mt19937_64 generator = [] {
        std::random_device device;
        std::seed_seq seed{device(), device(), device(), device(), device(), device(), device(), device()};
        return std::mt19937_64(seed);
    }();
    std::array<std::uint8_t, 16> bytes{};
    for (std::size_t half = 0; half < 2; ++half) {
        std::uint64_t word = generator();
        for (std::size_t i = 0; i < 8; ++i) {
            bytes[half * 8 + i] = static_cast<std::uint8_t>(word >> (8 * i));
        }
    }
    bytes[6] = static_cast<std::uint8_t>((bytes[6] & 0x0F) | 0x40);
    bytes[8] = static_cast<std::uint8_t>((bytes[8] & 0x3F) | 0x80);
    static constexpr char kHex[] = "0123456789abcdef";
    std::string out;
    out.reserve(36);
    for (std::size_t i = 0; i < bytes.size(); ++i) {
        if (i == 4 || i == 6 || i == 8 || i == 10) {
            out += '-';
        }
        out += kHex[bytes[i] >> 4];
        out += kHex[bytes[i] & 0x0F];
    }
    return out;
}

}  // namespace SCE::Saved::SessionId
