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
// M1 declared the base class and the `TemplateNotImplemented`
// sentinel. M2 added the first two named subtypes
// (`TemplateUnknownParam` + `TemplateMissingParam`). M3 (this
// header revision) adds the recursion-guard subtypes that arrive
// with recursive expansion: `TemplateCycle` + `TemplateTooDeep`.
// M4 adds the remaining four mirroring
// `sce-build/src/template.rs::TemplateError`:
//
//   M2: TemplateMissingParam, TemplateUnknownParam
//   M3: TemplateCycle, TemplateTooDeep                ← this revision
//   M4: TemplateNotFound, TemplateReadError,
//       TemplateMalformed, TemplateMissingAttribute
//   M5: delete TemplateNotImplemented (every shape is now a
//       proper named subtype; sentinel becomes dead code)
//
// Each future subtype maps one-to-one to a Rust DiagnosticCode.
// The 1:1 mapping is pinned at M4 landing by a drift test
// `cpp_template_subtypes_match_rust_diagnostic_codes` that
// counts variants and compares names between the two sides.
//
// Catch policy: `SCXMLParser::parseFile` and `parseContent`
// already catch `std::exception` broadly — `TemplateError` is a
// `std::runtime_error` subclass, so existing catch sites collect
// the message via `addError` without additional plumbing until
// M2 adds an adapter that distinguishes subtypes by rtti.

class TemplateError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

// M1/M2-era sentinel raised for any `<sce:use>` shape that has
// not yet been claimed by a proper named subtype. Carries a
// message naming the milestone that introduces proper handling,
// so an operator hitting the exception sees a pointed diagnostic
// rather than a silent mis-expansion or wrong-output bug.
//
// This class is removed in M5 once every shape has a proper
// named subtype — compile failure on removal is the signal that
// surfaces any latent skeleton code that must be cleaned up
// in the same commit.
class TemplateNotImplemented : public TemplateError {
public:
    using TemplateError::TemplateError;
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

}  // namespace SCE::parsing
