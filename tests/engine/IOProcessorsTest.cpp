// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
#include "scripting/JSEngine.h"
#include <gtest/gtest.h>
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
