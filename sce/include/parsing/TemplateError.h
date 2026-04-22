// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <stdexcept>
#include <string>

namespace SCE::parsing {

// C++ exception hierarchy for `sce:template` preprocessor
// failures thrown by `PugiXMLDocument::processSceTemplate`.
//
// Phase B tracked in `claudedocs/rfc-sce-template-phase-b.md`.
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
// Catch policy: `SCXMLParser::parseFile` and `parseContent`
// already catch `std::exception` broadly — `TemplateError` is a
// `std::runtime_error` subclass, so existing catch sites collect
// the message via `addError` without additional plumbing until a
// future adapter distinguishes subtypes by rtti.

class TemplateError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

// `<sce:use>` supplied an attribute that does not match any
// `<sce:param name="...">` declaration on the target template.
// Mirrors `sce-build/src/template.rs::TemplateError::UnknownParam`
// (fields `template`, `param`, `declared`) and maps 1:1 to the
// Rust `xml/template-unknown-param` DiagnosticCode. Agent
// dispatch in the AOT path keys on the message naming both the
// template and the offending parameter; the C++ runtime message
// follows the same shape so downstream repair heuristics stay
// text-agnostic to the language boundary.
class TemplateUnknownParam : public TemplateError {
public:
    using TemplateError::TemplateError;
};

// `<sce:use>` omitted a `<sce:param required="true">` that the
// target template declares. Mirrors
// `sce-build/src/template.rs::TemplateError::MissingParam`
// (fields `template`, `param`) and maps 1:1 to the Rust
// `xml/template-missing-param` DiagnosticCode.
class TemplateMissingParam : public TemplateError {
public:
    using TemplateError::TemplateError;
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
// for agent dispatch across the language boundary.
class TemplateCycle : public TemplateError {
public:
    using TemplateError::TemplateError;
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
    using TemplateError::TemplateError;
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
    using TemplateError::TemplateError;
};

// Resolved template file exists but could not be read —
// permission denied, I/O failure, or a libxml/pugixml I/O-level
// load failure that is NOT a malformed-document class. Mirrors
// `sce-build/src/template.rs::TemplateError::ReadError`
// (fields `template`, `source: std::io::Error`) and maps 1:1 to
// the Rust `xml/template-read-error` DiagnosticCode. Classified
// "Diagnostic-only" in the acceptance doc: an infrastructure
// failure the SCXML author cannot prevent by editing the document.
class TemplateReadError : public TemplateError {
public:
    using TemplateError::TemplateError;
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
// `TemplateMissingAttribute` instead so repair agents can
// dispatch call-site vs file-side fixes without text parsing.
class TemplateMalformed : public TemplateError {
public:
    using TemplateError::TemplateError;
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
// agents dispatching on the code do not have to re-parse the
// message body to pick the right fix kind.
class TemplateMissingAttribute : public TemplateError {
public:
    using TemplateError::TemplateError;
};

}  // namespace SCE::parsing
