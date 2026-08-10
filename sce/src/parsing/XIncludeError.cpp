// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/XIncludeError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

nlohmann::ordered_json XIncludeExpansionError::to_json() const {
    // Field order follows the Rust struct's member order so
    // canonicalised byte-diffs against `--error-format=json` agree:
    // envelope, message, location, actual, fix.
    nlohmann::ordered_json out = beginRecord();
    out["message"] = std::string{what()};
    appendLocation(out);
    if (actual_.has_value()) {
        out["actual"] = *actual_;
    }
    appendFix(out);
    return out;
}

void XIncludeExpansionError::appendFix(nlohmann::ordered_json &) const {}

void XIncludeMissingHref::appendFix(nlohmann::ordered_json &out) const {
    // Mirrors the Rust payload for this variant: the repair is
    // deterministic (`add_attribute`), so the consumer can apply it
    // without a candidate list. SCE_ERROR_CONTRACT.md §3.1.
    nlohmann::ordered_json fix;
    fix["kind"] = "add_attribute";
    fix["element"] = "xi:include";
    fix["attr"] = "href";
    out["fix"] = std::move(fix);
}

}  // namespace SCE::parsing
