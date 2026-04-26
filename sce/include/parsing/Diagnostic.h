// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"

#include <nlohmann/json.hpp>

#include <memory>
#include <optional>
#include <string_view>

// Abstract base for typed diagnostics emitted on the SCE wire.
//
// Mirrors the Rust `sce-build::forge::diagnostic::Diagnostic` envelope
// pinned by `schemas/sce-diagnostic.v1.schema.json`. The C++ side adds
// a second independent conformer to that schema; the Rust side stays
// the authoritative producer and the C++ side conforms field-for-field
// (Q2 in `claudedocs/rfc-sce-diagnostic-wire-unification.md` §1).
//
// Concrete subtypes (W1 promotes `SCE::parsing::TemplateError`) provide:
//   - `code()` — the schema's wire code string, e.g. `"xml/template-cycle"`.
//                Must appear in the Rust `DiagnosticCode` registry; the
//                drift guard lives in `sce-build/src/forge/diagnostic.rs`.
//   - `location()` — author-source `SourcePos` when the throw site
//                    populated one (Phase C P2 plumbing); `nullopt` for
//                    diagnostics raised before position data is computed.
//   - `to_json()` — single NDJSON record matching v1 schema. Field shape
//                   follows the Rust struct member order so canonicalised
//                   byte-diffs against `--error-format=json` agree.
//
// `addError(string)` on `SCXMLParser` continues to coexist (Q4-B); the
// typed surface here is opt-in for consumers that want structured
// diagnostics without re-parsing log text. RFC §W1 contract.

namespace SCE::parsing {

class Diagnostic {
public:
    virtual ~Diagnostic() = default;

    Diagnostic() = default;
    Diagnostic(const Diagnostic &) = default;
    Diagnostic(Diagnostic &&) = default;
    Diagnostic &operator=(const Diagnostic &) = default;
    Diagnostic &operator=(Diagnostic &&) = default;

    virtual std::string_view code() const noexcept = 0;

    // Const-ref return matches the existing `TemplateError::location()`
    // accessor (Phase C P2) so callers like `SCXMLParser::parseFile`
    // can continue binding `const auto &loc = *diag.location();`
    // without touching a temporary's storage.
    virtual const std::optional<SourcePos> &location() const noexcept = 0;

    virtual nlohmann::ordered_json to_json() const = 0;

    // Polymorphic copy. The boundary flatten in `SCXMLParser::parseFile`
    // catches a typed diagnostic by const-reference (the throw object is
    // owned by the catch frame) and needs to take ownership of an
    // independent heap copy so it can record it on the parser's
    // `diagnostics_` vector. A leaf-typed `clone()` is the standard
    // workaround for slicing through a base-class copy ctor; each leaf
    // returns `std::make_unique<Self>(*this)` so the dynamic type is
    // preserved and `to_json()` keeps dispatching to the right override.
    // RFC §W1 audit finding #1 closure (W2 deliverable).
    virtual std::unique_ptr<Diagnostic> clone() const = 0;
};

}  // namespace SCE::parsing
