// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <charconv>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <functional>
#include <limits>
#include <optional>
#include <stdexcept>
#include <string>
#include <type_traits>
#include <variant>
#include <vector>

#include "SCXMLTypes.h"
#include "core/HostProcessor.h"

namespace SCE {

/**
 * @brief The type one field of a typed host-run request declares
 *
 * A request (`sce:request`, SCE Accepted Subset §2.12) crosses to the host as
 * text, like every `<param>`. What makes it typed is that each value is held to
 * its field's type where the invocation starts — `requestFieldWire` — so the
 * text a host reads back is always one its field's type parses. The C++ port of
 * `sce_rust_runtime::host_processor`'s typed-request half; the rule is the same
 * in every runtime.
 *
 * ⚠ Header-only, and free of the runtime library, for the reason
 * `EventPayloadLift.h` is: a generated machine compiles against `sce/include`.
 * The one spelling this needs from the runtime — a fraction as every untyped
 * `<param>` spells one — is handed in by the caller.
 */
enum class RequestFieldKind : uint8_t {
    Uint8,
    Uint16,
    Uint32,
    Uint64,
    Int8,
    Int16,
    Int32,
    Int64,
    Float32,
    Float64,
    Bool,
    String,
    Bytes,
};

struct RequestFieldType {
    RequestFieldKind kind;
    /// A `bytes` field's `sce:max-size`; zero for every other kind.
    std::size_t cap = 0;
};

namespace TypedHostInvokeRequestDetail {

/// A whole number as a sign and a magnitude, so every 64-bit width — signed
/// and unsigned — is judged without a wider integer type.
struct Whole {
    bool negative = false;
    uint64_t magnitude = 0;
    /// Past every 64-bit width, so no field holds it whatever `magnitude` says.
    bool beyond = false;
};

/// 2^63, the first whole double no signed 64-bit count holds.
inline constexpr double TWO_63 = 9223372036854775808.0;

inline std::optional<Whole> wholeOf(const ::ScriptValue &value) {
    if (const auto *i = std::get_if<int64_t>(&value)) {
        const bool negative = *i < 0;
        const uint64_t magnitude = negative ? uint64_t{0} - static_cast<uint64_t>(*i) : static_cast<uint64_t>(*i);
        return Whole{negative, magnitude, false};
    }
    if (const auto *d = std::get_if<double>(&value)) {
        if (!std::isfinite(*d) || *d != std::trunc(*d)) {
            return std::nullopt;
        }
        const double m = std::fabs(*d);
        if (m >= 2 * TWO_63) {
            return Whole{*d < 0, 0, true};
        }
        // Below 2^64 every whole double converts exactly; at or past 2^63 it
        // is even, so its half does too.
        const uint64_t magnitude = m >= TWO_63 ? static_cast<uint64_t>(m / 2) * 2 : static_cast<uint64_t>(m);
        return Whole{*d < 0 && magnitude != 0, magnitude, false};
    }
    return std::nullopt;
}

inline bool fits(const Whole &w, RequestFieldKind kind) {
    if (w.beyond) {
        return false;
    }
    auto signedFits = [&w](uint64_t max) { return w.negative ? w.magnitude <= max + 1 : w.magnitude <= max; };
    auto unsignedFits = [&w](uint64_t max) { return (!w.negative || w.magnitude == 0) && w.magnitude <= max; };
    switch (kind) {
    case RequestFieldKind::Uint8:
        return unsignedFits(std::numeric_limits<uint8_t>::max());
    case RequestFieldKind::Uint16:
        return unsignedFits(std::numeric_limits<uint16_t>::max());
    case RequestFieldKind::Uint32:
        return unsignedFits(std::numeric_limits<uint32_t>::max());
    case RequestFieldKind::Uint64:
        return unsignedFits(std::numeric_limits<uint64_t>::max());
    case RequestFieldKind::Int8:
        return signedFits(static_cast<uint64_t>(std::numeric_limits<int8_t>::max()));
    case RequestFieldKind::Int16:
        return signedFits(static_cast<uint64_t>(std::numeric_limits<int16_t>::max()));
    case RequestFieldKind::Int32:
        return signedFits(static_cast<uint64_t>(std::numeric_limits<int32_t>::max()));
    case RequestFieldKind::Int64:
        return signedFits(static_cast<uint64_t>(std::numeric_limits<int64_t>::max()));
    default:
        return false;
    }
}

inline bool isWholeKind(RequestFieldKind kind) {
    return kind <= RequestFieldKind::Int64;
}

inline std::string quoted(const std::string &name) {
    return "'" + name + "'";
}

}  // namespace TypedHostInvokeRequestDetail

/**
 * @brief Hold one evaluated `<param>` to the field it supplies, and spell it
 * for the request
 * @param value What the data model evaluated the `<param>` to
 * @param name The field's name, for the refusal
 * @param type The field's declared type
 * @param fractionalWire How every untyped `<param>` spells a fraction
 * @param refusal Receives why, when the value does not fit
 * @return The request text, or `std::nullopt` with `refusal` set
 *
 * §scxml-6.4.1: an argument that cannot be evaluated starts nothing, and a
 * value the record's field cannot hold is such an argument — the host was
 * promised that record. The refusal is the sentence the error.execution event
 * carries, in the words the payload lift uses for a completion that does not
 * fit its record, because the two are the same judgement made on the two
 * halves of one invocation.
 *
 * The text returned is the value at the field's type — a whole number's
 * digits, a fraction as `fractionalWire` spells it — which is what
 * `requestField` parses back, so the adapter reading a checked request cannot
 * fail. A byte string rides as its byte-exact Latin-1 text, the spelling a
 * completion's byte field uses.
 */
inline std::optional<std::string> requestFieldWire(const ::ScriptValue &value, const std::string &name,
                                                   RequestFieldType type,
                                                   const std::function<std::string(double)> &fractionalWire,
                                                   std::string &refusal) {
    namespace D = TypedHostInvokeRequestDetail;
    if (D::isWholeKind(type.kind)) {
        const auto whole = D::wholeOf(value);
        if (!whole) {
            const bool isNumber = std::holds_alternative<double>(value) || std::holds_alternative<int64_t>(value);
            refusal = D::quoted(name) + (isNumber ? " is not a whole number" : " is not a number");
            return std::nullopt;
        }
        if (!D::fits(*whole, type.kind)) {
            refusal = D::quoted(name) + " does not fit the width its schema declares";
            return std::nullopt;
        }
        return (whole->negative ? "-" : "") + std::to_string(whole->magnitude);
    }
    switch (type.kind) {
    case RequestFieldKind::Float32:
    case RequestFieldKind::Float64: {
        double d = 0;
        if (const auto *i = std::get_if<int64_t>(&value)) {
            d = static_cast<double>(*i);
        } else if (const auto *f = std::get_if<double>(&value)) {
            d = *f;
        } else {
            refusal = D::quoted(name) + " is not a number";
            return std::nullopt;
        }
        // JSON, which a completion's record crosses as, has no spelling for
        // these, so a request may not carry one either.
        if (!std::isfinite(d)) {
            refusal = D::quoted(name) + " is not a finite number";
            return std::nullopt;
        }
        if (type.kind == RequestFieldKind::Float32) {
            if (std::fabs(d) > static_cast<double>(std::numeric_limits<float>::max())) {
                refusal = D::quoted(name) + " does not fit the width its schema declares";
                return std::nullopt;
            }
            d = static_cast<double>(static_cast<float>(d));
        }
        return fractionalWire(d);
    }
    case RequestFieldKind::Bool:
        if (const auto *b = std::get_if<bool>(&value)) {
            return std::string(*b ? "true" : "false");
        }
        refusal = D::quoted(name) + " is not a truth value";
        return std::nullopt;
    case RequestFieldKind::String:
        if (const auto *s = std::get_if<std::string>(&value)) {
            return *s;
        }
        refusal = D::quoted(name) + " is not a text";
        return std::nullopt;
    case RequestFieldKind::Bytes: {
        const auto *s = std::get_if<std::string>(&value);
        if (s == nullptr) {
            refusal = D::quoted(name) + " is not a byte string";
            return std::nullopt;
        }
        // The script engine hands text as UTF-8; each character up to U+00FF
        // is one byte of the Latin-1 spelling.
        std::size_t chars = 0;
        for (std::size_t i = 0; i < s->size(); ++chars) {
            const auto lead = static_cast<unsigned char>((*s)[i]);
            if (lead < 0x80) {
                i += 1;
            } else if (lead == 0xC2 || lead == 0xC3) {
                i += 2;
            } else {
                refusal = D::quoted(name) + " carries a character above U+00FF, which no single byte spells";
                return std::nullopt;
            }
        }
        if (chars > type.cap) {
            refusal = D::quoted(name) + " is " + std::to_string(chars) + " bytes, past the " +
                      std::to_string(type.cap) + " its schema declares";
            return std::nullopt;
        }
        return *s;
    }
    default:
        refusal = D::quoted(name) + " has a type this runtime cannot read";
        return std::nullopt;
    }
}

/**
 * @brief One field of a checked typed request, read back at its declared type
 * by the generated adapter
 *
 * Throws `std::logic_error` when the request does not carry the field as text
 * its type parses. A request that reached a typed adapter was checked field by
 * field where it started (`requestFieldWire`), so this is a broken promise
 * between two halves of generated code, not a value a host or document can
 * supply — and one that would otherwise hand the host a record the document
 * never sent.
 */
template <typename T> T requestField(const HostInvokeRequest &request, const char *name) {
    const auto it = request.params.find(name);
    if (it == request.params.end() || it->second.size() != 1) {
        throw std::logic_error("typed request '" + request.invokeId + "' does not carry '" + name +
                               "' exactly once, though its record declares it");
    }
    const std::string &text = it->second.front();
    const auto broken = [&]() {
        return std::logic_error("typed request '" + request.invokeId + "' carries '" + name + "' as '" + text +
                                "', which its type does not parse, though the start site checked it");
    };
    if constexpr (std::is_same_v<T, std::string>) {
        return text;
    } else if constexpr (std::is_same_v<T, std::vector<uint8_t>>) {
        // The Latin-1 spelling `requestFieldWire` checked, back as bytes.
        std::vector<uint8_t> out;
        for (std::size_t i = 0; i < text.size();) {
            const auto lead = static_cast<unsigned char>(text[i]);
            if (lead < 0x80) {
                out.push_back(lead);
                i += 1;
            } else if ((lead == 0xC2 || lead == 0xC3) && i + 1 < text.size()) {
                out.push_back(
                    static_cast<uint8_t>(((lead & 0x03) << 6) | (static_cast<unsigned char>(text[i + 1]) & 0x3F)));
                i += 2;
            } else {
                throw broken();
            }
        }
        return out;
    } else if constexpr (std::is_same_v<T, bool>) {
        if (text == "true") {
            return true;
        }
        if (text == "false") {
            return false;
        }
        throw broken();
    } else if constexpr (std::is_integral_v<T>) {
        T out{};
        const auto [end, ec] = std::from_chars(text.data(), text.data() + text.size(), out);
        if (ec != std::errc{} || end != text.data() + text.size()) {
            throw broken();
        }
        return out;
    } else {
        static_assert(std::is_floating_point_v<T>, "requestField reads a request field's declared type");
        char *end = nullptr;
        const double d = std::strtod(text.c_str(), &end);
        if (text.empty() || end != text.c_str() + text.size()) {
            throw broken();
        }
        return static_cast<T>(d);
    }
}

}  // namespace SCE
