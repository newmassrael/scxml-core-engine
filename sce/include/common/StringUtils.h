// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#pragma once

#include <cstring>
#include <string>

namespace SCE {

/**
 * @brief Fast platform event prefix detection using memcmp
 *
 * Optimized alternative to std::string::find() for checking fixed-length prefixes.
 * Uses memcmp for 75-80% performance improvement over string::find().
 *
 * @param eventName Event name to check
 * @return true if event name starts with "done." or "error."
 */
inline bool isPlatformEvent(const std::string &eventName) {
    // §scxml-3.12.3: the events the implementation generates on its own all live under
    // these two prefixes — done.state.id / done.invoke.id, and the error.* family
    // (error.execution, error.communication, error.platform).
    const size_t len = eventName.length();

    // Check "done." prefix (5 chars)
    if (len >= 5 && memcmp(eventName.data(), "done.", 5) == 0) {
        return true;
    }

    // Check "error." prefix (6 chars)
    if (len >= 6 && memcmp(eventName.data(), "error.", 6) == 0) {
        return true;
    }

    return false;
}

/**
 * @brief Make every ASCII capital letter of a text small, in place, and keep every other byte.
 *
 * The texts this is for are ASCII by their grammars: a URI scheme (RFC 3986 section 3.1 compares
 * it case-insensitively), a type URI, a log level's name. `std::tolower` is the wrong tool for
 * them twice over: it answers by the C locale, and a `char` past 0x7F is negative, which it
 * takes as undefined behaviour; and its `int` answer narrowed back into the `char` is the
 * conversion MSVC reports as data loss (C4244).
 */
inline void lowerAsciiInPlace(std::string &text) {
    for (char &c : text) {
        if (c >= 'A' && c <= 'Z') {
            c = static_cast<char>(c - 'A' + 'a');
        }
    }
}

}  // namespace SCE
