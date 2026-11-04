// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// The fence of a text template's untrusted hole (docs/adr/0087, section 3.1,
// item 4): the value is put between two markers, and every occurrence of
// either marker is first removed from it, so that the value cannot close the
// fence and write outside it.
//
// The removal repeats, taking the leftmost occurrence of either marker (the
// longer first where both start at one byte), until neither occurs: a value
// cannot rebuild a marker out of two halves around one that is removed. It
// ends because each pass shortens the value. The markers are printable ASCII,
// which no byte of a multi-byte UTF-8 sequence is, so removing them from the
// bytes of a text leaves a UTF-8 text.
//
// Holds no state and does no I/O, as every forge runtime header.

#pragma once

#include <algorithm>
#include <cstddef>
#include <string>
#include <string_view>

namespace SCE::Forge {

// `value` with every occurrence of `open` and `close` removed, by the rule of
// the header. An empty marker is no marker (a template's fence holds none; the
// build refuses it).
inline std::string removeMarkers(std::string_view value, std::string_view open, std::string_view close) {
    const std::size_t longest = std::max(open.size(), close.size());
    std::string text(value);
    std::size_t from = 0;
    for (;;) {
        std::size_t at = std::string::npos;
        std::size_t marker = 0;
        for (std::size_t i = from; i < text.size() && at == std::string::npos; ++i) {
            const std::string_view rest = std::string_view(text).substr(i);
            const bool opens = !open.empty() && rest.substr(0, open.size()) == open;
            const bool closes = !close.empty() && rest.substr(0, close.size()) == close;
            if (opens && closes) {
                at = i;
                marker = longest;
            } else if (opens) {
                at = i;
                marker = open.size();
            } else if (closes) {
                at = i;
                marker = close.size();
            }
        }
        if (at == std::string::npos) {
            return text;
        }
        text.erase(at, marker);
        // Before at - (longest - 1) nothing changed and nothing matched, so a
        // marker the removal made can only start at or after it.
        from = at >= longest - 1 ? at - (longest - 1) : 0;
    }
}

}  // namespace SCE::Forge
