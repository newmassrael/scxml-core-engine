// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// `error.*` events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. C++ AOT path.
//
// `discarded_event_is_observable` asked this for the EXTERNAL queue and stopped
// at its edge on the stated ground that an unmatched internal event "is the
// document's own business, and both ends of it are in the document". That is
// exactly right for an author's `<raise>` and exactly wrong for an error event,
// whose sender is the ENGINE. The host never wrote the document, cannot see the
// failure in the configuration, and is the only party able to act on it.
//
// Four outcomes the fixture separates, all four leaving the configuration on
// the same state:
//
//   poke               handled, no error            control: proves a run fired
//   whisper            author's <raise>, unmatched  NOT counted
//   boom in `idle`     error, unmatched             COUNTED — the silent failure
//   boom in `guarded`  error, HANDLED               not counted
//
// `boom` is one event name routed to two outcomes by state, so a count cannot
// be keyed off the event or the action — only off what the configuration did
// with the error the engine raised.
//
// The Interpreter channel of this same fixture is where the parity claim is
// checked; `UnhandledErrorIsObservableTest.cpp` asserts what that engine says
// about the same script.
//
// Sibling of `UnhandledErrorIsObservableTest.cpp` (Interpreter channel).
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(unhandled_error_is_observable ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "unhandled_error_is_observable_sm.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::unhandled_error_is_observable::unhandled_error_is_observable;

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

/// The axis: an error the engine raised that no active state answers is counted.
TEST(UnhandledErrorIsObservableAotTest, AnErrorNoTransitionAnsweredIsCounted) {
    auto sm = started();
    ASSERT_EQ(sm->unhandledErrorEvents(), 0u) << "no error has gone unhandled before the first event";

    sm->processEvent(SM::Event::Boom);

    EXPECT_EQ(sm->getPolicy().booms().value_or(-1), 1)
        << "`boom`'s transition did not run, so nothing below is measuring an error raised "
           "inside a transition that fired";
    EXPECT_EQ(sm->unhandledErrorEvents(), 1u)
        << "`boom`'s second <assign> has W3C 5.3's invalid empty location, so the engine raised "
           "error.execution — and `idle` declares no transition for it. The host driving this "
           "machine has no other way to learn its <assign> failed";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle) << "the error must not move the machine on its own";
}

/// The other half: an error the DOCUMENT answered must not be counted. A count
/// that is always non-zero is as useless as one that is always zero.
TEST(UnhandledErrorIsObservableAotTest, AnErrorTheDocumentHandledIsNotCounted) {
    auto sm = started();

    sm->processEvent(SM::Event::Go);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Guarded)
        << "`go` should have moved the machine to the state that answers errors";

    sm->processEvent(SM::Event::Boom);

    EXPECT_EQ(sm->getPolicy().caught().value_or(-1), 1)
        << "`guarded`'s error.execution transition did not run, so this test is not measuring a "
           "HANDLED error";
    EXPECT_EQ(sm->unhandledErrorEvents(), 0u)
        << "the same <assign> failed in `guarded`, where the document does declare a transition "
           "for error.execution. The document dealt with it, and its handling is already visible "
           "in the configuration — counting it would report the author's own error handling as a "
           "silent failure";
    EXPECT_FALSE(sm->lastUnhandledError().has_value()) << "nothing went unhandled, so there is no last one to name";
}

/// The boundary the count is drawn at: an author's own unmatched `<raise>` is
/// not an unhandled error. Both ends of that event are inside the document,
/// which is why `discardedExternalEvents()` stops at the external queue — and
/// why this count does not stop there.
TEST(UnhandledErrorIsObservableAotTest, AnAuthorsUnmatchedRaiseIsNotAnUnhandledError) {
    auto sm = started();

    sm->processEvent(SM::Event::Whisper);

    EXPECT_EQ(sm->unhandledErrorEvents(), 0u)
        << "`whisper` raises `unheard` and `retry.error.execution`, neither of which any state "
           "answers. Both are discarded exactly as an unmatched error is, and neither is one: "
           "the author wrote the raises and the absent handlers. `retry.error.execution` is the "
           "sharper half — it CONTAINS `error.` without starting with it, and W3C 3.12.2 "
           "reserves the prefix, not the substring";
    EXPECT_EQ(sm->getPolicy().heards().value_or(-1), 1)
        << "`whisper`'s third raise, `heard`, does match — and the transition it matches did not "
           "run. The count above is a byproduct of the internal drain, never its job: an "
           "implementation that only selects transitions for error events stops running the "
           "document for everything else";
    EXPECT_EQ(sm->discardedExternalEvents(), 0u)
        << "`whisper` itself was handled, so the external-queue count stays put — the internal "
           "events it raised are not on that queue at all";
}

/// Why the query has to exist: every pre-existing accessor answers the same for
/// a run that failed silently and one that did not fail at all.
TEST(UnhandledErrorIsObservableAotTest, TheUnhandledErrorIsNotDerivableFromAnyOtherAccessor) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);
    const auto cleanState = sm->getCurrentState();
    const auto cleanActive = sm->getActiveStates();
    const bool cleanRunning = sm->isRunning();
    const bool cleanFinal = sm->isInFinalState();
    const uint32_t cleanDiscarded = sm->discardedExternalEvents();

    sm->processEvent(SM::Event::Boom);

    // If any of these ever differ, the fixture stopped measuring what it claims:
    // the count below is only interesting because nothing else separates the two.
    EXPECT_EQ(sm->getCurrentState(), cleanState);
    EXPECT_EQ(sm->getActiveStates(), cleanActive);
    EXPECT_EQ(sm->isRunning(), cleanRunning);
    EXPECT_EQ(sm->isInFinalState(), cleanFinal);
    EXPECT_EQ(sm->discardedExternalEvents(), cleanDiscarded)
        << "layer three's discard count never sees the internal queue, so it cannot be the thing "
           "that separates these two runs";
    EXPECT_EQ(sm->unhandledErrorEvents(), 1u)
        << "the two are indistinguishable through every other accessor, so this count is the only "
           "thing that separates a silent failure from a clean run";
}

/// A count says something failed; a host repairing it needs the class of error.
TEST(UnhandledErrorIsObservableAotTest, TheEngineNamesTheErrorItDropped) {
    auto sm = started();
    ASSERT_FALSE(sm->lastUnhandledError().has_value()) << "nothing has gone unhandled yet";

    sm->processEvent(SM::Event::Boom);

    ASSERT_TRUE(sm->lastUnhandledError().has_value())
        << "the engine counted an unhandled error but reports none to name";
    EXPECT_EQ(sm->lastUnhandledError().value(), SM::Event::Error_execution)
        << "`error.execution` is the document's own executable content failing; "
           "`error.communication` would be a <send> that could not reach its target. Two "
           "different repairs, and a bare count separates neither";
}

/// The supervisor's actual failure mode: every round fails the same way and
/// nothing in the configuration ever changes.
TEST(UnhandledErrorIsObservableAotTest, AMachineFailingEveryRoundIsCountedEveryRound) {
    auto sm = started();

    for (uint32_t round = 1; round <= 3; ++round) {
        sm->processEvent(SM::Event::Boom);
        EXPECT_EQ(sm->unhandledErrorEvents(), round)
            << "round " << round
            << " did not add to the count; a supervisor polling this number is exactly who learns "
               "the loop is not making progress";
        EXPECT_EQ(sm->getCurrentState(), SM::State::Idle)
            << "the machine looks identical on every round, which is the problem";
    }
    EXPECT_EQ(sm->getPolicy().booms().value_or(-1), 3) << "all three rounds should have run their transition";
}

}  // namespace SCE::Tests
