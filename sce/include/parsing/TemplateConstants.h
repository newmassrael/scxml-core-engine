// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

#pragma once

#include <string_view>

namespace SCE::parsing {

// `sce:template` / `sce:use` preprocessor constants shared between
// the C++ Interpreter runtime (Phase B, this header) and the Rust
// AOT expander (`sce-build/src/template.rs`).
//
// Phase B is tracked in `claudedocs/rfc-sce-template-phase-b.md`
// and lands incrementally over M1-M5. The constants in this
// header are pinned against their Rust counterparts by drift
// tests in `sce-build/src/template.rs::tests`:
//
//   - `cpp_template_depth_matches_rust` (M1) reads this header
//     via `include_str!` and asserts MAX_TEMPLATE_DEPTH matches
//     `sce-build/src/template.rs::MAX_TEMPLATE_DEPTH`.
//   - `cpp_param_name_pattern_matches_rust` (M1) reads this
//     header via `include_str!` and asserts PARAM_NAME_PATTERN
//     matches the XSD's paramNameType pattern and the Rust
//     regex literal on the standard corpus.
//
// Changing either constant without updating the Rust mirror
// (or vice versa) produces a red test rather than silent
// cross-language drift.

// Maximum nesting depth for recursive `<sce:use>` expansion.
// Mirrors `sce-build/src/template.rs::MAX_TEMPLATE_DEPTH`.
//
// Consumed by M3 (recursive expansion + cycle detection). M1
// declares the constant but does not yet enforce it — the M1
// skeleton short-circuits on any non-passthrough shape via
// `TemplateNotImplemented`, so the recursion loop that reads
// this constant does not exist yet. RFC §3 M1 / §4 names M3 as
// the consuming milestone to satisfy
// `feedback_built_but_unconsumed.md`.
inline constexpr int MAX_TEMPLATE_DEPTH = 10;

// Pattern accepted for `<sce:param name="...">` identifiers.
// Mirrors the XSD grammar in `schemas/sce-forge-ext.xsd`
// (`<xs:simpleType name="paramNameType">`) and the Rust
// validator `is_valid_param_name` in `sce-build/src/template.rs`.
//
// Stored as the XSD lexical pattern literal so the drift test
// can byte-compare this constant against the XSD pattern
// attribute and the Rust regex corpus without translating
// between escape conventions. The XSD's `\-` is redundant inside
// a character class (the character class treats `-` as literal
// when not between two chars), but preserved verbatim because
// the anchoring test asserts byte equality, not regex
// equivalence.
//
// Consumed by M2 (parameter substitution + C++ param validator).
// M1 declared the constant; `is_valid_param_name` (below) is the
// M2 consumer. RFC §3 M1 / §4 named M2 as the consuming
// milestone.
inline constexpr std::string_view PARAM_NAME_PATTERN =
    R"pat([A-Za-z_][A-Za-z0-9_\-]*)pat";

// Validate a `<sce:param name>` identifier against
// `PARAM_NAME_PATTERN`. Returns true iff `name` matches
// `[A-Za-z_][A-Za-z0-9_\-]*` anchored to the whole string —
// the same contract as XSD `paramNameType` and the Rust
// `is_valid_param_name` in `sce-build/src/template.rs`.
//
// Implemented as an inline character-class check rather than a
// std::regex match: the pattern is small, fixed, and hot on the
// preprocessor path (called once per `<sce:param>` declaration
// and once per `{$token}` substitution attempt); std::regex
// would add a one-time compile cost and per-call allocation
// without behavioural gain. The pattern string itself is still
// the authoritative cross-language anchor via `PARAM_NAME_PATTERN`,
// pinned by the drift tests in `sce-build/src/template.rs::tests`.
inline bool is_valid_param_name(std::string_view name) noexcept {
    if (name.empty()) {
        return false;
    }
    const char first = name[0];
    const bool first_ok = (first >= 'A' && first <= 'Z') ||
                          (first >= 'a' && first <= 'z') ||
                          first == '_';
    if (!first_ok) {
        return false;
    }
    for (size_t i = 1; i < name.size(); ++i) {
        const char c = name[i];
        const bool rest_ok = (c >= 'A' && c <= 'Z') ||
                             (c >= 'a' && c <= 'z') ||
                             (c >= '0' && c <= '9') ||
                             c == '_' || c == '-';
        if (!rest_ok) {
            return false;
        }
    }
    return true;
}

}  // namespace SCE::parsing
