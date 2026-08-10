// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/Diagnostic.h"
#include "parsing/PositionMap.h"

#include <nlohmann/json.hpp>

#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

namespace SCE::parsing {

// SCXML semantic-validation typed exception family thrown by
// `SCE::SCXMLParser::parseScxmlNode` and `validateModel` under §wire-W5
// D1 (typed-throw, mirror of W4 D1-C). Each leaf carries a typed
// payload — `SCXMLParser::parseFile` / `parseContent` catch it on the
// `SemanticError` base and surface the typed instance via
// `getDiagnostics()` while populating the legacy `getErrorMessages()`
// string vector for Q4-B coexistence.
//
// Four of the five leaves REUSE existing `validation/*` wire codes
// per the W4 D4 fold precedent (concept identity over namespace
// duplication): `SemanticInitialStateUnknown` and
// `SemanticTransitionTargetUnknown` both map to
// `validation/invalid-reference` — the same wire code forge
// `ValidationError::InvalidReference` emits — `SemanticNoStates`
// maps to `validation/empty-collection`, and
// `SemanticHistoryDefaultMissing` to `validation/missing-element`.
// Wire-level consumers dispatching on `code()` receive the SAME
// branch for forge-document
// "name does not resolve" failures and SCXML-document
// "name does not resolve" failures; the fold is honest at the wire
// level. In-process C++ consumers can still distinguish the
// SCXML-specific subtypes via `dynamic_cast` if needed for richer
// payload access.
//
// The fifth leaf, `SemanticTopLevelScriptUnloaded`, carries a NEW
// wire code `scxml/top-level-script-unloaded` because §scxml-5.8
// has no forge analog — the rejection rule is unique to SCXML's
// document-loading semantics. §wire-W5 D2 documents the 1-NEW + 4-
// REUSE breakdown.
//
// Stage = "validation" for ALL five leaves: SCXML semantic validation
// IS post-parse semantic validation, the same analytical stage as
// forge `validation/*`. A separate `Stage::ScxmlSemantic` is
// deliberately not added: routing by stage would split one analytical
// stage across two wire values, which §wire-W5 anti-pattern #7 names
// as the mistake.
//
// NEW wire codes are declared only where a matching Rust producer
// exists; producer-less leaves reuse existing codes (§wire-W5).

class SemanticError : public std::runtime_error, public Diagnostic {
public:
    using std::runtime_error::runtime_error;

    // Reserved for location stamping. No throw site populates
    // this today (`parseScxmlNode` / `validateModel` callsites have
    // SCXML-element node pointers but no captured source position, so
    // there is nothing to stamp until the parse layer carries one).
    void setLocation(SourcePos pos) {
        location_ = std::move(pos);
    }

    // `Diagnostic` interface. Subtypes override `code()` with their
    // wire string and `to_json()` with payload-aware serialization.
    std::string_view code() const noexcept override = 0;

    const std::optional<SourcePos> &location() const noexcept override {
        return location_;
    }

    nlohmann::ordered_json to_json() const override = 0;

protected:
    // Shared envelope helper for subtypes — fills `v`, `id`, `code`,
    // `stage`, `message`, and optional `location`. Subtypes call this
    // first then append leaf-specific fields (`spec`, `actual`, `fix`)
    // in the schema's canonical key order.
    nlohmann::ordered_json baseEnvelope() const;

private:
    std::optional<SourcePos> location_;
};

// `<scxml initial="X">` or `<state initial="X">` references a state
// that is not declared. Mirrors Rust
// `ScxmlSemanticError::InitialStateUnknown` —
// emits `validation/invalid-reference` with the unresolved id in
// `actual` and the candidate state list in `fix.candidates` (when
// non-empty), enabling automated repair via `replace_one_of`. The
// fold with forge `ValidationError::InvalidReference` is the W4 D4
// precedent applied to SCXML's reference-resolution surface.
class SemanticInitialStateUnknown : public SemanticError {
public:
    enum class Scope {
        DocumentRoot,
        CompoundState,
    };

    SemanticInitialStateUnknown(std::string message, std::string state_id, Scope scope, std::string parent_id,
                                std::vector<std::string> available)
        : SemanticError(std::move(message)), state_id_(std::move(state_id)), scope_(scope),
          parent_id_(std::move(parent_id)), available_(std::move(available)) {}

    std::string_view code() const noexcept override {
        return "validation/invalid-reference";
    }

    nlohmann::ordered_json to_json() const override;

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<SemanticInitialStateUnknown>(*this);
    }

    // In-process C++ accessors for consumers that dispatch on
    // dynamic_cast and need the typed payload.
    const std::string &stateId() const noexcept {
        return state_id_;
    }

    Scope scope() const noexcept {
        return scope_;
    }

    const std::string &parentId() const noexcept {
        return parent_id_;
    }

    const std::vector<std::string> &available() const noexcept {
        return available_;
    }

private:
    std::string state_id_;
    Scope scope_;
    // Empty when scope_ == Scope::DocumentRoot.
    std::string parent_id_;
    std::vector<std::string> available_;
};

// `<transition target="X">` references a state that is not declared.
// Mirrors Rust `ScxmlSemanticError::TransitionTargetUnknown` —
// emits `validation/invalid-reference` with the unresolved target in
// `actual` and `fix.candidates` listing the declared states.
class SemanticTransitionTargetUnknown : public SemanticError {
public:
    SemanticTransitionTargetUnknown(std::string message, std::string state, std::string target,
                                    std::vector<std::string> available)
        : SemanticError(std::move(message)), state_(std::move(state)), target_(std::move(target)),
          available_(std::move(available)) {}

    std::string_view code() const noexcept override {
        return "validation/invalid-reference";
    }

    nlohmann::ordered_json to_json() const override;

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<SemanticTransitionTargetUnknown>(*this);
    }

    const std::string &state() const noexcept {
        return state_;
    }

    const std::string &target() const noexcept {
        return target_;
    }

    const std::vector<std::string> &available() const noexcept {
        return available_;
    }

private:
    std::string state_;
    std::string target_;
    std::vector<std::string> available_;
};

// `<history>` element declares no default configuration. The spec
// requires a single unconditional `<transition>` child naming the
// configuration to enter when the parent state has no stored history;
// without it the pseudostate can never be entered, so the declaration
// is unusable rather than merely incomplete. Mirrors Rust
// `ScxmlSemanticError::HistoryDefaultTransitionMissing`. Folded onto
// `validation/missing-element` per W4 D4 (concept identity with forge
// "required child element is absent" failures).
//
// `available` lists the containing state's children — the default
// configuration is restricted to that state's descendants, so it is
// the legal set the author picks from.
class SemanticHistoryDefaultMissing : public SemanticError {
public:
    SemanticHistoryDefaultMissing(std::string message, std::string history_id, std::string parent_id,
                                  std::vector<std::string> available)
        : SemanticError(std::move(message)), history_id_(std::move(history_id)), parent_id_(std::move(parent_id)),
          available_(std::move(available)) {}

    std::string_view code() const noexcept override {
        return "validation/missing-element";
    }

    nlohmann::ordered_json to_json() const override;

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<SemanticHistoryDefaultMissing>(*this);
    }

    const std::string &history_id() const noexcept {
        return history_id_;
    }

    const std::string &parent_id() const noexcept {
        return parent_id_;
    }

    const std::vector<std::string> &available() const noexcept {
        return available_;
    }

private:
    std::string history_id_;
    std::string parent_id_;
    std::vector<std::string> available_;
};

// SCXML document parsed successfully but contains no top-level
// `<state>`, `<parallel>`, or `<final>` child. §scxml-3.2 requires
// at least one root state — mirrors Rust `ScxmlSemanticError::NoStates`.
// Folded onto `validation/empty-collection` per W4 D4 (concept
// identity with forge "kind requires at least one X" failures).
class SemanticNoStates : public SemanticError {
public:
    using SemanticError::SemanticError;

    std::string_view code() const noexcept override {
        return "validation/empty-collection";
    }

    nlohmann::ordered_json to_json() const override;

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<SemanticNoStates>(*this);
    }
};

// Top-level `<script>` element rejected per §scxml-5.8 — either
// (a) empty content AND empty `src`, (b) `src` set but file failed to
// load, or (c) script body parse failure. The 1 NEW wire code RFC
// §wire-W5 D2 introduces; emits `spec` field with `"W3C SCXML §5.8"`.
//
// Payload `index` is the 1-based script element ordinal (parser-path
// captures it; analyzer-path leaves it empty); `src` is the offending
// `src` attribute value when set. Both are optional because the
// analyzer-side Rust producer (`analyzer::can_generate_static`) emits
// without parser-captured detail — the wire code identity holds
// across the asymmetry per §wire-W5 anti-pattern #5.
class SemanticTopLevelScriptUnloaded : public SemanticError {
public:
    SemanticTopLevelScriptUnloaded(std::string message, std::optional<std::size_t> index,
                                   std::optional<std::string> src)
        : SemanticError(std::move(message)), index_(std::move(index)), src_(std::move(src)) {}

    std::string_view code() const noexcept override {
        return "scxml/top-level-script-unloaded";
    }

    nlohmann::ordered_json to_json() const override;

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<SemanticTopLevelScriptUnloaded>(*this);
    }

    const std::optional<std::size_t> &index() const noexcept {
        return index_;
    }

    const std::optional<std::string> &src() const noexcept {
        return src_;
    }

private:
    std::optional<std::size_t> index_;
    std::optional<std::string> src_;
};

}  // namespace SCE::parsing
