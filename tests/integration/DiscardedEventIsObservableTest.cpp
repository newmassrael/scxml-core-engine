// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. Interpreter path.
//
// This channel is the reason the axis exists. The Interpreter has answered the
// question all along: `processEvent` returns a `TransitionResult` whose
// `success` is false when nothing matched, and `getStatistics().failedTransitions`
// counts those. The six generated engines computed the same fact at the same
// point of Appendix D's `mainEventLoop` and dropped it, so a document that grew
// up on the Interpreter and shipped as AOT lost a signal its host was reading.
//
// So the assertions below are not only "the Interpreter is right": they pin the
// numbers the AOT sibling has to match for the same script —
// `DiscardedEventIsObservableAotTest.cpp` runs it against
// `discardedExternalEvents()` and `lastDiscardedEvent()`.
//
//   poke    self transition       handled — success true, count unchanged
//   nudge   targetless internal   handled — success true, count unchanged
//   settle  no matching           DISCARDED — success false, count + 1
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml

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

class DiscardedEventIsObservableTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) +
            "/integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml";
        std::ifstream in(fixture);
        ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
        std::ostringstream buffer;
        buffer << in.rdbuf();

        sm_ = std::make_shared<StateMachine>(*engine_);
        auto eventRaiser = std::make_shared<EventRaiserImpl>();
        sm_->setEventRaiser(eventRaiser);
        ASSERT_TRUE(sm_->loadSCXMLFromString(buffer.str()));
        ASSERT_TRUE(sm_->start());
        ASSERT_EQ(sm_->getCurrentState(), "idle");
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

/// The axis: an event no active state answers is reported as such, not merely
/// dropped.
TEST_F(DiscardedEventIsObservableTest, AnEventNoActiveStateAnsweredIsReported) {
    const int before = sm_->getStatistics().failedTransitions;

    // `settle` is declared in `busy`; in `idle` it matches nothing.
    const auto result = sm_->processEvent("settle");

    EXPECT_FALSE(result.success)
        << "`settle` matches no transition in `idle`, and W3C SCXML 3.1.2 discards it. The "
           "Interpreter reports that outcome to its caller — which is the contract the generated "
           "engines now hold as well";
    EXPECT_EQ(sm_->getStatistics().failedTransitions, before + 1)
        << "the cumulative count is the shape a host polls; the AOT sibling's "
           "discardedExternalEvents() answers the same question";
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "a discarded event must not move the machine";
}

/// The other half: a handled event is not reported as discarded, including the
/// one that changes nothing.
TEST_F(DiscardedEventIsObservableTest, AHandledEventIsNotReportedAsDiscarded) {
    const int before = sm_->getStatistics().failedTransitions;

    const auto poked = sm_->processEvent("poke");
    EXPECT_TRUE(poked.success) << "`poke` matches a self transition in `idle`";

    const auto nudged = sm_->processEvent("nudge");
    EXPECT_TRUE(nudged.success)
        << "`nudge` matches a targetless internal transition: its actions run and no state is "
           "exited or entered. Handled is not the same as 'the configuration changed', which is "
           "the distinction the AOT engine's EventOutcome carries";

    EXPECT_EQ(sm_->getStatistics().failedTransitions, before)
        << "neither event was discarded, so nothing should have been counted";
}

/// The configuration cannot answer the question — which is why the engines have
/// to. Same assertion as the AOT sibling's, on the other engine.
TEST_F(DiscardedEventIsObservableTest, TheDiscardIsNotDerivableFromTheConfiguration) {
    sm_->processEvent("poke");
    const std::string handled = sm_->getCurrentState();

    sm_->processEvent("settle");
    const std::string discarded = sm_->getCurrentState();

    EXPECT_EQ(handled, discarded) << "this fixture exists because a handled event and a discarded one leave the same "
                                     "configuration; if they ever differ, the fixture stopped measuring what it claims";
}

}  // namespace Tests
}  // namespace SCE
