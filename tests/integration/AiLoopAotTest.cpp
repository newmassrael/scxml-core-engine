// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The AI supervision loop, driven through the C++ AOT engine.
//
// `examples/ai_loop/ai_loop.scxml` is a worked example: a statechart that
// supervises a long-running session, with `<parallel>` splitting the turn cycle
// from the liveness watch and the turn budget. Its sibling
// `backends/rust/tests/tests/ai_loop.rs` asks the Rust AOT engine these same
// questions; this file asks the C++ one, so a topology change that only one
// engine honours fails on the other.
//
// Why this file exists next to `examples/ai_loop/ai_loop_example.cpp` rather
// than inside it: the example asserts the OUTCOME of a run — that four
// scripted runs each end somewhere the document enumerates. Measured
// 2026-08-13 with `sce-build/tests/mutations/ai_loop_history_cpp.cases`:
// deleting the shared history filter outright left all four green, because a
// run whose `<history>` records nothing still converges. An outcome check
// cannot see a configuration, and the clauses this document exists to
// demonstrate are all about configurations. So the example stays a readable
// example, and the per-clause assertions live here where the rest of the
// integration suite is.
//
// Fixture: examples/ai_loop/ai_loop.scxml (shared with the Rust channel and
// with the example's own driver).

#include "ai_loop_sm.h"

#include <algorithm>
#include <gtest/gtest.h>
#include <vector>

#include "scripting/JSEngine.h"
#include "scripting/ScriptEngineProvider.h"

namespace SCE::Tests {

namespace {

using Machine = SCE::Generated::ai_loop::ai_loop;

/// The document's prompts and standing rules are datamodel values, so the
/// machine needs a live script engine. One per test keeps the runs
/// independent.
class AiLoopAotTest : public ::testing::Test {
protected:
    void SetUp() override {
        SCE::JSEngine::instance().initialize();
    }

    void TearDown() override {
        SCE::JSEngine::instance().shutdown();
    }

    /// Every state currently active, across all three regions. A parallel
    /// machine's claims are about the active SET, not about one current state.
    static std::vector<Machine::State> active(Machine &sm) {
        return sm.getPolicy().getActiveStates();
    }

    static bool holds(Machine &sm, Machine::State state) {
        const auto states = active(sm);
        return std::find(states.begin(), states.end(), state) != states.end();
    }

    /// One completed turn: the session finished its work, and the loop decides
    /// what next.
    static void turn(Machine &sm) {
        sm.processEvent(Machine::Event::Turn_done);
        sm.processEvent(Machine::Event::Judge);
    }

    /// A machine whose datamodel can evaluate. Aliasing constructor with a
    /// no-op deleter — the provider owns the engine's lifetime and this
    /// `shared_ptr` is a non-owning view, the same idiom the example driver
    /// and the W3C AOT harness use.
    static void boot(Machine &sm) {
        sm.setScriptEngine(std::shared_ptr<SCE::IScriptEngine>(&SCE::ScriptEngineProvider::getScriptEngine(),
                                                               [](SCE::IScriptEngine *) {}));
        sm.initialize();
    }

    /// A run whose first prompt has been sent — where most scenarios start.
    static void start(Machine &sm) {
        boot(sm);
        sm.processEvent(Machine::Event::Prompt_sent);
    }

    /// Rendered into a failure message; a bare enum tells the reader nothing.
    static std::string describe(Machine &sm) {
        std::string out = "[";
        for (const auto state : active(sm)) {
            if (out.size() > 1) {
                out += " | ";
            }
            out += sm.getPolicy().getStateName(state);
        }
        return out + "]";
    }
};

}  // namespace

TEST_F(AiLoopAotTest, AllThreeRegionsAreLiveAtOnce) {
    Machine sm;
    start(sm);

    EXPECT_TRUE(holds(sm, Machine::State::Working) && holds(sm, Machine::State::Alive) &&
                holds(sm, Machine::State::Within))
        << "the cycle, the liveness watch and the budget are orthogonal regions and must all be "
           "active at once; got "
        << describe(sm);
}

TEST_F(AiLoopAotTest, ReflectionFiresOnSchedule) {
    Machine sm;
    start(sm);

    int at = 0;
    for (int n = 1; n <= 10; ++n) {
        turn(sm);
        if (holds(sm, Machine::State::Reflecting)) {
            at = n;
            break;
        }
    }

    EXPECT_EQ(at, 8) << "the document sets `reflect_every` to 8, so the eighth completed turn is "
                        "the one that reflects; reflection fired at turn "
                     << at;
}

TEST_F(AiLoopAotTest, ReflectionGoesThroughARestartAndTheLoopRePrimes) {
    Machine sm;
    start(sm);
    for (int n = 1; n <= 8; ++n) {
        turn(sm);
    }

    sm.processEvent(Machine::Event::Reflect_applied);
    ASSERT_TRUE(holds(sm, Machine::State::Restarting))
        << "a session reads its context, MCP config and memory once, at its start, so applying a "
           "reflection has to REPLACE the session rather than reconfigure it; active: "
        << describe(sm);

    sm.processEvent(Machine::Event::Session_ready);
    EXPECT_TRUE(holds(sm, Machine::State::Priming))
        << "a replaced session starts empty and must be primed with the current prompts before it "
           "can take a turn; active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, TheBudgetEndsTheRunFromWhereverTheCycleIs) {
    Machine sm;
    start(sm);

    for (int n = 1; n <= 60; ++n) {
        if (holds(sm, Machine::State::Reflecting)) {
            sm.processEvent(Machine::Event::Reflect_none);
        }
        if (holds(sm, Machine::State::Exhausted)) {
            break;
        }
        // W3C SCXML 3.4: a region of an active `<parallel>` always holds an
        // atomic state. A region that keeps its root and loses its leaf still
        // reads as "present" in the configuration while nothing in it can
        // fire again, so the run below would simply never reach its budget —
        // naming the turn it happened is the difference between a diagnosis
        // and a timeout.
        ASSERT_TRUE(holds(sm, Machine::State::Within) || holds(sm, Machine::State::Spent))
            << "the budget region holds neither of its states at turn " << n << "; active: " << describe(sm);
        turn(sm);
    }

    EXPECT_TRUE(holds(sm, Machine::State::Exhausted))
        << "the budget is its own region precisely so the turn count is not something `judging` "
           "has to remember to check; active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, AStandingInstructionAnswersWithoutWakingAnybody) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Turn_blocked);
    ASSERT_TRUE(holds(sm, Machine::State::Screening))
        << "a dialog is screened against the rules the person wrote in advance before anybody is "
           "woken; active: "
        << describe(sm);

    sm.processEvent(Machine::Event::Screen_matched);
    EXPECT_TRUE(holds(sm, Machine::State::Working) && !holds(sm, Machine::State::Paused))
        << "a matched rule is a decision the person already made, so the run carries on and nobody "
           "is woken; active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, AnUnmatchedDialogWakesThePersonWhoAnswers) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Turn_blocked);
    sm.processEvent(Machine::Event::Screen_none);
    ASSERT_TRUE(holds(sm, Machine::State::Paused))
        << "the loop answers only what the person decided in advance; anything else stops it and "
           "waits; active: "
        << describe(sm);

    sm.processEvent(Machine::Event::Turn_done);
    EXPECT_TRUE(holds(sm, Machine::State::Judging))
        << "once the person has answered, the turn completes where it left off; active: " << describe(sm);
}

TEST_F(AiLoopAotTest, HoldAndResumeReturnToExactlyWhereTheCycleWas) {
    Machine sm;
    start(sm);
    turn(sm);

    sm.processEvent(Machine::Event::Hold);
    ASSERT_TRUE(holds(sm, Machine::State::Paused))
        << "a person looking at the work holds the cycle; active: " << describe(sm);

    sm.processEvent(Machine::Event::Resume);
    EXPECT_TRUE(holds(sm, Machine::State::Working))
        << "resuming puts the cycle back to work rather than ending the run; active: " << describe(sm);
}

TEST_F(AiLoopAotTest, ResumeReturnsSomewhereTheHistoryDefaultDoesNot) {
    // `<history id="where">` declares `<transition target="working"/>` as its
    // default, so a hold taken while the cycle is in `working` resumes there
    // whether history recorded anything or not — the test above cannot tell a
    // working history from one that records nothing.
    //
    // `priming` is the one place the two answers differ. The machine comes up
    // there, `hold` is declared above the cycle so it reaches, and the history
    // default names `working` — so resuming into `priming` is only possible if
    // the configuration was really recorded. This is the assertion that turns
    // the shared `HistoryHelper` filter red when it is broken.
    Machine sm;
    boot(sm);

    ASSERT_TRUE(holds(sm, Machine::State::Priming))
        << "the run starts with a session that exists and has not been prompted; active: " << describe(sm);

    sm.processEvent(Machine::Event::Hold);
    ASSERT_TRUE(holds(sm, Machine::State::Paused))
        << "a person can take over before the first prompt as readily as after one; active: " << describe(sm);

    sm.processEvent(Machine::Event::Resume);
    EXPECT_TRUE(holds(sm, Machine::State::Priming) && !holds(sm, Machine::State::Working))
        << "`<history>` must restore the state the cycle was actually in; landing in `working` "
           "here is the history default answering instead, which is what a history that records "
           "nothing looks like; active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, ThePersonInterruptsTheInnerSessionByHand) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Turn_interrupted);
    ASSERT_TRUE(holds(sm, Machine::State::Paused) && !holds(sm, Machine::State::Screening))
        << "a person typing into the session directly is not a dialog to screen — the loop stops "
           "driving and stays out of the way; active: "
        << describe(sm);

    sm.processEvent(Machine::Event::Turn_interrupted);
    EXPECT_TRUE(holds(sm, Machine::State::Paused))
        << "further interruptions keep it paused rather than fighting the person for the session; "
           "active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, NobodyComes) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Turn_blocked);
    sm.processEvent(Machine::Event::Screen_none);
    sm.processEvent(Machine::Event::Unattended);

    EXPECT_TRUE(holds(sm, Machine::State::Blocked))
        << "a question nobody answers ends the run in an outcome the document names, rather than "
           "leaving it prompting into the dark; active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, APaneThatDiesMidTurnIsNoticedAndRebuilt) {
    Machine sm;
    start(sm);

    // The cycle is sitting in `working`, waiting for a turn that will never
    // come because the process is gone. `watch` is the region that sees it.
    sm.processEvent(Machine::Event::Session_lost);
    ASSERT_TRUE(holds(sm, Machine::State::Restarting) && holds(sm, Machine::State::Rebuilding))
        << "a dead session has to be noticed independently of where the turn cycle happens to be, "
           "which is why the watch is its own region; active: "
        << describe(sm);

    sm.processEvent(Machine::Event::Session_ready);
    EXPECT_TRUE(holds(sm, Machine::State::Priming) && holds(sm, Machine::State::Alive))
        << "both regions recover together: the run re-primes and the watch goes back to alive; "
           "active: "
        << describe(sm);
}

TEST_F(AiLoopAotTest, OneCancelReachesEveryRegion) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Cancel);
    EXPECT_TRUE(holds(sm, Machine::State::Cancelled))
        << "cancel is one transition on the `<parallel>` itself rather than one per region, so a "
           "single event ends all three; active: "
        << describe(sm);
}

}  // namespace SCE::Tests
