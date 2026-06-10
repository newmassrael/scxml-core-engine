// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include "parsing/PositionMap.h"
#include "parsing/XIncludeError.h"

#include <pugixml.hpp>

#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <string>
#include <string_view>
#include <vector>

// String-level `<xi:include>` expander. Mirrors
// `sce-build/src/xinclude.rs::expand` line-for-line where practical
// so the C++ Interpreter side and the AOT pipeline produce
// byte-equivalent post-XInclude documents and identical PositionMap
// composition. The port is string-level (rather than DOM-mutating
// like the pugixml splice path it replaced) because
// `SCE::parsing::PositionMap` keys diagnostics to byte offsets in
// the expanded output — DOM mutation cannot produce a stable
// byte-offset coordinate space.
//
// `PugiXMLDocument::processXInclude` delegates to this expander;
// `cpp_xinclude_expander_matches_rust_shape` in
// `sce-build/src/xinclude.rs` tests pins the Rust ↔ C++ shape
// agreement.
//
// Failure model: expander-internal errors (missing href, not-found,
// cycle, too-deep, malformed, unsupported feature, read failure)
// throw a typed `XIncludeExpansionError` subtype declared in
// `parsing/XIncludeError.h` — each subtype mirrors a Rust
// `XIncludeError` variant and overrides `Diagnostic::code()` with
// the matching `xml/xinclude-*` wire string. Catch sites can bind
// `XIncludeExpansionError const &` for the legacy string surface
// or upcast through `Diagnostic` for the typed surface; both work
// against any subtype. §wire-W3.

namespace SCE::parsing {

// Maximum nesting depth for recursive `<xi:include>` expansion.
// Mirrors Rust `xinclude::MAX_XINCLUDE_DEPTH`. Two-way agreement
// of this constant is pinned by the Rust-side
// `xinclude_depth_matches_runtime` test, which verdicts against
// this header.
constexpr unsigned MAX_XINCLUDE_DEPTH = 10;

// W3C XInclude 1.0 namespace URI. Element matching is done by local
// name (`include`) for parity with the DOM-mutating pugixml
// expansion this expander replaced, which was lenient about
// namespace declarations. Exposed for diagnostic context.
inline constexpr std::string_view XINCLUDE_NS =
    "http://www.w3.org/2001/XInclude";

// Result of a successful XInclude expansion: the expanded source
// text (post-splice) and a `PositionMap` keyed against that text's
// bytes. Mirrors Rust's `(String, PositionMap)` return from
// `sce-build/src/xinclude.rs::expand`. Shape agreement is pinned
// by the `cpp_xinclude_expander_matches_rust_shape` drift test.
struct XIncludeExpandResult {
    std::string expanded_text;
    PositionMap positions;
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
// Failure modes throw a typed `XIncludeExpansionError` subtype
// (`XIncludeMissingHref`, `XIncludeNotFound`, `XIncludeReadError`,
// `XIncludeCycle`, `XIncludeTooDeep`, `XIncludeMalformed`,
// `XIncludeUnsupported`) carrying a diagnostic-ready message
// (href, search trail, cycle chain, depth limit). Pre-expansion
// (row, col) of the offending `<xi:include>` is currently embedded
// in the message text only; typed location stamping is deferred
// behind a separate consumer signal.
XIncludeExpandResult expandStringX(std::string_view content,
                                   std::string_view selfPath,
                                   std::string_view baseDir);

}  // namespace SCE::parsing
