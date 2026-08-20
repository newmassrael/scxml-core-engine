// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 says a macrostep is a chain of microsteps ending in a
// configuration where nothing is enabled by NULL. Appendix D's Principles and
// Constraints then say the chain need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C++
// Interpreter path.
//
// This engine's ceiling did come back, and reported the opposite of what had
// happened: `checkEventlessTransitions` returned false — "no eventless
// transition occurred" — after a hundred of them had, and the only other trace
// was a log line. `getStatistics()` is where a host already looks for what the
// machine has been doing, and it had nothing to say about a macrostep that had
// been stopped mid-walk.
//
// `ErrorCascadeIsBoundedTest.cpp` owns the chain built from errors; this one
// owns the chain built from transitions that need no event at all. The fixture
// separates a chain that stops on its own — a HUNDRED microsteps, exactly the
// ceiling — from one that cannot stop.
//
// Fixture: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
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

class EventlessMacrostepIsBoundedTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) +
            "/integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml";
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

    /// The fixture's `<assign>`s are the only witness of how far a chain got —
    /// the configuration alone cannot tell a chain that stopped from one that
    /// was stopped.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: `processEvent` comes back, and the statistics say why.
///
/// A document whose eventless transitions form a cycle has no end the document
/// can spell, so an engine that walks it to the letter never returns to its
/// caller — and the caller is the only party that could stop it.
TEST_F(EventlessMacrostepIsBoundedTest, AMacrostepThatCannotEndIsStopped) {
    ASSERT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "nothing has been refused before the machine has done anything";

    sm_->processEvent("spin");

    EXPECT_EQ(counter("spins"), "500")
        << "the chain must run exactly as far as the engine allows: a thousand microsteps, and the fixture's cycle "
           "takes two of them a lap — fewer means the document was cut off early, more means the ceiling moved";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u)
        << "the microstep past the budget was enabled and was not taken. Without this count the host sees a "
           "machine that is running, in a state the document names, having returned at once — and no way to learn "
           "that the configuration it is reading is not a stable one";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedMacrostepState, "spin_a")
        << "an eventless cycle is a closed walk through the state graph, and the count alone does not say which "
           "walk. This names a state on it, which is where an author looks first";
    EXPECT_TRUE(sm_->isRunning())
        << "the chain was cut, not the machine. §scxml-D allows the document; refusing to run it forever is the "
           "engine's decision to report, not a reason to stop a machine whose other states still work";
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends on its own is not refused, however long it is.
TEST_F(EventlessMacrostepIsBoundedTest, AChainThatEndsAtTheCeilingIsNotRefused) {
    sm_->processEvent("bounded");

    EXPECT_EQ(counter("laps"), "500")
        << "the guard `laps < 500` closes after five hundred laps, so the chain is a thousand microsteps long and "
           "then stops by itself";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "nothing was refused: the macrostep reached the stable configuration §scxml-3.13 describes. A long "
           "chain is not a runaway, and a ceiling that could not tell them apart would report every document that "
           "computes before it settles";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedMacrostepState, "")
        << "and nothing names a state, because nothing was stopped";
    EXPECT_TRUE(sm_->isRunning())
        << "a document that settles on its own must not be reported dead by an engine that just finished running "
           "it correctly";
    EXPECT_EQ(sm_->getCurrentState(), "bounded_a") << "the chain rests where its guard closed";
}

/// A count, not a flag: a second unbounded macrostep is refused the same way
/// the first was.
TEST_F(EventlessMacrostepIsBoundedTest, ASecondTruncatedMacrostepCountsAgain) {
    sm_->processEvent("spin");
    ASSERT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u) << "precondition: this test is about the SECOND refusal";

    // `reset` is the fixture's way back out of the cycle, and it moves the
    // machine on purpose: this engine completes a macrostep only after a
    // transition that does.
    sm_->processEvent("reset");
    ASSERT_EQ(sm_->getCurrentState(), "idle") << "reset is the way back out of the chain";

    sm_->processEvent("spin");

    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 2u)
        << "the second macrostep hit the same ceiling and must be counted again — a count that saturated at one "
           "would read as a machine that recovered";
    EXPECT_EQ(counter("spins"), "1000")
        << "and it really bought the document a full budget again rather than refusing on sight — the ceiling "
           "bounds a macrostep, it does not condemn a machine";
}

/// The control: an ordinary document is untouched by any of this. Without it,
/// an engine that refused every macrostep would pass the assertions above and
/// fail nothing.
TEST_F(EventlessMacrostepIsBoundedTest, AnOrdinaryMacrostepIsNotCounted) {
    sm_->processEvent("poke");

    EXPECT_EQ(counter("pokes"), "1") << "the run fired";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "a macrostep of one microstep ends the way the clause says it does";
    EXPECT_EQ(sm_->getCurrentState(), "idle");
}

}  // namespace Tests
}  // namespace SCE
