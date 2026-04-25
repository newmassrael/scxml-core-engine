// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"

#include <pugixml.hpp>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <stdexcept>
#include <string>
#include <string_view>
#include <vector>

// String-level `<xi:include>` expander. Mirrors
// `sce-build/src/xinclude.rs::expand` line-for-line where practical
// so the C++ Interpreter side and the AOT pipeline produce
// byte-equivalent post-XInclude documents and identical PositionMap
// composition. The port is string-level (rather than DOM-mutating
// like the pre-Phase-X path this expander replaces) because
// `SCE::parsing::PositionMap` keys diagnostics to byte offsets in
// the expanded output — DOM mutation cannot produce a stable
// byte-offset coordinate space.
//
// Phase X in `claudedocs/rfc-sce-template-phase-x.md` §3 B1 ships
// this expander; B2 swaps `PugiXMLDocument::processXInclude` to use
// it (replacing the DOM-mutation `processXIncludeRecursive` path);
// B3 adds the Rust ↔ C++ shape drift test.
//
// Failure model: expander-internal errors (missing href, not-found,
// cycle, too-deep, malformed) throw `XIncludeExpansionError`. Per
// Phase X RFC §1 Q3 these stay outside the typed `Diagnostic`
// family promotion (W3 territory) — `processXInclude` catches and
// records via the existing `errorMessage_` / `addError(string)`
// surface. Phase X exclusively addresses *post-expansion* coord
// remap; expander-internal errors already report at pre-expansion
// outer-document coordinates.

namespace SCE::parsing {

// Maximum nesting depth for recursive `<xi:include>` expansion.
// Mirrors Rust `xinclude::MAX_XINCLUDE_DEPTH`
// (sce-build/src/xinclude.rs:63). Phase X B3 adds
// `cpp_xinclude_expander_matches_rust_shape` to pin two-way
// agreement of this constant alongside the result-struct shape;
// the existing Rust-side `xinclude_depth_matches_runtime` test
// is updated in B3 to verdict against this header rather than the
// deleted `PugiXMLDocument::MAX_XINCLUDE_DEPTH` static.
constexpr unsigned MAX_XINCLUDE_DEPTH = 10;

// W3C XInclude 1.0 namespace URI. Element matching is done by local
// name (`include`) for parity with the pre-Phase-X pugixml runtime
// which was lenient about namespace declarations. Exposed for
// diagnostic context.
inline constexpr std::string_view XINCLUDE_NS =
    "http://www.w3.org/2001/XInclude";

// Result of a successful XInclude expansion: the expanded source
// text (post-splice) and a `PositionMap` keyed against that text's
// bytes. Mirrors Rust's `(String, PositionMap)` return from
// `sce-build/src/xinclude.rs::expand`. Three-way shape agreement
// is pinned by the Phase X B3 drift test.
struct XIncludeExpandResult {
    std::string expanded_text;
    PositionMap positions;
};

// Failure raised by the XInclude expander. Phase X RFC §1 Q3 keeps
// the expander on `std::runtime_error` derivative (no typed family
// promotion) — W3 milestone in the wire-unification RFC promotes
// `xml/xinclude-*` later under its own consumer signal. Until then
// `processXInclude` catches and forwards via `addError(string)`.
class XIncludeExpansionError : public std::runtime_error {
public:
    using std::runtime_error::runtime_error;
};

// String-level `<xi:include>` expansion entry point. Mirrors
// `sce-build/src/xinclude.rs::expand` structurally.
//
// `content` is the document source bytes. `selfPath` is the
// filesystem path of that content — seeds cycle detection and
// serves as the `FileOrigin::path` for outer-content regions in
// the produced `PositionMap`. `baseDir` is the directory that
// `<xi:include href="relative">` resolves against — typically
// `selfPath`'s parent directory. In-memory callers may pass an
// empty `selfPath`; the cycle-detection stack then begins empty
// and trips only once at least one fragment file has been loaded.
//
// Successful short-circuit: when `content` contains no "include"
// substring, returns `{std::string(content),
// PositionMap::identity(selfPath, content)}` without parsing.
//
// Failure modes throw `XIncludeExpansionError` carrying a
// diagnostic-ready message (href, search trail, cycle chain,
// depth limit). Pre-expansion (row, col) of the offending
// `<xi:include>` is currently embedded in the message text only;
// typed location surfacing is deferred to W3.
XIncludeExpandResult expandStringX(std::string_view content,
                                   std::string_view selfPath,
                                   std::string_view baseDir);

}  // namespace SCE::parsing
