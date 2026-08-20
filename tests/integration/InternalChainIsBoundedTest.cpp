// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.13 ends a macrostep at a configuration where nothing is enabled
// by NULL AND the internal queue is empty. Appendix D's Principles and
// Constraints then say that end need not exist: "A microstep always
// terminates. A macrostep may not. ... This is currently allowed." C++
// Interpreter path.
//
// `EventlessMacrostepIsBoundedTest.cpp` owns the half of that clause built from
// transitions that need no event. This one owns the other half: a `<raise>`
// answered by a transition that raises again. On this engine that chain is not
// even a loop — executable content dispatches back into the raiser, so each
// link is a stack frame — and before the ceiling reached this branch,
// `processEvent` did not return.
//
// The budget is the machine's and the queue is the raiser's, so this engine
// lends one to the other (`MicrostepBudget`). The refusal has to happen where
// the queue is, or it would consume the event it declined to run — and the
// second-refusal test below is what proves it did not.
//
// Fixture: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
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

class InternalChainIsBoundedTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml";
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
    /// every outcome leaves the machine in a state the configuration alone
    /// cannot tell apart from the others.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: `processEvent` comes back, and the statistics say why.
TEST_F(InternalChainIsBoundedTest, ARaiseChainThatCannotEndIsStopped) {
    ASSERT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "nothing has been refused before the machine has done anything";

    sm_->processEvent("spin");

    EXPECT_EQ(counter("links"), "1000")
        << "the chain must run exactly as far as the engine allows — fewer means the document was cut off early, "
           "more means the ceiling moved";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u)
        << "the microstep past the budget was queued and was not taken. Without this count the host sees a machine "
           "that is running, in a state the document names, having returned at once — and no way to learn that the "
           "configuration it is reading is not a stable one";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedMacrostepState, "spin")
        << "the count alone says a document somewhere cannot settle; this says where to look";
    EXPECT_TRUE(sm_->isRunning())
        << "the chain was cut, not the machine. §scxml-D allows the document; refusing to run it forever is the "
           "engine's decision to report, not a reason to stop a machine whose other states still work";
}

/// The other half, and the one that makes the count mean something: a chain
/// that ends on its own is not refused, however long it is.
TEST_F(InternalChainIsBoundedTest, ARaiseChainThatEndsAtTheCeilingIsNotRefused) {
    sm_->processEvent("bounded");

    EXPECT_EQ(counter("laps"), "1000")
        << "the guard `laps < 999` stops matching at the thousandth link, which raises nothing — so the queue "
           "empties and the chain stops by itself";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "nothing was refused: the macrostep reached the stable configuration §scxml-3.13 describes. A long chain "
           "is not a runaway, and a ceiling that could not tell them apart would report every document that "
           "computes before it settles";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedMacrostepState, "")
        << "and nothing names a state, because nothing was stopped";
    EXPECT_TRUE(sm_->isRunning())
        << "a document that settles on its own must not be reported dead by an engine that just finished running "
           "it correctly";
    EXPECT_EQ(sm_->getCurrentState(), "bounded") << "the chain rests where it ended";
}

/// The case a per-branch budget lets through: a chain that alternates one
/// `<raise>` with one eventless transition.
///
/// Neither branch of §scxml-D's inner loop reaches the ceiling on its own here,
/// so an engine that gives each branch a counter of its own runs this document
/// forever with both ceilings half spent.
TEST_F(InternalChainIsBoundedTest, AnAlternatingChainSpendsOneSharedBudget) {
    sm_->processEvent("alternate");

    EXPECT_EQ(counter("alts"), "500")
        << "the two branches share one budget, so a chain that alternates them gets five hundred laps out of a "
           "thousand microsteps. A thousand here would mean the internal branch had a ceiling of its own";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u)
        << "and the refusal is reported once, whichever branch was holding the budget when it ran out";
    EXPECT_EQ(sm_->getStatistics().lastTruncatedMacrostepState, "alt")
        << "named the same way as any other chain that could not settle";
}

/// What the refusal did with the links it would not run: it left them queued.
///
/// The fixture's `resume` chain is half again as long as the ceiling, so the
/// first macrostep is refused with five hundred links still to go and the
/// second one finishes them. An engine that dropped the queue stops at a
/// thousand and never finishes; one that ran the chain anyway finishes it in
/// the first macrostep. On this engine the refusal happens inside the raiser,
/// which is the only party that can decline a dispatch without consuming the
/// event — this is the assertion that says it did.
TEST_F(InternalChainIsBoundedTest, ARefusedChainIsLeftQueuedForTheNextMacrostep) {
    sm_->processEvent("resume");
    ASSERT_EQ(counter("beats"), "1000") << "the first macrostep spends the whole budget on the chain";
    ASSERT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u) << "precondition: the first macrostep was refused";

    sm_->processEvent("poke");

    EXPECT_EQ(counter("beats"), "1500")
        << "the second macrostep picked the chain up where the first was cut and ran it to its end — the refused "
           "links were left on the queue, not dropped";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 1u)
        << "and nothing was refused this time: the chain ended on its own inside the budget, which is an ordinary "
           "macrostep however long the document took to get there";
    EXPECT_TRUE(sm_->isRunning());
}

/// The control: an ordinary document is untouched by any of this. Without it,
/// an engine that refused every macrostep would pass the assertions above and
/// fail nothing.
TEST_F(InternalChainIsBoundedTest, AnOrdinaryMacrostepIsNotCounted) {
    sm_->processEvent("poke");

    EXPECT_EQ(counter("pokes"), "1") << "the run fired";
    EXPECT_EQ(sm_->getStatistics().truncatedMacrosteps, 0u)
        << "a macrostep of one microstep ends the way the clause says it does";
    EXPECT_EQ(sm_->getCurrentState(), "idle");
}

}  // namespace Tests
}  // namespace SCE
