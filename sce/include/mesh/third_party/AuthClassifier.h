// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE Mesh §16.7 row 10 — auth-fail classifier shared between
// generated TransportRouters (via codegen template) and direct unit
// tests. Keeps the classification rule in one place so the row-10
// contract is auditable in isolation rather than locked inside a
// minijinja template body.
//
// Q3 lock-in: transport-specific rejection signals classify on
// substring presence — Zenoh ZException::what() contains certificate
// / tls / auth / handshake tokens; SOMEIP availability=false on a
// binding that opted into SD-denial classification fires unconditionally
// (no text inspection needed; the SCXML author's deploy.yaml flag
// declares the contract with the SD responder).
//
// Header-only on purpose: the classifier touches only `<string>` and
// has no allocations beyond a transient lowercased copy of the input
// (kept small in practice — zenoh ZException::what() returns the
// vendored zenoh-cpp error message, bounded by zenoh's own
// formatting).
//
// docs/SCE_AXIS_6_PATTERNS.md A6-001 — production-deferred limitation.
// zenoh-cpp current versions wrap every connection error (including
// mTLS handshake failure) in a generic Z_ENETWORK ZException whose
// what() payload reads `"Failed to open session(Error code: -4 )"`.
// None of the manifest keywords match, so `isZenohAuthFailMessage`
// always returns false in production — the row-10 emit path is dead
// code under the current zenoh-cpp surface. The spec-side §16.7 row
// 10 contract remains valid (author-facing closure shipped in
// `73087043`), but live production emission of UNAUTHORIZED awaits
// zenoh-cpp upstream exposing a typed auth-failure discriminator that
// SCE can catch directly. The axis-6 CI fixture (Stage 2,
// `tests/mesh/AuthClassifierCIFixture.cpp::RowTenLimitationIsLockedIn`)
// locks the limitation in — when it ever fires, the remediation steps
// are listed there. Header-only future-proofing is retained so the
// runtime classifier transitions to live the moment upstream message
// vocabulary starts containing a manifest keyword.

#pragma once

#include <array>
#include <string>
#include <string_view>

namespace SCE::Mesh::ThirdParty {

/// Single-source-of-truth keyword manifest for the zenoh-cpp auth-fail
/// substring scan. Both the runtime classifier (`isZenohAuthFailMessage`)
/// and the axis-6 CI fixture (`AuthClassifierCIFixture`) consume this
/// array — adding a keyword requires adding a matching fixture line
/// exercising the keyword under a real zenoh-cpp handshake failure;
/// removing one fails the fixture. The conservative-widening rule from
/// `docs/SCE_AXIS_6_PATTERNS.md` becomes structurally enforced rather
/// than documentation-only.
inline constexpr std::array<std::string_view, 4> kZenohAuthFailKeywords{
    "certificate",
    "tls",
    "auth",
    "handshake",
};

/// SCE_MESH.md §16.7 row 10 — case-insensitive substring scan against
/// the four spec-named auth-fail keywords. Returns `true` iff the
/// input contains any of `certificate`, `tls`, `auth`, or `handshake`
/// (case-insensitive). Used by the generated zenoh ZException catch
/// block to decide between row 1 TRANSPORT_UNAVAILABLE (init failed
/// for a non-auth reason — config parse error, address in use, etc.)
/// and row 10 UNAUTHORIZED (init failed at the trust boundary).
///
/// Conservative widening rule: new upstream zenoh-cpp phrasings that
/// indicate auth failure should be added to the keyword set, never
/// removed. Authors guarding on `_event.data.reason == "UNAUTHORIZED"`
/// must continue to see auth failures classified as row 10 across
/// zenoh-cpp upgrades.
[[nodiscard]] inline bool isZenohAuthFailMessage(std::string_view what) noexcept {
    // Lowercase a transient copy so the case-insensitive scan keeps
    // the public-facing keyword list ASCII-literal-readable (no
    // mixed-case noise) and the caller's `what` argument unchanged.
    std::string lower;
    lower.reserve(what.size());
    for (char c : what) {
        lower.push_back(
            (c >= 'A' && c <= 'Z') ? static_cast<char>(c - 'A' + 'a') : c);
    }
    for (auto keyword : kZenohAuthFailKeywords) {
        if (lower.find(keyword) != std::string::npos) {
            return true;
        }
    }
    return false;
}

}  // namespace SCE::Mesh::ThirdParty
