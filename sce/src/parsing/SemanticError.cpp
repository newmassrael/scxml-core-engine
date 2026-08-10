// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/SemanticError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

namespace {

// Every `validation/*` and `scxml/*` `DiagnosticCode` shares the
// `validation` Stage in the Rust authority (see
// `DiagnosticCode::stage()` in `sce-build/src/forge/diagnostic.rs`
// and §wire-W5 D2's Stage::Validation reuse). Mirrors `kParseStage`
// in `ParseError.cpp` and `kXIncludeStage` in `XIncludeError.cpp`;
// stage constant stays file-local for the same reason — Rust
// prefix→stage table is not 1:1 (`mesh/deploy-*` map to
// `mesh-deploy`, etc.), so a shared mapping helper would carry per-
// prefix logic that grows with each future W milestone.
constexpr std::string_view kSemanticStage = "validation";

}  // namespace

nlohmann::ordered_json SemanticError::baseEnvelope() const {
    const std::string_view codeStr = code();
    const std::string fileStr = location().has_value() ? location()->file.string() : std::string{};
    const std::string_view messageView{what()};
    const std::string idStr = computeFnv1aDiagnosticId(codeStr, kSemanticStage, fileStr, messageView);

    nlohmann::ordered_json out = beginDiagnosticRecord(codeStr, kSemanticStage, idStr);
    out["message"] = std::string{messageView};

    if (location().has_value()) {
        nlohmann::ordered_json loc;
        loc["file"] = location()->file.string();
        loc["line"] = location()->row;
        loc["col"] = location()->col;
        out["location"] = std::move(loc);
    }

    return out;
}

namespace {

// Append `actual` and `fix` to a base envelope that is otherwise
// identical to the forge `validation/invalid-reference` payload. Used
// by both `SemanticInitialStateUnknown` and `SemanticTransitionTargetUnknown`
// — they share the same payload shape because the wire code itself
// is shared (W4 D4 fold).
void appendInvalidReferenceFields(nlohmann::ordered_json &out, const std::string &actual,
                                  const std::vector<std::string> &available) {
    out["actual"] = actual;
    if (!available.empty()) {
        nlohmann::ordered_json fix;
        fix["kind"] = "replace_one_of";
        fix["candidates"] = available;
        out["fix"] = std::move(fix);
    }
}

}  // namespace

nlohmann::ordered_json SemanticInitialStateUnknown::to_json() const {
    auto out = baseEnvelope();
    appendInvalidReferenceFields(out, state_id_, available_);
    return out;
}

nlohmann::ordered_json SemanticTransitionTargetUnknown::to_json() const {
    auto out = baseEnvelope();
    appendInvalidReferenceFields(out, target_, available_);
    return out;
}

nlohmann::ordered_json SemanticHistoryDefaultMissing::to_json() const {
    auto out = baseEnvelope();
    // `validation/missing-element` carries `actual` (the offending
    // element) and no `fix` on the Rust side: SCE_ERROR_CONTRACT §3.1
    // has no add-child-element fix variant, so the legal default
    // targets travel in `message` rather than as candidates. The C++
    // envelope mirrors that — an extra `fix` here would make the two
    // producers disagree on the same wire code.
    out["actual"] = history_id_;
    return out;
}

nlohmann::ordered_json SemanticNoStates::to_json() const {
    // `validation/empty-collection` carries no extra payload on the
    // Rust side either (only `key_fragments` for id derivation, no
    // `actual` / `fix` / `expected`); the C++ envelope mirrors that.
    return baseEnvelope();
}

nlohmann::ordered_json SemanticTopLevelScriptUnloaded::to_json() const {
    auto out = baseEnvelope();
    // The 1 NEW wire code with a `spec` anchor in this RFC. Inserted
    // after `stage` and before `message` to match the canonical key
    // ordering enforced by `code_serde_rename_matches_as_str` /
    // `CanonicalJsonStringHasAlphabeticalKeyOrder` test invariants on
    // the Rust side.
    nlohmann::ordered_json reordered;
    reordered["v"] = out.at("v");
    reordered["id"] = out.at("id");
    reordered["code"] = out.at("code");
    reordered["stage"] = out.at("stage");
    reordered["spec"] = "W3C SCXML §5.8";
    reordered["message"] = out.at("message");
    if (out.contains("location")) {
        reordered["location"] = std::move(out.at("location"));
    }
    if (src_.has_value()) {
        reordered["actual"] = *src_;
    }
    return reordered;
}

}  // namespace SCE::parsing
