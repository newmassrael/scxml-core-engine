// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C++ AOT
// path.
//
// `EventlessMacrostepIsBoundedAotTest.cpp` owns the half of that clause built
// from transitions that need no event. This one owns the other half: a
// `<raise>` answered by a transition that raises again. The eventless ceiling
// this engine already had did nothing for it — `processInternalQueue` drained
// the queue to exhaustion, and the shared `processInternalEventQueue` helper
// had no way to be told to stop — so the document did not return.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
// (canonical, shared with the Interpreter / C11 / Rust / Go / Kotlin / Python channels).
//
// Regeneration: the generated header is built by CMake via
//   sce_generate_static_integration_test(internal_chain_is_bounded ...)

#include "internal_chain_is_bounded_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::internal_chain_is_bounded::internal_chain_is_bounded;

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude — and the number is
/// exactly what this fixture exists to pin.
constexpr int64_t MAX_MICROSTEPS = 1000;

/// One lap of the alternating chain is two microsteps — one internal event, one
/// eventless transition — and only the internal half is counted, so a chain run
/// to the shared ceiling records half.
constexpr int64_t ALTERNATING_LAPS_AT_CEILING = MAX_MICROSTEPS / 2;

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

/// The axis: a macrostep whose `<raise>` chain cannot end is stopped, and the
/// host is told that it was.
TEST(InternalChainIsBoundedAotTest, ARaiseChainThatCannotEndIsStopped) {
    auto sm = started();
    ASSERT_EQ(sm->truncatedMacrosteps(), 0u) << "nothing has been refused before the machine has done anything";

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->getPolicy().links().value_or(-1), MAX_MICROSTEPS)
        << "the chain must run exactly as far as the engine allows — fewer means the document was cut off early, "
           "more means the ceiling moved";
    EXPECT_EQ(sm->truncatedMacrosteps(), 1u)
        << "the microstep past the budget was queued and was not taken. Without this count the host sees a "
           "machine that is running, in a state the document names, having returned at once — and no way to learn "
           "that the configuration it is reading is not a stable one";
    ASSERT_TRUE(sm->lastTruncatedMacrostepState().has_value())
        << "the engine counted a stopped macrostep but reports no state to name";
    EXPECT_EQ(sm->lastTruncatedMacrostepState().value(), SM::State::Spin)
        << "the count alone says a document somewhere cannot settle; this says where to look";
    EXPECT_TRUE(sm->isRunning())
        << "the chain was cut, not the machine. §scxml-D allows the document; refusing to run it forever is the "
           "engine's decision to report, not a reason to stop a machine whose other states still work";
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends on its own is not refused, however long it is.
TEST(InternalChainIsBoundedAotTest, ARaiseChainThatEndsAtTheCeilingIsNotRefused) {
    auto sm = started();

    sm->processEvent(SM::Event::Bounded);

    EXPECT_EQ(sm->getPolicy().laps().value_or(-1), MAX_MICROSTEPS)
        << "the guard `laps < 999` stops matching at the thousandth link, which raises nothing — so the queue "
           "empties and the chain stops by itself";
    EXPECT_EQ(sm->truncatedMacrosteps(), 0u)
        << "nothing was refused: the macrostep reached the stable configuration §scxml-3.13 describes, using every "
           "microstep it was allowed. A long chain is not a runaway";
    EXPECT_FALSE(sm->lastTruncatedMacrostepState().has_value())
        << "and nothing names a state, because nothing was stopped";
    EXPECT_TRUE(sm->isRunning())
        << "a document that settles on its own must not be reported dead by an engine that just finished running "
           "it correctly";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Bounded) << "the chain rests where it ended";
}

/// The case a per-branch budget lets through: a chain that alternates one
/// `<raise>` with one eventless transition.
///
/// Neither branch of §scxml-D's inner loop reaches the ceiling on its own here,
/// so an engine that gives each branch a counter of its own runs this document
/// forever with both ceilings half spent.
TEST(InternalChainIsBoundedAotTest, AnAlternatingChainSpendsOneSharedBudget) {
    auto sm = started();

    sm->processEvent(SM::Event::Alternate);

    EXPECT_EQ(sm->getPolicy().alts().value_or(-1), ALTERNATING_LAPS_AT_CEILING)
        << "the two branches share one budget, so a chain that alternates them gets five hundred laps out of a "
           "thousand microsteps. A thousand here would mean the internal branch had a ceiling of its own";
    EXPECT_EQ(sm->truncatedMacrosteps(), 1u)
        << "and the refusal is reported once, whichever branch was holding the budget when it ran out";
    ASSERT_TRUE(sm->lastTruncatedMacrostepState().has_value());
    EXPECT_EQ(sm->lastTruncatedMacrostepState().value(), SM::State::Alt)
        << "named the same way as any other chain that could not settle";
}

/// What the refusal did with the links it would not run: it left them queued.
///
/// The fixture's `resume` chain is half again as long as the ceiling, so the
/// first macrostep is refused with five hundred links still to go and the
/// second one finishes them. An engine that dropped the queue stops at a
/// thousand and never finishes; one that ran the chain anyway finishes it in
/// the first macrostep.
///
/// `poke` is here only to drive a second macrostep. What it does is not
/// asserted: this entry point takes the host's event ahead of the queued chain
/// rather than through the external queue, which is a divergence from the
/// six queue-driven channels and a debt of its own.
TEST(InternalChainIsBoundedAotTest, ARefusedChainIsLeftQueuedForTheNextMacrostep) {
    auto sm = started();

    sm->processEvent(SM::Event::Resume);
    ASSERT_EQ(sm->getPolicy().beats().value_or(-1), MAX_MICROSTEPS)
        << "the first macrostep spends the whole budget on the chain";
    ASSERT_EQ(sm->truncatedMacrosteps(), 1u) << "precondition: the first macrostep was refused";

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->getPolicy().beats().value_or(-1), MAX_MICROSTEPS + MAX_MICROSTEPS / 2)
        << "the second macrostep picked the chain up where the first was cut and ran it to its end — the refused "
           "links were left on the queue, not dropped";
    EXPECT_EQ(sm->truncatedMacrosteps(), 1u)
        << "and nothing was refused this time: the chain ended on its own inside the budget, which is an ordinary "
           "macrostep however long the document took to get there";
    EXPECT_TRUE(sm->isRunning());
}

/// The control: an ordinary document is untouched by any of this. Without it,
/// an engine that refused every macrostep would pass the assertions above and
/// fail nothing.
TEST(InternalChainIsBoundedAotTest, AnOrdinaryMacrostepIsNotCounted) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1) << "the run fired";
    EXPECT_EQ(sm->truncatedMacrosteps(), 0u) << "a macrostep of one microstep ends the way the clause says it does";
    EXPECT_FALSE(sm->lastTruncatedMacrostepState().has_value());
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle);
}

}  // namespace SCE::Tests
