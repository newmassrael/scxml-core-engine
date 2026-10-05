// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::JsonText — how SCE writes a string into JSON text.
//
// One form for every engine (ARCHITECTURE.md, "JSON Text (Single Source of
// Truth)"): `"` and `\` escaped, the five short forms \b \f \n \r \t, every
// other U+0000-U+001F as \u00xx in lowercase hex, and every other byte as it
// is. It is the form nlohmann's writer produces, so a string written here and
// one written through nlohmann are the same bytes. tests/json_text/
// string_escape.json holds the cases every engine is measured against.
//
// A float is written one way too (ARCHITECTURE.md, "JSON Number Text (Single
// Source of Truth)"): as ECMAScript's Number::toString spells it. tests/
// json_text/real_text.json holds those cases.
//
// Header-only and dependency-free because its callers include header-only
// helpers that generated code compiles without nlohmann.

#pragma once

#include <charconv>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <string_view>

namespace SCE::JsonText {

namespace detail {

/// The shortest scientific digits of a positive finite double, written into
/// `buffer` as `d[.ddd]e[+-]XX` (the exponent signed, two digits at least), and
/// their length: the form `std::to_chars(..., std::chars_format::scientific)`
/// gives.
///
/// Through `printf`, which every library has. libstdc++ declares no
/// floating-point `to_chars` before GCC 11, so a platform whose compiler is older
/// (the product's own build was GCC 9.5: `'std::chars_format' has not been
/// declared`, measured 2026-10-06) could not compile this header at all, and the
/// compiler is the platform's to choose and not ours. The smallest precision whose
/// output reads back as the same double is the shortest digits, and `printf`
/// rounds to nearest, so it is also the nearest of the shortest ones -- the pair
/// `to_chars` returns. `tests/common/JsonTextTest.cpp` holds the two to each other
/// over the whole table, so the fallback is measured on a compiler that has both.
inline std::size_t scientificByPrintf(char *buffer, std::size_t size, double value) {
    for (int precision = 0; precision <= 17; ++precision) {
        const int written = std::snprintf(buffer, size, "%.*e", precision, value);
        if (written > 0 && std::strtod(buffer, nullptr) == value) {
            return static_cast<std::size_t>(written);
        }
    }
    return 0;  // not reached: 17 significant digits read back every double
}

#if defined(__cpp_lib_to_chars) && __cpp_lib_to_chars >= 201611L
#define SCE_JSON_TEXT_HAS_FLOAT_TO_CHARS 1

inline std::size_t scientificByToChars(char *buffer, std::size_t size, double value) {
    const auto written = std::to_chars(buffer, buffer + size, value, std::chars_format::scientific);
    return static_cast<std::size_t>(written.ptr - buffer);
}

inline std::size_t scientific(char *buffer, std::size_t size, double value) {
    return scientificByToChars(buffer, size, value);
}
#else
inline std::size_t scientific(char *buffer, std::size_t size, double value) {
    return scientificByPrintf(buffer, size, value);
}
#endif

/// `numberText` over a chosen way of getting the shortest scientific digits.
template <typename Scientific> inline std::string numberTextWith(double value, Scientific scientific) {
    if (std::isnan(value)) {
        return "NaN";
    }
    if (std::isinf(value)) {
        return value > 0 ? "Infinity" : "-Infinity";
    }
    if (value == 0.0) {
        return "0";
    }
    char buffer[40];
    const std::size_t length = scientific(buffer, sizeof buffer, std::fabs(value));
    const std::string_view text(buffer, length);
    const std::size_t marker = text.find('e');
    std::string digits;
    for (const char c : text.substr(0, marker)) {
        // Digits only: the decimal point a locale writes is not one of them.
        if (c >= '0' && c <= '9') {
            digits += c;
        }
    }
    // The exponent is a sign and at least two digits.
    const int exponent = std::atoi(std::string(text.substr(marker + 1)).c_str());
    // The value is `0.<digits> * 10^point`; the specification's `k` and `n`.
    const int k = static_cast<int>(digits.size());
    const int point = exponent + 1;
    std::string out = value < 0 ? "-" : "";
    if (k <= point && point <= 21) {
        out += digits;
        out.append(static_cast<std::size_t>(point - k), '0');
    } else if (0 < point && point <= 21) {
        out.append(digits, 0, static_cast<std::size_t>(point));
        out += '.';
        out.append(digits, static_cast<std::size_t>(point), std::string::npos);
    } else if (-6 < point && point <= 0) {
        out += "0.";
        out.append(static_cast<std::size_t>(-point), '0');
        out += digits;
    } else {
        const int power = point - 1;
        out += digits[0];
        if (k > 1) {
            out += '.';
            out.append(digits, 1, std::string::npos);
        }
        out += 'e';
        out += power < 0 ? '-' : '+';
        out += std::to_string(std::abs(power));
    }
    return out;
}

}  // namespace detail

/// A 64-bit float as ECMAScript's `Number::toString` spells it, radix 10: the
/// fewest digits that read back as the same double, in decimal notation when
/// `1e-6 <= |x| < 1e21` and as `d[.ddd]e[+-]n` otherwise, no fraction on a whole
/// value and `0` for either zero. A value that is not finite is spelled `NaN`,
/// `Infinity` or `-Infinity`, as `String(x)` does; a JSON writer, which has no
/// spelling for those, tests `std::isfinite` before it calls this.
///
/// The shortest digits (and the nearest of them) are what `printf`'s `%g` family
/// cannot give: `%.17g` writes 0.1 as `0.10000000000000001` and the default
/// stream precision writes pi as `3.14159`. `std::to_chars` in scientific form
/// gives them where the library has it, and `printf` at the smallest precision
/// that reads back gives the same ones where it does not (`detail::scientific`).
/// The layout is the specification's.
inline std::string numberText(double value) {
    return detail::numberTextWith(value, detail::scientific);
}

/// Append `text` to `out` as the body of a JSON string: escaped, unquoted.
inline void appendEscaped(std::string &out, std::string_view text) {
    static constexpr char kHex[] = "0123456789abcdef";
    for (const char c : text) {
        switch (c) {
        case '"':
            out += "\\\"";
            break;
        case '\\':
            out += "\\\\";
            break;
        case '\b':
            out += "\\b";
            break;
        case '\f':
            out += "\\f";
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
        default: {
            // Compared as a byte: `char` is signed on the common ABIs, and a
            // signed comparison reads every UTF-8 byte above 0x7F as a
            // control character.
            const auto byte = static_cast<unsigned char>(c);
            if (byte < 0x20u) {
                out += "\\u00";
                out += kHex[byte >> 4];
                out += kHex[byte & 0x0Fu];
            } else {
                out += c;
            }
        }
        }
    }
}

/// `text` as the body of a JSON string: escaped, unquoted.
inline std::string escaped(std::string_view text) {
    std::string out;
    out.reserve(text.size());
    appendEscaped(out, text);
    return out;
}

/// `text` as a JSON string: escaped and quoted.
inline std::string quoted(std::string_view text) {
    std::string out;
    out.reserve(text.size() + 2);
    out += '"';
    appendEscaped(out, text);
    out += '"';
    return out;
}

}  // namespace SCE::JsonText
