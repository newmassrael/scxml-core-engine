// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"

#include <nlohmann/json.hpp>

#include <cstdint>
#include <memory>
#include <optional>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

// Abstract base for typed diagnostics emitted on the SCE wire.
//
// Mirrors the Rust `sce-build::forge::diagnostic::Diagnostic` envelope
// pinned by `schemas/sce-diagnostic.v1.schema.json`. The C++ side adds
// a second independent conformer to that schema; the Rust side stays
// the authoritative producer and the C++ side conforms
// field-for-field.
//
// Concrete subtypes (W1 promotes `SCE::parsing::TemplateError`) provide:
//   - `code()` — the schema's wire code string, e.g. `"xml/template-cycle"`.
//                Must appear in the Rust `DiagnosticCode` registry; the
//                drift guard lives in `sce-build/src/forge/diagnostic.rs`.
//   - `location()` — author-source `SourcePos` when the throw site
//                    populated one; `nullopt` for
//                    diagnostics raised before position data is computed.
//   - `to_json()` — single NDJSON record matching v1 schema. Field shape
//                   follows the Rust struct member order so canonicalised
//                   byte-diffs against `--error-format=json` agree.
//
// `addError(string)` on `SCXMLParser` continues to coexist; the
// typed surface here is opt-in for consumers that want structured
// diagnostics without re-parsing log text. §wire-W1 contract.

namespace SCE::parsing {

// Where a diagnostic points, in the shape the wire carries it.
//
// Mirrors Rust `forge::diagnostic::Location`: `file` is always present
// on a location, `line` / `col` are optional — and the optionality is
// load-bearing, not defensive. Several producers know *which* document
// failed without knowing where in it: Rust stamps
// `Located::new(err, path, None, None)` on exactly those paths and the
// schema's `location` object requires `file` alone. `SourcePos` (from
// PositionMap.h) cannot express that shape — it always carries a
// (row, col), because a position map entry always resolves to one —
// and using it here forced every C++ producer that knew only the file
// to emit nothing at all, which is how the C++ records ended up with
// no `location` where Rust's carried `{"file": ...}`.
struct DiagnosticLocation {
    std::string file;
    std::optional<std::uint32_t> line;
    std::optional<std::uint32_t> col;
};

class Diagnostic {
public:
    virtual ~Diagnostic() = default;

    Diagnostic(const Diagnostic &) = default;
    Diagnostic(Diagnostic &&) = default;
    Diagnostic &operator=(const Diagnostic &) = default;
    Diagnostic &operator=(Diagnostic &&) = default;

    virtual std::string_view code() const noexcept = 0;

    // The wire `stage` this leaf's family reports. Declared per family
    // rather than derived by splitting `code()` on `/` because the Rust
    // prefix→stage table is not 1:1 (`cli/*` codes map to `cli`,
    // `mesh/deploy-*` to `mesh-deploy`), so a derivation would carry
    // per-prefix logic that has to grow in lockstep with each future
    // milestone.
    virtual std::string_view stage() const noexcept = 0;

    // Const-ref return matches the existing `TemplateError::location()`
    // accessor so callers like `SCXMLParser::parseFile`
    // can continue binding `const auto &loc = *diag.location();`
    // without touching a temporary's storage.
    const std::optional<DiagnosticLocation> &location() const noexcept {
        return location_;
    }

    // The structured values this diagnostic's `id` is derived from —
    // the C++ mirror of Rust's `DiagnosticPayload::key_fragments`. Two
    // producers reporting the same logical error agree on the id only
    // if they agree on these, so they are part of the leaf's contract
    // and not an implementation detail of `to_json()`.
    const std::vector<std::string> &keyFragments() const noexcept {
        return keyFragments_;
    }

    // Attach the throw site's (row, col). The file may still be
    // unknown at that point — the layer that owns the document path
    // stamps it afterwards through `stampFile`.
    void setPosition(std::uint32_t line, std::uint32_t col) {
        if (!location_.has_value()) {
            location_.emplace();
        }
        location_->line = line;
        location_->col = col;
    }

    // Name the document this diagnostic belongs to. Called by the layer
    // that knows the path the caller supplied — the mirror of Rust's
    // parse-boundary `Located::new(err, scxml_path, ...)`, where the
    // raising layer likewise does not know it. Keeps any (row, col) a
    // throw site already attached, and never overwrites a file a lower
    // layer knew more precisely (an `<xi:include>`'d fragment, say).
    //
    // This is the location stamping the §wire-W3 design pin reserved,
    // hoisted here from the XInclude family so all four families are
    // stamped the same way.
    void stampFile(std::string file) {
        if (!location_.has_value()) {
            location_.emplace();
        }
        if (location_->file.empty()) {
            location_->file = std::move(file);
        }
    }

    void setLocation(DiagnosticLocation loc) {
        location_ = std::move(loc);
    }

    virtual nlohmann::ordered_json to_json() const = 0;

    // Stable canonical-JSON string for byte-diff parity. Re-parses
    // `to_json()`'s output through `nlohmann::json` (alphabetical
    // key order via `std::map`) and dumps with `dump(-1, ' ', false)`
    // — no whitespace, no key-order coupling to the
    // `nlohmann::ordered_json` insertion order on the producer side.
    // Matches the canonicalisation Rust output is expected to round-
    // trip through for any cross-side byte-diff consumer (§wire-W2
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
    // §wire-W1 audit finding #1 closure (W2 deliverable).
    virtual std::unique_ptr<Diagnostic> clone() const = 0;

protected:
    // Every leaf declares the structured values its id derives from, at
    // construction. Required rather than settable because an optional
    // one is an omission waiting to happen: a leaf that forgot would
    // still compile, still emit a schema-valid record, and still carry
    // an id no other producer can reproduce — the exact failure this
    // constructor exists to make impossible.
    explicit Diagnostic(std::vector<std::string> keyFragments) : keyFragments_(std::move(keyFragments)) {}

    // Open a v1 record with the envelope every leaf shares: `v`, `id`,
    // `generator`, `code`, `stage`, in the Rust struct's field order so
    // canonicalised byte-diffs against `--error-format=json` agree.
    // The caller appends its own `message` and any optional fields.
    //
    // The id is computed HERE, from `code()`, `stage()`, the location's
    // file and the declared key fragments — a leaf cannot feed the hash
    // anything else. It used to be computed by each `to_json()` body
    // from the rendered message text, which is schema-valid and
    // reproducible within one producer, and which no other producer can
    // match: Rust hashes structured values, so the two sides emitted
    // different ids for one logical error while `id` is the contract's
    // dedup key (SCE_ERROR_CONTRACT.md §2.1).
    nlohmann::ordered_json beginRecord() const;

    // Append `location` when the diagnostic knows which document it
    // belongs to. A location whose `file` is still empty names nothing,
    // so it is omitted entirely — the shape Rust emits when no
    // `Located` wrapper carried a path.
    void appendLocation(nlohmann::ordered_json &out) const;

private:
    std::optional<DiagnosticLocation> location_;
    std::vector<std::string> keyFragments_;
};

}  // namespace SCE::parsing
