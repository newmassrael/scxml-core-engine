// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The signing and sealing machine built with the bounds of its records moved (docs/adr/0109, D17): the
// fixture static_host_cose generated with `--limit cose_payload=8192` and its neighbours by
// `tests/CMakeLists.txt`, into a directory and an executable of their own. The reference host's COSE feature
// is given the machine's `StaticHostCoseLimits`, the constants the build resolved, and the machine signs a
// payload at the moved bound and verifies the object, and seals a plaintext at the moved bound and opens it.
// The Rust twin is a_build_with_larger_bounds_signs_and_seals_at_them_through_its_host
// (backends/rust/scenarios/tests/host_features.rs).

#include "common/Latin1Bytes.h"
#include "common/SceClock.h"
#include "core/StatePolicyConcepts.h"
#include "host/CoseHost.h"
#include "host/SodiumCoseCrypto.h"
#include "host/StandardHost.h"
#include "integration/StandardHostScenarioReplay.h"
#include "static_host_cose_sm.h"

#include <cstddef>
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <nlohmann/json.hpp>
#include <optional>
#include <string>
#include <vector>

namespace SCE::Tests {

namespace {

using json = nlohmann::json;
namespace host = SCE::StandardHost;
namespace large = SCE::Generated::static_host_cose;
using Machine = large::static_host_cose;

constexpr int kSettleTicks = 5;

/// Run @p machine until what it was handed is taken: a round of its scheduler's ticks where it keeps one, a
/// step otherwise.
template <typename Settled> void settle(Settled &machine) {
    if constexpr (::SCE::Core::NeedsEventSchedulerTrait<typename Settled::PolicyType>::value) {
        for (int round = 0; round < kSettleTicks; ++round) {
            machine.tick();
        }
    } else {
        machine.step();
    }
}

/// Raise @p event with @p data on @p machine, run it to completion, and answer what the machine publishes.
json told(Machine &machine, const std::string &event, const json &data) {
    EXPECT_TRUE(machine.resolveEventByName(event).has_value()) << "the machine's events do not match " << event;
    machine.raiseExternal(event, data.dump());
    settle(machine);
    return json::parse(machine.saveAt(0).toJson()).at("variables");
}

/// The bytes of a byte string the machine publishes, as its Latin-1 text.
size_t bytesOf(const json &text) {
    std::vector<uint8_t> bytes;
    EXPECT_EQ(Latin1Bytes::bytesOf(text.get<std::string>(), bytes), "") << "a byte string is its Latin-1 text";
    return bytes.size();
}

/// The bounds the build gave the machine's records, as the machine declares them.
host::NamedBounds buildBounds() {
    host::NamedBounds bounds;
    bounds.cosePayload = large::StaticHostCoseLimits::COSE_PAYLOAD;
    bounds.coseSigned = large::StaticHostCoseLimits::COSE_SIGNED;
    bounds.cosePlaintext = large::StaticHostCoseLimits::COSE_PLAINTEXT;
    bounds.coseSealed = large::StaticHostCoseLimits::COSE_SEALED;
    bounds.macMessage = large::StaticHostCoseLimits::MAC_MESSAGE;
    bounds.randomBytes = large::StaticHostCoseLimits::RANDOM_BYTES;
    return bounds;
}

}  // namespace

/// docs/adr/0109, D17: the machine built with its records' bounds moved signs and seals at them through a
/// host given the build's constants.
TEST(AStandardHostNamedBounds, ABuildWithLargerBoundsSignsAndSealsAtThemThroughItsHost) {
    const host::NamedBounds bounds = buildBounds();
    ASSERT_EQ(bounds.cosePayload, 8192u) << "the machine was built with its bounds moved";
    ASSERT_EQ(bounds.coseSigned, 8448u);
    ASSERT_EQ(bounds.cosePlaintext, 8448u);
    ASSERT_EQ(bounds.coseSealed, 8576u);

    host::ScriptedClock clock;
    HostScenario::ScriptTransport transport;
    host::SodiumCoseCrypto crypto;
    host::CoseFeature cose{crypto, bounds};
    host::ReferenceHost reference{clock, transport};
    ASSERT_TRUE(reference.addFeature(cose));
    ASSERT_TRUE(cose.setSigningKeys(
        {{"k", HostScenario::bytesOfHex("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60").value()}}));
    ASSERT_TRUE(cose.setSealingKeys(
        {{"box",
          HostScenario::bytesOfHex("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f").value()}}));
    cose.setTrustSets({{"mine", {"k"}}});

    Machine machine;
    host::serve(machine, reference);
    machine.setClock(std::make_shared<SCE::ManualClock>(0));
    ASSERT_EQ(machine.initialize(), std::nullopt);
    settle(machine);

    const std::string payload(bounds.cosePayload, 'p');
    const json signedVariables =
        told(machine, "host.cose.sign1.request", {{"key_id", "k"}, {"payload", payload}, {"external_aad", ""}});
    ASSERT_EQ(signedVariables.at("outcome"), "done") << signedVariables.dump();
    const json object = signedVariables.at("signed_object");
    EXPECT_GT(bytesOf(object), bounds.cosePayload);
    EXPECT_LE(bytesOf(object), bounds.coseSigned);
    const json verified =
        told(machine, "host.cose.verify1.request", {{"trust_set", "mine"}, {"cose", object}, {"external_aad", ""}});
    EXPECT_EQ(verified.at("outcome"), "done") << verified.dump();
    EXPECT_EQ(verified.at("valid"), true);
    EXPECT_EQ(verified.at("signer"), "k");
    EXPECT_EQ(verified.at("payload"), payload);

    const std::string plaintext(bounds.cosePlaintext, 'q');
    const json sealedVariables = told(machine, "host.cose.encrypt0.request",
                                      {{"key_id", "box"}, {"plaintext", plaintext}, {"external_aad", ""}});
    ASSERT_EQ(sealedVariables.at("outcome"), "done") << sealedVariables.dump();
    EXPECT_GT(bytesOf(sealedVariables.at("sealed_object")), bounds.cosePlaintext);
    EXPECT_LE(bytesOf(sealedVariables.at("sealed_object")), bounds.coseSealed);
    const json opened = told(machine, "cose.held.open", {{"key_id", "box"}, {"external_aad", ""}});
    EXPECT_EQ(opened.at("outcome"), "done") << opened.dump();
    EXPECT_EQ(opened.at("opened"), true);
    EXPECT_EQ(opened.at("plaintext"), plaintext);
}

}  // namespace SCE::Tests
