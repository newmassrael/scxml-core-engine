// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out. C++ AOT.
//
// The clause gives a payload three readings and names the third "otherwise".
// That word is where a belief leaves the system quietly. A host serializes
// `{"done":true}`, something truncates it to `{"done":`, and the clause is
// satisfied: the content becomes a string. The document then evaluates
// `_event.data.done`, finds nothing, and takes the transition it would have
// taken had the host sent a payload with no `done` field at all. Nothing is
// raised — the fallback is CORRECT behaviour, not an error — so before this
// fixture nothing anywhere said it had happened.
//
// These two deliveries are what no pre-existing accessor separates:
//
//   answer  {"done":              the payload never parsed
//   answer  {"ready":true}        it parsed; `done` is genuinely absent
//
// Sibling of `UndecodablePayloadIsReportedTest.cpp` (Interpreter channel),
// which asserts the same script against the other engine this repository
// ships — a document that moves between them must not lose the signal.
//
// Fixture:
// integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(undecodable_payload_is_reported ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "core/EventMetadata.h"
#include "scripting/ScriptEngineProvider.h"
#include "undecodable_payload_is_reported_sm.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::undecodable_payload_is_reported::undecodable_payload_is_reported;

/// Content that announces an object and stops. The shape a truncated write, a
/// half-flushed buffer or a serializer that died mid-record actually produces.
constexpr const char *TRUNCATED_OBJECT = R"({"done":)";
/// The same failure announced with `[`, under the other event name, so a
/// channel that reports "the last event" rather than "the last event that lost
/// a payload" cannot pass by accident.
constexpr const char *TRUNCATED_ARRAY = "[1,2";
/// W3C test 562 sends exactly this shape and requires it to arrive as a string.
/// Counting it would make the statistic fire on documents that are working.
constexpr const char *PROSE = "just a sentence";
/// What the host meant to send.
constexpr const char *INTACT_OBJECT = R"({"done":true})";

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->initialize();
    return sm;
}

void deliver(SM &sm, SM::Event event, const char *name, const char *payload) {
    sm.processEvent(event, SCE::Core::EventMetadata(name, payload));
}

}  // namespace

/// The axis: content that asked for the structured reading and did not get it
/// is counted.
TEST(UndecodablePayloadIsReportedAotTest, APayloadThatAnnouncedStructureAndDidNotParseIsCounted) {
    auto sm = started();
    ASSERT_EQ(sm->undecodablePayloads(), 0u) << "nothing has been delivered before the first event";

    deliver(*sm, SM::Event::Answer, "answer", TRUNCATED_OBJECT);

    EXPECT_EQ(sm->getPolicy().answers().value_or(-1), 1)
        << "the `answer` transition did not run, so nothing below is measuring a delivery that "
           "reached the document";
    EXPECT_EQ(sm->undecodablePayloads(), 1u)
        << "the host sent `" << TRUNCATED_OBJECT
        << "`, which announces an object and does not parse as one. W3C SCXML B.2.8.1 correctly "
           "delivers it as a string; the host that built it has no other way to learn its payload "
           "stopped being structure";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Waiting)
        << "the reading a payload got must not change which transition fired";
}

/// The other half. A count that also counts success cannot be used to detect
/// failure, and the reading the clause calls "otherwise" is the NORMAL outcome
/// for a document whose author wrote prose.
TEST(UndecodablePayloadIsReportedAotTest, ProseAndAPayloadThatParsedAreNotCounted) {
    auto sm = started();

    deliver(*sm, SM::Event::Note, "note", PROSE);
    EXPECT_EQ(sm->getPolicy().notes().value_or(-1), 1) << "the `note` transition did not run";
    EXPECT_EQ(sm->undecodablePayloads(), 0u)
        << "`" << PROSE
        << "` is the third reading working as W3C SCXML B.2.8.1 specifies and as W3C test 562 "
           "requires. A diagnostic that fires when nothing is wrong is one nobody reads";

    deliver(*sm, SM::Event::Answer, "answer", INTACT_OBJECT);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Accepted)
        << "the guard `_event.data.done` did not hold for `" << INTACT_OBJECT
        << "`, so the structured reading did not happen and the zero below would be proving nothing";
    EXPECT_EQ(sm->undecodablePayloads(), 0u) << "a payload that parsed was counted as one that did not";
}

/// Why the query has to exist at all: the two deliveries the fixture's comment
/// names are identical through every accessor a host had.
TEST(UndecodablePayloadIsReportedAotTest, TheLossIsNotDerivableFromAnyOtherAccessor) {
    auto broken = started();
    deliver(*broken, SM::Event::Answer, "answer", TRUNCATED_OBJECT);

    auto intact = started();
    // Valid JSON, and `done` is genuinely absent — the innocent explanation an
    // operator has to rule out.
    deliver(*intact, SM::Event::Answer, "answer", R"({"ready":true})");

    EXPECT_EQ(broken->getCurrentState(), intact->getCurrentState());
    EXPECT_EQ(broken->getActiveStates(), intact->getActiveStates());
    EXPECT_EQ(broken->isRunning(), intact->isRunning());
    EXPECT_EQ(broken->isInFinalState(), intact->isInFinalState());
    EXPECT_EQ(broken->getPolicy().answers().value_or(-1), intact->getPolicy().answers().value_or(-2))
        << "this fixture exists because a lost payload and an absent field are indistinguishable "
           "through the accessors a host had; if they ever differ, the fixture stopped measuring "
           "what it claims";

    EXPECT_EQ(broken->undecodablePayloads(), 1u)
        << "the two runs agree on everything else, so this count is the only thing that separates "
           "a broken sender from a working one";
    EXPECT_EQ(intact->undecodablePayloads(), 0u);
}

/// A count says a payload was lost; a host debugging a stalled supervisor needs
/// to know which delivery lost it.
TEST(UndecodablePayloadIsReportedAotTest, TheEngineNamesTheDeliveryThatLostItsPayload) {
    auto sm = started();
    EXPECT_FALSE(sm->lastUndecodablePayload().has_value()) << "nothing has been delivered yet";

    deliver(*sm, SM::Event::Answer, "answer", TRUNCATED_OBJECT);
    ASSERT_TRUE(sm->lastUndecodablePayload().has_value())
        << "the engine counted a lost payload but cannot say which delivery lost it";
    EXPECT_EQ(*sm->lastUndecodablePayload(), SM::Event::Answer);

    // A second loss, under the other event name: the accessor has to track the
    // last event THAT LOST A PAYLOAD, not the last event.
    deliver(*sm, SM::Event::Note, "note", TRUNCATED_ARRAY);
    EXPECT_EQ(sm->undecodablePayloads(), 2u) << "the count is a count, not a flag";
    ASSERT_TRUE(sm->lastUndecodablePayload().has_value());
    EXPECT_EQ(*sm->lastUndecodablePayload(), SM::Event::Note);

    // And a delivery that succeeds must leave both alone — otherwise the last
    // name would drift to whatever arrived most recently.
    deliver(*sm, SM::Event::Answer, "answer", INTACT_OBJECT);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Accepted)
        << "the intact payload did not take the guarded transition, so the two assertions below "
           "are not measuring a successful delivery";
    EXPECT_EQ(sm->undecodablePayloads(), 2u)
        << "a delivery that parsed moved a count that belongs to ones that did not";
    ASSERT_TRUE(sm->lastUndecodablePayload().has_value());
    EXPECT_EQ(*sm->lastUndecodablePayload(), SM::Event::Note)
        << "a delivery that parsed moved a name that belongs to one that did not";
}

}  // namespace SCE::Tests
