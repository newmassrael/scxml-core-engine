// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"
#include "parsing/TemplateError.h"

#include <cstddef>
#include <string>
#include <string_view>
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
