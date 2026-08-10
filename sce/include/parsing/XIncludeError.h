// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/Diagnostic.h"

#include <nlohmann/json.hpp>

#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

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
// `XIncludeExpansionError` implements `SCE::parsing::Diagnostic`
// (§wire-W3). Each subtype overrides `code()` with its
// `xml/xinclude-*` wire string; `stage()`, `location()` and
// `to_json()` come from the shared base, and the location the §wire-W3
// design pin reserved is now stamped — (row, col) at the throw site,
// the document path at the expander boundary.
//
// Each subtype takes the Rust variant's *fields*, not a rendered
// message: the constructor renders `what()` and declares the `id` key
// fragments from the same values, so the two cannot drift and a throw
// site cannot supply one without the other. Before this, throw sites
// passed a formatted string and the id was hashed from that string —
// which no other producer can reproduce, while `id` is the contract's
// dedup key (SCE_ERROR_CONTRACT.md §2.1). The rendered text mirrors
// the Rust `#[error(...)]` attribute on the matching variant
// verbatim, and `tests/parsing/CrossProducerDiagnosticId_test.cpp`
// runs both producers over one document to keep it that way.

class XIncludeExpansionError : public std::runtime_error, public Diagnostic {
public:
    // `Diagnostic` interface. Subtypes override `code()` with their
    // `xml/xinclude-*` wire string; `stage()`, `location()` and
    // `to_json()` are shared on the base because every XInclude
    // diagnostic carries the same envelope.
    std::string_view code() const noexcept override = 0;

    // Every `xml/xinclude-*` `DiagnosticCode` reports the `xml` stage
    // in the Rust authority — see `DiagnosticCode::stage()` in
    // `sce-build/src/forge/diagnostic.rs`.
    std::string_view stage() const noexcept override {
        return "xml";
    }

    nlohmann::ordered_json to_json() const override;

    // Rethrow this diagnostic attributed to `outerHref` — the
    // `<xi:include>` the operator can actually see and edit, rather
    // than a href reached transitively inside the included chain.
    // Mirrors Rust `remap_nested`.
    //
    // Pure rather than a defaulted "keep myself", because which
    // variants re-attribute and which keep their own identity is a
    // per-variant decision the Rust authority makes explicitly; a
    // default would let a new leaf inherit the wrong half of it
    // silently. The rethrow lives on the leaf because `throw *this`
    // in a base body would slice the dynamic type away.
    [[noreturn]] virtual void rethrowAttributedTo(const std::string &outerHref) const = 0;

protected:
    // `actual` is the wire field a repair tool acts on without parsing
    // the message — the offending href for most variants, the rejected
    // feature for `Unsupported`, absent for the two that name nothing.
    XIncludeExpansionError(std::string message, std::vector<std::string> keyFragments,
                           std::optional<std::string> actual)
        : std::runtime_error(std::move(message)), Diagnostic(std::move(keyFragments)), actual_(std::move(actual)) {}

    // Optional repair proposal, appended after `actual` in the
    // schema's key order. Only `MissingHref` has one; the default
    // emits nothing rather than an empty object, because `fix` absent
    // and `fix` empty mean different things to a consumer.
    virtual void appendFix(nlohmann::ordered_json &out) const;

private:
    std::optional<std::string> actual_;
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
//
// A unit variant carries no key fragments, so its id hashes to
// `code|stage|file` with no unit separator at all.
class XIncludeMissingHref : public XIncludeExpansionError {
public:
    XIncludeMissingHref()
        : XIncludeExpansionError("<xi:include> missing or empty `href` attribute", {}, std::nullopt) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-missing-href";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeMissingHref>(*this);
    }

    // Keeps its own identity: the missing `href` is on an element
    // inside the included file, and naming the outer include would
    // point the operator at a document that is not the one to edit.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
    }

protected:
    void appendFix(nlohmann::ordered_json &out) const override;
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
    XIncludeNotFound(std::string href, std::string searched)
        : XIncludeExpansionError("<xi:include href=\"" + href + "\">: file not found (searched: " + searched + ")",
                                 {href, searched}, href),
          searched_(std::move(searched)) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-not-found";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeNotFound>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerHref) const override {
        throw XIncludeNotFound(outerHref, searched_);
    }

private:
    std::string searched_;
};

// Resolved fragment file exists but could not be read — permission
// denied, I/O failure, or any other filesystem-level open/read
// failure raised by `std::ifstream`. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::ReadError` (fields
// `href`, `source: std::io::Error`) and maps 1:1 to the Rust
// `xml/xinclude-read-error` `DiagnosticCode`. Classified
// "Diagnostic-only" in the acceptance doc: an infrastructure
// failure the SCXML author cannot prevent by editing the document.
//
// `detail` renders into the message but stays out of the key
// fragments, exactly as on the Rust side: the underlying I/O text is
// the platform's, so hashing it would make the id platform-specific
// for an error the href already identifies.
class XIncludeReadError : public XIncludeExpansionError {
public:
    XIncludeReadError(std::string href, std::string detail)
        : XIncludeExpansionError("<xi:include href=\"" + href + "\">: cannot read: " + detail, {href}, href),
          detail_(std::move(detail)) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-read-error";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeReadError>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerHref) const override {
        throw XIncludeReadError(outerHref, detail_);
    }

private:
    std::string detail_;
};

// A cycle was detected in the `<xi:include>` graph — expanding the
// referenced fragment would revisit a file already on the recursion
// stack. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::Cycle` (fields `href`,
// `chain`) and maps 1:1 to the Rust `xml/xinclude-cycle`
// `DiagnosticCode`. The message renders the full chain as
// `outer → middle → inner` so the operator can see which file
// eventually loops back — same rendering convention as Rust's
// `render_chain`, which keeps the discriminant key stable for consumer
// dispatch across the language boundary.
class XIncludeCycle : public XIncludeExpansionError {
public:
    XIncludeCycle(std::string href, std::string chain)
        : XIncludeExpansionError("<xi:include href=\"" + href + "\">: cycle detected (" + chain + ")", {href, chain},
                                 href) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-cycle";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeCycle>(*this);
    }

    // Keeps its own identity: the chain it already renders names every
    // file involved, so the outer href would add nothing and lose the
    // href that closed the loop.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
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
    explicit XIncludeTooDeep(unsigned limit)
        : XIncludeExpansionError("<xi:include> nesting exceeds depth limit of " + std::to_string(limit),
                                 {std::to_string(limit)}, std::nullopt) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-too-deep";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeTooDeep>(*this);
    }

    // Keeps its own identity: the limit is a property of the expander,
    // not of any one include.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
    }
};

// Source document or included fragment failed to parse as
// well-formed XML — pugixml reported a parse error (mismatched
// tags, malformed entity, encoding failure, premature EOF, etc.).
// Mirrors `sce-build/src/xinclude.rs::XIncludeError::Malformed`
// (fields `href`, `detail`) and maps 1:1 to the Rust
// `xml/xinclude-malformed` `DiagnosticCode`. Covers both the outer
// document's initial parse and any nested fragment's reparse —
// Rust folds them at the variant level so the C++ side does too, and
// re-attributes a nested failure to the href that pulled the fragment
// in (`attribute_to_href` on the Rust side) so the operator is told
// which `<xi:include>` to look at rather than only that "a document"
// was malformed.
//
// `detail` is the XML engine's own parse text — roxmltree on the Rust
// side, pugixml here — so it renders into the message but stays out
// of the key fragments: hashing it would make the id engine-specific
// for an error the href already identifies.
class XIncludeMalformed : public XIncludeExpansionError {
public:
    XIncludeMalformed(std::string href, std::string detail)
        : XIncludeExpansionError("<xi:include href=\"" + href + "\">: included file is malformed: " + detail, {href},
                                 href),
          detail_(std::move(detail)) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-malformed";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeMalformed>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerHref) const override {
        throw XIncludeMalformed(outerHref, detail_);
    }

private:
    std::string detail_;
};

// `<xi:include>` requested an XInclude feature that the pugixml
// runtime does not implement: `parse="text"`, `xpointer=...`, or a
// `<xi:fallback>` alternative-content child. Mirrors
// `sce-build/src/xinclude.rs::XIncludeError::Unsupported` (fields
// `href`, `feature`) and maps 1:1 to the Rust
// `xml/xinclude-unsupported` `DiagnosticCode`. The C++ expander
// preserves this rejection set so the AOT and Interpreter pipelines agree on
// which inputs are accepted.
//
// `actual` carries the feature rather than the href, matching the
// Rust payload: the href is not what the consumer would act on here.
class XIncludeUnsupported : public XIncludeExpansionError {
public:
    XIncludeUnsupported(std::string href, std::string feature)
        : XIncludeExpansionError("<xi:include href=\"" + href + "\">: unsupported feature: " + feature, {href, feature},
                                 feature),
          feature_(std::move(feature)) {}

    std::string_view code() const noexcept override {
        return "xml/xinclude-unsupported";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<XIncludeUnsupported>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerHref) const override {
        throw XIncludeUnsupported(outerHref, feature_);
    }

private:
    std::string feature_;
};

}  // namespace SCE::parsing
