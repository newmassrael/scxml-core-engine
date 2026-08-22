// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 + Appendix D: an event handed to a machine that has already
// stopped is never looked at, and the host that sent it can find out. C++ AOT.
//
// Appendix D's main event loop exits when the machine reaches a top-level final
// state. Refusing what arrives afterwards is the clause; saying nothing about
// it is not. The silence is expensive because it looks like the two outcomes a
// host can already read:
//
//   dequeued, no transition matched            discardedExternalEvents()
//   dequeued, matched, guard said no           nothing, correctly
//   never dequeued — the machine had stopped   this
//
// Sibling of `UnseenEventIsReportedTest.cpp` (Interpreter channel), which
// asserts the same script against the other engine this repository ships.
//
// Fixture:
// integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
//
// Regeneration: automatic at CMake build time via
// `sce_generate_static_integration_test(unseen_event_is_reported ...)`
// under `${CMAKE_CURRENT_BINARY_DIR}/integration_static_generated/`.

#include "scripting/ScriptEngineProvider.h"
#include "unseen_event_is_reported_sm.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::unseen_event_is_reported::unseen_event_is_reported;

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

/// The axis: an event the host handed over after the machine stopped.
TEST(UnseenEventIsReportedAotTest, AnEventDeliveredAfterTheMachineStoppedIsCounted) {
    auto sm = started();
    ASSERT_EQ(sm->unseenExternalEvents(), 0u) << "nothing has been refused before the first event";

    sm->processEvent(SM::Event::Poke);
    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1)
        << "`poke`'s transition did not run, so nothing below is measuring a machine that was "
           "working first";

    sm->processEvent(SM::Event::Finish);
    ASSERT_TRUE(sm->isInFinalState()) << "`finish` should have taken the machine to its top-level final state";
    EXPECT_EQ(sm->unseenExternalEvents(), 0u)
        << "`finish` was itself dequeued and handled — the machine stopped BECAUSE of it, which is "
           "not the same as stopping before it";

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->unseenExternalEvents(), 1u)
        << "the host handed `poke` to a machine that had reached its final state. W3C SCXML "
           "Appendix D's loop had already ended, so the event was never looked at; before this "
           "count the host had no way to learn that";
    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1)
        << "the refused delivery ran the document's transition anyway — the count would then be "
           "reporting something that did not happen";
}

/// Why the query has to exist at all: every other accessor answers the same
/// before and after the refused delivery.
TEST(UnseenEventIsReportedAotTest, TheRefusalIsNotDerivableFromAnyOtherAccessor) {
    auto sm = started();
    sm->processEvent(SM::Event::Finish);

    const auto beforeState = sm->getCurrentState();
    const auto beforeActive = sm->getActiveStates();
    const bool beforeRunning = sm->isRunning();
    const bool beforeFinal = sm->isInFinalState();
    const uint32_t beforeDiscarded = sm->discardedExternalEvents();
    const auto beforePokes = sm->getPolicy().pokes().value_or(-1);

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->getCurrentState(), beforeState);
    EXPECT_EQ(sm->getActiveStates(), beforeActive);
    EXPECT_EQ(sm->isRunning(), beforeRunning);
    EXPECT_EQ(sm->isInFinalState(), beforeFinal);
    EXPECT_EQ(sm->discardedExternalEvents(), beforeDiscarded);
    EXPECT_EQ(sm->getPolicy().pokes().value_or(-2), beforePokes)
        << "this fixture exists because a refused delivery is indistinguishable through the "
           "accessors a host had; if they ever differ, the fixture stopped measuring what it "
           "claims";

    EXPECT_EQ(sm->unseenExternalEvents(), 1u)
        << "the two readings agree on everything else, so this count is the only thing that "
           "separates `the machine never looked` from `it looked and nothing matched`";
}

/// A discard and a refusal are different facts, and each has its own count.
TEST(UnseenEventIsReportedAotTest, ADiscardAndARefusalAreCountedSeparately) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);
    EXPECT_EQ(sm->discardedExternalEvents(), 0u) << "`poke` matched a targetless transition";
    EXPECT_EQ(sm->unseenExternalEvents(), 0u) << "the machine was running, so nothing was refused";

    sm->processEvent(SM::Event::Finish);
    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->discardedExternalEvents(), 0u)
        << "a refusal must not be reported as a discard: the first says the machine looked and "
           "nothing matched, the second says it never looked, and a host acts differently on each";
    EXPECT_EQ(sm->unseenExternalEvents(), 1u);
}

/// A count says an event went unlooked-at; a host debugging a supervisor that
/// stopped answering needs to know which one.
TEST(UnseenEventIsReportedAotTest, TheEngineNamesTheEventItNeverLookedAt) {
    auto sm = started();
    EXPECT_FALSE(sm->lastUnseenEvent().has_value()) << "nothing has been refused yet";

    sm->processEvent(SM::Event::Finish);
    sm->processEvent(SM::Event::Poke);
    ASSERT_TRUE(sm->lastUnseenEvent().has_value())
        << "the engine counted a refusal but cannot say which event it refused";
    EXPECT_EQ(*sm->lastUnseenEvent(), SM::Event::Poke);

    sm->processEvent(SM::Event::Finish);
    EXPECT_EQ(sm->unseenExternalEvents(), 2u) << "the count is a count, not a flag";
    ASSERT_TRUE(sm->lastUnseenEvent().has_value());
    EXPECT_EQ(*sm->lastUnseenEvent(), SM::Event::Finish) << "the name did not follow the second refusal";
}

}  // namespace SCE::Tests
