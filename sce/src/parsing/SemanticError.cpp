// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#include "parsing/SemanticError.h"

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <string>
#include <string_view>

namespace SCE::parsing {

// Every `validation/*` and `scxml/*` `DiagnosticCode` in this family
// reports the `validation` stage (§wire-W5 D2's `Stage::Validation`
// reuse). The value is declared once on `SemanticError::stage()`
// rather than as a file-local constant, because the base computes the
// id and needs to read it.

nlohmann::ordered_json SemanticError::baseEnvelope() const {
    nlohmann::ordered_json out = beginRecord();
    out["message"] = std::string{what()};
    appendLocation(out);
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

nlohmann::ordered_json SemanticWrongPipeline::to_json() const {
    // `actual` carries the kind that was declared, which is the one
    // thing the author has to change and the only payload the Rust
    // arm of `validation/wrong-pipeline` would have that this one can
    // reproduce. The two producers do not share an id for this code —
    // they reject the document through different stages, and the
    // cross-producer harness records that under this leaf's exemption
    // rather than pretending the fragments could line up.
    auto out = baseEnvelope();
    out["actual"] = kind_;
    return out;
}

nlohmann::ordered_json SemanticNoStates::to_json() const {
    // `validation/empty-collection` carries no extra payload on the
    // Rust side either (only `key_fragments` for id derivation, no
    // `actual` / `fix` / `expected`); the C++ envelope mirrors that.
    return baseEnvelope();
}

nlohmann::ordered_json SemanticTopLevelScriptUnloaded::to_json() const {
    // The one NEW wire code in this family that carries a `spec`
    // anchor. It is spliced in after `stage` — the position the Rust
    // struct declares it at — by rebuilding the envelope in order.
    //
    // The rebuild used to enumerate the keys it carried over and
    // dropped `generator` on the floor, so this leaf alone emitted a
    // record the shared schema rejects (`generator` is required). It
    // survived because the suite\'s per-family assertion restated the
    // required key set by hand instead of reading the schema. Copying
    // the envelope wholesale and inserting into it keeps any future
    // envelope field without a second edit here.
    auto envelope = baseEnvelope();
    nlohmann::ordered_json out;
    for (auto it = envelope.begin(); it != envelope.end(); ++it) {
        out[it.key()] = *it;
        if (it.key() == "stage") {
            out["spec"] = "W3C SCXML §5.8";
        }
    }
    if (src_.has_value()) {
        out["actual"] = *src_;
    }
    return out;
}

}  // namespace SCE::parsing
