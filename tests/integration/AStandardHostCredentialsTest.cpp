// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The credential operations of the reference host (sce/include/host/CredentialHost.h; docs/SCE_HOST_FEATURES.md,
// section 14; docs/adr/0109, D3) on libsodium: a token is the tag of the invocation's identity and the name under
// the issuing key, the same bytes the Rust host gives (backends/rust/host-features/tests/credentials.rs); a token
// issued twice under one identity is one token; a revocation revokes the name's tokens; and each failure and
// refusal of the two operations.
//
// Compiled into standard_host_scenario_test only in a build that found libsodium, whose target `sce_host_cose`
// it links (cmake/SCEHostCose.cmake).

#include "host/Cose.h"
#include "host/CoseHost.h"
#include "host/CredentialHost.h"
#include "host/SodiumCoseCrypto.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"

#include <cstddef>
#include <cstdint>
#include <gtest/gtest.h>
#include <map>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
using Bytes = std::vector<uint8_t>;

/// The issuing key the scripted random source gives the host's first issue.
const Bytes kIssuingKey(32, uint8_t{0x5D});

/// The token of `phone` issued as start 1 of the `<invoke>` `issue` of the session `session` under
/// kIssuingKey, which the Rust host gives for the same key and identity.
const char *const kPhoneToken = "sce1.KJG0XTyXotgk5bba3j0fmD5ZMWB6R6-oxTnxxKT8I8g";

/// The moment the host's clock reads, in milliseconds since the Unix epoch.
constexpr int64_t kNowMs = 1700000000000;

/// A random source that gives nothing.
class DryCrypto : public host::SodiumCoseCrypto {
public:
    std::optional<Bytes> random(size_t) override {
        return std::nullopt;
    }
};

/// A reference host with the credential operations over the key ring of a COSE feature, its clock at kNowMs.
struct CredentialRig {
    explicit CredentialRig(host::CoseCrypto &crypto) : cose(crypto), credentials(cose, crypto, clock) {
        clock.reading = host::Reading{kNowMs, 0};
        EXPECT_TRUE(reference.addFeature(cose));
        EXPECT_TRUE(reference.addFeature(credentials));
    }

    host::Outcome start(const std::string &operation, const host::Invocation &invocation, const host::Params &params) {
        const auto outcome = reference.start(operation, invocation, params);
        EXPECT_TRUE(outcome.has_value()) << operation << " is not served";
        return outcome.value_or(host::Outcome{});
    }

    /// The result record of a completion of `operation`.
    json done(const std::string &operation, const host::Invocation &invocation, const host::Params &params) {
        const host::Outcome outcome = start(operation, invocation, params);
        EXPECT_EQ(outcome.kind, host::Outcome::Kind::Done) << operation << ": " << outcome.data;
        return outcome.kind == host::Outcome::Kind::Done ? json::parse(outcome.data) : json::object();
    }

    /// The token an issue of `name` for `ttl` seconds answers as start `number` of the `<invoke>` `issue`.
    std::string issue(const std::string &name, const std::string &ttl, uint64_t number = 1) {
        const json fields = done(host::CREDENTIAL_ISSUE, host::Invocation{"session", "issue", number},
                                 {{"name", name}, {"ttl_s", ttl}});
        EXPECT_EQ(fields.at("error"), "") << fields.dump();
        return fields.value("token", std::string());
    }

    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::ReferenceHost reference{clock, transport};
    host::CoseFeature cose;
    host::CredentialFeature credentials;
};

}  // namespace

/// The token is `sce1.` and the base64url of the HMAC-SHA-256 under the issuing key of the identity and the
/// name, the bytes the Rust host gives; the host keeps its SHA-256 with the name, the moment and the expiry.
TEST(AStandardHostCredentials, ATokenIsTheTagOfTheIdentityAndTheNameUnderTheIssuingKey) {
    HostScenario::ScriptedRandomCrypto crypto;
    CredentialRig rig(crypto);
    crypto.queue(kIssuingKey);
    const std::string token = rig.issue("phone", "3600");
    EXPECT_EQ(token, kPhoneToken);
    ASSERT_EQ(rig.credentials.tokens().size(), 1u);
    const auto &[digest, kept] = *rig.credentials.tokens().begin();
    EXPECT_EQ(digest, crypto.sha256(Bytes(token.begin(), token.end())).value_or(Bytes{}))
        << "the host keeps the token's SHA-256, never the token";
    EXPECT_EQ(kept.name, "phone");
    EXPECT_EQ(kept.issuedMs, static_cast<uint64_t>(kNowMs));
    EXPECT_EQ(kept.expiresMs, static_cast<uint64_t>(kNowMs) + 3600000u);
    EXPECT_FALSE(kept.revoked);
}

/// A token issued twice under one identity, as a restore starts the invocation again, is one token, its
/// expiry the first issue's; another start, or another name, is another token.
TEST(AStandardHostCredentials, ATokenIssuedTwiceUnderOneIdentityIsOneToken) {
    HostScenario::ScriptedRandomCrypto crypto;
    CredentialRig rig(crypto);
    const std::string token = rig.issue("phone", "60");
    rig.clock.reading = host::Reading{kNowMs + 30000, 0};
    EXPECT_EQ(rig.issue("phone", "60"), token) << "the same identity is the same token";
    ASSERT_EQ(rig.credentials.tokens().size(), 1u) << "and one kept";
    EXPECT_EQ(rig.credentials.tokens().begin()->second.expiresMs, static_cast<uint64_t>(kNowMs) + 60000u)
        << "the repeat keeps the first issue's expiry";
    EXPECT_NE(rig.issue("phone", "60", 2), token) << "the next start of the invoke";
    EXPECT_NE(rig.issue("tablet", "60"), token) << "another name under the same identity";
    EXPECT_EQ(rig.credentials.tokens().size(), 3u);
}

/// A revocation revokes every token of the name not revoked yet and answers how many; a token issued again
/// under its first identity stays revoked; another name is untouched.
TEST(AStandardHostCredentials, ARevocationRevokesTheNamesTokens) {
    HostScenario::ScriptedRandomCrypto crypto;
    CredentialRig rig(crypto);
    rig.issue("phone", "60");
    rig.issue("phone", "60", 2);
    rig.issue("tablet", "60", 3);
    const host::Invocation revoke{"session", "revoke", 1};
    EXPECT_EQ(rig.done(host::CREDENTIAL_REVOKE, revoke, {{"name", "phone"}}),
              json::parse(R"({"error":"","revoked":2})"));
    size_t revoked = 0;
    for (const auto &entry : rig.credentials.tokens()) {
        EXPECT_EQ(entry.second.revoked, entry.second.name == "phone") << entry.second.name;
        revoked += entry.second.revoked ? 1 : 0;
    }
    EXPECT_EQ(revoked, 2u);
    rig.issue("phone", "60");
    for (const auto &entry : rig.credentials.tokens()) {
        if (entry.second.name == "phone") {
            EXPECT_TRUE(entry.second.revoked) << "a repeated issue does not make a revoked token live";
        }
    }
    EXPECT_EQ(rig.done(host::CREDENTIAL_REVOKE, host::Invocation{"session", "revoke", 2}, {{"name", "phone"}}),
              json::parse(R"({"error":"","revoked":0})"))
        << "nothing more to revoke";
}

/// A name that is empty or past its bound, and a `ttl_s` that is no count of seconds from 1 to the largest a
/// uint32 holds, are refused, and nothing is issued.
TEST(AStandardHostCredentials, ARequestOutOfItsBoundsIsRefused) {
    HostScenario::ScriptedRandomCrypto crypto;
    CredentialRig rig(crypto);
    const host::Invocation first{"session", "issue", 1};
    const std::string longName(257, 'n');
    const std::vector<host::Params> issues = {
        {{"name", ""}, {"ttl_s", "60"}},      {{"name", longName}, {"ttl_s", "60"}},
        {{"name", "phone"}, {"ttl_s", "0"}},  {{"name", "phone"}, {"ttl_s", "4294967296"}},
        {{"name", "phone"}, {"ttl_s", "-1"}}, {{"name", "phone"}},
    };
    for (const auto &params : issues) {
        EXPECT_EQ(rig.start(host::CREDENTIAL_ISSUE, first, params).kind, host::Outcome::Kind::Refuse);
    }
    for (const std::string &name : {std::string(), longName}) {
        EXPECT_EQ(rig.start(host::CREDENTIAL_REVOKE, first, {{"name", name}}).kind, host::Outcome::Kind::Refuse);
    }
    EXPECT_TRUE(rig.credentials.tokens().empty());
    rig.issue(std::string(256, 'n'), "4294967295");
}

/// An issue the host cannot carry out fails `unavailable` and keeps nothing: a clock that gives no time, and
/// a random source that gives no issuing key.
TEST(AStandardHostCredentials, AnIssueTheHostCannotCarryOutFailsUnavailable) {
    const json failed = json::parse(R"({"error":"unavailable","token":""})");
    {
        HostScenario::ScriptedRandomCrypto crypto;
        CredentialRig rig(crypto);
        rig.clock.reading.reset();
        EXPECT_EQ(rig.done(host::CREDENTIAL_ISSUE, host::Invocation{"session", "issue", 1},
                           {{"name", "phone"}, {"ttl_s", "60"}}),
                  failed)
            << "no time to issue from";
        EXPECT_TRUE(rig.credentials.tokens().empty());
    }
    DryCrypto dry;
    CredentialRig rig(dry);
    EXPECT_EQ(
        rig.done(host::CREDENTIAL_ISSUE, host::Invocation{"session", "issue", 1}, {{"name", "phone"}, {"ttl_s", "60"}}),
        failed)
        << "no issuing key to draw";
    EXPECT_TRUE(rig.credentials.tokens().empty());
}

/// The base64url of the C++ host is RFC 4648's, without padding, in the URL alphabet.
TEST(AStandardHostCredentials, TheBase64UrlOfABearerTokenIsRfc4648s) {
    host::SodiumCoseCrypto crypto;
    EXPECT_EQ(crypto.base64Url(Bytes(32, uint8_t{0xFB})).value_or(""), "-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_v7-_s");
    EXPECT_EQ(crypto.base64Url(Bytes{}).value_or("?"), "");
    EXPECT_EQ(crypto.base64Url(Bytes{'f', 'o', 'o', 'b'}).value_or(""), "Zm9vYg") << "RFC 4648 section 10";
}

}  // namespace SCE::Tests
