// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::Saved::JsonValue — the reader and writer a saved state's JSON text goes through.
//
// A saved state is one JSON document, the same on every backend, and what one backend
// saved another reads (docs/adr/0045). That asks two things of the reader that a
// general JSON library does not give: a number keeps the spelling it was written in, so
// a 64-bit value never passes through a double, and an object that names a member twice
// is refused, since a reader that takes the first and one that takes the last would read
// two values from one text. A value is null, a bool, a string, a number (its text), an
// array, or an object whose members stay in the order they were written.
//
// Header-only and dependency-free, as JsonText.h is, because generated machines compile
// without nlohmann. Cross-language siblings: backends/go/runtime/saved_json.go,
// backends/python/runtime/sce_runtime/saved_json.py, com.sce.runtime.Json (Kotlin) and
// sce_rust_runtime::json (Rust).

#pragma once

#include "common/JsonText.h"

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <variant>
#include <vector>

namespace SCE::Saved {

/// Why a text is not one JSON value, in the words every backend uses.
class JsonError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

/// A number kept in the spelling it was written in, so no width is lost.
struct JsonNumber {
    std::string text;

    bool operator==(const JsonNumber &other) const {
        return text == other.text;
    }
};

class JsonValue;
using JsonArray = std::vector<JsonValue>;

/// An object whose members stay in the order they were written, and none of whose names
/// repeats.
class JsonObject {
public:
    JsonObject() = default;

    [[nodiscard]] bool has(std::string_view key) const;
    /// The member `key`, or nullptr.
    [[nodiscard]] const JsonValue *get(std::string_view key) const;
    /// Sets `key`, replacing a member of that name in place and otherwise appending.
    void set(std::string key, JsonValue value);

    [[nodiscard]] const std::vector<std::string> &keys() const {
        return keys_;
    }

    [[nodiscard]] std::size_t size() const {
        return keys_.size();
    }

    [[nodiscard]] const JsonValue &valueAt(std::size_t index) const;

private:
    std::vector<std::string> keys_;
    std::vector<JsonValue> values_;
};

class JsonValue {
public:
    using Null = std::monostate;
    using Storage = std::variant<Null, bool, std::string, JsonNumber, JsonArray, JsonObject>;

    JsonValue() = default;

    JsonValue(std::nullptr_t) {}

    JsonValue(bool value) : storage_(value) {}

    JsonValue(const char *text) : storage_(std::string(text)) {}

    JsonValue(std::string text) : storage_(std::move(text)) {}

    JsonValue(JsonNumber number) : storage_(std::move(number)) {}

    JsonValue(JsonArray array) : storage_(std::move(array)) {}

    JsonValue(JsonObject object) : storage_(std::move(object)) {}

    [[nodiscard]] bool isNull() const {
        return std::holds_alternative<Null>(storage_);
    }

    [[nodiscard]] bool isBool() const {
        return std::holds_alternative<bool>(storage_);
    }

    [[nodiscard]] bool isString() const {
        return std::holds_alternative<std::string>(storage_);
    }

    [[nodiscard]] bool isNumber() const {
        return std::holds_alternative<JsonNumber>(storage_);
    }

    [[nodiscard]] bool isArray() const {
        return std::holds_alternative<JsonArray>(storage_);
    }

    [[nodiscard]] bool isObject() const {
        return std::holds_alternative<JsonObject>(storage_);
    }

    [[nodiscard]] bool asBool() const {
        return std::get<bool>(storage_);
    }

    [[nodiscard]] const std::string &asString() const {
        return std::get<std::string>(storage_);
    }

    [[nodiscard]] const JsonNumber &asNumber() const {
        return std::get<JsonNumber>(storage_);
    }

    [[nodiscard]] const JsonArray &asArray() const {
        return std::get<JsonArray>(storage_);
    }

    [[nodiscard]] const JsonObject &asObject() const {
        return std::get<JsonObject>(storage_);
    }

    [[nodiscard]] JsonObject &asObject() {
        return std::get<JsonObject>(storage_);
    }

    [[nodiscard]] JsonArray &asArray() {
        return std::get<JsonArray>(storage_);
    }

    [[nodiscard]] const Storage &storage() const {
        return storage_;
    }

private:
    Storage storage_;
};

inline bool JsonObject::has(std::string_view key) const {
    return get(key) != nullptr;
}

inline const JsonValue &JsonObject::valueAt(std::size_t index) const {
    return values_[index];
}

inline const JsonValue *JsonObject::get(std::string_view key) const {
    for (std::size_t at = 0; at < keys_.size(); ++at) {
        if (keys_[at] == key) {
            return &values_[at];
        }
    }
    return nullptr;
}

inline void JsonObject::set(std::string key, JsonValue value) {
    for (std::size_t at = 0; at < keys_.size(); ++at) {
        if (keys_[at] == key) {
            values_[at] = std::move(value);
            return;
        }
    }
    keys_.push_back(std::move(key));
    values_.push_back(std::move(value));
}

namespace detail {

inline JsonError malformed(const std::string &what) {
    return JsonError("not JSON (" + what + ")");
}

class Reader {
public:
    explicit Reader(std::string_view text) : text_(text) {}

    [[nodiscard]] bool atEnd() const {
        return at_ >= text_.size();
    }

    void skipWhitespace() {
        while (at_ < text_.size() &&
               (text_[at_] == ' ' || text_[at_] == '\t' || text_[at_] == '\n' || text_[at_] == '\r')) {
            ++at_;
        }
    }

    JsonValue readValue() {
        skipWhitespace();
        if (atEnd()) {
            throw malformed("it ends where a value was expected");
        }
        const char c = text_[at_];
        if (c == '{') {
            return readObject();
        }
        if (c == '[') {
            return readArray();
        }
        if (c == '"') {
            return JsonValue(readString());
        }
        if (c == 't') {
            readKeyword("true");
            return JsonValue(true);
        }
        if (c == 'f') {
            readKeyword("false");
            return JsonValue(false);
        }
        if (c == 'n') {
            readKeyword("null");
            return JsonValue();
        }
        if (c == '-' || (c >= '0' && c <= '9')) {
            return readNumber();
        }
        throw malformed(std::string("unexpected '") + c + "'");
    }

private:
    JsonValue readObject() {
        ++at_;  // '{'
        JsonObject out;
        skipWhitespace();
        if (!atEnd() && text_[at_] == '}') {
            ++at_;
            return JsonValue(std::move(out));
        }
        while (true) {
            skipWhitespace();
            if (atEnd() || text_[at_] != '"') {
                throw malformed("a field name was expected");
            }
            std::string key = readString();
            // An object whose names repeat has no single meaning: a reader that takes the
            // first and one that takes the last would read two values from one text, so it
            // is refused as the other backends refuse it.
            if (out.has(key)) {
                throw malformed("the field \"" + key + "\" appears twice");
            }
            skipWhitespace();
            if (atEnd() || text_[at_] != ':') {
                throw malformed("a ':' was expected");
            }
            ++at_;
            JsonValue value = readValue();
            out.set(std::move(key), std::move(value));
            skipWhitespace();
            if (atEnd()) {
                throw malformed("it ends unclosed");
            }
            const char c = text_[at_];
            if (c == ',') {
                ++at_;
            } else if (c == '}') {
                ++at_;
                return JsonValue(std::move(out));
            } else {
                throw malformed("a ',' or '}' was expected");
            }
        }
    }

    JsonValue readArray() {
        ++at_;  // '['
        JsonArray out;
        skipWhitespace();
        if (!atEnd() && text_[at_] == ']') {
            ++at_;
            return JsonValue(std::move(out));
        }
        while (true) {
            out.push_back(readValue());
            skipWhitespace();
            if (atEnd()) {
                throw malformed("it ends unclosed");
            }
            const char c = text_[at_];
            if (c == ',') {
                ++at_;
            } else if (c == ']') {
                ++at_;
                return JsonValue(std::move(out));
            } else {
                throw malformed("a ',' or ']' was expected");
            }
        }
    }

    static void appendUtf8(std::string &out, std::uint32_t point) {
        if (point < 0x80) {
            out += static_cast<char>(point);
        } else if (point < 0x800) {
            out += static_cast<char>(0xC0 | (point >> 6));
            out += static_cast<char>(0x80 | (point & 0x3F));
        } else if (point < 0x10000) {
            out += static_cast<char>(0xE0 | (point >> 12));
            out += static_cast<char>(0x80 | ((point >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (point & 0x3F));
        } else {
            out += static_cast<char>(0xF0 | (point >> 18));
            out += static_cast<char>(0x80 | ((point >> 12) & 0x3F));
            out += static_cast<char>(0x80 | ((point >> 6) & 0x3F));
            out += static_cast<char>(0x80 | (point & 0x3F));
        }
    }

    std::string readString() {
        ++at_;  // '"'
        std::string out;
        while (true) {
            if (atEnd()) {
                throw malformed("a text ends unclosed");
            }
            const char c = text_[at_++];
            if (c == '"') {
                return out;
            }
            if (c != '\\') {
                out += c;
                continue;
            }
            if (atEnd()) {
                throw malformed("an escape ends the text");
            }
            const char escape = text_[at_++];
            switch (escape) {
            case '"':
                out += '"';
                break;
            case '\\':
                out += '\\';
                break;
            case '/':
                out += '/';
                break;
            case 'b':
                out += '\b';
                break;
            case 'f':
                out += '\f';
                break;
            case 'n':
                out += '\n';
                break;
            case 'r':
                out += '\r';
                break;
            case 't':
                out += '\t';
                break;
            case 'u': {
                const std::uint32_t unit = readUnit();
                // A high surrogate followed by a `\u` low one is one character; one on its
                // own is no character, and reads as U+FFFD as the other backends read it.
                if (unit >= 0xD800 && unit <= 0xDFFF) {
                    std::uint32_t low = 0;
                    if (readLowSurrogate(unit, low)) {
                        appendUtf8(out, 0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00));
                    } else {
                        appendUtf8(out, 0xFFFD);
                    }
                } else {
                    appendUtf8(out, unit);
                }
                break;
            }
            default:
                throw malformed(std::string("unknown escape '\\") + escape + "'");
            }
        }
    }

    std::uint32_t readUnit() {
        if (text_.size() - at_ < 4) {
            throw malformed("a \\u escape is short");
        }
        std::uint32_t value = 0;
        for (int i = 0; i < 4; ++i) {
            const char d = text_[at_ + static_cast<std::size_t>(i)];
            std::uint32_t nibble = 0;
            if (d >= '0' && d <= '9') {
                nibble = static_cast<std::uint32_t>(d - '0');
            } else if (d >= 'a' && d <= 'f') {
                nibble = static_cast<std::uint32_t>(d - 'a' + 10);
            } else if (d >= 'A' && d <= 'F') {
                nibble = static_cast<std::uint32_t>(d - 'A' + 10);
            } else {
                throw malformed("a \\u escape is not hex");
            }
            value = value * 16 + nibble;
        }
        at_ += 4;
        return value;
    }

    /// The `\u` low surrogate that completes `high`, consumed only when it does.
    bool readLowSurrogate(std::uint32_t high, std::uint32_t &low) {
        if (high >= 0xDC00 || text_.substr(at_, 2) != "\\u") {
            return false;
        }
        const std::size_t saved = at_;
        at_ += 2;
        try {
            low = readUnit();
        } catch (const JsonError &) {
            at_ = saved;
            return false;
        }
        if (low < 0xDC00 || low > 0xDFFF) {
            at_ = saved;
            return false;
        }
        return true;
    }

    JsonValue readNumber() {
        const std::size_t start = at_;
        const auto digits = [this] {
            while (!atEnd() && text_[at_] >= '0' && text_[at_] <= '9') {
                ++at_;
            }
        };
        if (text_[at_] == '-') {
            ++at_;
        }
        digits();
        if (!atEnd() && text_[at_] == '.') {
            ++at_;
            digits();
        }
        if (!atEnd() && (text_[at_] == 'e' || text_[at_] == 'E')) {
            ++at_;
            if (!atEnd() && (text_[at_] == '+' || text_[at_] == '-')) {
                ++at_;
            }
            digits();
        }
        std::string number(text_.substr(start, at_ - start));
        if (number.empty() || number == "-") {
            throw malformed("a number has no digits");
        }
        return JsonValue(JsonNumber{std::move(number)});
    }

    void readKeyword(std::string_view word) {
        if (text_.substr(at_, word.size()) != word) {
            throw malformed("expected '" + std::string(word) + "'");
        }
        at_ += word.size();
    }

    std::string_view text_;
    std::size_t at_ = 0;
};

inline void writeInto(const JsonValue &value, std::string &out) {
    if (value.isNull()) {
        out += "null";
    } else if (value.isBool()) {
        out += value.asBool() ? "true" : "false";
    } else if (value.isNumber()) {
        out += value.asNumber().text;
    } else if (value.isString()) {
        out += '"';
        JsonText::appendEscaped(out, value.asString());
        out += '"';
    } else if (value.isArray()) {
        out += '[';
        bool first = true;
        for (const JsonValue &item : value.asArray()) {
            if (!first) {
                out += ',';
            }
            first = false;
            writeInto(item, out);
        }
        out += ']';
    } else {
        const JsonObject &object = value.asObject();
        out += '{';
        for (std::size_t at = 0; at < object.size(); ++at) {
            if (at != 0) {
                out += ',';
            }
            out += '"';
            JsonText::appendEscaped(out, object.keys()[at]);
            out += "\":";
            writeInto(object.valueAt(at), out);
        }
        out += '}';
    }
}

}  // namespace detail

/// Reads `text` as exactly one JSON value, surrounding whitespace aside. Throws
/// JsonError, in the words every backend uses, when it is not one.
inline JsonValue parseJson(std::string_view text) {
    detail::Reader reader(text);
    JsonValue value = reader.readValue();
    reader.skipWhitespace();
    if (!reader.atEnd()) {
        throw JsonError("more than one JSON value");
    }
    return value;
}

/// `value` as compact JSON text. A number is written in the spelling it carries, so a
/// value parseJson read is written back unchanged.
inline std::string writeJson(const JsonValue &value) {
    std::string out;
    detail::writeInto(value, out);
    return out;
}

}  // namespace SCE::Saved
