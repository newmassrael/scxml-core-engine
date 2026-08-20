// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// `selectEventlessTransitions()` after every microstep, and drains the
// internal queue in the same inner loop. It never asks whether the microstep
// it just took moved the machine — it cannot, because W3C SCXML 3.13 defines a
// transition with no `target` as one that exits and enters nothing and runs
// its content in place. C++ Interpreter path.
//
// This engine asked. Its macrostep loop lives in the branch of
// `executeTransition` that has entered a target state; the branch that handles
// a transition with no target ran the content and returned, so the chain that
// content enabled was never walked. The host got back a machine that is
// running, in a state the document names, in a configuration the clause says
// is not stable. Measured 2026-08-20 while building
// `eventless_macrostep_is_bounded`, where the other five engines walked the
// chain and this one did not.
//
// `EventlessMacrostepIsBoundedTest.cpp` owns how FAR a chain may run; this one
// owns whether the chain is entered at all.
//
// Fixture:
// integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// (canonical, shared with the AOT / C11 / Rust / Go / Kotlin / Python channels).

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

class TargetlessTransitionCompletesMacrostepTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/targetless_transition_completes_macrostep/"
                                    "targetless_transition_completes_macrostep.scxml";
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

    /// The fixture's `<assign>`s are the only witness of how far the macrostep
    /// got: every outcome here leaves the machine in a state the configuration
    /// alone cannot tell apart from a macrostep that stopped one microstep
    /// early.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: a transition that moves nothing still ends a microstep, so the
/// macrostep continues into whatever its content enabled.
///
/// `chained == 1, polished == 0` is the signature of an engine that resumes the
/// chain only after a transition that MOVED the machine: it takes the link that
/// moves and stops before the link that does not. `chained == 0` is the
/// signature of one that never entered the chain at all. Both are failures of
/// the same clause, and the two counters are what tell them apart.
TEST_F(TargetlessTransitionCompletesMacrostepTest, ATargetlessTransitionDoesNotEndTheMacrostep) {
    sm_->processEvent("arm");

    ASSERT_EQ(counter("armed"), "1")
        << "the targetless transition ran its content — without this the rest measures a lost event rather than a "
           "stopped macrostep";
    EXPECT_EQ(counter("chained"), "1")
        << "and the eventless transition that content enabled was taken in the SAME macrostep, which is the whole "
           "of what Appendix D's inner loop promises a host";
    EXPECT_EQ(counter("polished"), "1")
        << "including the chain's last link, which is targetless itself: an engine that walks the chain only while "
           "the machine keeps moving stops exactly here";
    EXPECT_EQ(sm_->getCurrentState(), "settled")
        << "and the host is handed the stable configuration, not the one the machine was passing through";
}

/// The other side of the same inner loop: what a targetless transition raises
/// is answered before the host gets control back.
TEST_F(TargetlessTransitionCompletesMacrostepTest, ARaiseFromATargetlessTransitionIsAnsweredInTheSameMacrostep) {
    sm_->processEvent("ping");

    EXPECT_EQ(counter("answered"), "1")
        << "the internal event the targetless transition raised was dequeued and matched inside this macrostep";
    EXPECT_EQ(sm_->getCurrentState(), "idle")
        << "neither transition moves the machine, which is the point: the macrostep has to continue anyway";
}

/// The control, and the reason a zero above means anything: a targetless
/// transition that enables nothing leaves the machine exactly where it was, and
/// having run is still observable.
TEST_F(TargetlessTransitionCompletesMacrostepTest, ATargetlessTransitionThatEnablesNothingChangesNothingElse) {
    sm_->processEvent("quiet");

    EXPECT_EQ(counter("quiet"), "1") << "the transition fired";
    EXPECT_EQ(counter("chained"), "0")
        << "and nothing else did: the eventless transition's guard is still closed, so an engine that walked the "
           "chain here would be firing a transition the document did not enable";
    EXPECT_EQ(counter("polished"), "0");
    EXPECT_EQ(counter("answered"), "0");
    EXPECT_EQ(sm_->getCurrentState(), "idle");
    EXPECT_TRUE(sm_->isRunning());
}

/// The other microstep that ends where it began: a transition whose target is
/// its own source.
///
/// It is not targetless — W3C SCXML 3.13 gives it an exit and an entry — and it
/// is here so both engines answer the same document. The AOT engine is where
/// the two cases shared a line of code.
TEST_F(TargetlessTransitionCompletesMacrostepTest, AnEventlessSelfTransitionExitsAndReEnters) {
    sm_->processEvent("recycle");

    EXPECT_EQ(counter("entries"), "2")
        << "the state is entered once by `recycle` and once more by the eventless self transition its entry "
           "enabled — a self transition exits and re-enters, so <onentry> runs again";
    EXPECT_EQ(sm_->getCurrentState(), "recycled")
        << "and the guard closes behind it, so the machine rests here rather than spinning";
}

/// A macrostep, not a one-shot: the second targetless transition is followed
/// the same way the first was.
TEST_F(TargetlessTransitionCompletesMacrostepTest, TheSecondTargetlessTransitionIsFollowedToo) {
    sm_->processEvent("quiet");
    sm_->processEvent("ping");
    ASSERT_EQ(counter("answered"), "1") << "precondition: this test is about the SECOND raise";

    sm_->processEvent("ping");

    EXPECT_EQ(counter("answered"), "2")
        << "the raise in the third macrostep was answered like the one in the second — the inner loop belongs to "
           "every macrostep, not to the first";
    EXPECT_EQ(counter("quiet"), "1");
    EXPECT_EQ(sm_->getCurrentState(), "idle");
}

}  // namespace Tests
}  // namespace SCE
