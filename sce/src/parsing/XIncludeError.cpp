// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/XIncludeError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

namespace {

// Every `xml/xinclude-*` `DiagnosticCode` shares the `xml` Stage in
// the Rust authority (see `DiagnosticCode::stage()` in
// `sce-build/src/forge/diagnostic.rs`). Mirrors `kTemplateStage` in
// `TemplateError.cpp`; both stage constants stay file-local because
// the Rust prefix→stage table is not 1:1 (e.g. `cli/*` codes map to
// `cli`, `mesh/deploy-*` map to `mesh-deploy`), so a shared
// stage-mapping helper would carry per-prefix logic that would have
// to grow in lockstep with each future W milestone — deferred until
// a third stage shows up.
constexpr std::string_view kXIncludeStage = "xml";

}  // namespace

nlohmann::ordered_json XIncludeExpansionError::to_json() const {
    const std::string_view codeStr = code();
    const std::string fileStr = location_.has_value() ? location_->file.string() : std::string{};
    const std::string_view messageView{what()};
    const std::string idStr = computeFnv1aDiagnosticId(codeStr, kXIncludeStage, fileStr, messageView);

    nlohmann::ordered_json out = beginDiagnosticRecord(codeStr, kXIncludeStage, idStr);
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
