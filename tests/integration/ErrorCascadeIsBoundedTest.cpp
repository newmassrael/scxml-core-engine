// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2 says an error event nothing matches is ignored. It says
// nothing about an error event something DOES match, answered by a handler
// that fails the same way every time: the failure raises `error.execution`,
// the same transition answers it, and the drain never empties. Interpreter
// path.
//
// This is the engine a document is written on, so it is where a broken error
// handler is first met — and it met it the worst way: measured 2026-08-19,
// `processEvent("spin")` never came back at all, because executable content
// dispatches into the raiser again and each link was a new stack frame. The
// six generated engines spun in a loop; this one recursed.
//
// What it owes the host is therefore two things — the call comes back, and
// `getStatistics().errorCascadeEvents` says why it did.
// `ErrorCascadeIsBoundedAotTest.cpp` asks the same of the AOT engine.
//
//   poke      handled, no error             control: proves a run fired
//   boom      one error, unmatched          the clause's own case
//   settle    a chain that STOPS by itself  three links, then its guard closes
//   spin      a chain that cannot stop      the engine has to end it
//
// Fixture: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml

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

class ErrorCascadeIsBoundedTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                    "/integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml";
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

    /// The fixture's `<assign>`s are the only witness that a handler ran —
    /// every outcome this document separates leaves the same configuration.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: `processEvent` comes back. A handler whose failure re-raises the
/// event it is handling has no way out that the document can spell, so an
/// engine that keeps feeding it never returns to its caller — and the caller
/// is the only party that could stop it.
TEST_F(ErrorCascadeIsBoundedTest, AHandlerThatCannotHandleItsErrorGivesTheCallerBack) {
    sm_->processEvent("spin");

    EXPECT_EQ(sm_->getCurrentState(), "runaway")
        << "`spin` moves the machine, and `runaway`'s error handler is targetless, so nothing after it moves again";
    EXPECT_EQ(counter("runs"), "100")
        << "`runaway`'s handler must run exactly as many times as the raiser allows links in a chain — fewer means "
           "the document was cut off early, more means the ceiling moved";
    const auto stats = sm_->getStatistics();
    EXPECT_EQ(stats.errorCascadeEvents, 1u)
        << "the handler's <assign> failed again on the last allowed link, and the error it raised is the one the "
           "raiser refused to queue. Coming back without saying why leaves the host with a machine that looks "
           "idle and a document whose error handling cannot work";
    EXPECT_EQ(stats.lastErrorCascadeEvent, "error.execution")
        << "a count alone does not name the repair: error.execution is a handler whose own content fails, "
           "error.communication one that answers an unreachable target by talking to it again";
    EXPECT_TRUE(sm_->isRunning()) << "ending the chain must not end the machine";
}

/// The other half, and the one that makes any ceiling meaningful: a chain that
/// ends on its own must run to its own end, not to the engine's.
TEST_F(ErrorCascadeIsBoundedTest, AChainThatEndsOnItsOwnRunsToItsOwnEnd) {
    sm_->processEvent("settle");

    EXPECT_EQ(counter("repairs"), "3")
        << "`settling`'s handler repairs three times and then its `repairs < 3` guard stops matching. Three links "
           "is what a real repair strategy looks like, and no engine may interrupt it";
    EXPECT_EQ(sm_->getStatistics().errorCascadeEvents, 0u)
        << "nothing was refused: the chain ended on the document's own terms. A ceiling that fired here would "
           "report every document that fails often as one that cannot stop failing";
    EXPECT_EQ(sm_->getCurrentState(), "settling") << "the handler is targetless; the chain does not move the machine";
}

/// The machine is still a machine afterwards: cutting the chain must not cost
/// the document the states that work.
TEST_F(ErrorCascadeIsBoundedTest, TheMachineStillAnswersAfterItsChainIsCut) {
    sm_->processEvent("spin");
    ASSERT_EQ(sm_->getCurrentState(), "runaway");

    sm_->processEvent("poke");

    EXPECT_EQ(counter("pokes"), "1")
        << "`runaway` answers `poke` with a targetless transition. An engine that ended the chain by ending the "
           "machine would leave the host with a dead document instead of a bounded one";
}

/// The control, on this engine: one failure with nobody to answer it is the
/// clause's own case and has nothing to do with a chain.
TEST_F(ErrorCascadeIsBoundedTest, OneErrorNobodyAnsweredIsNotAChain) {
    const auto boomed = sm_->processEvent("boom");

    EXPECT_TRUE(boomed.success) << "`boom` matches a self transition in `idle`; the failure is inside its body";
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "the error must not move the machine on its own";
    EXPECT_EQ(counter("runs"), "0") << "no handler answered it, so no chain began";
    EXPECT_EQ(sm_->getStatistics().errorCascadeEvents, 0u)
        << "no handler ran, so no handler raised anything: a count keyed off how OFTEN a document fails would "
           "already have moved here";
}

}  // namespace Tests
}  // namespace SCE
