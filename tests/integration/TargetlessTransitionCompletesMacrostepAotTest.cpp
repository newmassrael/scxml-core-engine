// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D's main event loop returns to
// `selectEventlessTransitions()` after every microstep, and drains the
// internal queue in the same inner loop. It never asks whether the microstep
// it just took moved the machine — it cannot, because W3C SCXML 3.13 defines a
// transition with no `target` as one that exits and enters nothing and runs
// its content in place. C++ AOT path.
//
// This engine asked. `executeTransition` returned early on a transition that
// left the configuration where it was, and the early return skipped both the
// eventless check and the caller's queue drain — so a document could be handed
// back to the host mid-chain, with `getCurrentState()` naming a state the
// document was supposed to have left and nothing anywhere saying so. Measured
// 2026-08-20 while building `eventless_macrostep_is_bounded`, where the other
// five engines walked the chain and this one did not.
//
// `EventlessMacrostepIsBoundedAotTest.cpp` owns how FAR a chain may run; this
// one owns whether the chain is entered at all.
//
// Fixture:
// integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// (canonical, shared with the Interpreter / C11 / Rust / Go / Kotlin / Python channels).
//
// Regeneration: the generated header is built by CMake via
//   sce_generate_static_integration_test(targetless_transition_completes_macrostep ...)

#include "scripting/ScriptEngineProvider.h"
#include "targetless_transition_completes_macrostep_sm.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::targetless_transition_completes_macrostep::targetless_transition_completes_macrostep;

std::unique_ptr<SM> started() {
    auto sm = std::make_unique<SM>();
    if constexpr (SM::PolicyType::NEEDS_SCRIPT_ENGINE) {
        sm->setScriptEngine(std::shared_ptr<::SCE::IScriptEngine>(&::SCE::ScriptEngineProvider::getScriptEngine(),
                                                                  [](::SCE::IScriptEngine *) {}));
    }
    sm->initialize();
    return sm;
}

}  // namespace

/// The axis: a transition that moves nothing still ends a microstep, so the
/// macrostep continues into whatever its content enabled.
///
/// `chained == 1, polished == 0` is the signature of an engine that resumes the
/// chain only after a transition that MOVED the machine: it takes the link that
/// moves and stops before the link that does not. `chained == 0` is the
/// signature of one that never entered the chain at all. Both are failures of
/// the same clause, and the two counters are what tell them apart.
TEST(TargetlessTransitionCompletesMacrostepAotTest, ATargetlessTransitionDoesNotEndTheMacrostep) {
    auto sm = started();

    sm->processEvent(SM::Event::Arm);

    ASSERT_EQ(sm->getPolicy().armed().value_or(-1), 1)
        << "the targetless transition ran its content — without this the rest measures a lost event rather than a "
           "stopped macrostep";
    EXPECT_EQ(sm->getPolicy().chained().value_or(-1), 1)
        << "and the eventless transition that content enabled was taken in the SAME macrostep, which is the whole "
           "of what Appendix D's inner loop promises a host";
    EXPECT_EQ(sm->getPolicy().polished().value_or(-1), 1)
        << "including the chain's last link, which is targetless itself: an engine that walks the chain only while "
           "the machine keeps moving stops exactly here";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Settled)
        << "and the host is handed the stable configuration, not the one the machine was passing through";
}

/// The other side of the same inner loop: what a targetless transition raises
/// is answered before the host gets control back.
///
/// This engine's direct `processEvent` path is where that is easiest to lose —
/// the caller's queue drain is the very work the early return skipped.
TEST(TargetlessTransitionCompletesMacrostepAotTest, ARaiseFromATargetlessTransitionIsAnsweredInTheSameMacrostep) {
    auto sm = started();

    sm->processEvent(SM::Event::Ping);

    EXPECT_EQ(sm->getPolicy().answered().value_or(-1), 1)
        << "the internal event the targetless transition raised was dequeued and matched inside this macrostep";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle)
        << "neither transition moves the machine, which is the point: the macrostep has to continue anyway";
}

/// The control, and the reason a zero above means anything: a targetless
/// transition that enables nothing leaves the machine exactly where it was, and
/// having run is still observable.
TEST(TargetlessTransitionCompletesMacrostepAotTest, ATargetlessTransitionThatEnablesNothingChangesNothingElse) {
    auto sm = started();

    sm->processEvent(SM::Event::Quiet);

    EXPECT_EQ(sm->getPolicy().quiet().value_or(-1), 1) << "the transition fired";
    EXPECT_EQ(sm->getPolicy().chained().value_or(-1), 0)
        << "and nothing else did: the eventless transition's guard is still closed, so an engine that walked the "
           "chain here would be firing a transition the document did not enable";
    EXPECT_EQ(sm->getPolicy().polished().value_or(-1), 0);
    EXPECT_EQ(sm->getPolicy().answered().value_or(-1), 0);
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle);
    EXPECT_TRUE(sm->isRunning());
}

/// The other microstep that ends where it began: a transition whose target is
/// its own source.
///
/// It is not targetless — W3C SCXML 3.13 gives it an exit and an entry — but
/// the loop that dropped the targetless one dropped this too, in the same line
/// of code and for the same reason: it continued only while the configuration
/// kept changing. `entries == 1` is that engine, having selected the transition
/// and run none of it.
TEST(TargetlessTransitionCompletesMacrostepAotTest, AnEventlessSelfTransitionExitsAndReEnters) {
    auto sm = started();

    sm->processEvent(SM::Event::Recycle);

    EXPECT_EQ(sm->getPolicy().entries().value_or(-1), 2)
        << "the state is entered once by `recycle` and once more by the eventless self transition its entry "
           "enabled — a self transition exits and re-enters, so <onentry> runs again";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Recycled)
        << "and the guard closes behind it, so the machine rests here rather than spinning";
}

/// A macrostep, not a one-shot: the second targetless transition is followed
/// the same way the first was.
TEST(TargetlessTransitionCompletesMacrostepAotTest, TheSecondTargetlessTransitionIsFollowedToo) {
    auto sm = started();

    sm->processEvent(SM::Event::Quiet);
    sm->processEvent(SM::Event::Ping);
    ASSERT_EQ(sm->getPolicy().answered().value_or(-1), 1) << "precondition: this test is about the SECOND raise";

    sm->processEvent(SM::Event::Ping);

    EXPECT_EQ(sm->getPolicy().answered().value_or(-1), 2)
        << "the raise in the third macrostep was answered like the one in the second — the inner loop belongs to "
           "every macrostep, not to the first";
    EXPECT_EQ(sm->getPolicy().quiet().value_or(-1), 1);
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle);
}

}  // namespace SCE::Tests
