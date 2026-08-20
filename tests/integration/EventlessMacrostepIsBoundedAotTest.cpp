// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C++ AOT
// path.
//
// This engine answered that case a third way. Measured 2026-08-20: where the
// Python engine ran the document forever and the other five stopped the chain
// and carried on, this one called `stop()` — so the same document came back
// dead here and merely paused there. Its ceiling also counted loop turns
// rather than microsteps taken, which put the verdict one short: a chain of
// ninety-nine microsteps that settled on its own was stopped too, by an engine
// that had already finished running it correctly.
//
// `ErrorCascadeIsBoundedAotTest.cpp` owns the chain built from errors; this
// one owns the chain built from transitions that need no event at all. The
// fixture separates a chain that stops on its own — a HUNDRED microsteps,
// exactly the ceiling — from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// (canonical, shared with the Interpreter / C11 / Rust / Go / Kotlin / Python channels).
//
// Regeneration: the generated header is built by CMake via
//   sce_generate_static_integration_test(eventless_macrostep_is_bounded ...)

#include "eventless_macrostep_is_bounded_sm.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>
#include <memory>

namespace SCE::Tests {
namespace {

using SM = SCE::Generated::eventless_macrostep_is_bounded::eventless_macrostep_is_bounded;

/// The ceiling the engine applies, spelled here rather than read back from it.
/// A test that asked the engine for its own limit would agree with any limit,
/// including one an edit moved by three orders of magnitude — and the number is
/// exactly what this fixture exists to pin.
constexpr int64_t MAX_MICROSTEPS = 1000;

/// One lap of either chain is two microsteps (`_a` to `_b`, then back) and only
/// the `_a` edge counts, so a chain run to the ceiling records half.
constexpr int64_t LAPS_AT_CEILING = MAX_MICROSTEPS / 2;

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

/// The axis: a macrostep whose eventless chain cannot end is stopped, and the
/// host is told that it was.
TEST(EventlessMacrostepIsBoundedAotTest, AMacrostepThatCannotEndIsStopped) {
    auto sm = started();
    ASSERT_EQ(sm->truncatedMacrosteps(), 0u) << "nothing has been refused before the machine has done anything";

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->getPolicy().spins().value_or(-1), LAPS_AT_CEILING)
        << "the chain must run exactly as far as the engine allows — fewer means the document was cut off early, "
           "more means the ceiling moved";
    EXPECT_EQ(sm->truncatedMacrosteps(), 1u)
        << "the microstep past the budget was enabled and was not taken. Without this count the host sees a "
           "machine that is running, in a state the document names, having returned at once — and no way to learn "
           "that the configuration it is reading is not a stable one";
    ASSERT_TRUE(sm->lastTruncatedMacrostepState().has_value())
        << "the engine counted a stopped macrostep but reports no state to name";
    EXPECT_EQ(sm->lastTruncatedMacrostepState().value(), SM::State::Spin_a)
        << "an eventless cycle is a closed walk through the state graph, and the count alone does not say which "
           "walk. This names a state on it, which is where an author looks first";
    EXPECT_TRUE(sm->isRunning())
        << "the chain was cut, not the machine. §scxml-D allows the document; refusing to run it forever is the "
           "engine's decision to report, not a reason to stop a machine whose other states still work";
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends on its own is not refused, however long it is.
///
/// This is the assertion the previous ceiling failed. Its loop counted turns
/// and then tested `>=`, so a chain that settled at the ceiling was reported as
/// a runaway — and the report called `stop()`.
TEST(EventlessMacrostepIsBoundedAotTest, AChainThatEndsAtTheCeilingIsNotRefused) {
    auto sm = started();

    sm->processEvent(SM::Event::Bounded);

    EXPECT_EQ(sm->getPolicy().laps().value_or(-1), LAPS_AT_CEILING)
        << "the guard `laps < 500` closes after five hundred laps, so the chain is a thousand microsteps long and "
           "then stops by itself";
    EXPECT_EQ(sm->truncatedMacrosteps(), 0u)
        << "nothing was refused: the macrostep reached the stable configuration §scxml-3.13 describes, using every "
           "microstep it was allowed. A long chain is not a runaway";
    EXPECT_FALSE(sm->lastTruncatedMacrostepState().has_value())
        << "and nothing names a state, because nothing was stopped";
    EXPECT_TRUE(sm->isRunning())
        << "a document that settles on its own must not be reported dead by an engine that just finished running "
           "it correctly";
    EXPECT_EQ(sm->getCurrentState(), SM::State::Bounded_a) << "the chain rests where its guard closed";
}

/// A count, not a flag: a second unbounded macrostep is refused the same way
/// the first was.
TEST(EventlessMacrostepIsBoundedAotTest, ASecondTruncatedMacrostepCountsAgain) {
    auto sm = started();

    sm->processEvent(SM::Event::Spin);
    ASSERT_EQ(sm->truncatedMacrosteps(), 1u) << "precondition: this test is about the SECOND refusal";

    // `reset` is the fixture's way back out of the cycle, and it moves the
    // machine on purpose: this engine completes a macrostep only after a
    // transition that does.
    sm->processEvent(SM::Event::Reset);
    ASSERT_EQ(sm->getCurrentState(), SM::State::Idle) << "reset is the way back out of the chain";

    sm->processEvent(SM::Event::Spin);

    EXPECT_EQ(sm->truncatedMacrosteps(), 2u)
        << "the second macrostep hit the same ceiling and must be counted again — a count that saturated at one "
           "would read as a machine that recovered";
    EXPECT_EQ(sm->getPolicy().spins().value_or(-1), 2 * LAPS_AT_CEILING)
        << "and it really bought the document a full budget again rather than refusing on sight — the ceiling "
           "bounds a macrostep, it does not condemn a machine";
}

/// The control: an ordinary document is untouched by any of this. Without it,
/// an engine that refused every macrostep would pass the assertions above and
/// fail nothing.
TEST(EventlessMacrostepIsBoundedAotTest, AnOrdinaryMacrostepIsNotCounted) {
    auto sm = started();

    sm->processEvent(SM::Event::Poke);

    EXPECT_EQ(sm->getPolicy().pokes().value_or(-1), 1) << "the run fired";
    EXPECT_EQ(sm->truncatedMacrosteps(), 0u) << "a macrostep of one microstep ends the way the clause says it does";
    EXPECT_FALSE(sm->lastTruncatedMacrostepState().has_value());
    EXPECT_EQ(sm->getCurrentState(), SM::State::Idle);
}

}  // namespace SCE::Tests
