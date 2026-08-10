// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/ParseError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

nlohmann::ordered_json ParseError::to_json() const {
    // Field order follows the Rust struct's member order so
    // canonicalised byte-diffs against `--error-format=json` agree:
    // envelope, message, location, expected, actual.
    nlohmann::ordered_json out = beginRecord();
    out["message"] = std::string{what()};
    appendLocation(out);
    appendExpected(out);
    if (actual_.has_value()) {
        out["actual"] = *actual_;
    }
    return out;
}

void ParseError::appendExpected(nlohmann::ordered_json &) const {}

void ParseWrongRootElement::appendExpected(nlohmann::ordered_json &out) const {
    // Mirrors the Rust payload: the accepted set is closed and has one
    // member, so the consumer can report it without reading the spec.
    out["expected"] = nlohmann::ordered_json::array({"scxml"});
}

}  // namespace SCE::parsing
