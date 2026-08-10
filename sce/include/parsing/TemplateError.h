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

// C++ exception hierarchy for `sce:template` preprocessor
// failures thrown by `PugiXMLDocument::processSceTemplate`.
//
// Each subtype maps one-to-one to a Rust
// `sce-build/src/template.rs::TemplateError` variant and the
// `xml/template-*` DiagnosticCode it emits. The 1:1 mapping is
// pinned by the drift test
// `cpp_template_subtypes_match_rust_diagnostic_codes` in
// `sce-build/src/template.rs::tests`, which counts declarations
// and compares names between the two sides so a commit that adds
// or renames a variant on one side without updating the other
// surfaces as red rather than silent cross-language drift.
//
// `TemplateError` implements `SCE::parsing::Diagnostic` (§wire-W1).
// Each subtype overrides `code()` with its `xml/template-*` wire
// string; the shared base contributes `stage()`, `location()` and
// `to_json()` (the v1 schema NDJSON record).
//
// Each subtype takes the Rust variant's *fields*, not a rendered
// message: the constructor renders `what()` and declares the `id` key
// fragments from the same values, so a throw site cannot supply one
// without the other and the two cannot drift. The rendered text
// mirrors the Rust `#[error(...)]` attribute on the matching variant
// verbatim; `tests/parsing/CrossProducerDiagnosticId_test.cpp` runs
// both producers over one document to keep it that way.

class TemplateError : public std::runtime_error, public Diagnostic {
public:
    // `Diagnostic` interface. Subtypes override `code()` with their
    // `xml/template-*` wire string; `stage()`, `location()` and
    // `to_json()` are shared on the base because every template
    // diagnostic carries the same envelope.
    std::string_view code() const noexcept override = 0;

    // Every `xml/template-*` `DiagnosticCode` reports the `xml` stage
    // in the Rust authority — see `DiagnosticCode::stage()` in
    // `sce-build/src/forge/diagnostic.rs`.
    std::string_view stage() const noexcept override {
        return "xml";
    }

    nlohmann::ordered_json to_json() const override;

    // Rethrow attributed to the `<sce:use>` the operator can see,
    // rather than a template reached transitively inside the expanded
    // chain. Mirrors Rust `template.rs::remap_nested`; pure for the
    // same reason its XInclude twin is (see `XIncludeError.h`).
    [[noreturn]] virtual void rethrowAttributedTo(const std::string &outerTemplate) const = 0;

protected:
    TemplateError(std::string message, std::vector<std::string> keyFragments, std::optional<std::string> actual)
        : std::runtime_error(std::move(message)), Diagnostic(std::move(keyFragments)), actual_(std::move(actual)) {}

    // Optional repair proposal, appended after `actual`. Two leaves
    // carry one; the default emits nothing rather than an empty
    // object, because `fix` absent and `fix` empty mean different
    // things to a consumer.
    virtual void appendFix(nlohmann::ordered_json &out) const;

private:
    std::optional<std::string> actual_;
};

// `<sce:use>` supplied an attribute that does not match any
// `<sce:param name="...">` declaration on the target template.
// Mirrors `sce-build/src/template.rs::TemplateError::UnknownParam`
// (fields `template`, `param`, `declared`) and maps 1:1 to the
// Rust `xml/template-unknown-param` DiagnosticCode. Consumer
// dispatch in the AOT path keys on the message naming both the
// template and the offending parameter; the C++ runtime message
// follows the same shape so downstream repair heuristics stay
// text-agnostic to the language boundary.
class TemplateUnknownParam : public TemplateError {
public:
    TemplateUnknownParam(std::string templateHref, std::string param, std::string declared)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: unknown parameter \'" + param +
                            "\' (declared: " + declared + ")",
                        {templateHref, param, declared}, param),
          param_(std::move(param)), declared_(std::move(declared)) {}

    std::string_view code() const noexcept override {
        return "xml/template-unknown-param";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateUnknownParam>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerTemplate) const override {
        throw TemplateUnknownParam(outerTemplate, param_, declared_);
    }

private:
    std::string param_;
    std::string declared_;
};

// `<sce:use>` omitted a `<sce:param required="true">` that the
// target template declares. Mirrors
// `sce-build/src/template.rs::TemplateError::MissingParam`
// (fields `template`, `param`) and maps 1:1 to the Rust
// `xml/template-missing-param` DiagnosticCode.
class TemplateMissingParam : public TemplateError {
public:
    TemplateMissingParam(std::string templateHref, std::string param)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: missing required parameter \'" + param + "\'",
                        {templateHref, param}, param),
          param_(std::move(param)) {}

    std::string_view code() const noexcept override {
        return "xml/template-missing-param";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateMissingParam>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerTemplate) const override {
        throw TemplateMissingParam(outerTemplate, param_);
    }

protected:
    void appendFix(nlohmann::ordered_json &out) const override;

private:
    std::string param_;
};

// A cycle was detected in the template inclusion graph — expanding
// the referenced template would revisit a file already on the
// recursion stack. Mirrors
// `sce-build/src/template.rs::TemplateError::Cycle`
// (fields `template`, `chain`) and maps 1:1 to the Rust
// `xml/template-cycle` DiagnosticCode. The message renders the
// full chain as `outer → middle → inner` so the operator can see
// which file eventually loops back — same rendering convention as
// Rust's `render_chain`, which keeps the discriminant key stable
// for consumer dispatch across the language boundary.
class TemplateCycle : public TemplateError {
public:
    TemplateCycle(std::string templateHref, std::string chain)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: cycle detected (" + chain + ")",
                        {templateHref, chain}, templateHref) {}

    std::string_view code() const noexcept override {
        return "xml/template-cycle";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateCycle>(*this);
    }

    // Keeps its own identity: the rendered chain already names every
    // file involved, and the outer template would lose the reference
    // that closed the loop.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
    }
};

// Recursion exceeded
// `SCE::parsing::MAX_TEMPLATE_DEPTH`, catching pathological (but
// acyclic) template chains where each file pulls in another
// without looping back. Mirrors
// `sce-build/src/template.rs::TemplateError::TooDeep`
// (field `limit`) and maps 1:1 to the Rust
// `xml/template-too-deep` DiagnosticCode. The message names the
// depth limit so the operator can see the enforced bound without
// reading the header.
class TemplateTooDeep : public TemplateError {
public:
    explicit TemplateTooDeep(int limit)
        : TemplateError("<sce:use> template nesting exceeds depth limit of " + std::to_string(limit),
                        {std::to_string(limit)}, std::nullopt) {}

    std::string_view code() const noexcept override {
        return "xml/template-too-deep";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateTooDeep>(*this);
    }

    // Keeps its own identity: the limit is a property of the expander,
    // not of any one `<sce:use>`.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
    }
};

// `<sce:use template="...">` named a file that could not be
// located against the caller's base directory (nor any fallback
// the resolver searched). Mirrors
// `sce-build/src/template.rs::TemplateError::NotFound`
// (fields `template`, `searched`) and maps 1:1 to the Rust
// `xml/template-not-found` DiagnosticCode. The message carries the
// referenced template name and the searched paths so the operator
// can pick the right one without guessing.
class TemplateNotFound : public TemplateError {
public:
    TemplateNotFound(std::string templateHref, std::string searched)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: file not found (searched: " + searched + ")",
                        {templateHref, searched}, templateHref),
          searched_(std::move(searched)) {}

    std::string_view code() const noexcept override {
        return "xml/template-not-found";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateNotFound>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerTemplate) const override {
        throw TemplateNotFound(outerTemplate, searched_);
    }

private:
    std::string searched_;
};

// Resolved template file exists but could not be read —
// permission denied, I/O failure, or a libxml/pugixml I/O-level
// load failure that is NOT a malformed-document class. Mirrors
// `sce-build/src/template.rs::TemplateError::ReadError`
// (fields `template`, `source: std::io::Error`) and maps 1:1 to
// the Rust `xml/template-read-error` DiagnosticCode. Classified
// "Diagnostic-only" in the acceptance doc: an infrastructure
// failure the SCXML author cannot prevent by editing the document.
// `detail` renders into the message but stays out of the key
// fragments, exactly as on the Rust side: the underlying I/O text is
// the platform\'s, so hashing it would make the id platform-specific
// for a failure the template name already identifies.
class TemplateReadError : public TemplateError {
public:
    TemplateReadError(std::string templateHref, std::string detail)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: cannot read: " + detail, {templateHref},
                        templateHref),
          detail_(std::move(detail)) {}

    std::string_view code() const noexcept override {
        return "xml/template-read-error";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateReadError>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerTemplate) const override {
        throw TemplateReadError(outerTemplate, detail_);
    }

private:
    std::string detail_;
};

// Template file was read but either (a) is not well-formed XML,
// (b) its root element is not `<sce:template>`, or (c) a
// `<sce:param>` declaration is ill-formed (missing `name`,
// invalid name pattern, duplicate name, bad `required` value, or
// both `required="true"` and `default="..."` declared
// simultaneously). Mirrors
// `sce-build/src/template.rs::TemplateError::Malformed`
// (fields `template`, `detail`) and maps 1:1 to the Rust
// `xml/template-malformed` DiagnosticCode. The three repair
// surfaces share a single subtype because each points at the
// template file itself; call-site attribute omissions ride
// `TemplateMissingAttribute` instead so repair consumers can
// dispatch call-site vs file-side fixes without text parsing.
// `detail` renders into the message but stays out of the key
// fragments: SCE authors it for most branches, but two of them carry
// the XML engine\'s own parse text, and a variant\'s key shape has to
// hold for every instance of it. The template name is what a consumer
// dedups on; the reason travels in `message`. Mirrors the Rust arm.
class TemplateMalformed : public TemplateError {
public:
    TemplateMalformed(std::string templateHref, std::string detail)
        : TemplateError("<sce:use template=\"" + templateHref + "\">: template is malformed: " + detail, {templateHref},
                        templateHref),
          detail_(std::move(detail)) {}

    std::string_view code() const noexcept override {
        return "xml/template-malformed";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateMalformed>(*this);
    }

    [[noreturn]] void rethrowAttributedTo(const std::string &outerTemplate) const override {
        throw TemplateMalformed(outerTemplate, detail_);
    }

private:
    std::string detail_;
};

// `<sce:use>` is missing the required `template` attribute, or
// the attribute is present but its value is the empty string
// (empty never resolves against any base directory, so the
// failure is classified at the call site rather than as
// `TemplateNotFound`). Mirrors
// `sce-build/src/template.rs::TemplateError::MissingTemplateAttribute`
// (unit variant) and maps 1:1 to the Rust
// `xml/template-missing-attribute` DiagnosticCode. The
// separation from `TemplateMalformed` keeps
// `xml/template-malformed` focused on file-side issues so repair
// consumers dispatching on the code do not have to re-parse the
// message body to pick the right fix kind.
// A unit variant carries no key fragments, so its id hashes to
// `code|stage|file` with no unit separator at all.
class TemplateMissingAttribute : public TemplateError {
public:
    TemplateMissingAttribute() : TemplateError("<sce:use> missing required `template` attribute", {}, std::nullopt) {}

    std::string_view code() const noexcept override {
        return "xml/template-missing-attribute";
    }

    std::unique_ptr<Diagnostic> clone() const override {
        return std::make_unique<TemplateMissingAttribute>(*this);
    }

    // Keeps its own identity: the omission is on a `<sce:use>` inside
    // the included template, and naming the outer one would point the
    // operator at a document that is not the one to edit.
    [[noreturn]] void rethrowAttributedTo(const std::string &) const override {
        throw *this;
    }

protected:
    void appendFix(nlohmann::ordered_json &out) const override;
};

}  // namespace SCE::parsing
