// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"

#include <nlohmann/json.hpp>

#include <memory>
#include <optional>
#include <string>
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
// diagnostics without re-parsing log text. RFC §wire-W1 contract.

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

    // Stable canonical-JSON string for byte-diff parity. Re-parses
    // `to_json()`'s output through `nlohmann::json` (alphabetical
    // key order via `std::map`) and dumps with `dump(-1, ' ', false)`
    // — no whitespace, no key-order coupling to the
    // `nlohmann::ordered_json` insertion order on the producer side.
    // Matches the canonicalisation Rust output is expected to round-
    // trip through for any cross-side byte-diff consumer (RFC §wire-W2
    // deliverable item #3 / RFC §4 risk row "canonicalisation hides
    // semantic divergence" — only key order and whitespace are
    // normalised, never field names or values).
    std::string to_canonical_json_string() const;

    // Polymorphic copy. The boundary flatten in `SCXMLParser::parseFile`
    // catches a typed diagnostic by const-reference (the throw object is
    // owned by the catch frame) and needs to take ownership of an
    // independent heap copy so it can record it on the parser's
    // `diagnostics_` vector. A leaf-typed `clone()` is the standard
    // workaround for slicing through a base-class copy ctor; each leaf
    // returns `std::make_unique<Self>(*this)` so the dynamic type is
    // preserved and `to_json()` keeps dispatching to the right override.
    // RFC §wire-W1 audit finding #1 closure (W2 deliverable).
    virtual std::unique_ptr<Diagnostic> clone() const = 0;
};

// Content-addressed FNV-1a 64-bit id shared between every concrete
// `Diagnostic` subtype's `to_json()` impl. Mirrors
// `sce-build/src/forge/diagnostic.rs::Fnv1a64` byte-for-byte (same
// OFFSET / PRIME constants, byte-XOR-then-multiply inner loop) so C++
// and Rust producers share an id namespace. The canonical key shape is
//
//     code | stage | file_or_empty <0x1f> messageFragment
//
// matching `compute_id` in the same Rust module. Returned string
// satisfies the schema's `^fnv1a:[0-9a-f]{16}$` pattern.
//
// Note: the Rust producer feeds *structured* `key_fragments` into the
// hash (template/param/declared etc.), while the C++ producer in W1+W3
// derives its key from the rendered message text — schema-valid but not
// byte-equivalent to Rust's id for the same logical error. Lifting
// structured fields onto the C++ subtypes is W3+ scope (RFC §1 Q5)
// and would be the only way to make ids byte-match across sides.
std::string
computeFnv1aDiagnosticId(std::string_view code, std::string_view stage,
                         std::string_view file,
                         std::string_view messageFragment);

}  // namespace SCE::parsing
