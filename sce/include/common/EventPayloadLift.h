// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#pragma once

#include "common/JsonText.h"
#include "common/Latin1Bytes.h"
#include "core/LogMacros.h"

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstdio>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

namespace SCE::Common {

/**
 * @brief The class of a refusal: the enum `sce:std/lift/refusal_code.scxml` declares, each
 * code at the number that document gives it (docs/adr/0087, section 2.3)
 */
enum class RefusalCode : uint8_t {
    NotJson = 1,
    NotObject = 2,
    TooDeep = 3,
    Missing = 4,
    NullNotOptional = 5,
    WrongKind = 6,
    BadNumber = 7,
    OutOfRange = 8,
    TooLong = 9,
    TooMany = 10,
    UnknownVariant = 11,
    NotByteText = 12,
    NulInText = 13,
    /** A list short of the lower count its type states (docs/adr/0086, section 15) */
    TooFew = 14,
};

/**
 * @brief Why an event's data cannot be read as its schema's fields, or no refusal at all
 *
 * The sentence is what the delivery's `error.execution` carries. Beside it the refusal
 * carries what a statechart reads as data (docs/adr/0087, section 2.4): its code, the
 * place as a strict RFC 6901 pointer (`/items/2/tags/0`, `~` written `~0` and `/` written
 * `~1`, empty for the whole text), and the bound the value broke, for a text, a byte
 * string or a list past it. A default-constructed refusal is none: `empty()`.
 *
 * It reads as its sentence wherever a sentence is asked for, so a generated lift that
 * reports a refusal as the text of `error.execution` takes it as one.
 */
class EventPayloadRefusal {
public:
    EventPayloadRefusal() = default;

    EventPayloadRefusal(RefusalCode code, std::string pointer, std::string reason,
                        std::optional<uint32_t> bound = std::nullopt)
        : reason_(std::move(reason)), code_(code), pointer_(std::move(pointer)), bound_(bound) {}

    /** @brief Whether this is no refusal: the read it answers filled what it read into */
    bool empty() const {
        return reason_.empty();
    }

    /** @brief The sentence the delivery's `error.execution` carries */
    const std::string &reason() const {
        return reason_;
    }

    /** @brief The class of the refusal */
    RefusalCode code() const {
        return code_;
    }

    /** @brief The place refused, as a strict RFC 6901 pointer: empty for the whole text */
    const std::string &pointer() const {
        return pointer_;
    }

    /** @brief The bound the value broke; empty for a refusal that names none */
    std::optional<uint32_t> bound() const {
        return bound_;
    }

    /** @brief The refusal as its sentence, which is what `error.execution` carries */
    operator const std::string &() const {
        return reason_;
    }

    /** @brief Whether the refusal says `sentence` */
    friend bool operator==(const EventPayloadRefusal &refusal, const std::string &sentence) {
        return refusal.reason_ == sentence;
    }

    friend bool operator!=(const EventPayloadRefusal &refusal, const std::string &sentence) {
        return refusal.reason_ != sentence;
    }

    friend bool operator==(const EventPayloadRefusal &refusal, const char *sentence) {
        return refusal.reason_ == sentence;
    }

    friend bool operator!=(const EventPayloadRefusal &refusal, const char *sentence) {
        return refusal.reason_ != sentence;
    }

    /** @brief The name the enum declares `code` by (`notJson`); empty for a number it does not declare */
    static const char *codeName(RefusalCode code) {
        static const char *const names[] = {
            "",          "notJson",    "notObject", "tooDeep", "missing",        "nullNotOptional", "wrongKind",
            "badNumber", "outOfRange", "tooLong",   "tooMany", "unknownVariant", "notByteText",     "nulInText",
            "tooFew"};
        const auto number = static_cast<size_t>(code);
        return number < sizeof(names) / sizeof(names[0]) ? names[number] : "";
    }

    /**
     * @brief `token` below the strict RFC 6901 pointer `prefix` (empty for the whole text):
     * `/` and the token with `~` written `~0` and `/` written `~1` (RFC 6901, section 3)
     */
    static std::string pointerBelow(const std::string &prefix, const std::string &token) {
        std::string out = prefix + "/";
        for (const char c : token) {
            if (c == '~') {
                out += "~0";
            } else if (c == '/') {
                out += "~1";
            } else {
                out += c;
            }
        }
        return out;
    }

private:
    std::string reason_;
    RefusalCode code_ = RefusalCode::NotJson;
    std::string pointer_;
    std::optional<uint32_t> bound_;
};

namespace detail {

/**
 * @brief The walk over a payload's text that every reader below shares
 *
 * One class of static functions rather than free ones, so the header's public
 * surface (embed/MANIFEST.json) names one scanner and not each of its steps.
 */
class EventPayloadScan {
public:
    /**
     * Why a list of `held` elements is no value of a type whose lower count is `least`
     * (docs/adr/0086, section 15), in the words the reader and the raise's check both use.
     */
    static std::string countShort(size_t held, size_t least) {
        return "holds " + std::to_string(held) + " elements, fewer than the " + std::to_string(least) +
               " its type declares";
    }

    static const char *skipSpace(const char *p) {
        while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n') {
            p++;
        }
        return p;
    }

    /** Past one JSON string, from its opening quote. nullptr when unclosed. */
    static const char *skipString(const char *p) {
        if (*p != '"') {
            return nullptr;
        }
        p++;
        while (*p != '\0') {
            if (*p == '\\') {
                if (p[1] == '\0') {
                    return nullptr;
                }
                p += 2;
                continue;
            }
            if (*p == '"') {
                return p + 1;
            }
            p++;
        }
        return nullptr;
    }

    /**
     * Past one object or array, counting nesting. A payload may carry fields
     * this document's schema does not name, and a nested one still has to be
     * walked past to reach the fields that follow.
     */
    static const char *skipContainer(const char *p) {
        const char open = *p;
        const char close = (open == '{') ? '}' : ']';
        int depth = 0;
        while (*p != '\0') {
            if (*p == '"') {
                const char *after = skipString(p);
                if (after == nullptr) {
                    return nullptr;
                }
                p = after;
                continue;
            }
            if (*p == open) {
                depth++;
            } else if (*p == close) {
                depth--;
                if (depth == 0) {
                    return p + 1;
                }
            }
            p++;
        }
        return nullptr;
    }

    /** Past one value: nullptr when a string or a container never closes. */
    static const char *skipValue(const char *p) {
        if (*p == '"') {
            return skipString(p);
        }
        if (*p == '{' || *p == '[') {
            return skipContainer(p);
        }
        // A number, or one of the three keywords. Either way it ends where the next
        // member does.
        while (*p != '\0' && *p != ',' && *p != '}' && *p != ']') {
            p++;
        }
        return p;
    }

    static const char *valueEnd(const char *p) {
        const char *after = skipValue(p);
        return after == nullptr ? p : after;
    }

    /** Whether the JSON string at `p` spells exactly `name`. */
    static bool stringIs(const char *p, const char *name) {
        // A name is the text it spells: a sender may write any character of a name
        // as an escape, and every runtime reads the name the escapes spell
        // (tests/json_text/payload_refusal.json).
        std::string spelled;
        return walkText(p, spelled).empty() && spelled == name;
    }

    /** The value `name` carries in the object at `object`, or nullptr when it names none. */
    static const char *findIn(const char *object, const char *name) {
        if (object == nullptr) {
            return nullptr;
        }
        const char *p = skipSpace(object + 1);  // past '{'
        while (*p != '\0' && *p != '}') {
            if (*p != '"') {
                return nullptr;
            }
            const bool matched = stringIs(p, name);
            const char *afterKey = skipString(p);
            if (afterKey == nullptr) {
                return nullptr;
            }
            p = skipSpace(afterKey);
            if (*p != ':') {
                return nullptr;
            }
            p = skipSpace(p + 1);
            if (matched) {
                return p;
            }
            const char *afterValue = valueEnd(p);
            p = skipSpace(afterValue);
            if (*p == ',') {
                p = skipSpace(p + 1);
                continue;
            }
            break;
        }
        return nullptr;
    }

    /** Four hex digits at `digits`, as the number they spell; false at the first that is not one. */
    static bool readHex4(const char *digits, unsigned &code) {
        code = 0;
        for (int i = 0; i < 4; i++) {
            const char c = digits[i];
            unsigned digit;
            if (c >= '0' && c <= '9') {
                digit = static_cast<unsigned>(c - '0');
            } else if (c >= 'a' && c <= 'f') {
                digit = static_cast<unsigned>(c - 'a' + 10);
            } else if (c >= 'A' && c <= 'F') {
                digit = static_cast<unsigned>(c - 'A' + 10);
            } else {
                return false;
            }
            code = (code << 4) | digit;
        }
        return true;
    }

    /** The UTF-8 of the character at code point `code`, appended to `out`. */
    static void appendUtf8(std::string &out, unsigned code) {
        if (code < 0x80u) {
            out += static_cast<char>(code);
        } else if (code < 0x800u) {
            out += static_cast<char>(0xC0u | (code >> 6));
            out += static_cast<char>(0x80u | (code & 0x3Fu));
        } else if (code < 0x10000u) {
            out += static_cast<char>(0xE0u | (code >> 12));
            out += static_cast<char>(0x80u | ((code >> 6) & 0x3Fu));
            out += static_cast<char>(0x80u | (code & 0x3Fu));
        } else {
            out += static_cast<char>(0xF0u | (code >> 18));
            out += static_cast<char>(0x80u | ((code >> 12) & 0x3Fu));
            out += static_cast<char>(0x80u | ((code >> 6) & 0x3Fu));
            out += static_cast<char>(0x80u | (code & 0x3Fu));
        }
    }

    /** One JSON string's characters, unescaped, as UTF-8. */
    static std::string walkText(const char *p, std::string &out) {
        if (*p != '"') {
            return "is not a text";
        }
        p++;
        while (*p != '\0' && *p != '"') {
            if (*p == '\\') {
                p++;
                switch (*p) {
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
                    unsigned code = 0;
                    if (!readHex4(p + 1, code)) {
                        return "is not a text (a \\u escape is not hex)";
                    }
                    p += 4;
                    if (code >= 0xD800u && code <= 0xDBFFu) {
                        // The first half of a character past U+FFFF, whose second
                        // half is the escape that follows.
                        unsigned low = 0;
                        if (p[1] != '\\' || p[2] != 'u' || !readHex4(p + 3, low) || low < 0xDC00u || low > 0xDFFFu) {
                            return "is not a text (a \\u escape is half of a character)";
                        }
                        code = 0x10000u + ((code - 0xD800u) << 10) + (low - 0xDC00u);
                        p += 6;
                    } else if (code >= 0xDC00u && code <= 0xDFFFu) {
                        return "is not a text (a \\u escape is half of a character)";
                    }
                    appendUtf8(out, code);
                    break;
                }
                default:
                    return "is not a text (unknown escape)";
                }
                p++;
                continue;
            }
            out += *p;
            p++;
        }
        if (*p != '"') {
            return "is not a text (it ends unclosed)";
        }
        return {};
    }

    /** A whole number's sign and magnitude, or why it is neither, its class in `code`. */
    static std::string readWhole(const char *p, bool &negative, uint64_t &magnitude, RefusalCode &code) {
        if (*p != '-' && (*p < '0' || *p > '9')) {
            code = RefusalCode::WrongKind;
            return "is not a number";
        }
        negative = (*p == '-');
        const char *digits = negative ? p + 1 : p;
        if (*digits < '0' || *digits > '9') {
            code = RefusalCode::BadNumber;
            return "is not a number";
        }
        uint64_t value = 0;
        const char *d = digits;
        for (; *d >= '0' && *d <= '9'; d++) {
            const uint64_t digit = static_cast<uint64_t>(*d - '0');
            if (value > (std::numeric_limits<uint64_t>::max() - digit) / 10u) {
                code = RefusalCode::OutOfRange;
                return "does not fit the width its schema declares";
            }
            value = value * 10u + digit;
        }
        // A fraction or an exponent is a different value, and rounding it would
        // answer a guard for something nobody sent.
        if (*d == '.' || *d == 'e' || *d == 'E') {
            code = RefusalCode::BadNumber;
            return "is not a whole number";
        }
        magnitude = value;
        return {};
    }

    /**
     * The digits of a 64-bit field written as a text, from just past its opening quote: whole, canonical,
     * closed; or why not, its class in `code`.
     */
    static std::string readWholeText(const char *p, bool isSigned, bool &negative, uint64_t &magnitude,
                                     RefusalCode &code) {
        negative = (*p == '-');
        const char *digits = negative ? p + 1 : p;
        if (negative && !isSigned) {
            code = RefusalCode::BadNumber;
            return "is not a whole number written as digits";
        }
        const char *d = digits;
        uint64_t value = 0;
        for (; *d >= '0' && *d <= '9'; d++) {
            const uint64_t digit = static_cast<uint64_t>(*d - '0');
            if (value > (std::numeric_limits<uint64_t>::max() - digit) / 10u) {
                code = RefusalCode::OutOfRange;
                return "does not fit the width its schema declares";
            }
            value = value * 10u + digit;
        }
        const bool canonical =
            d != digits && *d == '"' && (*digits != '0' || d == digits + 1) && !(negative && value == 0);
        if (!canonical) {
            code = RefusalCode::BadNumber;
            return "is not a whole number written as digits";
        }
        magnitude = value;
        return {};
    }

    /**
     * Whether a magnitude and sign fit `T`, writing it to `out` when they do; the clause why not, its class
     * in `code`: below zero is no spelling of an unsigned number, and anything else outside is past the width.
     */
    template <typename T> static std::string fit(bool negative, uint64_t magnitude, T &out, RefusalCode &code) {
        if (negative) {
            if constexpr (std::is_signed_v<T>) {
                // -(min) does not fit T when min is its floor, so the check is
                // made on the magnitude instead.
                const uint64_t floorMagnitude =
                    static_cast<uint64_t>(-(static_cast<int64_t>(std::numeric_limits<T>::min()) + 1)) + 1u;
                if (magnitude > floorMagnitude) {
                    code = RefusalCode::OutOfRange;
                    return "does not fit the width its schema declares";
                }
                out = (magnitude == floorMagnitude) ? std::numeric_limits<T>::min()
                                                    : static_cast<T>(-static_cast<int64_t>(magnitude));
                return {};
            } else {
                code = RefusalCode::BadNumber;
                return "does not fit the width its schema declares";
            }
        }
        if (magnitude > static_cast<uint64_t>(std::numeric_limits<T>::max())) {
            code = RefusalCode::OutOfRange;
            return "does not fit the width its schema declares";
        }
        out = static_cast<T>(magnitude);
        return {};
    }

    /** `segment` below `prefix`, as a JSON Pointer's reference tokens are joined. */
    static std::string join(const std::string &prefix, const std::string &segment) {
        return prefix.empty() ? segment : prefix + "/" + segment;
    }

    /**
     * Past one JSON number at `p` in RFC 8259's grammar (section 6): `-`, then `0` or a digit that is not
     * `0` and more digits, then a fraction and an exponent, each with at least one digit. nullptr when `p`
     * spells none.
     */
    static const char *skipNumber(const char *p) {
        if (*p == '-') {
            p++;
        }
        if (*p == '0') {
            p++;
        } else if (*p >= '1' && *p <= '9') {
            while (*p >= '0' && *p <= '9') {
                p++;
            }
        } else {
            return nullptr;
        }
        if (*p == '.') {
            p++;
            if (*p < '0' || *p > '9') {
                return nullptr;
            }
            while (*p >= '0' && *p <= '9') {
                p++;
            }
        }
        if (*p == 'e' || *p == 'E') {
            p++;
            if (*p == '+' || *p == '-') {
                p++;
            }
            if (*p < '0' || *p > '9') {
                return nullptr;
            }
            while (*p >= '0' && *p <= '9') {
                p++;
            }
        }
        return p;
    }

    /**
     * Past one JSON string at `p` in RFC 8259's grammar (sections 7 and 8.2): closed, no character below
     * U+0020 standing unescaped, every escape one of the eight short ones or `\u` and four hex digits, and
     * a `\u` escape of the first half of a UTF-16 pair followed at once by one of the second half, a second
     * half never alone. nullptr when `p` spells none: a half of a character names no character, so the text
     * is no JSON text every runtime reads alike (tests/json_text/payload_refusal.json).
     */
    static const char *skipWellFormedString(const char *p) {
        if (*p != '"') {
            return nullptr;
        }
        p++;
        while (*p != '"') {
            if (*p == '\0' || static_cast<unsigned char>(*p) < 0x20u) {
                return nullptr;
            }
            if (*p == '\\') {
                p++;
                unsigned code = 0;
                if (*p == 'u') {
                    if (!readHex4(p + 1, code)) {
                        return nullptr;
                    }
                    p += 4;
                    if (code >= 0xD800u && code <= 0xDBFFu) {
                        unsigned low = 0;
                        if (p[1] != '\\' || p[2] != 'u' || !readHex4(p + 3, low) || low < 0xDC00u || low > 0xDFFFu) {
                            return nullptr;
                        }
                        p += 6;
                    } else if (code >= 0xDC00u && code <= 0xDFFFu) {
                        return nullptr;
                    }
                } else if (std::string_view("\"\\/bfnrt").find(*p) == std::string_view::npos || *p == '\0') {
                    return nullptr;
                }
            }
            p++;
        }
        return p + 1;
    }

    /**
     * The name at `p`, a JSON string, unescaped into `name`, and `p` moved past the `:` that follows it; or
     * why it is not one. A name holding half of a character is compared as it is spelled.
     */
    static std::string readMemberName(const char *&p, std::string &name) {
        const char *after = skipWellFormedString(p);
        if (after == nullptr) {
            return "a field name was expected";
        }
        if (!walkText(p, name).empty()) {
            name.assign(p + 1, after - 1);
        }
        p = skipSpace(after);
        if (*p != ':') {
            return "a ':' was expected";
        }
        p = skipSpace(p + 1);
        return {};
    }

    /**
     * Why the text at `text` is not exactly one JSON value (RFC 8259) at every depth, or empty when it is;
     * `trailing` is set when it is one JSON value and more.
     *
     * An object whose names repeat has no single meaning: a reader that takes the first and one that takes
     * the last would read two values from one text, so it is refused, at any depth and under a key no schema
     * declares too, as every runtime's reader refuses it (docs/adr/0087, section 2.3). Names are compared as
     * the texts they spell, so an escape is the character it names. The containers open are kept on a stack,
     * so the walk takes no call per level.
     */
    static std::string checkText(const char *text, bool &trailing) {
        trailing = false;
        std::vector<char> open;
        std::vector<std::vector<std::string>> names;
        const char *p = skipSpace(text);
        for (;;) {
            // A value is expected at `p`.
            if (*p == '{' || *p == '[') {
                const char kind = *p;
                p = skipSpace(p + 1);
                if (*p == (kind == '{' ? '}' : ']')) {
                    p++;
                } else {
                    open.push_back(kind);
                    if (kind == '{') {
                        names.emplace_back();
                        std::string name;
                        const std::string refusal = readMemberName(p, name);
                        if (!refusal.empty()) {
                            return refusal;
                        }
                        names.back().push_back(std::move(name));
                    }
                    continue;
                }
            } else if (*p == '"') {
                const char *after = skipWellFormedString(p);
                if (after == nullptr) {
                    return "a text is not closed, or holds an escape JSON does not have";
                }
                p = after;
            } else if (std::string_view(p).substr(0, 4) == "true" || std::string_view(p).substr(0, 4) == "null") {
                p += 4;
            } else if (std::string_view(p).substr(0, 5) == "false") {
                p += 5;
            } else {
                const char *after = skipNumber(p);
                if (after == nullptr) {
                    return *p == '\0' ? "it ends where a value was expected" : "a value was expected";
                }
                p = after;
            }
            // A value ended at `p`: close what it ends, or go on to the next member.
            for (;;) {
                p = skipSpace(p);
                if (open.empty()) {
                    trailing = *p != '\0';
                    return trailing ? "it carries more than one JSON value" : std::string{};
                }
                const char kind = open.back();
                if (*p == ',') {
                    p = skipSpace(p + 1);
                    if (kind == '{') {
                        std::string name;
                        const std::string refusal = readMemberName(p, name);
                        if (!refusal.empty()) {
                            return refusal;
                        }
                        std::vector<std::string> &seen = names.back();
                        if (std::find(seen.begin(), seen.end(), name) != seen.end()) {
                            return "the field \"" + name + "\" appears twice";
                        }
                        seen.push_back(std::move(name));
                    }
                    break;
                }
                if (*p == (kind == '{' ? '}' : ']')) {
                    p++;
                    if (kind == '{') {
                        names.pop_back();
                    }
                    open.pop_back();
                    continue;
                }
                return kind == '{' ? "a ',' or '}' was expected" : "a ',' or ']' was expected";
            }
        }
    }
};

}  // namespace detail

class EventPayloadFields;

/**
 * @brief One value of an event's data, at the path through every depth that names it
 *
 * The path is a field's name, then each index and field name below it, joined by
 * `/`; the pointer is the strict RFC 6901 pointer to the same place, which a refusal
 * carries beside its sentence. Every reader answers an empty refusal when it filled
 * `out`, else why it did not, the path first; `out` is then as it was. Nothing is
 * copied but what a reader hands back: the value borrows the characters of the event
 * record, which outlives it while the event is being dispatched.
 */
class EventPayloadValue {
public:
    EventPayloadValue() = default;

    EventPayloadValue(const char *at, std::string path, std::string pointer)
        : at_(at), path_(std::move(path)), pointer_(std::move(pointer)) {}

    /** @brief The path that names this value */
    const std::string &path() const {
        return path_;
    }

    /** @brief The strict RFC 6901 pointer to this value (`/items/2`) */
    const std::string &pointer() const {
        return pointer_;
    }

    /** @brief Whether the value is `null`, the one spelling of an absent value */
    bool isNull() const {
        return at_ != nullptr && std::string_view(at_).substr(0, 4) == "null" && !isWordCharacter(at_[4]);
    }

    /** @brief A refusal of this value, of class `code`, saying `what` of it after its path */
    EventPayloadRefusal refuse(RefusalCode code, const std::string &what,
                               std::optional<uint32_t> bound = std::nullopt) const {
        return EventPayloadRefusal(code, pointer_, "'" + path_ + "' " + what, bound);
    }

    /** @brief The refusal of the text `name` this value carries, which the enum `alias` does not declare */
    EventPayloadRefusal unknownVariant(const std::string &name, const std::string &alias) const {
        return refuse(RefusalCode::UnknownVariant, "(" + name + ") is not a variant of " + alias);
    }

    EventPayloadRefusal readBool(bool &out) const {
        if (std::string_view(at_).substr(0, 4) == "true" && !isWordCharacter(at_[4])) {
            out = true;
            return {};
        }
        if (std::string_view(at_).substr(0, 5) == "false" && !isWordCharacter(at_[5])) {
            out = false;
            return {};
        }
        return wrongKind("is not a truth value");
    }

    /**
     * @brief The text the value carries, held to no bound: what an enum's variant is
     * named by, which its reader matches against the names its enum declares
     *
     * A text holding U+0000 is refused: a runtime that holds a text NUL-terminated
     * would end it there, so no runtime holds one, and every reader refuses the same
     * texts (docs/adr/0087, section 2.3). A byte string may hold the byte zero.
     */
    EventPayloadRefusal readName(std::string &out) const {
        std::string text;
        EventPayloadRefusal refusal = readCharacters(text);
        if (!refusal.empty()) {
            return refusal;
        }
        if (text.find('\0') != std::string::npos) {
            return refuse(RefusalCode::NulInText, "carries U+0000, which no text holds");
        }
        out = std::move(text);
        return {};
    }

    /**
     * @brief A text of at most `bound` UTF-8 bytes (docs/adr/0086, section 4.4): one
     * past the bound is refused, at whatever depth it stands, and never cut
     */
    EventPayloadRefusal readText(size_t bound, std::string &out) const {
        std::string text;
        EventPayloadRefusal refusal = readName(text);
        if (!refusal.empty()) {
            return refusal;
        }
        if (text.size() > bound) {
            return pastBytes(text.size(), bound);
        }
        out = std::move(text);
        return {};
    }

    /**
     * @brief A byte string of at most `bound` bytes
     *
     * JSON has no byte string, so the wire carries the byte-exact Latin-1 text
     * the inject seam writes, and this reads it back the same way: every one of
     * the 256 values is one character and back (`Latin1Bytes`). A byte above 0x7F
     * is therefore the two bytes of its character in the text, and is one byte
     * here; a character past U+00FF is no byte and is refused. Printable ASCII —
     * what a bytes guard compares — is the same bytes under either reading.
     */
    EventPayloadRefusal readBytes(size_t bound, std::vector<uint8_t> &out) const {
        if (*at_ != '"') {
            return wrongKind("is not a byte string");
        }
        std::string text;
        EventPayloadRefusal refusal = readCharacters(text);
        if (!refusal.empty()) {
            return refusal;
        }
        std::vector<uint8_t> bytes;
        const std::string notBytes = Latin1Bytes::bytesOf(text, bytes);
        if (!notBytes.empty()) {
            // `Latin1Bytes` refuses a character past U+00FF, which is no byte, and a
            // text that is not UTF-8, which is no JSON text (RFC 8259, section 8.1).
            const bool pastOneByte = notBytes.rfind("carries a character above U+00FF", 0) == 0;
            return refuse(pastOneByte ? RefusalCode::NotByteText : RefusalCode::NotJson, notBytes);
        }
        if (bytes.size() > bound) {
            return pastBytes(bytes.size(), bound);
        }
        out = std::move(bytes);
        return {};
    }

    /**
     * @brief A whole number, at the width its type declares
     *
     * ⚠ A truth value is not a number here. JSON spells both, and a schema that
     * declared `uint32` and received `true` has been handed something its own
     * type says cannot occur. A value the width cannot hold is refused rather
     * than wrapped — silently truncating is how a guard on a boundary answers
     * for a value the document never received.
     */
    template <typename T> EventPayloadRefusal readInteger(T &out) const {
        static_assert(std::is_integral_v<T>, "readInteger is for whole-number values");
        if (isNull()) {
            return wrongKind("is not a number");
        }
        bool negative = false;
        uint64_t magnitude = 0;
        RefusalCode code = RefusalCode::BadNumber;
        std::string refusal = detail::EventPayloadScan::readWhole(at_, negative, magnitude, code);
        if (refusal.empty() && negative && magnitude == 0u) {
            // `-0` is a JSON number and no spelling of a whole number: every runtime refuses it, as it
            // refuses `"-0"` for a 64-bit integer.
            code = RefusalCode::BadNumber;
            refusal = "is not a whole number (-0)";
        }
        if (refusal.empty()) {
            refusal = detail::EventPayloadScan::fit(negative, magnitude, out, code);
        }
        return refusal.empty() ? EventPayloadRefusal{} : refuse(code, refusal);
    }

    /**
     * @brief A 64-bit whole number
     *
     * A 64-bit integer crosses as the text of its digits, because a reader that takes
     * a JSON number through a double carries one past 2^53 with its low bits wrong
     * and no error. It is read from that text, or from a number exact in a double
     * (`|n| <= 2^53`); a number past that is refused here on every engine, so the
     * same payload gives the same field wherever it is read.
     */
    template <typename T> EventPayloadRefusal readWide(T &out) const {
        static_assert(std::is_integral_v<T> && sizeof(T) == 8, "readWide is for 64-bit whole-number values");
        if (isNull()) {
            return wrongKind("is not a number");
        }
        bool negative = false;
        uint64_t magnitude = 0;
        RefusalCode code = RefusalCode::BadNumber;
        std::string refusal;
        if (*at_ == '"') {
            refusal = detail::EventPayloadScan::readWholeText(at_ + 1, std::is_signed_v<T>, negative, magnitude, code);
        } else {
            refusal = detail::EventPayloadScan::readWhole(at_, negative, magnitude, code);
            if (refusal.empty() && negative && magnitude == 0u) {
                // `-0` is no whole number, as `"-0"` is not.
                code = RefusalCode::BadNumber;
                refusal = "is not a whole number (-0)";
            } else if (refusal.empty() && magnitude > (uint64_t{1} << 53) - 1u) {
                code = RefusalCode::BadNumber;
                refusal = "is a number past 2^53 - 1, which is not exact everywhere; a 64-bit integer that large is "
                          "written as a text of its digits";
            } else if (code == RefusalCode::OutOfRange) {
                // A 64-bit integer as a number is held to 2^53 - 1 however large the
                // number is: one past every width is no spelling of it either.
                code = RefusalCode::BadNumber;
            }
        }
        if (refusal.empty()) {
            refusal = detail::EventPayloadScan::fit(negative, magnitude, out, code);
        }
        return refusal.empty() ? EventPayloadRefusal{} : refuse(code, refusal);
    }

    /** @brief A fractional value, at the width its type declares */
    template <typename T> EventPayloadRefusal readFloating(T &out) const {
        static_assert(std::is_floating_point_v<T>, "readFloating is for fractional values");
        if (*at_ != '-' && (*at_ < '0' || *at_ > '9')) {
            return wrongKind("is not a number");
        }
        double wide = 0.0;
        try {
            wide = std::stod(std::string(at_, detail::EventPayloadScan::valueEnd(at_) - at_));
        } catch (const std::out_of_range &) {
            // A number past every double's range is past every real width.
            return refuse(RefusalCode::OutOfRange, "is not a number this width can hold");
        } catch (const std::exception &) {
            return refuse(RefusalCode::BadNumber, "is not a number this width can hold");
        }
        // A JSON number is a double wherever it is read, so a float holds the single
        // nearest that double, and a number past the largest single does not fit,
        // as a whole number past its width does not. Narrowing it anyway would be
        // undefined where the double is out of range.
        if (std::fabs(wide) > static_cast<double>(std::numeric_limits<T>::max())) {
            return refuse(RefusalCode::OutOfRange, "does not fit the width its schema declares");
        }
        out = static_cast<T>(wide);
        return {};
    }

    /**
     * @brief A value of `optional<T>` (docs/adr/0055): absent where the value is
     * `null`, and otherwise what `read(value, present)` reads, held to `T`
     */
    template <typename T, typename Read> EventPayloadRefusal readOptional(std::optional<T> &out, Read read) const {
        if (isNull()) {
            out.reset();
            return {};
        }
        T present{};
        EventPayloadRefusal refusal = read(*this, present);
        if (refusal.empty()) {
            out = std::move(present);
        }
        return refusal;
    }

    /**
     * @brief A value of `list<T>[bound]`: an array of at most `bound` elements, each
     * read by `read(element, into)` at the path of its index, in order
     *
     * The elements are counted before any is read: an array longer than `bound` is
     * refused whole, and no element of it is decoded (ADR 0077, section 2).
     */
    template <typename T, typename Read>
    EventPayloadRefusal readList(size_t bound, std::vector<T> &out, Read read) const {
        return readListWithin(0, bound, out, read);
    }

    /**
     * @brief A value of `list<T>[least..bound]`: an array of at least `least` and at most
     * `bound` elements (docs/adr/0086, section 15), each read by `read` at its index
     *
     * Both counts are held before any element is read: an array longer than `bound` is
     * refused as `TooMany`, one shorter than `least` as `TooFew` naming `least`, and no
     * element of either is decoded.
     */
    template <typename T, typename Read>
    EventPayloadRefusal readListWithin(size_t least, size_t bound, std::vector<T> &out, Read read) const {
        if (*at_ != '[') {
            return wrongKind("is not a list");
        }
        std::vector<const char *> elements;
        const char *p = detail::EventPayloadScan::skipSpace(at_ + 1);
        if (*p != ']') {
            for (;;) {
                elements.push_back(p);
                const char *after = detail::EventPayloadScan::skipValue(p);
                if (after == nullptr) {
                    return refuse(RefusalCode::NotJson, "is not a list (it ends unclosed)");
                }
                p = detail::EventPayloadScan::skipSpace(after);
                if (*p == ',') {
                    p = detail::EventPayloadScan::skipSpace(p + 1);
                    continue;
                }
                if (*p == ']') {
                    break;
                }
                return refuse(RefusalCode::NotJson, "is not a list");
            }
        }
        if (elements.size() > bound) {
            return refuse(RefusalCode::TooMany,
                          "holds " + std::to_string(elements.size()) + " elements, past the " + std::to_string(bound) +
                              " its type declares",
                          boundOf(bound));
        }
        if (elements.size() < least) {
            return refuse(RefusalCode::TooFew, detail::EventPayloadScan::countShort(elements.size(), least),
                          boundOf(least));
        }
        std::vector<T> items;
        items.reserve(elements.size());
        for (size_t index = 0; index < elements.size(); ++index) {
            T item{};
            const std::string token = std::to_string(index);
            EventPayloadRefusal refusal =
                read(EventPayloadValue(elements[index], detail::EventPayloadScan::join(path_, token),
                                       EventPayloadRefusal::pointerBelow(pointer_, token)),
                     item);
            if (!refusal.empty()) {
                return refusal;
            }
            items.push_back(std::move(item));
        }
        out = std::move(items);
        return {};
    }

    /**
     * @brief A value of a record type: an object whose fields `read(fields)` reads
     *
     * `defaults`, for a record whose schema a later revision added fields to, is the
     * JSON object of their defaults, a literal the generator spells from that schema
     * and which outlives the read, and a field the value lacks is read out of it
     * (docs/adr/0086, section 10.3); nullptr for a schema with none.
     */
    template <typename Read> EventPayloadRefusal readRecord(const char *defaults, Read read) const;

    /**
     * @brief A value of a union type (docs/adr/0092, section 4): one object holding its tag
     * under the key `tag` and the fields of the case the tag names beside it
     *
     * The tag is read first, by `readTag(tagValue, tagOut)`, the reader of the tag's enum
     * `Tag`, wherever it stands in the object; `readCase(tag, object)` is then handed the case
     * the tag named and this same value, and reads the case's record from it with
     * `readRecord` (an empty case reads nothing). The refusals come in one order on every
     * runtime: a value that is not an object; the tag absent (`Missing` at the tag's
     * pointer), then whatever the enum's reader refuses of it (`null`, not a text, a name the
     * enum does not declare); then the case's record. A key the case does not declare is
     * never asked for, so it is stepped over, as a record steps over one.
     */
    template <typename Tag, typename ReadTag, typename ReadCase>
    EventPayloadRefusal readUnion(const char *tag, ReadTag readTag, ReadCase readCase) const;

private:
    /** Whether `c` continues a keyword, so that `nullx` is no `null`. */
    static bool isWordCharacter(char c) {
        return (c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z') || (c >= '0' && c <= '9') || c == '_';
    }

    /** A bound as a refusal names it: one no `uint32` holds is held at the largest. */
    static uint32_t boundOf(size_t bound) {
        return bound > std::numeric_limits<uint32_t>::max() ? std::numeric_limits<uint32_t>::max()
                                                            : static_cast<uint32_t>(bound);
    }

    /** The characters of the JSON string at the value, unescaped, or why it is no text. */
    EventPayloadRefusal readCharacters(std::string &out) const {
        if (*at_ != '"') {
            return wrongKind("is not a text");
        }
        const std::string refusal = detail::EventPayloadScan::walkText(at_, out);
        // A text the escapes of which name no character is no JSON text.
        return refusal.empty() ? EventPayloadRefusal{} : refuse(RefusalCode::NotJson, refusal);
    }

    /** The refusal of `null` where the type is not optional, or `otherwise`. */
    EventPayloadRefusal wrongKind(const char *otherwise) const {
        return isNull() ? refuse(RefusalCode::NullNotOptional, "is null, and its type is not optional")
                        : refuse(RefusalCode::WrongKind, otherwise);
    }

    EventPayloadRefusal pastBytes(size_t held, size_t bound) const {
        return refuse(RefusalCode::TooLong,
                      "is " + std::to_string(held) + " bytes, past the " + std::to_string(bound) + " its type declares",
                      boundOf(bound));
    }

    const char *at_ = nullptr;
    std::string path_;
    std::string pointer_;
};

/**
 * @brief Lifting an event's typed `_event.data` view out of the data it carries
 *
 * NL→IR Item C1 Path A gives a schema'd event a typed payload that the natively
 * lowered guards read. One producer fills it: the generated `raise<Event>`
 * inject seam. Every other producer — `<send>` with `<param>`, namelist or
 * `<content>`, an invoke forwarding an event either way, autoforward,
 * BasicHTTP, mesh — fills `EventWithMetadata::data`, the wire §scxml-5.10
 * describes and §scxml-B-2-8-1 reads.
 *
 * Until this header the two never met, so the same guard answered differently
 * depending on where its event came from, and a typed payload could not cross
 * an invoke boundary at all. What the lift refuses is what the SCRIPT ENGINE
 * already refuses for the same guard, measured on the same document: no data, a
 * missing field, or a value of another type each give `error.execution` and a
 * guard that does not fire (§scxml-3.13). A native lowering that answered
 * differently would make the optimisation observable, which is the one thing it
 * may not be.
 *
 * A field may be of any type of the language (docs/adr/0086): a list of any
 * type, an optional value, a record, nested to any depth. Each is read as one
 * `EventPayloadValue` at the path that names it (`value`), and three combinators
 * walk the constructors that hold other values, each calling the reader of its
 * part: `readList`, `readOptional` and `readRecord`. A value outside its type is
 * refused before anything of it is kept, and the refusal names the path through
 * every depth as a JSON Pointer (RFC 6901): `'items/2/tags/0' is 9 bytes, past
 * the 8 its type declares`. A key a record's schema does not declare is stepped
 * over.
 *
 * ⚠ HEADER-ONLY, and that is a requirement rather than a convenience: a
 * generated C++ machine compiles and LINKS against `sce/include` alone, with no
 * runtime library — `event_schema_bytes_guard.rs` builds one that way and is
 * the gate that says so. The first draft of this header called
 * `EventDataHelper::jsonStringToScriptValue`, which reads the same wire for the
 * script engine, and that one call took the property away: two undefined
 * references at link time for every payload machine. Reuse that cannot be
 * linked is not reuse.
 *
 * ⚠ So the JSON read here is a second reader, and what keeps it from drifting
 * from the engine's is a test rather than a shared symbol: the per-backend
 * `statechart_lifted` gates ask the same five questions of all six backends,
 * and the refusals were measured against the Lua path on the same document.
 *
 * Cross-language siblings: `sce_runtime.event_payload` (Python),
 * `sce.LiftPayload` (Go), `sce/event_payload.h` (C11), whose structure this
 * mirrors.
 */
class EventPayloadFields {
public:
    /**
     * @brief Read an event's data as the fields it names
     * @param data The event's `_event.data` wire
     * @param out Receives the fields when the read succeeds
     * @return An empty refusal when `out` was filled, else why it was not
     *
     * The JSON read is §scxml-B-2-8-1's second rung, the same rung the script
     * engine takes for the same string. The whole text is held to the JSON
     * grammar first, so a text that is not one JSON value, or holds an object
     * that repeats a name at any depth, is refused before any field is read
     * (docs/adr/0087, section 2.3). Nothing is copied: `out` borrows the
     * characters, so the event record must outlive it — which it does, the lift
     * runs while the event is being dispatched.
     */
    static EventPayloadRefusal decode(const std::string &data, EventPayloadFields &out) {
        out.object_ = nullptr;
        out.defaults_ = nullptr;
        out.prefix_.clear();
        out.pointerPrefix_.clear();
        const char *p = detail::EventPayloadScan::skipSpace(data.c_str());
        if (*p == '\0') {
            return EventPayloadRefusal(RefusalCode::NotJson, "", "the event carries no data");
        }
        bool trailing = false;
        const std::string notJson = detail::EventPayloadScan::checkText(p, trailing);
        if (*p != '{') {
            // §scxml-B-2-8-1's other rungs: a bare value or a space-normalized
            // string names no fields. The code tells one JSON value that is not an
            // object from a text that is not JSON at all.
            return EventPayloadRefusal(notJson.empty() ? RefusalCode::NotObject : RefusalCode::NotJson, "",
                                       "the event's data is not an object of named fields");
        }
        if (trailing) {
            return EventPayloadRefusal(RefusalCode::NotJson, "", "the event's data carries more than one JSON value");
        }
        if (!notJson.empty()) {
            return EventPayloadRefusal(RefusalCode::NotJson, "", "the event's data is not JSON (" + notJson + ")");
        }
        out.object_ = p;
        return {};
    }

    /**
     * @brief Read an event's data for a schema a later revision of which added fields
     * @param data The event's `_event.data` wire
     * @param defaults The JSON object of those fields' defaults, which the generator
     *        spells from the schema as a string literal that outlives `out`
     * @param out Receives the fields when the read succeeds
     * @return An empty refusal when `out` was filled, else why it was not
     *
     * A field the delivery lacks is read out of `defaults` instead
     * (docs/SCE_ACCEPTED_SUBSET.md, "An event schema that grows"). A delivery with
     * no data at all lacks every field, so it is read as one that names none: a
     * field of the first revision is then refused by name, and a schema all of
     * whose fields came later reads its defaults. A field the delivery does carry
     * is read as it came, and held to its type as any other value is.
     */
    static EventPayloadRefusal decode(const std::string &data, const char *defaults, EventPayloadFields &out) {
        out.object_ = nullptr;
        out.defaults_ = nullptr;
        out.prefix_.clear();
        out.pointerPrefix_.clear();
        if (*detail::EventPayloadScan::skipSpace(data.c_str()) != '\0') {
            EventPayloadRefusal refusal = decode(data, out);
            if (!refusal.empty()) {
                return refusal;
            }
        }
        return out.takeDefaults(defaults);
    }

    /**
     * @brief Read the text a `<sce:lift>` reads as the fields of a record of a schema
     * (docs/adr/0087, section 2.1)
     * @param text The text read, which must outlive `out`: `out` borrows its characters
     * @param depth The deepest container a value of the schema holds
     * @param pathBound The room of the refusal record's `path`, which a `tooDeep` pointer is
     *        held to (`liftTooDeep`)
     * @param defaults For a schema a later revision of which added fields, the JSON object of
     *        their defaults, a literal that outlives `out`; nullptr for none
     * @param out Receives the fields when the read succeeds
     * @return An empty refusal when `out` was filled, else why it was not
     *
     * Three refusals come before the fields are read, in this order: a text that is empty or
     * only whitespace (`notJson`); a container opened deeper than `depth`, decided over the
     * text's brackets alone before it is read as JSON (`liftTooDeep`, `tooDeep`); and a text
     * that is not one JSON object, as a delivery is refused (`decode`).
     */
    static EventPayloadRefusal lift(const std::string &text, uint32_t depth, uint32_t pathBound, const char *defaults,
                                    EventPayloadFields &out) {
        if (*detail::EventPayloadScan::skipSpace(text.c_str()) == '\0') {
            return EventPayloadRefusal(RefusalCode::NotJson, "", "the text is empty");
        }
        EventPayloadRefusal refusal = liftTooDeep(text, depth, pathBound);
        if (refusal.empty()) {
            refusal = decode(text, out);
        }
        if (refusal.empty() && defaults != nullptr) {
            refusal = out.takeDefaults(defaults);
        }
        return refusal;
    }

    /**
     * @brief The refusal of a container `text` opens deeper than `depth`, the deepest a value of
     * a lift's schema holds (docs/adr/0087, section 2.1, item 4), or no refusal when it opens none
     *
     * Decided over the brackets alone, before the text is read as JSON, so that every runtime
     * decides it alike and no reader descends past the type: a text is skipped from its opening
     * quote to the next quote no backslash escapes, each `{` and `[` outside one opens a level
     * and each `}` and `]` closes one, and the first opened past `depth` is refused, whatever the
     * text holds after it. A text that never closes ends the walk with nothing refused, and the
     * reading of the text as JSON refuses it.
     *
     * The pointer names the container refused (docs/adr/0087, section 2.4): for each container it
     * stands in, outermost first, the element's index in an array (the `,` the scan met at that
     * level since the array opened) or the member's name in an object (the last text closed at
     * that level before a `:`, since the object opened or its last `,`), each an RFC 6901 token.
     * It stops before the first place it cannot write: a member with no name before its colon, a
     * name that is no text (no JSON string, not UTF-8, or holding U+0000), or a token that would
     * take the pointer past `pathBound` UTF-8 bytes, the room of the refusal record's `path`. The
     * bound is `depth`. The scan keeps one entry per open container, at most `depth` of them.
     */
    static EventPayloadRefusal liftTooDeep(const std::string &text, uint32_t depth, uint32_t pathBound) {
        std::vector<LiftScanLevel> open;
        for (size_t at = 0; at < text.size(); ++at) {
            const char c = text[at];
            LiftScanLevel *top = open.empty() ? nullptr : &open.back();
            if (c == '"') {
                const size_t start = at;
                ++at;
                while (at < text.size() && text[at] != '"') {
                    at += text[at] == '\\' ? 2 : 1;
                }
                if (at >= text.size()) {
                    return {};
                }
                if (top != nullptr) {
                    top->lastText = std::string_view(text).substr(start, at + 1 - start);
                }
            } else if (c == ':' && top != nullptr) {
                top->member = top->lastText;
                top->lastText = {};
            } else if (c == ',' && top != nullptr) {
                top->member = {};
                top->lastText = {};
                ++top->index;
            } else if (c == '{' || c == '[') {
                if (open.size() >= depth) {
                    return EventPayloadRefusal(RefusalCode::TooDeep, liftScanPointer(open, pathBound),
                                               "the text opens a container deeper than the " + std::to_string(depth) +
                                                   " levels a value of its schema holds",
                                               depth);
                }
                open.push_back(LiftScanLevel{c == '[', 0, {}, {}});
            } else if ((c == '}' || c == ']') && top != nullptr) {
                open.pop_back();
            }
        }
        return {};
    }

private:
    /** One container `liftTooDeep` stands in: whether it is an array, the elements it has passed,
     * the last text closed in it, and the member a `:` after that text named. */
    struct LiftScanLevel {
        bool array;
        uint64_t index;
        std::string_view lastText;
        std::string_view member;
    };

    /** Whether `bytes` are UTF-8: no stray continuation, no overlong form, no surrogate, nothing
     * past U+10FFFF. */
    static bool isUtf8(std::string_view bytes) {
        size_t at = 0;
        while (at < bytes.size()) {
            const auto lead = static_cast<unsigned char>(bytes[at]);
            size_t more = 0;
            uint32_t code = lead;
            if (lead >= 0xF0u && lead <= 0xF4u) {
                more = 3;
                code = lead & 0x07u;
            } else if (lead >= 0xE0u && lead <= 0xEFu) {
                more = 2;
                code = lead & 0x0Fu;
            } else if (lead >= 0xC2u && lead <= 0xDFu) {
                more = 1;
                code = lead & 0x1Fu;
            } else if (lead >= 0x80u) {
                return false;
            }
            if (more > 0 && at + more >= bytes.size()) {
                return false;
            }
            for (size_t i = 1; i <= more; ++i) {
                const auto next = static_cast<unsigned char>(bytes[at + i]);
                if ((next & 0xC0u) != 0x80u) {
                    return false;
                }
                code = (code << 6) | (next & 0x3Fu);
            }
            if ((more == 2 && (code < 0x800u || (code >= 0xD800u && code <= 0xDFFFu))) ||
                (more == 3 && (code < 0x10000u || code > 0x10FFFFu))) {
                return false;
            }
            at += more + 1;
        }
        return true;
    }

    /** The pointer to the place `open` stands at, as `liftTooDeep` writes it. */
    static std::string liftScanPointer(const std::vector<LiftScanLevel> &open, uint32_t pathBound) {
        std::string pointer;
        for (const LiftScanLevel &level : open) {
            std::string token;
            if (level.array) {
                token = std::to_string(level.index);
            } else {
                const std::string member(level.member);
                if (member.empty() || !isUtf8(member) ||
                    detail::EventPayloadScan::skipWellFormedString(member.c_str()) != member.c_str() + member.size() ||
                    !detail::EventPayloadScan::walkText(member.c_str(), token).empty() ||
                    token.find('\0') != std::string::npos) {
                    break;
                }
            }
            std::string below = EventPayloadRefusal::pointerBelow(pointer, token);
            if (below.size() > pathBound) {
                break;
            }
            pointer = std::move(below);
        }
        return pointer;
    }

public:
    /**
     * @brief What a `<sce:lift>` reads (docs/adr/0087, section 2.1): `text` as a record of the
     * schema named `schema`, whose fields `read(fields, out)` reads, as a delivery of the schema is
     * read; or the refusal of the first place that is not one, its code, pointer and bound the
     * statechart keeps as a record
     *
     * `lift` says what is refused before the fields are read. Each outcome is logged with the
     * schema's name and, for a refusal, its code, pointer and bound; the text itself is not, since
     * nothing bounds what a sender writes in it. `out` is filled only when no refusal is answered.
     */
    template <typename T, typename Read>
    static EventPayloadRefusal liftRecord(const std::string &text, const char *schema, uint32_t depth,
                                          uint32_t pathBound, const char *defaults, T &out, Read read) {
        EventPayloadFields fields;
        EventPayloadRefusal refusal = lift(text, depth, pathBound, defaults, fields);
        T lifted{};
        if (refusal.empty()) {
            refusal = read(static_cast<const EventPayloadFields &>(fields), lifted);
        }
        if (refusal.empty()) {
            SCE_LOG_INFO("<sce:lift>: the text is a record of {}", schema);
            out = std::move(lifted);
        } else {
            SCE_LOG_INFO("<sce:lift>: the text is no record of {}: {} at '{}', bound {} ({})", schema,
                         EventPayloadRefusal::codeName(refusal.code()), refusal.pointer(),
                         refusal.bound().has_value() ? std::to_string(*refusal.bound()) : std::string("none"),
                         refusal.reason());
        }
        return refusal;
    }

    /**
     * @brief The value of the field `name`, at its path
     *
     * The delivered value, else the default a later revision of the schema gave it,
     * else a refusal naming the path. A member the schema does not declare is never
     * asked for, so it is stepped over.
     */
    EventPayloadRefusal value(const char *name, EventPayloadValue &out) const {
        const std::string path = detail::EventPayloadScan::join(prefix_, name);
        std::string pointer = EventPayloadRefusal::pointerBelow(pointerPrefix_, name);
        const char *p = find(name);
        if (p == nullptr) {
            return EventPayloadRefusal(RefusalCode::Missing, std::move(pointer),
                                       "the event's data has no '" + path + "'");
        }
        out = EventPayloadValue(p, path, std::move(pointer));
        return {};
    }

    /**
     * @brief The refusal of the text `variant` the field `name` carries, which the enum
     * `alias` does not declare a variant by: what a generated reader of an enum field
     * answers once it has matched every name the enum declares
     */
    EventPayloadRefusal unknownVariant(const char *name, const std::string &variant, const std::string &alias) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.unknownVariant(variant, alias) : refusal;
    }

    /**
     * @brief Whether an optional field (docs/adr/0055) is absent
     *
     * `null` is the one spelling of absence on the wire, and any other value is a
     * present one, which the reader of the field's type then reads. A delivery
     * that lacks the key is refused: a field of the first revision is carried by
     * every delivery, absent or not, so a missing key is no absent value.
     */
    EventPayloadRefusal readAbsent(const char *name, bool &absent) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        if (refusal.empty()) {
            absent = v.isNull();
        }
        return refusal;
    }

    EventPayloadRefusal readBool(const char *name, bool &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readBool(out) : refusal;
    }

    /** @brief A text field, as the text it carries; `EventPayloadValue::readText` holds it to its bound */
    EventPayloadRefusal readString(const char *name, std::string &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readName(out) : refusal;
    }

    /** @brief A byte-string field, held to no bound; `EventPayloadValue::readBytes` holds it to its own */
    EventPayloadRefusal readBytes(const char *name, std::vector<uint8_t> &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readBytes(std::numeric_limits<size_t>::max(), out) : refusal;
    }

    /** @brief A whole-number field, at the width its schema declares (`EventPayloadValue::readInteger`) */
    template <typename T> EventPayloadRefusal readInteger(const char *name, T &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readInteger(out) : refusal;
    }

    /** @brief A 64-bit whole-number field (`EventPayloadValue::readWide`) */
    template <typename T> EventPayloadRefusal readWide(const char *name, T &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readWide(out) : refusal;
    }

    /** @brief A fractional field, at the width its schema declares */
    template <typename T> EventPayloadRefusal readFloating(const char *name, T &out) const {
        EventPayloadValue v;
        EventPayloadRefusal refusal = value(name, v);
        return refusal.empty() ? v.readFloating(out) : refusal;
    }

    // ── The writing half: the wire an inject seam fills beside the payload ──

    /** @brief One value of a scalar type as JSON, at the spelling its type deserves */
    template <typename T> static std::string json(const T &value) {
        if constexpr (std::is_same_v<T, bool>) {
            return value ? "true" : "false";
        } else if constexpr (std::is_same_v<T, std::string>) {
            return quote(value);
        } else if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
            // See EventPayloadValue::readBytes for why Latin-1.
            return quote(Latin1Bytes::textOf(value));
        } else if constexpr (std::is_floating_point_v<T>) {
            // The one spelling of a float (ARCHITECTURE.md, "JSON Number
            // Text"); `%.17g` wrote 0.1 as `0.10000000000000001`. JSON has no
            // spelling for one that is not finite.
            const auto number = static_cast<double>(value);
            return std::isfinite(number) ? JsonText::numberText(number) : "null";
        } else if constexpr (std::is_signed_v<T>) {
            return std::to_string(static_cast<long long>(value));
        } else {
            return std::to_string(static_cast<unsigned long long>(value));
        }
    }

    /** @brief One field of the wire, at the spelling its type deserves. */
    template <typename T> static std::string field(const char *name, const T &value) {
        return quote(name) + ":" + json(value);
    }

    /** @brief One field of the wire whose value is already JSON: a list or a record its writer spelled */
    static std::string raw(const char *name, const std::string &json) {
        return quote(name) + ":" + json;
    }

    /**
     * @brief An optional field (docs/adr/0055) that is absent: the key is always
     * written, and `null` is the one spelling of absence.
     */
    static std::string absent(const char *name) {
        return quote(name) + ":null";
    }

    /** @brief The fields an inject seam was given, as the object they spell. */
    static std::string wire(std::initializer_list<std::string> fields) {
        std::string out = "{";
        bool first = true;
        for (const auto &f : fields) {
            if (!first) {
                out += ',';
            }
            first = false;
            out += f;
        }
        out += '}';
        return out;
    }

    /**
     * @brief One list as JSON (docs/adr/0086, section 3.1): an array of what `write`
     * spells each element as, in the list's order; the empty list is `[]`
     */
    template <typename T, typename Write> static std::string list(const std::vector<T> &items, Write write) {
        std::string out = "[";
        for (size_t i = 0; i < items.size(); ++i) {
            if (i > 0) {
                out += ',';
            }
            out += write(items[i]);
        }
        out += ']';
        return out;
    }

    /**
     * @brief One record as JSON: each member a field's id and the JSON of its value,
     * written ascending by the id's bytes whatever order they are given in
     * (ARCHITECTURE.md, "JSON Object Key Order", at every depth)
     */
    static std::string record(std::vector<std::pair<std::string, std::string>> members) {
        // `std::string`'s order is its characters' as unsigned bytes.
        std::sort(members.begin(), members.end(),
                  [](const auto &left, const auto &right) { return left.first < right.first; });
        std::string out = "{";
        for (size_t i = 0; i < members.size(); ++i) {
            if (i > 0) {
                out += ',';
            }
            out += quote(members[i].first) + ":" + members[i].second;
        }
        out += '}';
        return out;
    }

    /**
     * @brief One union value as JSON (docs/adr/0092, section 2.1): the tag under the key
     * `tag`, its value `variant`, the case's name as its enum spells it, beside `members`,
     * the fields of the case's record (none for an empty case), the tag written among them
     * as `record` writes a record's fields
     */
    static std::string unionCase(const std::string &tag, const std::string &variant,
                                 std::vector<std::pair<std::string, std::string>> members) {
        members.emplace_back(tag, quote(variant));
        return record(std::move(members));
    }

private:
    friend class EventPayloadValue;

    /** Take `defaults` as the schema's defaults, or refuse a text that is no object. */
    EventPayloadRefusal takeDefaults(const char *defaults) {
        const char *d = detail::EventPayloadScan::skipSpace(defaults);
        if (*d != '{') {
            // A text that is none is the generator's, not a sender's, and is
            // refused as no JSON object of the whole.
            return EventPayloadRefusal(RefusalCode::NotJson, "",
                                       "the schema's defaults are not a JSON object, which the generator never writes");
        }
        defaults_ = d;
        return {};
    }

    /**
     * The value `name` carries: in the delivery, else in the schema's defaults
     * when the decode was given them, else nullptr.
     */
    const char *find(const char *name) const {
        const char *delivered = detail::EventPayloadScan::findIn(object_, name);
        return delivered != nullptr ? delivered : detail::EventPayloadScan::findIn(defaults_, name);
    }

    static std::string quote(const std::string &text) {
        return JsonText::quoted(text);
    }

    /** At the '{' of the payload object, inside the event record's own data. */
    const char *object_ = nullptr;

    /**
     * At the '{' of the schema's defaults, a literal the generated machine
     * holds; nullptr for a schema no later revision added a field to.
     */
    const char *defaults_ = nullptr;

    /** The path of the record these are the fields of: empty for the payload. */
    std::string prefix_;

    /** The strict RFC 6901 pointer to the record: empty for the payload. */
    std::string pointerPrefix_;
};

template <typename Read> EventPayloadRefusal EventPayloadValue::readRecord(const char *defaults, Read read) const {
    if (*at_ != '{') {
        return wrongKind("is not a record");
    }
    EventPayloadFields fields;
    fields.object_ = at_;
    fields.prefix_ = path_;
    fields.pointerPrefix_ = pointer_;
    if (defaults != nullptr) {
        EventPayloadRefusal refusal = fields.takeDefaults(defaults);
        if (!refusal.empty()) {
            return refusal;
        }
    }
    return read(static_cast<const EventPayloadFields &>(fields));
}

template <typename Tag, typename ReadTag, typename ReadCase>
EventPayloadRefusal EventPayloadValue::readUnion(const char *tag, ReadTag readTag, ReadCase readCase) const {
    if (*at_ != '{') {
        return wrongKind("is not a union");
    }
    EventPayloadFields object;
    object.object_ = at_;
    object.prefix_ = path_;
    object.pointerPrefix_ = pointer_;
    EventPayloadValue tagValue;
    EventPayloadRefusal refusal = object.value(tag, tagValue);
    if (!refusal.empty()) {
        return refusal;
    }
    Tag chosen{};
    refusal = readTag(static_cast<const EventPayloadValue &>(tagValue), chosen);
    if (!refusal.empty()) {
        return refusal;
    }
    return readCase(static_cast<const Tag &>(chosen), *this);
}

/**
 * @brief The checks a generated raise runs over a value a host built, before it
 * encodes the event (docs/adr/0086, section 5.4)
 *
 * C++ has no checked constructor on a plain struct, so the raise runs the check its
 * composition generates, one call per constructor, and refuses the event whole. Each
 * answers an empty string, or why the value is not one of its type, the path first,
 * in the words the payload reader uses.
 */
class EventPayloadBounds {
public:
    static std::string text(const std::string &path, const std::string &value, size_t bound) {
        return value.size() > bound ? past(path, value.size(), bound) : std::string{};
    }

    static std::string bytes(const std::string &path, const std::vector<uint8_t> &value, size_t bound) {
        return value.size() > bound ? past(path, value.size(), bound) : std::string{};
    }

    /** `each(path, element)` holds each element to its own type, at the path of its index. */
    template <typename T, typename Each>
    static std::string list(const std::string &path, const std::vector<T> &items, size_t bound, Each each) {
        return listWithin(path, items, 0, bound, each);
    }

    /**
     * A list of at least `least` and at most `bound` elements (docs/adr/0086, section 15.6),
     * each held to its own type by `each`: a list is built from empty an element at a time,
     * so its lower count is held where it is written.
     */
    template <typename T, typename Each>
    static std::string listWithin(const std::string &path, const std::vector<T> &items, size_t least, size_t bound,
                                  Each each) {
        if (items.size() > bound) {
            return "'" + path + "' holds " + std::to_string(items.size()) + " elements, past the " +
                   std::to_string(bound) + " its type declares";
        }
        if (items.size() < least) {
            return "'" + path + "' " + detail::EventPayloadScan::countShort(items.size(), least);
        }
        for (size_t index = 0; index < items.size(); ++index) {
            std::string refusal = each(detail::EventPayloadScan::join(path, std::to_string(index)), items[index]);
            if (!refusal.empty()) {
                return refusal;
            }
        }
        return {};
    }

    /** `present(path, value)` holds a present value to its type; an absent one holds nothing to check. */
    template <typename T, typename Present>
    static std::string optional(const std::string &path, const std::optional<T> &value, Present present) {
        return value.has_value() ? present(path, *value) : std::string{};
    }

private:
    static std::string past(const std::string &path, size_t held, size_t bound) {
        return "'" + path + "' is " + std::to_string(held) + " bytes, past the " + std::to_string(bound) +
               " its type declares";
    }
};

}  // namespace SCE::Common
