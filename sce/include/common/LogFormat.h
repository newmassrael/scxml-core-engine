// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE::detail::formatLog — the text of an SCE_LOG_* message (docs/adr/0060).
//
// Every translation unit formats a message the same way, so the formatter uses only
// what every one of them has: the standard library at the interface standard. Its
// grammar is the one every SCE_LOG_* call writes:
//
// - `{}` is replaced by the next argument;
// - `{{` and `}}` are one literal brace each, as in std::format;
// - a `{}` with no argument left is written as it stands, `{}`;
// - an argument with no `{}` left is not written;
// - any other brace is written as it stands.
//
// An argument is written through its `operator<<`, in the classic locale, so a number
// reads the same whatever locale the program set. Where `<<` would write a value
// otherwise than std::format's `{}` did when SCE formatted with it, the value is written
// as `{}` wrote it: a `bool` as `true` or `false`, and a `signed char` or
// `unsigned char` (`int8_t`, `uint8_t`) as a number. A null `const char *` is written
// as `(null)`. A floating-point value is written as the shortest text that reads back
// to it, by `JsonText::numberText`, the one number spelling SCE has: `{}` wrote the
// shortest digits too, and `<<` would write six significant ones.
// An argument with no `operator<<` does not compile, as it did not with std::format.

#pragma once

#include "common/JsonText.h"

#include <locale>
#include <ostream>
#include <sstream>
#include <string>
#include <type_traits>

namespace SCE::detail {

/**
 * @brief Write the pattern from `cursor` on, with `{{` and `}}` as one brace each
 *
 * With `stopAtField`, stop at the first `{}` and answer the position after it.
 * Without it, a `{}` is written as it stands. Null once the text has ended.
 */
inline const char *writeLogText(std::ostream &out, const char *cursor, bool stopAtField) {
    while (*cursor != '\0') {
        const char current = cursor[0];
        const char next = cursor[1];
        if (current == '{' && next == '}') {
            if (stopAtField) {
                return cursor + 2;
            }
            out << "{}";
            cursor += 2;
        } else if ((current == '{' || current == '}') && next == current) {
            out.put(current);
            cursor += 2;
        } else {
            out.put(current);
            ++cursor;
        }
    }
    return nullptr;
}

/// One argument, as the grammar at the top of this header writes it.
template <typename T> void writeLogArgument(std::ostream &out, const T &value) {
    if constexpr (std::is_same_v<T, bool>) {
        out << (value ? "true" : "false");
    } else if constexpr (std::is_same_v<T, signed char> || std::is_same_v<T, unsigned char>) {
        out << static_cast<int>(value);
    } else if constexpr (std::is_same_v<T, const char *> || std::is_same_v<T, char *>) {
        out << (value != nullptr ? static_cast<const char *>(value) : "(null)");
    } else if constexpr (std::is_floating_point_v<T>) {
        out << ::SCE::JsonText::numberText(static_cast<double>(value));
    } else {
        out << value;
    }
}

/// The text before the next `{}` and the argument that fills it; null once the
/// pattern has no `{}` left, and from then on nothing is written.
template <typename T> const char *writeLogField(std::ostream &out, const char *cursor, const T &value) {
    if (cursor == nullptr) {
        return nullptr;
    }
    const char *after = writeLogText(out, cursor, true);
    if (after != nullptr) {
        writeLogArgument(out, value);
    }
    return after;
}

/**
 * @brief `pattern` with each `{}` replaced by the next of `args`
 *
 * See the grammar at the top of this header. A null `pattern` is the empty text.
 */
template <typename... Args> std::string formatLog(const char *pattern, const Args &...args) {
    std::ostringstream out;
    out.imbue(std::locale::classic());
    const char *cursor = pattern != nullptr ? pattern : "";
    ((cursor = writeLogField(out, cursor, args)), ...);
    if (cursor != nullptr) {
        writeLogText(out, cursor, false);
    }
    return out.str();
}

}  // namespace SCE::detail
