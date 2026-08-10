// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/TemplateError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

nlohmann::ordered_json TemplateError::to_json() const {
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

void TemplateError::appendFix(nlohmann::ordered_json &) const {}

void TemplateMissingAttribute::appendFix(nlohmann::ordered_json &out) const {
    // Mirrors the Rust payload: the repair is deterministic, so no
    // candidate list is needed. SCE_ERROR_CONTRACT.md §3.1.
    nlohmann::ordered_json fix;
    fix["kind"] = "add_attribute";
    fix["element"] = "sce:use";
    fix["attr"] = "template";
    out["fix"] = std::move(fix);
}

void TemplateMissingParam::appendFix(nlohmann::ordered_json &out) const {
    // The omitted parameter is supplied as an attribute on the
    // `<sce:use>` element, so the repair names that attribute —
    // matching the Rust payload for this variant.
    nlohmann::ordered_json fix;
    fix["kind"] = "add_attribute";
    fix["element"] = "sce:use";
    fix["attr"] = param_;
    out["fix"] = std::move(fix);
}

}  // namespace SCE::parsing
