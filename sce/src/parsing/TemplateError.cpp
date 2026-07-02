// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/TemplateError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

namespace {

// Every `xml/template-*` DiagnosticCode shares the `xml` Stage in the
// Rust authority (see `DiagnosticCode::stage()` in
// `sce-build/src/forge/diagnostic.rs`). Hard-coded here rather than
// derived by string-prefix-split because the prefix→stage table on
// the Rust side is not 1:1 (e.g. `cli/*` codes map to `cli`,
// `mesh/deploy-*` map to `mesh-deploy`).
constexpr std::string_view kTemplateStage = "xml";

}  // namespace

nlohmann::ordered_json TemplateError::to_json() const {
    const std::string_view codeStr = code();
    const std::string fileStr = location_.has_value() ? location_->file.string() : std::string{};
    const std::string_view messageView{what()};
    const std::string idStr = computeFnv1aDiagnosticId(codeStr, kTemplateStage, fileStr, messageView);

    nlohmann::ordered_json out;
    out["v"] = 1;
    out["id"] = idStr;
    out["code"] = codeStr;
    out["stage"] = kTemplateStage;
    out["message"] = std::string{messageView};

    if (location_.has_value()) {
        nlohmann::ordered_json loc;
        loc["file"] = location_->file.string();
        loc["line"] = location_->row;
        loc["col"] = location_->col;
        out["location"] = std::move(loc);
    }

    return out;
}

}  // namespace SCE::parsing
