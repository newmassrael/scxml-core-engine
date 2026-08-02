// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// SCE-VERIFIES: mesh-16.7
//
// SCE Mesh §16.7 row 10 — AuthClassifier (header-only) unit tests.
//
// Pins the Q3-lock keyword set the row-10 classifier uses to
// distinguish row 1 TRANSPORT_UNAVAILABLE (init failed for non-auth
// reasons) from row 10 UNAUTHORIZED (init failed at the trust
// boundary). The generated zenoh router's ZException catch block
// delegates to this helper, so a regression here directly breaks
// the §16.7 row 10 emit contract.

#include "mesh/third_party/AuthClassifier.h"

#include <gtest/gtest.h>

using SCE::Mesh::ThirdParty::isZenohAuthFailMessage;

TEST(AuthClassifierTest, MatchesCertificate) {
    EXPECT_TRUE(isZenohAuthFailMessage("Certificate verification failed"));
    EXPECT_TRUE(isZenohAuthFailMessage("peer presented an invalid certificate"));
    EXPECT_TRUE(isZenohAuthFailMessage("CERTIFICATE_VERIFY_FAILED"));
}

TEST(AuthClassifierTest, MatchesTls) {
    EXPECT_TRUE(isZenohAuthFailMessage("TLS handshake error"));
    EXPECT_TRUE(isZenohAuthFailMessage("error in tls layer"));
    EXPECT_TRUE(isZenohAuthFailMessage("tls record corrupted"));
}

TEST(AuthClassifierTest, MatchesAuth) {
    EXPECT_TRUE(isZenohAuthFailMessage("authentication denied by peer"));
    EXPECT_TRUE(isZenohAuthFailMessage("AUTH error"));
    EXPECT_TRUE(isZenohAuthFailMessage("authorization failure"));
}

TEST(AuthClassifierTest, MatchesHandshake) {
    EXPECT_TRUE(isZenohAuthFailMessage("Handshake aborted by remote"));
    EXPECT_TRUE(isZenohAuthFailMessage("handshake protocol violation"));
}

TEST(AuthClassifierTest, RejectsNonAuthErrors) {
    // Connection-layer / config-layer rejections must NOT be classified
    // as row 10 — they belong on the row 1 path.
    EXPECT_FALSE(isZenohAuthFailMessage("connection refused"));
    EXPECT_FALSE(isZenohAuthFailMessage("address already in use"));
    EXPECT_FALSE(isZenohAuthFailMessage("invalid endpoint format"));
    EXPECT_FALSE(isZenohAuthFailMessage("network unreachable"));
    EXPECT_FALSE(isZenohAuthFailMessage(""));
    EXPECT_FALSE(isZenohAuthFailMessage("timeout"));
}

TEST(AuthClassifierTest, CaseInsensitiveAcrossAsciiLetters) {
    // Q3-lock: classifier must be case-insensitive so a future
    // zenoh-cpp version capitalising error tokens (or vendoring a
    // different upstream phrasing) does not silently slip past the
    // classification.
    EXPECT_TRUE(isZenohAuthFailMessage("CERTIFICATE error"));
    EXPECT_TRUE(isZenohAuthFailMessage("Certificate error"));
    EXPECT_TRUE(isZenohAuthFailMessage("certificate error"));
    EXPECT_TRUE(isZenohAuthFailMessage("cErTiFiCaTe error"));
}

TEST(AuthClassifierTest, MatchesSubstringAnywhereInMessage) {
    // The classifier checks substring presence (not prefix/suffix), so
    // a wrapper that prepends a stage label or vendors a stack-trace
    // suffix still classifies correctly.
    EXPECT_TRUE(isZenohAuthFailMessage("[init] Certificate verification failed at depth 1"));
    EXPECT_TRUE(isZenohAuthFailMessage("Failed to open zenoh::Session: TLS error: peer rejected"));
}

TEST(AuthClassifierTest, SubstringRuleAcceptsAuthorityLikeWords) {
    // The substring rule is intentionally minimal: any message
    // containing one of the four keywords (case-insensitive) classifies
    // as row 10. This pins a deliberate trade-off: the cost of a
    // false positive (an UNAUTHORIZED raise on a non-auth init
    // failure where the message happens to contain `auth` as a
    // substring, e.g. "authority validation failed") is one extra
    // SCXML transition the author can disambiguate via
    // `transport_status`. The cost of a false negative (a real auth
    // failure misclassified as row 1) is silent trust erosion.
    //
    // Common words that DO contain `auth`: authority, author, author
    // ised, authentication, etc. These all classify as row 10.
    EXPECT_TRUE(isZenohAuthFailMessage("authority validation failed"));
    EXPECT_TRUE(isZenohAuthFailMessage("author signature missing"));

    // Common words that do NOT contain any of the four keywords
    // (substring-wise) stay on the row 1 path. `automatic` contains
    // `auto`, not `auth`; `health-check` contains `health`, not
    // `handshake`. The classifier discriminates correctly on the
    // exact substring boundary.
    EXPECT_FALSE(isZenohAuthFailMessage("automatic retry exhausted"));
    EXPECT_FALSE(isZenohAuthFailMessage("health-check failed"));
}

TEST(AuthClassifierTest, EveryManifestKeywordFires) {
    // Drift guard: every keyword listed in kZenohAuthFailKeywords must
    // independently trigger isZenohAuthFailMessage when embedded in an
    // otherwise non-auth message. Adding a keyword to the manifest
    // without exercising it via this test (or removing one while a
    // per-keyword test still covers it) creates a silent split between
    // manifest declaration and runtime behaviour — exactly the axis-6
    // drift this header is meant to eliminate.
    for (auto keyword : SCE::Mesh::ThirdParty::kZenohAuthFailKeywords) {
        std::string msg = "zenoh peer reported '";
        msg.append(keyword.data(), keyword.size());
        msg.append("' fault");
        EXPECT_TRUE(isZenohAuthFailMessage(msg))
            << "manifest keyword '" << keyword << "' did not fire isZenohAuthFailMessage()";
    }
}
