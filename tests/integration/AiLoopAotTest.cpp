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
#include <cstdint>
#include <gtest/gtest.h>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "core/EventMetadata.h"
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

// A person answering does not re-introduce the session to itself.
//
// `paused` is a sibling of `running`, so answering targets `judging` and enters
// `running` on the way - as an ANCESTOR. §scxml-D-addAncestorStatesToEnter adds
// such a state without its default initial child, and here the default is
// `priming`, whose `<onentry>` sends the opening prompt. An engine that gives
// every entered compound state its default leaves the cycle in two states at
// once and the host, reading the configuration, sends the start prompt again -
// measured 2026-08-15 on both AOT engines, with every W3C fixture green and the
// rest of this file green with them.
//
// The clause itself is pinned across all seven channels by
// `integration_resources/ancestor_entry_is_not_default_entry/`. This test is the
// worked example's own stake in it, so a regression here fails as a supervision
// bug rather than as an abstract entry-set one. The Rust channel asserts the
// same clause on the same document
// (`answering_a_question_does_not_re_prime_the_session`).
TEST_F(AiLoopAotTest, AnsweringAQuestionDoesNotRePrimeTheSession) {
    Machine sm;
    start(sm);

    sm.processEvent(Machine::Event::Turn_blocked);
    sm.processEvent(Machine::Event::Screen_none);
    sm.processEvent(Machine::Event::Turn_done);

    ASSERT_TRUE(holds(sm, Machine::State::Judging))
        << "the answered turn has to land in `judging`; active: " << describe(sm);
    EXPECT_FALSE(holds(sm, Machine::State::Priming))
        << "`running` has two children active at once. `priming` sends `prompt.start`, so a host "
           "driving this configuration re-sends the opening prompt every time a person answers a "
           "dialog; active: "
        << describe(sm);
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

// §scxml-5.3: the machine answers what its own datamodel holds.
//
// A host supervising this loop has to size its own work against the budget the
// document declares. Without an accessor the only readable copy is the script
// engine's, reached with an engine handle, a session id and the variable's
// name spelled as a string — three things a consumer should not need, none of
// them checked by a compiler.
//
// The half that decides the shape is `turns`: it is authored 0 and assigned on
// every completed turn, so an accessor answering the AUTHORED literal would
// keep saying 0 for the whole run. What a consumer asks for is the value the
// machine is holding now, which is why the read goes to whoever owns the
// datamodel rather than to a copy taken at generation time.
//
// The Rust channel asserts the same clause on the same document
// (`the_machine_answers_what_its_own_datamodel_holds`), so an accessor that
// only one backend emits fails on the other.
TEST_F(AiLoopAotTest, TheMachineAnswersWhatItsOwnDatamodelHolds) {
    Machine sm;
    start(sm);

    EXPECT_EQ(sm.getPolicy().max_turns(), std::optional<int64_t>(40))
        << "the authored budget must be readable off the machine itself, in the host's own type";
    EXPECT_EQ(sm.getPolicy().reflect_every(), std::optional<int64_t>(8)) << "so must the reflection cadence";
    EXPECT_EQ(sm.getPolicy().screen_permissions(), std::optional<bool>(false))
        << "a standing answer to permission dialogs is a promise about what the loop may do "
           "unattended, and a host must be able to inspect it";

    ASSERT_EQ(sm.getPolicy().turns(), std::optional<int64_t>(0))
        << "no turn has completed yet, so the bookkeeping still reads its authored value";
    turn(sm);
    EXPECT_EQ(sm.getPolicy().turns(), std::optional<int64_t>(1))
        << "the accessor must report what the datamodel HOLDS, not what the document authored - a "
           "value frozen at generation time would still say 0 here, and `max_turns` itself is "
           "assigned in the consumer's own copy of this loop";
}

// The strategy a host edits is the strategy it can read back.
//
// The budget above is the numeric half of the datamodel. This is the other
// half, and it is the half the example's own comment calls editable: the north
// star, the milestone, the prompts built from them, the marker that ends the
// run. A supervisor that is going to send `start_prompt` has to be able to see
// what it is about to send, and a UI over this loop has nothing to display
// without these.
//
// They were unreadable for the same reason none of them looked unusual: the
// document spells its strings with `'...'`, and the classifier deciding which
// variables get an accessor tested for `"`. Eight of the sixteen declarations
// were silently untyped, so this file could assert the budget and pass while
// the strategy was not reachable at all.
//
// `start_prompt` is asserted through its parts rather than as one literal,
// because it is a concatenation: it exists to prove that a value the document
// COMPUTES from its strings is readable too, not only the ones it spells out.
//
// The Rust channel asserts the same clause on the same document
// (`the_strategy_a_host_edits_is_the_strategy_it_can_read_back`).
TEST_F(AiLoopAotTest, TheStrategyAHostEditsIsTheStrategyItCanReadBack) {
    Machine sm;
    start(sm);

    EXPECT_EQ(sm.getPolicy().done_marker(), std::optional<std::string>("MILESTONE REACHED"))
        << "the marker that decides when the run has converged must be readable off the machine - a "
           "host matching the session's report against it cannot ask the document";
    EXPECT_EQ(sm.getPolicy().north_star(),
              std::optional<std::string>("(edit me) the outcome this loop exists to reach"))
        << "the goal the author edits is the first thing a supervisor displays";
    EXPECT_EQ(sm.getPolicy().milestone(), std::optional<std::string>("(edit me) the next checkpoint on the way there"))
        << "so is the checkpoint it is working toward";

    const auto start_prompt = sm.getPolicy().start_prompt();
    ASSERT_TRUE(start_prompt.has_value())
        << "the prompt the loop sends into a fresh session must be readable before it is sent";
    EXPECT_NE(start_prompt->find("(edit me) the outcome this loop exists to reach"), std::string::npos)
        << "the composed prompt must carry the authored strings it was built from, so a host "
           "reading it sees what the session will receive: "
        << *start_prompt;
    EXPECT_NE(start_prompt->find("Report what you did"), std::string::npos)
        << "including the instruction half: " << *start_prompt;
}

// The standing instructions are readable, which is what makes them standing.
//
// `screen_rules` is the block that decides when a person is NOT woken. The
// document keeps it in the authored half deliberately - its own comment says the
// loop is carrying out a decision made in advance and written down - and a
// decision written down where nobody can read it back is indistinguishable from
// the loop deciding on its own authority. A supervisor showing a human "these
// three questions are being answered for you" has to get the list from the
// machine.
//
// The parts asserted here are the ones a reader acts on - which question is
// matched and what answer it gets - rather than the whole text, so that
// reformatting the block inside the document does not fail this.
//
// The Rust channel asserts the same clause on the same document
// (`the_standing_instructions_can_be_read_back_off_the_machine`), so a
// structured accessor that only one backend emits fails on the other.
TEST_F(AiLoopAotTest, TheStandingInstructionsCanBeReadBackOffTheMachine) {
    Machine sm;
    start(sm);

    const auto rules = sm.getPolicy().screen_rules();
    ASSERT_TRUE(rules.has_value())
        << "the standing-instruction table must be readable off the machine - a host that cannot "
           "list it cannot show anyone which questions are being answered without them";

    EXPECT_EQ(rules->rfind('[', 0), 0U) << "the block is authored as an array and must come back as one: " << *rules;
    for (const auto *question : {"design-decision", "design-proposal", "multiple-choice"}) {
        EXPECT_NE(rules->find(question), std::string::npos)
            << "`" << question << "` is screened by the document but absent from what the machine reports: " << *rules;
    }
    EXPECT_NE(rules->find("Rethink for the most durable answer"), std::string::npos)
        << "the reply a screened question receives is the half a person most needs to see, and it "
           "is what distinguishes carrying out a decision from making one: "
        << *rules;
}

// A structured variable answers with what it is holding, not with what it was
// declared as.
//
// The scalar readers refuse a value of another type, and this asserts the json
// one does too - from both directions. A write into the session must be visible,
// because a reader frozen at generation time would answer the document's literal
// for the whole run; and a scalar written into a variable declared structured
// must read as "cannot answer" rather than as the scalar's own JSON.
//
// The writes go through `setVariable`, which takes a value rather than source
// text. That is the half of the engine interface that is the same whichever
// engine a deployment injected - `evaluateExpression` takes the ENGINE's
// language - so a test written in either language would be asserting about the
// injection rather than about the reader.
//
// The Rust channel asserts the same clause on the same document
// (`a_structured_read_follows_the_assignment_and_refuses_another_type`).
TEST_F(AiLoopAotTest, AStructuredReadFollowsTheAssignmentAndRefusesAnotherType) {
    Machine sm;
    start(sm);

    ASSERT_TRUE(sm.getPolicy().sessionId_.has_value()) << "a started machine holds a session";
    const auto sessionId = sm.getPolicy().sessionId_.value();
    auto &scriptEngine = SCE::ScriptEngineProvider::getScriptEngine();

    auto later = std::make_shared<::ScriptObject>();
    later->properties["when"] = ::ScriptValue{std::string("later")};
    auto table = std::make_shared<::ScriptArray>();
    table->elements.push_back(::ScriptValue{later});
    ASSERT_TRUE(scriptEngine.setVariable(sessionId, "screen_rules", ::ScriptValue{table}).get().isSuccess())
        << "the session takes a structured value";

    const auto after = sm.getPolicy().screen_rules();
    ASSERT_TRUE(after.has_value()) << "a reassigned structured variable is still readable";
    EXPECT_NE(after->find("later"), std::string::npos)
        << "the reader answered with something other than what the session now holds: " << *after;
    EXPECT_EQ(after->find("design-decision"), std::string::npos)
        << "the reader answered with the authored table after the session was assigned another one: " << *after;

    ASSERT_TRUE(
        scriptEngine.setVariable(sessionId, "screen_rules", ::ScriptValue{static_cast<int64_t>(5)}).get().isSuccess())
        << "the session takes a scalar too";
    EXPECT_EQ(sm.getPolicy().screen_rules(), std::nullopt)
        << "a variable declared structured and now holding a number must report that the machine "
           "cannot answer. `5` is valid JSON, so a reader that forwarded whatever the serializer "
           "produced would hand a consumer a document shape that no longer exists.";
}

// What a reflection writes is what the restarted session is primed with.
//
// This is the loop's whole reason for having a restart state: `reflecting`
// rewrites the prompts and `restarting` replaces the session so a fresh one
// reads them. Both halves are invisible to an outcome - a run converges just the
// same whether the text it sent afterwards was the reflection's, the author's,
// or empty - so the example's own driver reads the prompts it actually sent and
// this reads what the machine holds.
//
// It is asserted because the example was wrong here: its host wrote
// `{"start_prompt":"","turn_prompt":"","milestone":"refined"}`, so the document
// came back holding two empty strings and the fresh session was primed with
// nothing at all, under a scenario titled "restarts into the improved prompts".
// Measured 2026-08-15 in the example's own output.
//
// The payload arrives through `processEvent(Event, const EventMetadata &)` -
// the public door a host with a payload to deliver uses - so what is under test
// includes the seam that carries `_event.data` into the three `<assign>`s.
// The Rust channel asserts the same clause on the same document
// (`what_a_reflection_writes_is_what_the_machine_then_holds`).
TEST_F(AiLoopAotTest, WhatAReflectionWritesIsWhatTheMachineThenHolds) {
    Machine sm;
    start(sm);

    const auto authored = sm.getPolicy().start_prompt();
    ASSERT_TRUE(authored.has_value()) << "a started loop can read its opening prompt";

    for (int n = 1; n <= 8; ++n) {
        turn(sm);
    }
    ASSERT_TRUE(holds(sm, Machine::State::Reflecting))
        << "the document sets `reflect_every` to 8, so the eighth completed turn reflects; active: " << describe(sm);

    sm.processEvent(Machine::Event::Reflect_applied,
                    SCE::Core::EventMetadata("reflect.applied", R"({"start_prompt":"Resuming. Milestone: refined",)"
                                                                R"("turn_prompt":"Continue toward: refined",)"
                                                                R"("milestone":"refined"})"));

    EXPECT_EQ(sm.getPolicy().milestone(), std::optional<std::string>("refined"))
        << "the reflection's milestone did not reach the datamodel, so the restart it is about to "
           "pay for improves nothing";

    const auto after = sm.getPolicy().start_prompt();
    ASSERT_TRUE(after.has_value()) << "the prompt a restarted session is primed with must still be readable";
    EXPECT_EQ(*after, "Resuming. Milestone: refined") << "the machine is not holding what the reflection wrote";
    EXPECT_NE(*after, *authored) << "the reflection has to have changed something, or this test would pass against a "
                                    "machine that ignored it";
    EXPECT_FALSE(after->empty()) << "an empty prompt is what a host sends when reflection erased it, and the run still "
                                    "converges - which is why this is asserted rather than watched";
}

// A machine that has not been booted cannot answer, and says so.
//
// The failure this refuses is the one a default-valued member would produce:
// a freshly constructed machine reporting the document's literal as though a
// session had been created and initialised it. Nothing has read the document
// at this point, so `nullopt` is the only honest answer.
TEST_F(AiLoopAotTest, AnUninitialisedMachineSaysItCannotAnswer) {
    Machine sm;

    EXPECT_EQ(sm.getPolicy().max_turns(), std::nullopt)
        << "before initialize() there is no session holding a datamodel, and answering 40 would be "
           "a claim about a run that has not started";
}

}  // namespace SCE::Tests
