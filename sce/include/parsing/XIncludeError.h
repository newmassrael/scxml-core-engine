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

namespace SCE::parsing {

// C++ exception hierarchy for `<xi:include>` preprocessor failures
// thrown by `SCE::parsing::expandStringX` (string-level XInclude
// expander mirroring `sce-build/src/xinclude.rs::expand`).
//
// Each subtype maps one-to-one to a Rust
// `sce-build/src/xinclude.rs::XIncludeError` variant and the
// `xml/xinclude-*` `DiagnosticCode` it emits. The 1:1 mapping is
// pinned by the drift test
// `cpp_xinclude_subtypes_match_rust_diagnostic_codes` in
// `sce-build/src/xinclude.rs::tests`, which counts declarations and
// compares names between the two sides so a commit that adds or
// renames a variant on one side without updating the other surfaces
// as red rather than silent cross-language drift. Mirrors the W1
// `TemplateError` pattern in `parsing/TemplateError.h`.
//
// W3 (`claudedocs/rfc-sce-diagnostic-wire-unification.md` §wire-W3) makes
// `XIncludeExpansionError` implement `SCE::parsing::Diagnostic`. Each
// subtype overrides `code()` with its `xml/xinclude-*` wire string;
// the shared base contributes `location()` (currently always
// `nullopt` — expander throw sites do not stamp source locations
// today; deferred until a consumer asks per RFC §wire-W3 design pin) and
// `to_json()` (the v1 schema NDJSON record). Subtype field shape is
// unchanged — the existing message-string ctor stays the throw-site
// API so the 10 expander throw sites read as before.

class XIncludeExpansionError : public std::runtime_error,
                               public Diagnostic {
public:
    using std::runtime_error::runtime_error;

    // Attach a `SourcePos` to this error. Currently unused by the
    // expander throw sites — the `<xi:include>` element's pre-
    // expansion (row, col) is embedded in the message text only.
    // Reserved for the location-stamping milestone (RFC §wire-W3 design
    // pin "(ii) stamps with the throw-site's pre-expansion outer-
    // document (row, col)") so consumers do not need to reparse
    // message text for coordinates once stamping lands.
    void setLocation(SourcePos pos) {
        location_ = std::move(pos);
    }

    // `Diagnostic` interface. Subtypes override `code()` with their
    // `xml/xinclude-*` wire string; `location()` and `to_json()` are
    // shared on the base because every XInclude diagnostic carries
    // the same envelope (stage = "xml", id derived through the same
    // canonical key shape, message read from `runtime_error::what()`).
    std::string_view code() const noexcept override = 0;
    const std::optional<SourcePos> &location() const noexcept override {
        return location_;
    }
    nlohmann::ordered_json to_json() const override;

private:
    std::optional<SourcePos> location_;
};

// `<xi:include>` is missing the required `href` attribute, or the
// attribute is present but its value is the empty string (an empty
// href never resolves against any base directory, so the failure is
// classified at the call site rather than as `XIncludeNotFound`).
// Mirrors `sce-build/src/xinclude.rs::XIncludeError::MissingHref`
// (unit variant) and maps 1:1 to the Rust
// `xml/xinclude-missing-href` `DiagnosticCode`. Both the missing-
// href and empty-href shapes collapse onto this single subtype
// because Rust's authority folds them at the variant level.
class XIncludeMissingHref : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-missing-href";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeMissingHref>(*this);
    }
};

// `<xi:include href="...">` named a file that could not be located
// against the caller's base directory (nor any fallback the
// resolver searched: absolute, base-relative, current working
// directory). Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::NotFound` (fields
// `href`, `searched`) and maps 1:1 to the Rust
// `xml/xinclude-not-found` `DiagnosticCode`. The message carries
// the offending href and the searched-paths trail so the operator
// can pick the right one without guessing.
class XIncludeNotFound : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-not-found";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeNotFound>(*this);
    }
};

// Resolved fragment file exists but could not be read — permission
// denied, I/O failure, or any other filesystem-level open/read
// failure raised by `std::ifstream`. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::ReadError` (fields
// `href`, `source: std::io::Error`) and maps 1:1 to the Rust
// `xml/xinclude-read-error` `DiagnosticCode`. Classified
// "Diagnostic-only" in the acceptance doc: an infrastructure
// failure the SCXML author cannot prevent by editing the document.
class XIncludeReadError : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-read-error";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeReadError>(*this);
    }
};

// A cycle was detected in the `<xi:include>` graph — expanding the
// referenced fragment would revisit a file already on the recursion
// stack. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::Cycle` (fields `href`,
// `chain`) and maps 1:1 to the Rust `xml/xinclude-cycle`
// `DiagnosticCode`. The message renders the full chain as
// `outer → middle → inner` so the operator can see which file
// eventually loops back — same rendering convention as Rust's
// `render_chain`, which keeps the discriminant key stable for agent
// dispatch across the language boundary.
class XIncludeCycle : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-cycle";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeCycle>(*this);
    }
};

// Recursion exceeded `SCE::parsing::MAX_XINCLUDE_DEPTH`, catching
// pathological (but acyclic) include chains where each fragment
// pulls in another without looping back. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::TooDeep` (field
// `limit`) and maps 1:1 to the Rust `xml/xinclude-too-deep`
// `DiagnosticCode`. The message names the depth limit so the
// operator can see the enforced bound without reading the header.
class XIncludeTooDeep : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-too-deep";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeTooDeep>(*this);
    }
};

// Source document or included fragment failed to parse as
// well-formed XML — pugixml reported a parse error (mismatched
// tags, malformed entity, encoding failure, premature EOF, etc.).
// Mirrors `sce-build/src/xinclude.rs::XIncludeError::Malformed`
// (fields `href`, `detail`) and maps 1:1 to the Rust
// `xml/xinclude-malformed` `DiagnosticCode`. Covers both the outer
// document's initial parse and any nested fragment's reparse —
// Rust folds them at the variant level so the C++ side does too.
class XIncludeMalformed : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-malformed";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeMalformed>(*this);
    }
};

// `<xi:include>` requested an XInclude feature that the pugixml
// runtime does not implement: `parse="text"`, `xpointer=...`, or a
// `<xi:fallback>` alternative-content child. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::Unsupported` (fields
// `href`, `feature`) and maps 1:1 to the Rust
// `xml/xinclude-unsupported` `DiagnosticCode`. Phase X preserves
// this rejection set so the AOT and Interpreter pipelines agree on
// which inputs are accepted.
class XIncludeUnsupported : public XIncludeExpansionError {
public:
    using XIncludeExpansionError::XIncludeExpansionError;
    std::string_view code() const noexcept override {
        return "xml/xinclude-unsupported";
    }
    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeUnsupported>(*this);
    }
};

}  // namespace SCE::parsing
