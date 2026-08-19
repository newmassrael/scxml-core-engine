// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.1.2: "If no transition matches in any state, the event is
// discarded" — and the host that fed it in can find out. C++ AOT path.
//
// Three outcomes leave the configuration identical, so no accessor that existed
// before this fixture separates them:
//
//   poke    self transition       handled (exits and re-enters `idle`)
//   nudge   targetless internal   handled (actions only, no exit/entry)
//   settle  no matching           DISCARDED — the host's event went nowhere
//
// The Interpreter channel of this same fixture already had the answer:
// `StateMachine::processEvent` returns a `TransitionResult` whose `success` is
// false, and `getStatistics().failedTransitions` counts them. Both engines ship
// from this repository, so a document that moves from one to the other must not
// lose the signal — `DiscardedEventIsObservableTest.cpp` asserts the same
// script against the Interpreter and the two counts are compared there.
//
// `nudge` is in the fixture because this engine's own `EventOutcome` carries
// two bools for a reason: a targetless internal transition runs its actions and
// still reports `configurationChanged == false`. A count keyed off that bool
// would call a handled event discarded.
//
// Sibling of `DiscardedEventIsObservableTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(discarded_event_is_observable ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "discarded_event_is_observable_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::discarded_event_is_observable::discarded_event_is_observable;

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

/// The axis: an event the machine knows but no active state answers is counted.
TEST(DiscardedEventIsObservableAotTest, AnEventNoActiveStateAnsweredIsCounted) {
    auto sm = started();
    ASSERT_EQ(sm->discardedExternalEvents(), 0u) << "nothing has been discarded before the first event";

    // `settle` is declared in `busy`, so it is in the machine's vocabulary and
    // the host can name it — it just matches nothing while the machine is in
    // `idle`. That is the host-side wiring mistake this count exists for.
    sm->processEvent(SM::Event::Settle);

    EXPECT_EQ(sm->discardedExternalEvents(), 1u)
        << "`settle` reached the engine in `idle`, where no transition matches it. W3C SCXML "
           "3.1.2 discards it; the host that sent it has no other way to learn its event went "
           "nowhere";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle) << "a discarded event must not move the machine";
}

/// The other half: a handled event must NOT be counted, including the one that
/// changes nothing. A count that is always non-zero is as useless as one that
/// is always zero.
TEST(DiscardedEventIsObservableAotTest, AHandledEventIsNotCounted) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);
    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1)
        << "`poke`'s self transition did not run, so nothing below is measuring a handled event";
    EXPECT_EQ(sm->discardedExternalEvents(), 0u)
        << "`poke` matched a self transition — handled, and the configuration is unchanged only "
           "because the transition returns to its own source";

    sm->processEvent(SM::Event::Nudge);
    EXPECT_EQ(sm->getPolicy().nudges().value_or(-1), 1) << "`nudge`'s targetless transition did not run";
    EXPECT_EQ(sm->discardedExternalEvents(), 0u)
        << "`nudge` matched a targetless internal transition: its actions ran and no state was "
           "exited or entered, which is exactly why the count cannot be keyed off "
           "EventOutcome::configurationChanged";
}

/// Why the query has to exist at all: every pre-existing accessor answers the
/// same for a handled event and a discarded one.
TEST(DiscardedEventIsObservableAotTest, TheDiscardIsNotDerivableFromAnyOtherAccessor) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);
    const auto handledState = sm->getCurrentState();
    const auto handledActive = sm->getActiveStates();
    const bool handledRunning = sm->isRunning();
    const bool handledFinal = sm->isInFinalState();

    sm->processEvent(SM::Event::Settle);

    EXPECT_EQ(sm->getCurrentState(), handledState);
    EXPECT_EQ(sm->getActiveStates(), handledActive);
    EXPECT_EQ(sm->isRunning(), handledRunning);
    EXPECT_EQ(sm->isInFinalState(), handledFinal)
        << "this fixture exists because a handled event and a discarded one are indistinguishable "
           "through the accessors a host had; if they ever differ, the fixture stopped measuring "
           "what it claims";
    EXPECT_EQ(sm->discardedExternalEvents(), 1u)
        << "the two are indistinguishable through every other accessor, so the count is the only "
           "thing that separates them";
}

/// A count says something went nowhere; a host debugging a stalled supervisor
/// needs to know which event did.
TEST(DiscardedEventIsObservableAotTest, TheEngineNamesTheEventItDiscarded) {
    auto sm = started();
    EXPECT_FALSE(sm->lastDiscardedEvent().has_value()) << "nothing has been discarded yet";

    sm->processEvent(SM::Event::Settle);

    ASSERT_TRUE(sm->lastDiscardedEvent().has_value())
        << "the engine counted a discard but cannot say which event it was";
    EXPECT_EQ(sm->lastDiscardedEvent().value(), SM::Event::Settle);
}

/// The supervisor's actual failure mode: the machine moved on, and the events
/// the host keeps sending no longer match anything.
TEST(DiscardedEventIsObservableAotTest, AnEventTheMachineHasMovedPastIsCounted) {
    auto sm = started();
    sm->processEvent(SM::Event::Go);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Busy) << "`go` should have moved the machine out of `idle`";

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->discardedExternalEvents(), 1u)
        << "the machine left `idle`, so `poke` no longer matches — the host that kept sending it "
           "is exactly who the count is for";
    ASSERT_TRUE(sm->lastDiscardedEvent().has_value());
    EXPECT_EQ(sm->lastDiscardedEvent().value(), SM::Event::Poke);
}

}  // namespace SCE::Tests
