// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out.
// Interpreter path.
//
// This channel is the one that already answered — and it is exactly why the
// count exists. `processEvent` returns a `TransitionResult` whose `success` is
// false and whose `errorMessage` names the reason, so a caller who READS the
// return value has always been able to tell. The six generated engines have no
// return value to carry that, so a host polling statistics could not; a
// document that grew up here and shipped as AOT lost the signal.
//
// So the assertions below pin both halves: the report this engine already
// gives, and the count its AOT sibling has to match for the same script.
//
// Fixture:
// integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml

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

class UnseenEventIsReportedTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(*engine_);
        raiser_ = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(raiser_);
        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());
        ASSERT_EQ(sm_->getCurrentState(), "working");
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// The fixture's `pokes` counts the deliveries that ran `poke`'s
    /// transition — the one witness of whether an event was looked at.
    std::string pokes() {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), "pokes").get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `pokes` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: an event handed over after the machine stopped is reported, not
/// merely dropped.
TEST_F(UnseenEventIsReportedTest, AnEventDeliveredAfterTheMachineStoppedIsReported) {
    ASSERT_EQ(sm_->getStatistics().unseenExternalEvents, 0u) << "nothing has been refused before the first event";

    ASSERT_TRUE(sm_->processEvent("poke").success) << "`poke` matches a targetless transition in `working`";
    ASSERT_TRUE(sm_->processEvent("finish").success) << "`finish` should have taken the machine to `done`";
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 0u)
        << "`finish` was itself handled — the machine stopped BECAUSE of it, which is not the same "
           "as stopping before it";

    const auto refused = sm_->processEvent("poke");

    EXPECT_FALSE(refused.success)
        << "this engine's own report is the half the generated ones lack: a caller that reads the "
           "TransitionResult has always been able to tell a refusal from a discard";
    EXPECT_EQ(refused.errorMessage, "State machine not running") << "and the message is what names the reason";
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 1u)
        << "the count is the half a host polling statistics needs, and the AOT sibling's "
           "unseenExternalEvents() answers the same question";
}

/// Why the count has to exist alongside the report: `success == false` is also
/// what a DISCARD answers, so the boolean alone does not separate them.
TEST_F(UnseenEventIsReportedTest, TheRefusalIsNotDerivableFromTheSuccessFlagAlone) {
    // A discard: `settle` is not in this document at all, so nothing matches it
    // while the machine is still running.
    const auto discarded = sm_->processEvent("settle");
    EXPECT_FALSE(discarded.success) << "an event nothing matches is discarded, and reported as unsuccessful";
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 0u)
        << "a discard is not a refusal: the machine looked, and nothing matched";

    ASSERT_TRUE(sm_->processEvent("finish").success);
    const auto refused = sm_->processEvent("poke");

    EXPECT_EQ(refused.success, discarded.success)
        << "both answer false, which is why the boolean alone cannot tell a host which of the two "
           "happened — that is what the count is for";
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 1u);
}

/// A count says an event went unlooked-at; a host debugging a supervisor that
/// stopped answering needs to know which one.
TEST_F(UnseenEventIsReportedTest, TheEngineNamesTheEventItNeverLookedAt) {
    EXPECT_TRUE(sm_->getStatistics().lastUnseenEventName.empty()) << "nothing has been refused yet";

    ASSERT_TRUE(sm_->processEvent("finish").success);
    (void)sm_->processEvent("poke");
    EXPECT_EQ(sm_->getStatistics().lastUnseenEventName, "poke")
        << "the engine counted a refusal but cannot say which event it refused";

    (void)sm_->processEvent("finish");
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 2u) << "the count is a count, not a flag";
    EXPECT_EQ(sm_->getStatistics().lastUnseenEventName, "finish") << "the name did not follow the second refusal";
}

/// The other way an event goes unlooked-at: it was already on the external
/// queue when the loop ended.
///
/// Every test above hands the event over AFTER the machine stopped, which is
/// the door — `processEvent`'s `!isRunning_` branch. Appendix D's loop has a
/// second place where an event stops being looked at, and it is not the door:
/// the loop checks for a top-level final state before it dequeues again, so
/// whatever is waiting on the external queue when the machine gets there is
/// never taken. `UnseenEventIsReportedAotTest` pins the same case on the AOT
/// engine, whose loop counts it the same way.
///
/// The raiser is told to queue rather than deliver, because a raiser that
/// delivers at once hands the event straight to the running machine, which
/// takes it. `pokes` is the half that makes this a statement about the loop:
/// it says the event was never looked at, rather than looked at and matched.
TEST_F(UnseenEventIsReportedTest, AnEventStillQueuedWhenTheLoopEndsIsCounted) {
    raiser_->setImmediateMode(false);
    ASSERT_TRUE(raiser_->raiseExternalEvent("poke", ""));
    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 0u) << "queuing is not refusing; the machine is still running";

    ASSERT_TRUE(sm_->processEvent("finish").success)
        << "the host's event is taken ahead of the queue, as it is on the AOT engine";
    ASSERT_FALSE(sm_->isRunning()) << "`finish` should have taken the machine to its top-level final state";

    EXPECT_EQ(sm_->getStatistics().unseenExternalEvents, 1u)
        << "`poke` was on the external queue when the machine reached its final state, so the loop "
           "ended without ever dequeuing it. A host that read only the door's count would be told "
           "nothing about the events its own queue still held";
    EXPECT_EQ(sm_->getStatistics().lastUnseenEventName, "poke") << "and the count names the event it recorded";
    EXPECT_EQ(pokes(), "0") << "`poke`'s transition ran, so the event WAS looked at and this count is measuring "
                               "something other than the loop abandoning its queue";
    EXPECT_FALSE(raiser_->hasQueuedEvents())
        << "the queue ends with the interpretation: left in place, each event would be counted again by "
           "whatever pumped this raiser next";
}

}  // namespace Tests
}  // namespace SCE
