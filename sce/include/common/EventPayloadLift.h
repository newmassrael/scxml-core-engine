// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
#pragma once

#include <cstdint>
#include <cstdio>
#include <limits>
#include <string>
#include <type_traits>
#include <vector>

namespace SCE::Common {

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
     * @return An empty string when `out` was filled, else why it was not
     *
     * The JSON read is §scxml-B-2-8-1's second rung, the same rung the script
     * engine takes for the same string. Nothing is copied: `out` borrows the
     * characters, so the event record must outlive it — which it does, the lift
     * runs while the event is being dispatched.
     */
    static std::string decode(const std::string &data, EventPayloadFields &out) {
        out.object_ = nullptr;
        const char *p = skipSpace(data.c_str());
        if (*p == '\0') {
            return "the event carries no data";
        }
        if (*p != '{') {
            // §scxml-B-2-8-1's other rungs: a bare value or a space-normalized
            // string names no fields.
            return "the event's data is not an object of named fields";
        }
        out.object_ = p;
        return {};
    }

    std::string readBool(const char *name, bool &out) const {
        const char *p = find(name);
        if (p == nullptr) {
            return missing(name);
        }
        if (std::string_view(p).substr(0, 4) == "true") {
            out = true;
            return {};
        }
        if (std::string_view(p).substr(0, 5) == "false") {
            out = false;
            return {};
        }
        return std::string("'") + name + "' is not a truth value";
    }

    std::string readString(const char *name, std::string &out) const {
        const char *p = find(name);
        if (p == nullptr) {
            return missing(name);
        }
        std::string text;
        const std::string refusal = walkText(p, text);
        if (!refusal.empty()) {
            return std::string("'") + name + "' " + refusal;
        }
        out = std::move(text);
        return {};
    }

    /**
     * @brief A byte-string field
     *
     * JSON has no byte string, so the wire carries the byte-exact Latin-1 text
     * the inject seam writes, and this reads it back the same way: every one of
     * the 256 values is one character and back. Printable ASCII — what a bytes
     * guard compares — is the same bytes under either reading.
     */
    std::string readBytes(const char *name, std::vector<uint8_t> &out) const {
        std::string text;
        const std::string refusal = readString(name, text);
        if (!refusal.empty()) {
            return refusal;
        }
        out.assign(text.begin(), text.end());
        return {};
    }

    /**
     * @brief A whole-number field, at the width its schema declares
     *
     * ⚠ A truth value is not a number here. JSON spells both, and a schema that
     * declared `uint32` and received `true` has been handed something its own
     * type says cannot occur. A value the width cannot hold is refused rather
     * than wrapped — silently truncating is how a guard on a boundary answers
     * for a value the document never received.
     */
    template <typename T> std::string readInteger(const char *name, T &out) const {
        static_assert(std::is_integral_v<T>, "readInteger is for whole-number fields");
        const char *p = find(name);
        if (p == nullptr) {
            return missing(name);
        }
        bool negative = false;
        uint64_t magnitude = 0;
        const std::string refusal = readWhole(p, negative, magnitude);
        if (!refusal.empty()) {
            return std::string("'") + name + "' " + refusal;
        }
        if (negative) {
            if constexpr (std::is_signed_v<T>) {
                // -(min) does not fit T when min is its floor, so the check is
                // made on the magnitude instead.
                const uint64_t floorMagnitude =
                    static_cast<uint64_t>(-(static_cast<int64_t>(std::numeric_limits<T>::min()) + 1)) + 1u;
                if (magnitude > floorMagnitude) {
                    return width(name);
                }
                out = (magnitude == floorMagnitude) ? std::numeric_limits<T>::min()
                                                    : static_cast<T>(-static_cast<int64_t>(magnitude));
                return {};
            } else {
                return width(name);
            }
        }
        if (magnitude > static_cast<uint64_t>(std::numeric_limits<T>::max())) {
            return width(name);
        }
        out = static_cast<T>(magnitude);
        return {};
    }

    /** @brief A fractional field, at the width its schema declares */
    template <typename T> std::string readFloating(const char *name, T &out) const {
        static_assert(std::is_floating_point_v<T>, "readFloating is for fractional fields");
        const char *p = find(name);
        if (p == nullptr) {
            return missing(name);
        }
        if (*p != '-' && (*p < '0' || *p > '9')) {
            return std::string("'") + name + "' is not a number";
        }
        try {
            out = static_cast<T>(std::stod(std::string(p, valueEnd(p) - p)));
        } catch (const std::exception &) {
            return std::string("'") + name + "' is not a number this width can hold";
        }
        return {};
    }

    // ── The writing half: the wire an inject seam fills beside the payload ──

    /** @brief One field of the wire, at the spelling its type deserves. */
    template <typename T> static std::string field(const char *name, const T &value) {
        if constexpr (std::is_same_v<T, bool>) {
            return quote(name) + ":" + (value ? "true" : "false");
        } else if constexpr (std::is_same_v<T, std::string>) {
            return quote(name) + ":" + quote(value);
        } else if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
            // See readBytes for why Latin-1.
            return quote(name) + ":" + quote(std::string(value.begin(), value.end()));
        } else if constexpr (std::is_floating_point_v<T>) {
            char buffer[32];
            std::snprintf(buffer, sizeof(buffer), "%.17g", static_cast<double>(value));
            return quote(name) + ":" + buffer;
        } else if constexpr (std::is_signed_v<T>) {
            return quote(name) + ":" + std::to_string(static_cast<long long>(value));
        } else {
            return quote(name) + ":" + std::to_string(static_cast<unsigned long long>(value));
        }
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

private:
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

    static const char *valueEnd(const char *p) {
        if (*p == '"') {
            const char *after = skipString(p);
            return after == nullptr ? p : after;
        }
        if (*p == '{' || *p == '[') {
            const char *after = skipContainer(p);
            return after == nullptr ? p : after;
        }
        while (*p != '\0' && *p != ',' && *p != '}' && *p != ']') {
            p++;
        }
        return p;
    }

    /** Whether the JSON string at `p` spells exactly `name`. */
    static bool stringIs(const char *p, const char *name) {
        p++;  // past the opening quote
        while (*name != '\0') {
            // An escaped field name is legal JSON and no schema writes one, so
            // it cannot be the name being looked for.
            if (*p == '\0' || *p == '"' || *p == '\\' || *p != *name) {
                return false;
            }
            p++;
            name++;
        }
        return *p == '"';
    }

    /** The value `name` carries, or nullptr when the object does not name it. */
    const char *find(const char *name) const {
        if (object_ == nullptr) {
            return nullptr;
        }
        const char *p = skipSpace(object_ + 1);  // past '{'
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

    /** One JSON string's characters, unescaped. */
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
                    for (int i = 1; i <= 4; i++) {
                        const char c = p[i];
                        unsigned digit;
                        if (c >= '0' && c <= '9') {
                            digit = static_cast<unsigned>(c - '0');
                        } else if (c >= 'a' && c <= 'f') {
                            digit = static_cast<unsigned>(c - 'a' + 10);
                        } else if (c >= 'A' && c <= 'F') {
                            digit = static_cast<unsigned>(c - 'A' + 10);
                        } else {
                            return "is not a text (a \\u escape is not hex)";
                        }
                        code = (code << 4) | digit;
                    }
                    if (code > 0xFFu) {
                        return "carries a character above U+00FF, which no single byte spells";
                    }
                    out += static_cast<char>(code);
                    p += 4;
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

    /** A whole number's sign and magnitude, or why it is neither. */
    static std::string readWhole(const char *p, bool &negative, uint64_t &magnitude) {
        if (*p != '-' && (*p < '0' || *p > '9')) {
            return "is not a number";
        }
        negative = (*p == '-');
        const char *digits = negative ? p + 1 : p;
        if (*digits < '0' || *digits > '9') {
            return "is not a number";
        }
        uint64_t value = 0;
        const char *d = digits;
        for (; *d >= '0' && *d <= '9'; d++) {
            const uint64_t digit = static_cast<uint64_t>(*d - '0');
            if (value > (std::numeric_limits<uint64_t>::max() - digit) / 10u) {
                return "does not fit the width its schema declares";
            }
            value = value * 10u + digit;
        }
        // A fraction or an exponent is a different value, and rounding it would
        // answer a guard for something nobody sent.
        if (*d == '.' || *d == 'e' || *d == 'E') {
            return "is not a whole number";
        }
        magnitude = value;
        return {};
    }

    static std::string missing(const char *name) {
        return std::string("the event's data has no '") + name + "'";
    }

    static std::string width(const char *name) {
        return std::string("'") + name + "' does not fit the width its schema declares";
    }

    static std::string quote(const std::string &text) {
        std::string out;
        out.reserve(text.size() + 2);
        out += '"';
        for (const char c : text) {
            switch (c) {
            case '"':
                out += "\\\"";
                break;
            case '\\':
                out += "\\\\";
                break;
            case '\n':
                out += "\\n";
                break;
            case '\r':
                out += "\\r";
                break;
            case '\t':
                out += "\\t";
                break;
            default:
                if (static_cast<unsigned char>(c) < 0x20u) {
                    char buffer[7];
                    std::snprintf(buffer, sizeof(buffer), "\\u%04x", static_cast<unsigned>(c) & 0xFFu);
                    out += buffer;
                } else {
                    out += c;
                }
            }
        }
        out += '"';
        return out;
    }

    /** At the '{' of the payload object, inside the event record's own data. */
    const char *object_ = nullptr;
};

}  // namespace SCE::Common
