// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — the CBOR (RFC 8949) items a `sce:encoding="cbor"`
// codec reads and writes (SCE_FORGE.md §4.6.1): one definite-length map
// whose keys are small unsigned integers and whose values are unsigned
// integers, booleans, text strings and byte strings.
//
// Mirrors `backends/rust/forge-runtime/src/cbor.rs`, rule for rule. A
// generated codec calls these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 §4.2.1): every head in its shortest
// form, so two backends given the same value write the same bytes. Reading
// takes a head in any valid length — what it reads is the value — and
// refuses what the codec cannot read as its entry: a reserved
// additional-information value, an indefinite length, another major type.
//
// Decode refusals are `std::nullopt` / `false`, the C++ decode convention
// (sce/forge/codec.h); the reason is not carried. A reader that refuses may
// have advanced the cursor, so a generated decode works on a copy of the
// caller's cursor and assigns it back only once the whole map is read.

#pragma once

#include "sce/forge/codec.h"

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Forge::Cbor {

/// Major type 0, an unsigned integer.
inline constexpr std::uint8_t kMajorUnsigned = 0;
/// Major type 2, a byte string.
inline constexpr std::uint8_t kMajorBytes = 2;
/// Major type 3, a UTF-8 text string.
inline constexpr std::uint8_t kMajorText = 3;
/// Major type 5, a map.
inline constexpr std::uint8_t kMajorMap = 5;
/// Major type 7, simple values (`false` = 20, `true` = 21).
inline constexpr std::uint8_t kMajorSimple = 7;

/// The deepest an unknown entry's value may nest before a skip refuses it
/// (SCE_FORGE.md §4.6.1).
inline constexpr std::uint32_t kMaxSkipDepth = 16;

// ── Writing ─────────────────────────────────────────────────────────────

/// Write a head of `major` carrying `value`, in its shortest form.
[[nodiscard]] inline std::optional<CodecError> write_head(SceSink &w, std::uint8_t major,
                                                          std::uint64_t value) noexcept {
    const auto m = static_cast<std::uint8_t>(major << 5);
    std::uint8_t head[9];
    std::size_t n = 0;
    auto be = [&](std::size_t width) {
        for (std::size_t i = width; i-- > 0;) {
            head[n++] = static_cast<std::uint8_t>(value >> (8 * i));
        }
    };
    if (value < 24) {
        head[n++] = static_cast<std::uint8_t>(m | value);
    } else if (value <= 0xFFU) {
        head[n++] = static_cast<std::uint8_t>(m | 24);
        be(1);
    } else if (value <= 0xFFFFU) {
        head[n++] = static_cast<std::uint8_t>(m | 25);
        be(2);
    } else if (value <= 0xFFFFFFFFU) {
        head[n++] = static_cast<std::uint8_t>(m | 26);
        be(4);
    } else {
        head[n++] = static_cast<std::uint8_t>(m | 27);
        be(8);
    }
    return w.write_bytes(head, n);
}

/// Write an unsigned integer.
[[nodiscard]] inline std::optional<CodecError> write_uint(SceSink &w, std::uint64_t value) noexcept {
    return write_head(w, kMajorUnsigned, value);
}

/// Write `false` or `true`.
[[nodiscard]] inline std::optional<CodecError> write_bool(SceSink &w, bool value) noexcept {
    return w.write_u8(static_cast<std::uint8_t>((kMajorSimple << 5) | (value ? 21 : 20)));
}

/// Write a text string.
[[nodiscard]] inline std::optional<CodecError> write_text(SceSink &w, const std::string &value) noexcept {
    if (auto e = write_head(w, kMajorText, value.size())) {
        return e;
    }
    return w.write_bytes(reinterpret_cast<const std::uint8_t *>(value.data()), value.size());
}

/// Write a byte string.
[[nodiscard]] inline std::optional<CodecError> write_bytes(SceSink &w,
                                                           const std::vector<std::uint8_t> &value) noexcept {
    if (auto e = write_head(w, kMajorBytes, value.size())) {
        return e;
    }
    return w.write_bytes(value.data(), value.size());
}

/// Write the head of a definite-length map of `entries` entries.
[[nodiscard]] inline std::optional<CodecError> write_map_head(SceSink &w, std::uint64_t entries) noexcept {
    return write_head(w, kMajorMap, entries);
}

// ── Reading ─────────────────────────────────────────────────────────────

/// One head: its major type and the value its additional information
/// carries (for a string or a map, the length).
struct Head {
    std::uint8_t major;
    std::uint64_t value;
};

namespace detail {

[[nodiscard]] inline std::optional<std::uint64_t> read_be(SceCursor &c, std::size_t n) noexcept {
    const std::uint8_t *p = c.peek_slice(n);
    if (p == nullptr) {
        return std::nullopt;
    }
    std::uint64_t value = 0;
    for (std::size_t i = 0; i < n; ++i) {
        value = (value << 8) | p[i];
    }
    (void)c.advance(n);
    return value;
}

/// The payload of a string whose head said `len`, as a pointer into the
/// input — or `nullptr` when the input is shorter.
[[nodiscard]] inline const std::uint8_t *read_payload(SceCursor &c, std::uint64_t len) noexcept {
    if (len > c.remaining()) {
        return nullptr;
    }
    const std::uint8_t *p = c.peek_slice(static_cast<std::size_t>(len));
    (void)c.advance(static_cast<std::size_t>(len));
    return p;
}

}  // namespace detail

/// Read one head. An indefinite length and the reserved values 28–30 are
/// refused.
[[nodiscard]] inline std::optional<Head> read_head(SceCursor &c) noexcept {
    auto initial = detail::read_be(c, 1);
    if (!initial) {
        return std::nullopt;
    }
    const auto major = static_cast<std::uint8_t>(*initial >> 5);
    const auto info = static_cast<std::uint8_t>(*initial & 0x1F);
    std::optional<std::uint64_t> value;
    if (info < 24) {
        value = info;
    } else if (info == 24) {
        value = detail::read_be(c, 1);
    } else if (info == 25) {
        value = detail::read_be(c, 2);
    } else if (info == 26) {
        value = detail::read_be(c, 4);
    } else if (info == 27) {
        value = detail::read_be(c, 8);
    } else {
        return std::nullopt;
    }
    if (!value) {
        return std::nullopt;
    }
    return Head{major, *value};
}

namespace detail {

[[nodiscard]] inline std::optional<std::uint64_t> expect(SceCursor &c, std::uint8_t major) noexcept {
    auto head = read_head(c);
    if (!head || head->major != major) {
        return std::nullopt;
    }
    return head->value;
}

}  // namespace detail

/// Read an unsigned integer.
[[nodiscard]] inline std::optional<std::uint64_t> read_uint(SceCursor &c) noexcept {
    return detail::expect(c, kMajorUnsigned);
}

/// Read an unsigned integer an entry of `max` holds.
[[nodiscard]] inline std::optional<std::uint64_t> read_uint_upto(SceCursor &c, std::uint64_t max) noexcept {
    auto value = read_uint(c);
    if (!value || *value > max) {
        return std::nullopt;
    }
    return value;
}

/// Read `false` or `true`.
[[nodiscard]] inline std::optional<bool> read_bool(SceCursor &c) noexcept {
    auto head = read_head(c);
    if (!head || head->major != kMajorSimple || (head->value != 20 && head->value != 21)) {
        return std::nullopt;
    }
    return head->value == 21;
}

/// Read a UTF-8 text string of at most `max_size` bytes (`std::nullopt`:
/// no bound).
[[nodiscard]] inline std::optional<std::string> read_text(SceCursor &c,
                                                          std::optional<std::uint64_t> max_size) noexcept {
    auto len = detail::expect(c, kMajorText);
    if (!len || (max_size && *len > *max_size)) {
        return std::nullopt;
    }
    const std::uint8_t *p = detail::read_payload(c, *len);
    if (p == nullptr || !is_valid_utf8(p, static_cast<std::size_t>(*len))) {
        return std::nullopt;
    }
    return std::string(reinterpret_cast<const char *>(p), static_cast<std::size_t>(*len));
}

/// Read a byte string of at most `max_size` bytes (`std::nullopt`: no bound).
[[nodiscard]] inline std::optional<std::vector<std::uint8_t>>
read_bytes(SceCursor &c, std::optional<std::uint64_t> max_size) noexcept {
    auto len = detail::expect(c, kMajorBytes);
    if (!len || (max_size && *len > *max_size)) {
        return std::nullopt;
    }
    const std::uint8_t *p = detail::read_payload(c, *len);
    if (p == nullptr) {
        return std::nullopt;
    }
    return std::vector<std::uint8_t>(p, p + static_cast<std::size_t>(*len));
}

/// Read a byte string of exactly `length` bytes.
[[nodiscard]] inline std::optional<std::vector<std::uint8_t>> read_bytes_exact(SceCursor &c,
                                                                               std::uint64_t length) noexcept {
    auto len = detail::expect(c, kMajorBytes);
    if (!len || *len != length) {
        return std::nullopt;
    }
    const std::uint8_t *p = detail::read_payload(c, *len);
    if (p == nullptr) {
        return std::nullopt;
    }
    return std::vector<std::uint8_t>(p, p + static_cast<std::size_t>(*len));
}

/// Read the head of a definite-length map: how many entries follow.
[[nodiscard]] inline std::optional<std::uint64_t> read_map_len(SceCursor &c) noexcept {
    return detail::expect(c, kMajorMap);
}

namespace detail {

[[nodiscard]] inline bool skip_at(SceCursor &c, std::uint32_t depth) noexcept {
    if (depth >= kMaxSkipDepth) {
        return false;
    }
    auto head = read_head(c);
    if (!head) {
        return false;
    }
    switch (head->major) {
    // Unsigned, negative, simple / float: the head is the whole item.
    case 0:
    case 1:
    case 7:
        return true;
    case 2:
    case 3:
        return read_payload(c, head->value) != nullptr;
    case 4:
        for (std::uint64_t i = 0; i < head->value; ++i) {
            if (!skip_at(c, depth + 1)) {
                return false;
            }
        }
        return true;
    case 5:
        for (std::uint64_t i = 0; i < head->value; ++i) {
            if (!skip_at(c, depth + 1) || !skip_at(c, depth + 1)) {
                return false;
            }
        }
        return true;
    // A tag: its one enclosed item follows.
    case 6:
        return skip_at(c, depth + 1);
    default:
        return false;
    }
}

}  // namespace detail

/// Skip one item — the value of a key the codec does not declare — and
/// everything nested in it, refusing one nested deeper than `kMaxSkipDepth`.
[[nodiscard]] inline bool skip(SceCursor &c) noexcept {
    return detail::skip_at(c, 0);
}

}  // namespace SCE::Forge::Cbor
