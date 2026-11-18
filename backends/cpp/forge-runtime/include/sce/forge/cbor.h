// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// sce_forge_runtime — the CBOR (RFC 8949) items a `sce:encoding="cbor"`
// codec reads and writes (SCE_FORGE.md §4.6.1, docs/adr/0101): one
// definite-length map whose keys are small unsigned integers, and below it
// the form of each type of the language: an integer of either sign, a float,
// a truth value, a text or byte string, an array, a record's map keyed by its
// fields' ids as text, and a union's map of one entry keyed by its case's
// variant name.
//
// Mirrors `backends/rust/forge-runtime/src/cbor.rs`, rule for rule. A
// generated codec calls these; it spells no CBOR of its own.
//
// Writing is deterministic (RFC 8949 section 4.2.1): every head in its
// shortest form and a float in the shortest precision that holds it exactly,
// so two backends given the same value write the same bytes. Reading takes
// the definite-length profile of CBOR and, within it, every well-formed
// encoding of a value (a head longer than it needs, a float wider than it
// needs), and refuses by the class of what it refuses (docs/adr/0101 section
// 3.3): not well-formed or outside the profile (`CborMalformed`), an item the
// place does not take (`CborWrongType`), a value outside its type
// (`CborOutOfRange`).
//
// A read answers a `Decoded`: the item, or the `CodecError` that refused it
// (docs/adr/0099, amendment of 2026-11-16), each kind the one the Rust
// reader answers for the same bytes. A reader that refuses may have advanced
// the cursor, so a generated decode works on a copy of the caller's cursor
// and assigns it back only once the whole map is read.

#pragma once

#include "sce/forge/codec.h"
#include "sce/forge/span.h"

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <optional>
#include <string>
#include <string_view>
#include <vector>

namespace SCE::Forge::Cbor {

/// Major type 0, an unsigned integer.
inline constexpr std::uint8_t kMajorUnsigned = 0;
/// Major type 1, a negative integer `-1 - n`.
inline constexpr std::uint8_t kMajorNegative = 1;
/// Major type 2, a byte string.
inline constexpr std::uint8_t kMajorBytes = 2;
/// Major type 3, a UTF-8 text string.
inline constexpr std::uint8_t kMajorText = 3;
/// Major type 4, an array.
inline constexpr std::uint8_t kMajorArray = 4;
/// Major type 5, a map.
inline constexpr std::uint8_t kMajorMap = 5;
/// Major type 6, a tagged item.
inline constexpr std::uint8_t kMajorTag = 6;
/// Major type 7, simple values (`false` = 20, `true` = 21) and floats.
inline constexpr std::uint8_t kMajorSimple = 7;

/// The deepest an unknown entry's value may nest before a skip refuses it
/// (SCE_FORGE.md §4.6.1).
inline constexpr std::uint32_t kMaxSkipDepth = 16;

namespace detail {

/// The additional information of `false` and `true` (RFC 8949 section 3.3).
inline constexpr std::uint8_t kSimpleFalse = 20;
inline constexpr std::uint8_t kSimpleTrue = 21;

/// The additional information of a half, a single and a double precision
/// float.
inline constexpr std::uint8_t kInfoHalf = 25;
inline constexpr std::uint8_t kInfoSingle = 26;
inline constexpr std::uint8_t kInfoDouble = 27;

/// The least simple value the two-byte form may carry: one below it is not
/// well-formed.
inline constexpr std::uint64_t kLeastTwoByteSimple = 32;

/// The one NaN written, the quiet NaN of half precision (docs/adr/0086,
/// section 9.2).
inline constexpr std::uint16_t kNanHalf = 0x7e00;

[[nodiscard]] inline std::uint64_t bits_of(double x) noexcept {
    std::uint64_t bits = 0;
    std::memcpy(&bits, &x, sizeof bits);
    return bits;
}

[[nodiscard]] inline std::uint32_t bits_of(float x) noexcept {
    std::uint32_t bits = 0;
    std::memcpy(&bits, &x, sizeof bits);
    return bits;
}

[[nodiscard]] inline double double_of(std::uint64_t bits) noexcept {
    double x = 0;
    std::memcpy(&x, &bits, sizeof x);
    return x;
}

[[nodiscard]] inline float float_of(std::uint32_t bits) noexcept {
    float x = 0;
    std::memcpy(&x, &bits, sizeof x);
    return x;
}

/// The half precision float that is `x` exactly, as its bits, or
/// `std::nullopt` when no half is (IEEE 754 binary16: 5 bits of exponent, 10
/// of fraction, subnormal below 2^-14). `x` is not a NaN.
[[nodiscard]] inline std::optional<std::uint16_t> half_of(double x) noexcept {
    const std::uint64_t bits = bits_of(x);
    const auto sign = static_cast<std::uint16_t>((bits >> 48) & 0x8000U);
    const auto exponent = static_cast<int>((bits >> 52) & 0x7ffU);
    const std::uint64_t fraction = bits & ((std::uint64_t{1} << 52) - 1);
    if (exponent == 0x7ff) {
        // An infinity; a NaN is written before this is asked.
        if (fraction != 0) {
            return std::nullopt;
        }
        return static_cast<std::uint16_t>(sign | 0x7c00U);
    }
    if (exponent == 0) {
        // Zero, or a double's subnormal, which is far below a half's least.
        if (fraction != 0) {
            return std::nullopt;
        }
        return sign;
    }
    const int power = exponent - 1023;
    if (power >= -14 && power <= 15) {
        // A half's normal number keeps the top 10 bits of the fraction and
        // nothing below them.
        if ((fraction & ((std::uint64_t{1} << 42) - 1)) != 0) {
            return std::nullopt;
        }
        return static_cast<std::uint16_t>(sign | (static_cast<std::uint16_t>(power + 15) << 10) |
                                          static_cast<std::uint16_t>(fraction >> 42));
    }
    if (power >= -24 && power <= -15) {
        // A half's subnormal number is k * 2^-24 for k from 1 to 1023.
        const std::uint64_t significand = fraction | (std::uint64_t{1} << 52);
        const auto shift = static_cast<unsigned>(28 - power);
        if ((significand & ((std::uint64_t{1} << shift) - 1)) != 0) {
            return std::nullopt;
        }
        return static_cast<std::uint16_t>(sign | static_cast<std::uint16_t>(significand >> shift));
    }
    return std::nullopt;
}

/// The single precision float that is `x` exactly, or `std::nullopt` when no
/// single is. `x` is not a NaN.
[[nodiscard]] inline std::optional<float> single_of(double x) noexcept {
    // A double past a single's range converts to no single (the conversion
    // is undefined there), so the range is judged first.
    if (std::isfinite(x) && std::fabs(x) > static_cast<double>(std::numeric_limits<float>::max())) {
        return std::nullopt;
    }
    const auto single = static_cast<float>(x);
    if (static_cast<double>(single) != x) {
        return std::nullopt;
    }
    return single;
}

/// The value of the half precision float whose bits are `half` (RFC 8949
/// Appendix D).
[[nodiscard]] inline double half_value(std::uint16_t half) noexcept {
    const int exponent = (half >> 10) & 0x1f;
    const int fraction = half & 0x3ff;
    double magnitude = 0;
    if (exponent == 0) {
        magnitude = std::ldexp(static_cast<double>(fraction), -24);
    } else if (exponent == 0x1f) {
        magnitude = fraction == 0 ? std::numeric_limits<double>::infinity() : std::numeric_limits<double>::quiet_NaN();
    } else {
        magnitude = std::ldexp(static_cast<double>(fraction + 1024), exponent - 25);
    }
    return (half & 0x8000U) != 0 ? -magnitude : magnitude;
}

}  // namespace detail

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

/// Write a signed integer: major type 0 for zero or more, and below zero
/// major type 1, whose head carries `-1 - value` (RFC 8949 section 3.1).
[[nodiscard]] inline std::optional<CodecError> write_int(SceSink &w, std::int64_t value) noexcept {
    if (value >= 0) {
        return write_head(w, kMajorUnsigned, static_cast<std::uint64_t>(value));
    }
    // -1 - value is the bitwise complement of value, which never overflows.
    return write_head(w, kMajorNegative, ~static_cast<std::uint64_t>(value));
}

/// Write a float in the shortest of half, single and double precision that
/// holds it exactly, and a NaN as `f9 7e 00` (RFC 8949 sections 4.1 and
/// 4.2.1). A `float32` and a `float64` share it: a single is a double
/// exactly.
[[nodiscard]] inline std::optional<CodecError> write_float(SceSink &w, double x) noexcept {
    std::uint8_t item[9];
    std::size_t n = 0;
    auto be = [&](std::uint64_t bits, std::size_t width) {
        for (std::size_t i = width; i-- > 0;) {
            item[n++] = static_cast<std::uint8_t>(bits >> (8 * i));
        }
    };
    if (std::isnan(x)) {
        item[n++] = static_cast<std::uint8_t>((kMajorSimple << 5) | detail::kInfoHalf);
        be(detail::kNanHalf, 2);
    } else if (const auto half = detail::half_of(x)) {
        item[n++] = static_cast<std::uint8_t>((kMajorSimple << 5) | detail::kInfoHalf);
        be(*half, 2);
    } else if (const auto single = detail::single_of(x)) {
        item[n++] = static_cast<std::uint8_t>((kMajorSimple << 5) | detail::kInfoSingle);
        be(detail::bits_of(*single), 4);
    } else {
        item[n++] = static_cast<std::uint8_t>((kMajorSimple << 5) | detail::kInfoDouble);
        be(detail::bits_of(x), 8);
    }
    return w.write_bytes(item, n);
}

/// Write `false` or `true`.
[[nodiscard]] inline std::optional<CodecError> write_bool(SceSink &w, bool value) noexcept {
    return w.write_u8(
        static_cast<std::uint8_t>((kMajorSimple << 5) | (value ? detail::kSimpleTrue : detail::kSimpleFalse)));
}

/// Write a text string, viewed where it lives: a `std::string` and a printed
/// codec's `Text` alike, as an entry's value, or the key of a record's field.
[[nodiscard]] inline std::optional<CodecError> write_text(SceSink &w, std::string_view value) noexcept {
    if (auto e = write_head(w, kMajorText, value.size())) {
        return e;
    }
    return w.write_bytes(reinterpret_cast<const std::uint8_t *>(value.data()), value.size());
}

/// Write a byte string, viewed where it lives: a `std::vector` and a printed
/// codec's `Bytes` alike.
[[nodiscard]] inline std::optional<CodecError> write_bytes(SceSink &w, Span<const std::uint8_t> value) noexcept {
    if (auto e = write_head(w, kMajorBytes, value.size())) {
        return e;
    }
    return w.write_bytes(value.data(), value.size());
}

/// Write the head of a definite-length array of `items` items: a list an
/// entry holds.
[[nodiscard]] inline std::optional<CodecError> write_array_head(SceSink &w, std::uint64_t items) noexcept {
    return write_head(w, kMajorArray, items);
}

/// Write the head of a definite-length map of `entries` entries.
[[nodiscard]] inline std::optional<CodecError> write_map_head(SceSink &w, std::uint64_t entries) noexcept {
    return write_head(w, kMajorMap, entries);
}

/// Write the head of a union's value: a map of one entry keyed by the case's
/// variant name as a text string (docs/adr/0101 section 2.1). The case's
/// record, or the empty map for a case that holds none, is written after it.
[[nodiscard]] inline std::optional<CodecError> write_variant(SceSink &w, std::string_view name) noexcept {
    if (auto e = write_map_head(w, 1)) {
        return e;
    }
    return write_text(w, name);
}

// ── Reading ─────────────────────────────────────────────────────────────

/// One head: its major type, its additional information, and the value that
/// information carries (for a string, an array or a map, the length; for a
/// float, its bits).
struct Head {
    std::uint8_t major;
    std::uint8_t info;
    std::uint64_t value;
};

/// What a skip answers when it skipped its item whole: a skip has no value to
/// answer, so its `Decoded` holds this or the refusal.
struct Skipped {};

namespace detail {

[[nodiscard]] inline Decoded<std::uint64_t> read_be(SceCursor &c, std::size_t n) noexcept {
    const std::uint8_t *p = c.peek_slice(n);
    if (p == nullptr) {
        return CodecError::NeedMoreBytes;
    }
    std::uint64_t value = 0;
    for (std::size_t i = 0; i < n; ++i) {
        value = (value << 8) | p[i];
    }
    (void)c.advance(n);
    return value;
}

/// The payload of a string whose head said `len`, as a pointer into the
/// input, refused as `NeedMoreBytes` when the input is shorter.
[[nodiscard]] inline Decoded<const std::uint8_t *> read_payload(SceCursor &c, std::uint64_t len) noexcept {
    if (len > c.remaining()) {
        return CodecError::NeedMoreBytes;
    }
    const std::uint8_t *p = c.peek_slice(static_cast<std::size_t>(len));
    (void)c.advance(static_cast<std::size_t>(len));
    return p;
}

}  // namespace detail

/// Read one head. What is not well-formed (RFC 8949 Appendix F: the reserved
/// values 28-30, a break code, a simple value in the two-byte form below 32)
/// and an indefinite length, which is outside the profile the codec reads
/// (docs/adr/0101 section 3.1), are refused as `CborMalformed`.
[[nodiscard]] inline Decoded<Head> read_head(SceCursor &c) noexcept {
    const auto initial = detail::read_be(c, 1);
    if (!initial) {
        return initial.error();
    }
    const auto major = static_cast<std::uint8_t>(*initial >> 5);
    const auto info = static_cast<std::uint8_t>(*initial & 0x1F);
    if (info < 24) {
        return Head{major, info, info};
    }
    std::size_t width = 0;
    switch (info) {
    case 24:
        width = 1;
        break;
    case 25:
        width = 2;
        break;
    case 26:
        width = 4;
        break;
    case 27:
        width = 8;
        break;
    default:
        return CodecError::CborMalformed;
    }
    const auto value = detail::read_be(c, width);
    if (!value) {
        return value.error();
    }
    if (major == kMajorSimple && info == 24 && *value < detail::kLeastTwoByteSimple) {
        return CodecError::CborMalformed;
    }
    return Head{major, info, *value};
}

namespace detail {

/// The value of a head of `major`; a head of another major type is an item
/// the place does not take, `CborWrongType`.
[[nodiscard]] inline Decoded<std::uint64_t> expect(SceCursor &c, std::uint8_t major) noexcept {
    const auto head = read_head(c);
    if (!head) {
        return head.error();
    }
    if (head->major != major) {
        return CodecError::CborWrongType;
    }
    return head->value;
}

}  // namespace detail

/// Read an unsigned integer; a negative one, as any other item, is
/// `CborWrongType`.
[[nodiscard]] inline Decoded<std::uint64_t> read_uint(SceCursor &c) noexcept {
    return detail::expect(c, kMajorUnsigned);
}

/// Read an unsigned integer an entry of `max` holds; past it, `CborOutOfRange`.
[[nodiscard]] inline Decoded<std::uint64_t> read_uint_upto(SceCursor &c, std::uint64_t max) noexcept {
    const auto value = read_uint(c);
    if (!value) {
        return value.error();
    }
    if (*value > max) {
        return CodecError::CborOutOfRange;
    }
    return *value;
}

/// Read a signed integer of `least` to `greatest`, the range of its type:
/// major type 0, or major type 1 below zero. A number outside the range is
/// `CborOutOfRange`, another item `CborWrongType`.
[[nodiscard]] inline Decoded<std::int64_t> read_int_within(SceCursor &c, std::int64_t least,
                                                           std::int64_t greatest) noexcept {
    const auto head = read_head(c);
    if (!head) {
        return head.error();
    }
    if (head->major == kMajorUnsigned) {
        if (greatest < 0 || head->value > static_cast<std::uint64_t>(greatest)) {
            return CodecError::CborOutOfRange;
        }
        return static_cast<std::int64_t>(head->value);
    }
    if (head->major == kMajorNegative) {
        // The item is -1 - n, which is at least `least` when n is at most
        // -1 - least, the bitwise complement of `least`.
        if (least >= 0 || head->value > ~static_cast<std::uint64_t>(least)) {
            return CodecError::CborOutOfRange;
        }
        return static_cast<std::int64_t>(~head->value);
    }
    return CodecError::CborWrongType;
}

/// Read a `float64`: a half, a single or a double, each read to the value it
/// is exactly. Any other item, an integer among them, is `CborWrongType`.
[[nodiscard]] inline Decoded<double> read_float(SceCursor &c) noexcept {
    const auto head = read_head(c);
    if (!head) {
        return head.error();
    }
    if (head->major == kMajorSimple && head->info == detail::kInfoHalf) {
        return detail::half_value(static_cast<std::uint16_t>(head->value));
    }
    if (head->major == kMajorSimple && head->info == detail::kInfoSingle) {
        return static_cast<double>(detail::float_of(static_cast<std::uint32_t>(head->value)));
    }
    if (head->major == kMajorSimple && head->info == detail::kInfoDouble) {
        return detail::double_of(head->value);
    }
    return CodecError::CborWrongType;
}

/// Read a `float32`: a float of any precision whose value a single holds
/// exactly. A double that no single is, is `CborOutOfRange`; a NaN of any
/// precision is a NaN.
[[nodiscard]] inline Decoded<float> read_float32(SceCursor &c) noexcept {
    const auto x = read_float(c);
    if (!x) {
        return x.error();
    }
    if (std::isnan(*x)) {
        return std::numeric_limits<float>::quiet_NaN();
    }
    const auto single = detail::single_of(*x);
    if (!single) {
        return CodecError::CborOutOfRange;
    }
    return *single;
}

/// Read `false` or `true`. Every other item, `null` and a float among them,
/// is `CborWrongType`.
[[nodiscard]] inline Decoded<bool> read_bool(SceCursor &c) noexcept {
    const auto head = read_head(c);
    if (!head) {
        return head.error();
    }
    if (head->major != kMajorSimple || (head->info != detail::kSimpleFalse && head->info != detail::kSimpleTrue)) {
        return CodecError::CborWrongType;
    }
    return head->info == detail::kSimpleTrue;
}

/// Read a UTF-8 text string of at most `max_size` bytes (`std::nullopt`:
/// no bound), its length judged before its bytes are read. A longer one is
/// `CborOutOfRange`, one that is not UTF-8 `InvalidUtf8`, one in chunks (an
/// indefinite length) `CborMalformed`.
[[nodiscard]] inline Decoded<std::string> read_text(SceCursor &c, std::optional<std::uint64_t> max_size) noexcept {
    const auto len = detail::expect(c, kMajorText);
    if (!len) {
        return len.error();
    }
    if (max_size && *len > *max_size) {
        return CodecError::CborOutOfRange;
    }
    const auto p = detail::read_payload(c, *len);
    if (!p) {
        return p.error();
    }
    if (!is_valid_utf8(*p, static_cast<std::size_t>(*len))) {
        return CodecError::InvalidUtf8;
    }
    return std::string(reinterpret_cast<const char *>(*p), static_cast<std::size_t>(*len));
}

/// Read a byte string of at most `max_size` bytes (`std::nullopt`: no
/// bound); a longer one is `CborOutOfRange`, one in chunks (an indefinite
/// length) `CborMalformed`.
[[nodiscard]] inline Decoded<std::vector<std::uint8_t>> read_bytes(SceCursor &c,
                                                                   std::optional<std::uint64_t> max_size) noexcept {
    const auto len = detail::expect(c, kMajorBytes);
    if (!len) {
        return len.error();
    }
    if (max_size && *len > *max_size) {
        return CodecError::CborOutOfRange;
    }
    const auto p = detail::read_payload(c, *len);
    if (!p) {
        return p.error();
    }
    return std::vector<std::uint8_t>(*p, *p + static_cast<std::size_t>(*len));
}

/// Read a byte string of exactly `length` bytes; another length is
/// `CborWrongLength`.
[[nodiscard]] inline Decoded<std::vector<std::uint8_t>> read_bytes_exact(SceCursor &c, std::uint64_t length) noexcept {
    const auto len = detail::expect(c, kMajorBytes);
    if (!len) {
        return len.error();
    }
    if (*len != length) {
        return CodecError::CborWrongLength;
    }
    const auto p = detail::read_payload(c, *len);
    if (!p) {
        return p.error();
    }
    return std::vector<std::uint8_t>(*p, *p + static_cast<std::size_t>(*len));
}

/// Read the head of a definite-length array: how many items follow. An
/// indefinite length is refused, as a map's is.
[[nodiscard]] inline Decoded<std::uint64_t> read_array_len(SceCursor &c) noexcept {
    return detail::expect(c, kMajorArray);
}

/// Read the head of a definite-length map: how many entries follow.
[[nodiscard]] inline Decoded<std::uint64_t> read_map_len(SceCursor &c) noexcept {
    return detail::expect(c, kMajorMap);
}

/// Read the head of a union's value and name its case (docs/adr/0101
/// sections 2.1 and 3.3): a map, judged for exactly one entry before its key
/// is read (`CborUnionEntries`), whose key is a text string (`CborWrongType`)
/// naming one of `names` (`CborUnknownVariant`). Answers the case's index in
/// `names`; the case's record, or the empty map, follows.
[[nodiscard]] inline Decoded<std::size_t> read_variant(SceCursor &c, Span<const std::string_view> names) noexcept {
    const auto entries = read_map_len(c);
    if (!entries) {
        return entries.error();
    }
    if (*entries != 1) {
        return CodecError::CborUnionEntries;
    }
    const auto name = read_text(c, std::nullopt);
    if (!name) {
        return name.error();
    }
    for (std::size_t i = 0; i < names.size(); ++i) {
        if (names.data()[i] == *name) {
            return i;
        }
    }
    return CodecError::CborUnknownVariant;
}

namespace detail {

[[nodiscard]] inline Decoded<Skipped> skip_at(SceCursor &c, std::uint32_t depth) noexcept {
    if (depth >= kMaxSkipDepth) {
        return CodecError::CborTooDeep;
    }
    const auto head = read_head(c);
    if (!head) {
        return head.error();
    }
    switch (head->major) {
    // Unsigned, negative, simple / float: the head is the whole item.
    case kMajorUnsigned:
    case kMajorNegative:
    case kMajorSimple:
        return Skipped{};
    case kMajorBytes:
    case kMajorText: {
        const auto payload = read_payload(c, head->value);
        if (!payload) {
            return payload.error();
        }
        return Skipped{};
    }
    case kMajorArray:
        for (std::uint64_t i = 0; i < head->value; ++i) {
            const auto item = skip_at(c, depth + 1);
            if (!item) {
                return item.error();
            }
        }
        return Skipped{};
    case kMajorMap:
        for (std::uint64_t i = 0; i < head->value; ++i) {
            const auto key = skip_at(c, depth + 1);
            if (!key) {
                return key.error();
            }
            const auto value = skip_at(c, depth + 1);
            if (!value) {
                return value.error();
            }
        }
        return Skipped{};
    // A tag: its one enclosed item follows.
    case kMajorTag:
        return skip_at(c, depth + 1);
    default:
        return CodecError::CborMalformed;
    }
}

}  // namespace detail

/// Skip one item — the value of a key the codec does not declare — and
/// everything nested in it, refusing one nested deeper than `kMaxSkipDepth`
/// as `CborTooDeep` and an indefinite length anywhere in it as
/// `CborMalformed`.
[[nodiscard]] inline Decoded<Skipped> skip(SceCursor &c) noexcept {
    return detail::skip_at(c, 0);
}

}  // namespace SCE::Forge::Cbor
