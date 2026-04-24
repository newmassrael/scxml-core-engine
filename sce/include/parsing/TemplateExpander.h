// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"
#include "parsing/TemplateError.h"

#include <pugixml.hpp>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <string>
#include <string_view>
#include <unordered_map>
#include <vector>

// String-level `<sce:use>` / `<sce:template>` expander. Mirrors
// `sce-build/src/template.rs` line-for-line where practical, so a
// document accepted by the AOT path yields a byte-equivalent
// post-preprocessor document on the C++ Interpreter side. The
// port is string-level (rather than DOM-mutating) because
// `SCE::parsing::PositionMap` keys diagnostics to byte offsets in
// the expanded output — a DOM-mutation pipeline cannot produce a
// stable byte-offset coordinate space (RFC §1 Q1 Hybrid choice in
// `claudedocs/rfc-sce-template-phase-c.md`). The companion
// `PugiXMLDocument::processSceTemplate` drives the expander with
// post-XInclude serialised bytes, then re-parses the expander
// output into the DOM.
//
// Phase C P2 in `claudedocs/rfc-sce-template-phase-c.md` §3 P2
// delivers this expander; P1 shipped the `PositionMap` primitive
// this expander produces.

namespace SCE::parsing {

// Result of a successful template expansion: the expanded source
// text and a `PositionMap` keyed against that text's bytes.
// Mirrors Rust's `(String, PositionMap)` return from
// `sce-build/src/template.rs::expand`.
struct TemplateExpandResult {
    std::string expanded_text;
    PositionMap positions;
};

// String-level `<sce:use>` expansion entry point. Mirrors
// `sce-build/src/template.rs::expand` structurally.
//
// `content` is the post-XInclude source bytes. `selfPath` is the
// filesystem path of that content — seeds cycle detection and
// serves as the caller-file path for `CallSiteOrigin` entries in
// the produced `PositionMap`. `baseDir` is the directory that
// `<sce:use template="relative">` resolves against — typically
// `selfPath`'s parent directory. In-memory callers may pass an
// empty `selfPath`; the cycle-detection stack then begins empty
// and trips only once at least one template file has been
// loaded.
//
// Successful short-circuit: when `content` contains no `sce:use`
// substring, returns `{std::string(content),
// PositionMap::identity(selfPath, content)}` without parsing.
//
// Failure modes throw one of the `TemplateError` subtypes defined
// in `parsing/TemplateError.h`; each subtype maps 1:1 to a Rust
// `xml/template-*` `DiagnosticCode`. P2 leaves
// `TemplateError::location()` unpopulated on most throw sites — P5
// wires `PositionMap::lookup`-derived `SourcePos` values through
// to the thrown subtype.
TemplateExpandResult expandString(std::string_view content,
                                  std::string_view selfPath,
                                  std::string_view baseDir);

namespace detail {

// Byte range in the caller source. `start` is the `<`'s position,
// `end` is one past the element's final `>`.
struct ByteRange {
    std::size_t start;
    std::size_t end;
};

// Declaration of a single `<sce:param>` inside a template file.
// Mirrors Rust `sce-build/src/template.rs::ParamDecl`. `hasDefault`
// separates "declared with default" from "declared without default"
// so the downstream binder can apply the RFC §3.1 three-state
// fallback (caller value > default > empty) without an extra
// Optional wrapper.
struct ParamDecl {
    std::string name;
    bool required = false;
    bool hasDefault = false;
    std::string defaultValue;
};

// One emitted byte range plus its origin. Ordered output from
// `applySubstitutionWithTracking`; the caller shifts the offsets
// by the body splice offset when composing into a larger
// PositionMap.
struct SubstitutionEntry {
    std::size_t out_start;
    std::size_t out_end;
    Origin origin;
};

// Result of a single body-level `{$name}` substitution pass.
// Mirrors Rust's tuple return from
// `apply_substitution_with_tracking` — the rendered body plus a
// contiguous vector of origin-tagged output ranges covering every
// emitted byte.
struct SubstitutionResult {
    std::string substituted;
    std::vector<SubstitutionEntry> entries;
};

// Single-pass lexical `{$name}` substitution with per-region
// origin tracking. Mirrors
// `sce-build/src/template.rs::apply_substitution_with_tracking`
// line-for-line — atomic (replacements do not cascade), undeclared
// `{$name}` tokens emitted verbatim as template-file bytes, empty
// param values skipped (no zero-length `CallSite` entry emitted).
//
// `body` is the span of the template file being substituted.
// `bodySourceOffset` is that span's start offset inside the
// template-file source (so a File-origin entry records
// `source_offset = bodySourceOffset + body-local offset`).
// `templatePath` labels the template file for File-origin entries.
// `params` is the bound-name → literal-value map; missing names
// in this map are treated as undeclared and their tokens are
// emitted verbatim. `(callerFile, callerRow, callerCol)` is the
// depth-1 collapse point RFC §6.3 Q3 mandates — every byte of a
// successful substitution gets the *same* (callerRow, callerCol)
// regardless of where inside the substituted value it lands.
SubstitutionResult applySubstitutionWithTracking(
    std::string_view body, std::size_t bodySourceOffset,
    const std::filesystem::path &templatePath,
    const std::unordered_map<std::string, std::string> &params,
    const std::filesystem::path &callerFile, std::uint32_t callerRow,
    std::uint32_t callerCol);

// Parse a `<sce:param>` declaration node. Validates the `name`
// pattern (XSD `paramNameType` via
// `SCE::parsing::is_valid_param_name`), the mutual exclusion of
// `required="true"` and `default="..."`, and the boolean
// `required` literal. Throws `TemplateMalformed` with the
// offending template's href baked into the message for every
// failure path. Mirrors
// `sce-build/src/template.rs::parse_param_decl`.
ParamDecl parseParamDecl(pugi::xml_node node,
                         std::string_view templateHref);

// Collect caller `<sce:use>` bindings. Every attribute other
// than the reserved `template` and `xmlns` / `xmlns:*` namespace
// declarations contributes one binding keyed by the attribute's
// local name. Mirrors
// `sce-build/src/template.rs::collect_use_bindings` but filters
// the pugixml-exposed namespace declarations that roxmltree
// hides at the parser layer.
std::unordered_map<std::string, std::string> collectUseBindings(
    pugi::xml_node useNode);

// Extract body byte ranges of a post-substitution, post-recursion
// template document. Every non-`<sce:param>` child of the
// `<sce:template>` root contributes one byte range inside
// `expandedTemplate`; the caller splices each range into the
// outer document in place of the `<sce:use>` node and composes
// the matching `PositionMap` slice into the outer map. Throws
// `TemplateMalformed` when `expandedTemplate` is not well-formed
// or its root is not `<sce:template>`. Mirrors
// `sce-build/src/template.rs::extract_template_body_ranges`.
std::vector<ByteRange> extractTemplateBodyRanges(
    std::string_view expandedTemplate, std::string_view templateHref);

// Find the byte offset one-past the closing `>` of the XML element
// starting at `start` in `source`. `tagName` is the element's full
// prefixed local name (e.g. `"sce:use"`). Handles self-closing
// elements, open-close pairs, and skips `<!--…-->` / `<![CDATA[…]]>`
// sections + quoted attribute values inside the body. Depth-tracks
// nested elements with the same `tagName` so the returned offset is
// the *matching* close, not the first `</tagName>` encountered.
//
// Precondition: `source.substr(start)` starts with `<tagName` and
// the enclosing document is well-formed (the caller has already
// validated it via pugixml). On malformed input (e.g. runaway
// open-close pairing) returns `source.size()`, letting the call
// site surface a `TemplateMalformed` instead of this scanner
// asserting.
std::size_t findElementEnd(std::string_view source, std::size_t start,
                           std::string_view tagName);

// Collect byte ranges of every top-level `<sce:use>` element in
// `source`. Top-level means "not nested inside another `<sce:use>`"
// — mirrors Rust's `collect_uses` walker in
// `sce-build/src/template.rs`. Parses `source` via pugixml
// internally to walk the element structure once.
//
// Throws `TemplateMalformed` when `source` is not well-formed XML.
// Returns an empty vector when `source` contains no `<sce:use>`.
std::vector<ByteRange> collectTopLevelSceUseRanges(std::string_view source);

}  // namespace detail

}  // namespace SCE::parsing
