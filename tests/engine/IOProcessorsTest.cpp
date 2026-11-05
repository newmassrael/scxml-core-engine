// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael

// W3C SCXML C.1.1 / C.2.3 — what `_ioprocessors` must contain.
//
// C.1.1: "SCXML Processors MUST maintain a
//  'http://www.w3.org/TR/scxml/#SCXMLEventProcessor' entry in the
//  _ioprocessors system variable. The Processor MUST maintain a 'location'
//  field inside this entry whose value holds an address that external entities
//  can use to communicate with this SCXML session using the SCXML Event I/O
//  Processor."
//
// C.2.3 says the same of
// 'http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor', for processors that
// support it.
//
// Both entries used to be missing. Every session was seeded with the single
// name "scxml", so the specification's entry names resolved to undefined and
// no HTTP entry existed at all. W3C test 522 did not notice because it reaches
// the HTTP endpoint through a literal URL rather than through the variable the
// test exists to exercise.

#include "common/IOProcessorHelper.h"
#include "core/EventOrigin.h"
#include "scripting/JSEngine.h"
#include <gtest/gtest.h>
#include <optional>
#include <string>

namespace {

using SCE::IOProcessorDescriptor;
using SCE::IOProcessorHelper;

constexpr const char *SCXML_ENTRY = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor";
constexpr const char *BASIC_HTTP_ENTRY = "http://www.w3.org/TR/scxml/#BasicHTTPEventProcessor";

std::string locationOf(const std::vector<IOProcessorDescriptor> &descriptors, const std::string &name) {
    for (const auto &descriptor : descriptors) {
        if (descriptor.name == name) {
            return descriptor.location;
        }
    }
    return "";
}

bool contains(const std::vector<IOProcessorDescriptor> &descriptors, const std::string &name) {
    for (const auto &descriptor : descriptors) {
        if (descriptor.name == name) {
            return true;
        }
    }
    return false;
}

TEST(IOProcessorEntries, ScxmlProcessorIsPublishedUnderItsSpecificationName) {
    const auto descriptors = IOProcessorHelper::build("session-1");

    EXPECT_TRUE(contains(descriptors, SCXML_ENTRY));
    EXPECT_FALSE(locationOf(descriptors, SCXML_ENTRY).empty());
}

TEST(IOProcessorEntries, ScxmlAliasResolvesToTheSameLocation) {
    const auto descriptors = IOProcessorHelper::build("session-1");

    EXPECT_EQ(locationOf(descriptors, "scxml"), locationOf(descriptors, SCXML_ENTRY));
}

TEST(IOProcessorEntries, ScxmlLocationNamesTheSession) {
    const auto descriptors = IOProcessorHelper::build("session-1");

    EXPECT_EQ(locationOf(descriptors, SCXML_ENTRY), "sce://scxml/session-1");
}

TEST(IOProcessorEntries, SessionIdIsPercentEncodedIntoTheLocation) {
    // Session ids arrive from <invoke> and from embedders, so they are not
    // constrained to characters a URI can carry unescaped.
    const auto descriptors = IOProcessorHelper::build("a b/c#d");

    EXPECT_EQ(locationOf(descriptors, SCXML_ENTRY), "sce://scxml/a%20b%2Fc%23d");
}

// === The SCXML processor's address codec (docs/adr/0090, test 6) ===
//
// `scxmlLocation` writes the address a session publishes and the SCXML
// processor stamps as the origin of what the session sends; a reply sent to it
// is routed by `sessionIdFromScxmlLocation`. The two are one mapping only if a
// session id survives the round trip, reserved characters included.

TEST(ScxmlAddressCodec, ASessionIdWithReservedCharactersSurvivesTheRoundTrip) {
    for (const std::string id : {"session-1", "a b/c#d", "parent:invoke.1", "x%2Fy", "100%"}) {
        const std::string location = IOProcessorHelper::scxmlLocation(id);
        EXPECT_EQ(location.rfind("sce://scxml/", 0), 0u) << location;
        EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation(location), id) << location;
    }
}

TEST(ScxmlAddressCodec, AReservedCharacterIsPercentEncodedOnce) {
    // `%` is itself reserved: an id that already looks encoded is encoded
    // again, so decoding gives back the id rather than what it looked like.
    EXPECT_EQ(IOProcessorHelper::scxmlLocation("x%2Fy"), "sce://scxml/x%252Fy");
    EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation("sce://scxml/x%252Fy"), "x%2Fy");
}

TEST(ScxmlAddressCodec, AnAddressOfAnotherProcessorNamesNoSession) {
    EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation("mesh://motor"), "");
    EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation("motor"), "");
    EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation("sce://scxml/"), "");
    EXPECT_EQ(IOProcessorHelper::sessionIdFromScxmlLocation(""), "");
}

TEST(ScxmlAddressCodec, TheSendersPairIsTheProcessorsTypeAndItsLocation) {
    const SCE::Core::EventOrigin origin = IOProcessorHelper::scxmlOrigin("a b");
    EXPECT_EQ(origin.processor, SCXML_ENTRY);
    EXPECT_EQ(origin.address, IOProcessorHelper::scxmlLocation("a b"));
    EXPECT_FALSE(IOProcessorHelper::scxmlOriginOf("").has_value()) << "no sender has no address to publish";
    EXPECT_EQ(IOProcessorHelper::scxmlOriginOf("a b"), std::optional<SCE::Core::EventOrigin>(origin));
}

TEST(ScxmlAddressCodec, AProcessorAliasIsReadAsItsTypeUri) {
    EXPECT_EQ(IOProcessorHelper::canonicalProcessorType("scxml"), SCXML_ENTRY);
    EXPECT_EQ(IOProcessorHelper::canonicalProcessorType("basichttp"), BASIC_HTTP_ENTRY);
    EXPECT_EQ(IOProcessorHelper::canonicalProcessorType(SCXML_ENTRY), SCXML_ENTRY);
    EXPECT_EQ(IOProcessorHelper::canonicalProcessorType("x-sce-host"), "x-sce-host");
    EXPECT_EQ(IOProcessorHelper::canonicalProcessorType(""), "");
}

// docs/adr/0090, section 7: a format-2 origin, as a saved state or a queue
// snapshot stores it, is read back as the pair and the sender.
TEST(StoredOrigin, AFormat2OriginIsReadAsThePairItStoodFor) {
    // Empty both: none.
    const auto none = IOProcessorHelper::readStoredOrigin("", "");
    EXPECT_FALSE(none.origin.has_value());
    EXPECT_EQ(none.sender, "");
    // A bare id with the SCXML type: that session's pair, the id its sender.
    const auto bare = IOProcessorHelper::readStoredOrigin("root.work.1.kid", SCXML_ENTRY);
    EXPECT_EQ(bare.origin, std::optional<SCE::Core::EventOrigin>(IOProcessorHelper::scxmlOrigin("root.work.1.kid")));
    EXPECT_EQ(bare.sender, "root.work.1.kid");
    // An SCXML location, whatever the stored type: the pair, the decoded id its sender.
    const auto located = IOProcessorHelper::readStoredOrigin("sce://scxml/a%20b", "");
    EXPECT_EQ(located.origin, std::optional<SCE::Core::EventOrigin>(IOProcessorHelper::scxmlOrigin("a b")));
    EXPECT_EQ(located.sender, "a b");
    // Anything else: the pair as stored, no sender — a host's bare address keeps its type.
    const auto host = IOProcessorHelper::readStoredOrigin("motor", "x-sce-host");
    EXPECT_EQ(host.origin, std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin("x-sce-host", "motor")));
    EXPECT_EQ(host.sender, "");
    const auto mesh = IOProcessorHelper::readStoredOrigin("mesh://motor", SCXML_ENTRY);
    EXPECT_EQ(mesh.origin, std::optional<SCE::Core::EventOrigin>(SCE::Core::EventOrigin(SCXML_ENTRY, "mesh://motor")));
    EXPECT_EQ(mesh.sender, "");
    // A waiting send's origin names its sender, bare or located.
    EXPECT_EQ(IOProcessorHelper::readStoredSender("fixture-root"), "fixture-root");
    EXPECT_EQ(IOProcessorHelper::readStoredSender("sce://scxml/a%20b"), "a b");
}

TEST(StoredOrigin, AnEventIsStoredAsFormat2WritesIt) {
    // A session's event stores its sender's id; any other stores the address.
    EXPECT_EQ(IOProcessorHelper::storedOriginText(IOProcessorHelper::scxmlOrigin("kid"), "kid"), "kid");
    EXPECT_EQ(IOProcessorHelper::storedOriginText(SCE::Core::EventOrigin("x-sce-host", "motor"), ""), "motor");
    EXPECT_EQ(IOProcessorHelper::storedOriginText(std::nullopt, ""), "");
}

TEST(IOProcessorEntries, NoHttpEntryWhenNoEndpointIsDeployed) {
    // Support for the BasicHTTP processor is optional. A session with no
    // listener advertises no address rather than one nothing answers on.
    const auto descriptors = IOProcessorHelper::build("session-1");

    EXPECT_FALSE(contains(descriptors, BASIC_HTTP_ENTRY));
    EXPECT_FALSE(contains(descriptors, "basichttp"));
}

TEST(IOProcessorEntries, HttpEntryCarriesTheDeployedAccessUri) {
    const auto descriptors = IOProcessorHelper::build("session-1", "http://localhost:8080/test");

    EXPECT_EQ(locationOf(descriptors, BASIC_HTTP_ENTRY), "http://localhost:8080/test");
    EXPECT_EQ(locationOf(descriptors, "basichttp"), "http://localhost:8080/test");
}

// === Rendering into a live session ===

class IOProcessorsInScript : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &SCE::JSEngine::instance();
        engine_->reset();
        ASSERT_TRUE(engine_->createSession(kSession));
    }

    void TearDown() override {
        engine_->destroySession(kSession);
    }

    std::string evaluateString(const std::string &expression) {
        auto result = engine_->evaluateExpression(kSession, expression).get();
        EXPECT_TRUE(result.isSuccess()) << expression << ": " << result.getErrorMessage();
        return result.getValue<std::string>();
    }

    static constexpr const char *kSession = "ioprocessors-session";
    SCE::JSEngine *engine_ = nullptr;
};

TEST_F(IOProcessorsInScript, BothSpellingsReachTheSameLocation) {
    ASSERT_TRUE(
        engine_->setupSystemVariables(kSession, "machine", IOProcessorHelper::build(kSession)).get().isSuccess());

    const std::string aliasLocation = evaluateString("_ioprocessors['scxml'].location");
    EXPECT_FALSE(aliasLocation.empty());
    EXPECT_EQ(evaluateString(std::string("_ioprocessors['") + SCXML_ENTRY + "'].location"), aliasLocation);
}

TEST_F(IOProcessorsInScript, DeployedHttpEndpointIsReadable) {
    ASSERT_TRUE(engine_
                    ->setupSystemVariables(kSession, "machine",
                                           IOProcessorHelper::build(kSession, "http://localhost:8080/test"))
                    .get()
                    .isSuccess());

    EXPECT_EQ(evaluateString("_ioprocessors['basichttp'].location"), "http://localhost:8080/test");
    EXPECT_EQ(evaluateString(std::string("_ioprocessors['") + BASIC_HTTP_ENTRY + "'].location"),
              "http://localhost:8080/test");
}

TEST_F(IOProcessorsInScript, UndeployedHttpProcessorIsAbsentRatherThanEmpty) {
    ASSERT_TRUE(
        engine_->setupSystemVariables(kSession, "machine", IOProcessorHelper::build(kSession)).get().isSuccess());

    auto result = engine_->evaluateExpression(kSession, "typeof _ioprocessors['basichttp']").get();
    ASSERT_TRUE(result.isSuccess());
    EXPECT_EQ(result.getValue<std::string>(), "undefined");
}

TEST_F(IOProcessorsInScript, QuoteInAnExternalValueStaysAValue) {
    // The session name is the <scxml> element's 'name' attribute and the
    // access URI comes from the deployment; neither is trusted source text.
    // Setup used to splice all three into the evaluated script, so a quote in
    // any of them closed the literal and the rest parsed as code.
    ASSERT_TRUE(engine_
                    ->setupSystemVariables(kSession, "'; globalThis.escaped = true; var x = '",
                                           IOProcessorHelper::build(kSession, "http://localhost:8080/a'b"))
                    .get()
                    .isSuccess());

    auto escaped = engine_->evaluateExpression(kSession, "typeof globalThis.escaped").get();
    ASSERT_TRUE(escaped.isSuccess());
    EXPECT_EQ(escaped.getValue<std::string>(), "undefined");

    EXPECT_EQ(evaluateString("_name"), "'; globalThis.escaped = true; var x = '");
    EXPECT_EQ(evaluateString("_ioprocessors['basichttp'].location"), "http://localhost:8080/a'b");
}

}  // namespace
