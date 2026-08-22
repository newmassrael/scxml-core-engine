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
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(eventRaiser);
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

    IScriptEngine *engine_ = nullptr;
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

}  // namespace Tests
}  // namespace SCE
