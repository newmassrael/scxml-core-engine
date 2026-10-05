// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string>
#include <string_view>

namespace SCE {

/**
 * §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the stem of the document a hybrid
 * `<invoke>`'s evaluated `srcexpr` names — what the value is matched against the
 * declared `sce:candidates` by. Port of Rust `document_stem`.
 *
 * An expression is free to compute `file:x.scxml`, `./x.scxml`, an absolute path
 * or a Windows one for the same document, so the value is reduced to what the
 * build named the generated child by: the last path segment, without a `file:`
 * scheme and without its extension. A leading dot is a name, not an extension,
 * as the build reads it when it derives the candidate's stem.
 *
 * tests/document_stem/document_stem.json is the one table every engine's reader
 * and the build's are measured against.
 */
inline std::string documentStem(std::string_view value) {
    const auto slash = value.find_last_of("/\\");
    std::string_view name = slash == std::string_view::npos ? value : value.substr(slash + 1);
    constexpr std::string_view kFileScheme = "file:";
    if (name.substr(0, kFileScheme.size()) == kFileScheme) {
        name.remove_prefix(kFileScheme.size());
    }
    const auto dot = name.find_last_of('.');
    if (dot != std::string_view::npos && dot > 0) {
        name = name.substr(0, dot);
    }
    return std::string(name);
}

}  // namespace SCE
