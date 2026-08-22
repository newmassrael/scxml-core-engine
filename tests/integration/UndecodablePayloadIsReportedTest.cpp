// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML B.2.8.1: a payload the datamodel could not read arrives as a
// space-normalized string, and the host that built it can find out.
// Interpreter path.
//
// This channel is the one where the gap is most easily missed, because the
// Interpreter DOES report an event nothing matched — `processEvent` returns a
// `TransitionResult` whose `success` is false. That reporting is about the
// TRANSITION, not the payload: here every delivery matches a transition and
// returns success, and the run in which the payload was lost is byte-identical
// to the run in which the field was legitimately absent.
//
//   answer  {"done":              matched, success true — the payload never parsed
//   answer  {"ready":true}        matched, success true — `done` genuinely absent
//
// So the assertions below are not only "the Interpreter is right": they pin the
// numbers the AOT sibling has to match for the same script.
// `UndecodablePayloadIsReportedAotTest.cpp` runs it against
// `undecodablePayloads()` and `lastUndecodablePayload()`; this one reads
// `getStatistics().undecodablePayloads` and `lastUndecodablePayloadEvent`. A
// document that grows up on one engine and ships on the other must not lose the
// signal.
//
// Fixture:
// integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml

#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <fstream>
#include <gtest/gtest.h>
#include <sstream>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

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

}  // namespace

class UndecodablePayloadIsReportedTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) +
            "/integration_resources/undecodable_payload_is_reported/undecodable_payload_is_reported.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(*engine_);
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(eventRaiser);
        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());
        ASSERT_EQ(sm_->getCurrentState(), "waiting");
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: content that asked for the structured reading and did not get it
/// is reported, not merely delivered as a string.
TEST_F(UndecodablePayloadIsReportedTest, APayloadThatAnnouncedStructureAndDidNotParseIsReported) {
    ASSERT_EQ(sm_->getStatistics().undecodablePayloads, 0u) << "nothing has been delivered before the first event";

    const auto result = sm_->processEvent("answer", TRUNCATED_OBJECT);

    EXPECT_TRUE(result.success) << "the delivery matched the unguarded `answer` transition. That is the whole problem: "
                                   "the transition-level report the Interpreter already gives says nothing about the "
                                   "payload, because the payload's reading does not change which transition fired";
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 1u)
        << "the host sent `" << TRUNCATED_OBJECT
        << "`, which announces an object and does not parse as one. W3C SCXML B.2.8.1 correctly "
           "delivers it as a string; the host that built it has no other way to learn its payload "
           "stopped being structure. The AOT sibling's undecodablePayloads() answers the same "
           "question";
    EXPECT_EQ(sm_->getCurrentState(), "waiting") << "the reading a payload got must not change which transition fired";
}

/// The other half. A count that also counts success cannot be used to detect
/// failure, and the reading the clause calls "otherwise" is the NORMAL outcome
/// for a document whose author wrote prose.
TEST_F(UndecodablePayloadIsReportedTest, ProseAndAPayloadThatParsedAreNotReported) {
    ASSERT_TRUE(sm_->processEvent("note", PROSE).success);
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 0u)
        << "`" << PROSE
        << "` is the third reading working as W3C SCXML B.2.8.1 specifies and as W3C test 562 "
           "requires. A diagnostic that fires when nothing is wrong is one nobody reads";

    ASSERT_TRUE(sm_->processEvent("answer", INTACT_OBJECT).success);
    ASSERT_EQ(sm_->getCurrentState(), "accepted")
        << "the guard `_event.data.done` did not hold for `" << INTACT_OBJECT
        << "`, so the structured reading did not happen and the zero below would be proving nothing";
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 0u) << "a payload that parsed was counted as one that did not";
}

/// Why the query has to exist at all: on this engine the transition-level
/// report — the one thing the Interpreter had that the generated engines did
/// not — answers the same for both deliveries.
TEST_F(UndecodablePayloadIsReportedTest, TheLossIsNotDerivableFromTheTransitionResult) {
    const auto broken = sm_->processEvent("answer", TRUNCATED_OBJECT);
    const std::string brokenState = sm_->getCurrentState();
    const uint32_t brokenCount = sm_->getStatistics().undecodablePayloads;

    // Valid JSON, and `done` is genuinely absent — the innocent explanation an
    // operator has to rule out. Same machine, next delivery: the fixture's
    // `answer` transition is targetless, so `waiting` is unchanged.
    const auto intact = sm_->processEvent("answer", R"({"ready":true})");

    EXPECT_EQ(broken.success, intact.success);
    EXPECT_EQ(brokenState, sm_->getCurrentState())
        << "this fixture exists because a lost payload and an absent field are indistinguishable "
           "through what a host had; if they ever differ, the fixture stopped measuring what it "
           "claims";
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, brokenCount)
        << "the second delivery parsed, so the count must not have moved — and the first one is "
           "the only thing that separates a broken sender from a working one";
    EXPECT_EQ(brokenCount, 1u);
}

/// A count says a payload was lost; a host debugging a stalled supervisor needs
/// to know which delivery lost it.
TEST_F(UndecodablePayloadIsReportedTest, TheEngineNamesTheDeliveryThatLostItsPayload) {
    EXPECT_TRUE(sm_->getStatistics().lastUndecodablePayloadEvent.empty()) << "nothing has been delivered yet";

    ASSERT_TRUE(sm_->processEvent("answer", TRUNCATED_OBJECT).success);
    EXPECT_EQ(sm_->getStatistics().lastUndecodablePayloadEvent, "answer")
        << "the engine counted a lost payload but cannot say which delivery lost it";

    // A second loss, under the other event name: the record has to track the
    // last event THAT LOST A PAYLOAD, not the last event.
    ASSERT_TRUE(sm_->processEvent("note", TRUNCATED_ARRAY).success);
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 2u) << "the count is a count, not a flag";
    EXPECT_EQ(sm_->getStatistics().lastUndecodablePayloadEvent, "note");

    // And a delivery that succeeds must leave both alone — otherwise the last
    // name would drift to whatever arrived most recently.
    ASSERT_TRUE(sm_->processEvent("answer", INTACT_OBJECT).success);
    ASSERT_EQ(sm_->getCurrentState(), "accepted")
        << "the intact payload did not take the guarded transition, so the two assertions below "
           "are not measuring a successful delivery";
    EXPECT_EQ(sm_->getStatistics().undecodablePayloads, 2u)
        << "a delivery that parsed moved a count that belongs to ones that did not";
    EXPECT_EQ(sm_->getStatistics().lastUndecodablePayloadEvent, "note")
        << "a delivery that parsed moved a name that belongs to one that did not";
}

}  // namespace Tests
}  // namespace SCE
