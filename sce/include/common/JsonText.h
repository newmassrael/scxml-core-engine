// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
// Header-only and dependency-free because its callers include header-only
// helpers that generated code compiles without nlohmann.

#pragma once

#include <string>
#include <string_view>

namespace SCE::JsonText {

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
