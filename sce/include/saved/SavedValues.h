// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Saved — how each `sce:type` is written into a saved state and read back: only if it
// is one, since a value of another kind, or one its width cannot hold, is refused rather
// than converted. The functions a generated machine's `save` and `restore` call, one per
// type, named as the type is.
//
// A value is written as the JSON value model holds it (a JsonNumber, a string, a bool, an
// array, an object, null) and read back from it, a refusal naming what holds a value that
// is not the type's.
//
// Cross-language siblings: backends/go/runtime/saved_values.go,
// backends/python/runtime/sce_runtime/saved_values.py, com.sce.runtime.SavedValues (Kotlin)
// and the `SavedValue` trait of sce_rust_runtime::saved_state (Rust).

#pragma once

#include "common/JsonText.h"
#include "saved/SavedJson.h"
#include "saved/SavedState.h"

#include <charconv>
#include <cmath>
#include <cstdint>
#include <cstdlib>
#include <limits>
#include <map>
#include <optional>
#include <string>
#include <utility>
#include <vector>

namespace SCE::Saved {

namespace detail {

/// The refusal of `value`, saved for `what`, which is not `kind` (`a number`, `a text`, ...).
/// A null is named apart: it is an absent value (docs/adr/0055), which only a place the
/// restoring build declares optional reads, so a state saved after a widening is refused at
/// its first null by a build from before it.
[[noreturn]] inline void notA(const JsonValue &value, const std::string &what, const std::string &kind) {
    if (value.isNull()) {
        refuse("'" + what + "' is absent (null), and its type is not optional");
    }
    refuse("'" + what + "' is not " + kind);
}

inline const std::string &savedNumber(const JsonValue &value, const std::string &what) {
    if (!value.isNumber()) {
        notA(value, what, "a number");
    }
    return value.asNumber().text;
}

/// Whether `text` is an optional minus and then digits, nothing else.
inline bool isSignedDigits(const std::string &text) {
    std::size_t at = (!text.empty() && text[0] == '-') ? 1 : 0;
    if (at >= text.size()) {
        return false;
    }
    for (; at < text.size(); ++at) {
        if (text[at] < '0' || text[at] > '9') {
            return false;
        }
    }
    return true;
}

/// `text` as a signed 64-bit whole number, when it is one.
inline std::optional<std::int64_t> parseInt64(const std::string &text) {
    if (!isSignedDigits(text)) {
        return std::nullopt;
    }
    std::int64_t value = 0;
    const auto result = std::from_chars(text.data(), text.data() + text.size(), value);
    if (result.ec != std::errc() || result.ptr != text.data() + text.size()) {
        return std::nullopt;
    }
    return value;
}

/// `text` as an unsigned 64-bit whole number, when it is nothing but digits that fit.
inline std::optional<std::uint64_t> parseUint64(const std::string &text) {
    if (text.empty()) {
        return std::nullopt;
    }
    for (const char c : text) {
        if (c < '0' || c > '9') {
            return std::nullopt;
        }
    }
    std::uint64_t value = 0;
    const auto result = std::from_chars(text.data(), text.data() + text.size(), value);
    if (result.ec != std::errc() || result.ptr != text.data() + text.size()) {
        return std::nullopt;
    }
    return value;
}

/// A number of at least `least` and at most `most`, written as a whole.
inline std::int64_t savedWhole(const JsonValue &value, const std::string &what, std::int64_t least, std::int64_t most) {
    const std::string &text = savedNumber(value, what);
    const auto n = parseInt64(text);
    if (!n.has_value() || *n < least || *n > most) {
        refuse("'" + what + "' (" + text + ") is not a whole number its type can hold");
    }
    return *n;
}

inline const std::string &savedWideText(const JsonValue &value, const std::string &what) {
    if (!value.isString()) {
        notA(value, what, "a 64-bit whole number written as a text");
    }
    return value.asString();
}

inline double savedReal(const JsonValue &value, const std::string &what) {
    if (value.isString()) {
        const std::string &text = value.asString();
        if (text == "NaN") {
            return std::numeric_limits<double>::quiet_NaN();
        }
        if (text == "Infinity") {
            return std::numeric_limits<double>::infinity();
        }
        if (text == "-Infinity") {
            return -std::numeric_limits<double>::infinity();
        }
    }
    const std::string &text = savedNumber(value, what);
    // A number past the range of a 64-bit real reads as the infinity it rounds to, as every
    // other backend reads it; only a text that is no number is refused.
    char *end = nullptr;
    const double real = std::strtod(text.c_str(), &end);
    if (text.empty() || end != text.c_str() + text.size()) {
        refuse("'" + what + "' (" + text + ") is not a number");
    }
    return real;
}

}  // namespace detail

// ── Write: a value's saved form ──────────────────────────────────────────────

inline JsonValue savedWriteUint8(std::uint8_t value) {
    return JsonNumber{std::to_string(value)};
}

inline JsonValue savedWriteUint16(std::uint16_t value) {
    return JsonNumber{std::to_string(value)};
}

inline JsonValue savedWriteUint32(std::uint32_t value) {
    return JsonNumber{std::to_string(value)};
}

inline JsonValue savedWriteInt8(std::int8_t value) {
    return JsonNumber{std::to_string(static_cast<int>(value))};
}

inline JsonValue savedWriteInt16(std::int16_t value) {
    return JsonNumber{std::to_string(value)};
}

inline JsonValue savedWriteInt32(std::int32_t value) {
    return JsonNumber{std::to_string(value)};
}

/// A 64-bit integer is written as a text: a reader that holds JSON numbers as doubles would
/// lose its low bits, and a saved state outlives the backend that wrote it.
inline JsonValue savedWriteUint64(std::uint64_t value) {
    return JsonValue(std::to_string(value));
}

inline JsonValue savedWriteInt64(std::int64_t value) {
    return JsonValue(std::to_string(value));
}

/// A real's saved form: the one number text every engine writes (ARCHITECTURE.md, "JSON
/// Number Text"), and, for a value that is not finite, which JSON has no spelling for, the
/// text every backend reads it back from.
inline JsonValue savedWriteFloat64(double value) {
    if (std::isnan(value)) {
        return JsonValue("NaN");
    }
    if (std::isinf(value)) {
        return JsonValue(value > 0 ? "Infinity" : "-Infinity");
    }
    return JsonNumber{JsonText::numberText(value)};
}

/// A 32-bit real's saved form: that of the 64-bit real it widens to, which is exact, so the
/// number a reader that holds JSON numbers as doubles finds is the number the machine holds.
inline JsonValue savedWriteFloat32(float value) {
    return savedWriteFloat64(static_cast<double>(value));
}

inline JsonValue savedWriteBool(bool value) {
    return JsonValue(value);
}

inline JsonValue savedWriteString(const std::string &value) {
    return JsonValue(value);
}

/// A byte string's byte-exact Latin-1 text, each byte the character of that code point
/// (docs/adr/0005, decision 2): the spelling it takes on the wire, so one value is one text
/// on every backend, and a JSON reader that has no byte string reads it as a string. The
/// text is UTF-8, as every string is, so a byte past 0x7F is its two-byte form.
inline JsonValue savedWriteBytes(const std::vector<std::uint8_t> &value) {
    std::string text;
    text.reserve(value.size());
    for (const std::uint8_t byte : value) {
        if (byte < 0x80) {
            text += static_cast<char>(byte);
        } else {
            text += static_cast<char>(0xC0 | (byte >> 6));
            text += static_cast<char>(0x80 | (byte & 0x3F));
        }
    }
    return JsonValue(std::move(text));
}

/// A list, each element by `write`.
template <typename T, typename Write> JsonValue savedWriteList(const std::vector<T> &value, Write write) {
    JsonArray items;
    items.reserve(value.size());
    for (const T &item : value) {
        items.push_back(write(item));
    }
    return JsonValue(std::move(items));
}

/// A value that may be absent (docs/adr/0055, section 4): null where it is absent, and what
/// `write` makes of the value where it is present.
template <typename T, typename Write> JsonValue savedWriteOptional(const std::optional<T> &value, Write write) {
    if (!value.has_value()) {
        return JsonValue();
    }
    return write(*value);
}

// ── Read: a value back from its saved form ───────────────────────────────────

inline std::uint8_t savedReadUint8(const JsonValue &value, const std::string &what) {
    return static_cast<std::uint8_t>(detail::savedWhole(value, what, 0, 0xFF));
}

inline std::uint16_t savedReadUint16(const JsonValue &value, const std::string &what) {
    return static_cast<std::uint16_t>(detail::savedWhole(value, what, 0, 0xFFFF));
}

inline std::uint32_t savedReadUint32(const JsonValue &value, const std::string &what) {
    return static_cast<std::uint32_t>(detail::savedWhole(value, what, 0, 0xFFFFFFFFLL));
}

inline std::int8_t savedReadInt8(const JsonValue &value, const std::string &what) {
    return static_cast<std::int8_t>(detail::savedWhole(value, what, -128, 127));
}

inline std::int16_t savedReadInt16(const JsonValue &value, const std::string &what) {
    return static_cast<std::int16_t>(detail::savedWhole(value, what, -32768, 32767));
}

inline std::int32_t savedReadInt32(const JsonValue &value, const std::string &what) {
    return static_cast<std::int32_t>(detail::savedWhole(value, what, INT32_MIN, INT32_MAX));
}

inline std::uint64_t savedReadUint64(const JsonValue &value, const std::string &what) {
    const std::string &text = detail::savedWideText(value, what);
    const auto n = detail::parseUint64(text);
    if (!n.has_value()) {
        detail::refuse("'" + what + "' (" + text + ") is not a whole number its type can hold");
    }
    return *n;
}

inline std::int64_t savedReadInt64(const JsonValue &value, const std::string &what) {
    const std::string &text = detail::savedWideText(value, what);
    const auto n = detail::parseInt64(text);
    if (!n.has_value()) {
        detail::refuse("'" + what + "' (" + text + ") is not a whole number its type can hold");
    }
    return *n;
}

inline float savedReadFloat32(const JsonValue &value, const std::string &what) {
    return static_cast<float>(detail::savedReal(value, what));
}

inline double savedReadFloat64(const JsonValue &value, const std::string &what) {
    return detail::savedReal(value, what);
}

inline bool savedReadBool(const JsonValue &value, const std::string &what) {
    if (!value.isBool()) {
        detail::notA(value, what, "a truth value");
    }
    return value.asBool();
}

inline std::string savedReadString(const JsonValue &value, const std::string &what) {
    if (!value.isString()) {
        detail::notA(value, what, "a text");
    }
    return value.asString();
}

/// A text read back only if it holds no more than the `capacity` UTF-8 bytes the machine
/// bounds it by: a machine never holds more, and a restored one must not be the first to.
inline std::string savedReadBoundedString(const JsonValue &value, const std::string &what, std::size_t capacity) {
    std::string text = savedReadString(value, what);
    if (text.size() > capacity) {
        detail::refuse("'" + what + "' holds " + std::to_string(text.size()) + " UTF-8 bytes, past the " +
                       std::to_string(capacity) + " it is bounded by");
    }
    return text;
}

/// A byte string read back from its Latin-1 text (savedWriteBytes), only if it holds no more
/// than the `capacity` bytes the machine bounds it by. A character past U+00FF is no byte,
/// and is refused rather than cut.
inline std::vector<std::uint8_t> savedReadBytes(const JsonValue &value, const std::string &what, std::size_t capacity) {
    const std::string text = savedReadString(value, what);
    std::vector<std::uint8_t> bytes;
    bytes.reserve(text.size());
    std::size_t at = 0;
    while (at < text.size()) {
        const std::uint32_t point = SessionId::detail::nextCodePoint(text, at);
        if (point > 0xFF) {
            char hex[16];
            std::snprintf(hex, sizeof hex, "%04X", static_cast<unsigned>(point));
            detail::refuse("'" + what + "' holds U+" + hex + ", which is past U+00FF and so no byte");
        }
        bytes.push_back(static_cast<std::uint8_t>(point));
    }
    if (bytes.size() > capacity) {
        detail::refuse("'" + what + "' holds " + std::to_string(bytes.size()) + " bytes, past the " +
                       std::to_string(capacity) + " it is bounded by");
    }
    return bytes;
}

/// A list read back element by element, only if it holds no more than the `capacity` the
/// machine bounds it by: a machine never holds more, and a restored one must not be the
/// first to.
template <typename Element>
auto savedReadList(const JsonValue &value, const std::string &what, std::size_t capacity, Element element) {
    using T = decltype(element(value, what));
    if (!value.isArray()) {
        detail::notA(value, what, "an array");
    }
    const JsonArray &items = value.asArray();
    if (items.size() > capacity) {
        detail::refuse("'" + what + "' holds " + std::to_string(items.size()) + " elements, past the " +
                       std::to_string(capacity) + " it is bounded by");
    }
    std::vector<T> read;
    read.reserve(items.size());
    for (std::size_t at = 0; at < items.size(); ++at) {
        read.push_back(element(items[at], what + "[" + std::to_string(at) + "]"));
    }
    return read;
}

/// A value that may be absent read back (docs/adr/0055, section 4): nullopt where the saved
/// state holds null, and otherwise the value `present` reads, held to its type as any value
/// is. Only a place the machine declares optional reads through this, so a null saved where
/// the machine holds a value of `T` is refused by the reader of `T`, naming what holds it.
template <typename Present>
auto savedReadOptional(const JsonValue &value, const std::string &what, Present present)
    -> std::optional<decltype(present(value, what))> {
    if (value.isNull()) {
        return std::nullopt;
    }
    return present(value, what);
}

/// The reader of a text held to `capacity` UTF-8 bytes, for a place that reads through a
/// function value (savedReadOptional).
inline auto savedBoundedString(std::size_t capacity) {
    return [capacity](const JsonValue &value, const std::string &what) {
        return savedReadBoundedString(value, what, capacity);
    };
}

/// The reader of a byte string held to `capacity` bytes, for a place that reads through a
/// function value (savedReadOptional).
inline auto savedBoundedBytes(std::size_t capacity) {
    return
        [capacity](const JsonValue &value, const std::string &what) { return savedReadBytes(value, what, capacity); };
}

/// An enum value read back from the name its document declares for the variant: refused
/// unless it is a text and one of `variants`, by that name. `alias` is the enum, as the
/// document imports it, which a refusal names.
template <typename T>
T savedReadVariant(const JsonValue &value, const std::string &what, const std::string &alias,
                   const std::map<std::string, T> &variants) {
    const std::string declared = savedReadString(value, what);
    const auto found = variants.find(declared);
    if (found == variants.end()) {
        detail::refuse("'" + what + "' (" + declared + ") is not a variant of " + alias);
    }
    return found->second;
}

/// The field `name` of a saved record value: what a generated record reads each field with.
inline const JsonValue &savedField(const JsonValue &value, const std::string &record, const std::string &name) {
    if (!value.isObject()) {
        detail::refuse("'" + record + "' is not an object");
    }
    const JsonValue *field = value.asObject().get(name);
    if (field == nullptr) {
        detail::refuse("'" + record + "' has no field '" + name + "'");
    }
    return *field;
}

}  // namespace SCE::Saved
