// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 3.12.2: the processor MUST signal its own failures by raising
// `error.*` events into the internal queue, and the same paragraph says they
// "are ignored if no transition is found that matches them". Being ignored is
// the clause. Being unable to say it happened is not. Interpreter path.
//
// This channel is where the parity claim is checked. The Interpreter raises its
// errors WITH a message — `DataModelInitializer` and `ActionExecutorImpl` call
// `eventRaiser_->raiseEvent("error.execution", <what failed>)` unconditionally,
// whether or not the document declares a transition for it — and its statistics
// have always counted a transition selection that matched nothing. The six
// generated engines raised the same event and dropped the outcome, so a
// document that grew up here and shipped as AOT lost a signal its host was
// reading. `UnhandledErrorIsObservableAotTest.cpp` runs the same script against
// `unhandledErrorEvents()` and `lastUnhandledError()`.
//
//   poke               handled, no error            control: proves a run fired
//   whisper            author's <raise>, unmatched  not an error
//   boom in `idle`     error, unmatched             the silent failure
//   boom in `guarded`  error, HANDLED               `caught` goes up
//
// Fixture: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml

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

class UnhandledErrorIsObservableTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        const std::string fixture =
            std::string(SCE_PROJECT_ROOT) +
            "/integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml";
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

    /// The fixture's `<assign>`s are the only witness that a transition ran —
    /// every outcome this document separates leaves the same configuration.
    std::string counter(const std::string &name) {
        auto result = engine_->evaluateExpression(sm_->getSessionId(), name).get();
        EXPECT_TRUE(result.isSuccess()) << "the fixture declares `" << name << "` in its datamodel";
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

/// The Interpreter raises the error whether or not the document answers it —
/// which is what makes the count on the generated side meaningful rather than
/// an artefact of when the engine bothers to raise.
TEST_F(UnhandledErrorIsObservableTest, TheErrorIsRaisedWhetherOrNotTheDocumentAnswersIt) {
    const auto boomed = sm_->processEvent("boom");
    EXPECT_TRUE(boomed.success) << "`boom` matches a self transition in `idle`; the failure is inside its body, "
                                   "not in selecting it";
    EXPECT_EQ(counter("booms"), "1") << "`boom`'s transition did not run, so nothing here is measuring an error";
    EXPECT_EQ(counter("caught"), "0")
        << "`idle` declares no transition for error.execution, so nothing in the document answered it. "
           "The error was still raised — the Interpreter's raise sites are unconditional — and it went nowhere";
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "the error must not move the machine on its own";
}

/// The other half, on this engine: in `guarded` the same failure is answered,
/// and the document's own counter is what shows it.
TEST_F(UnhandledErrorIsObservableTest, AnErrorTheDocumentAnswersRunsItsHandler) {
    ASSERT_TRUE(sm_->processEvent("go").success);
    ASSERT_EQ(sm_->getCurrentState(), "guarded");

    sm_->processEvent("boom");

    EXPECT_EQ(counter("caught"), "1")
        << "`guarded` declares a transition for error.execution, so the error the engine raised "
           "was answered by the document. This is the outcome the AOT sibling must NOT count";
    EXPECT_EQ(counter("booms"), "1") << "the transition that failed still ran its first <assign>";
}

/// The configuration cannot answer the question — which is why the engines have
/// to. Same assertion as the AOT sibling's, on the other engine.
TEST_F(UnhandledErrorIsObservableTest, TheSilentFailureIsNotDerivableFromTheConfiguration) {
    sm_->processEvent("poke");
    const std::string clean = sm_->getCurrentState();

    sm_->processEvent("boom");
    const std::string failed = sm_->getCurrentState();

    EXPECT_EQ(clean, failed) << "this fixture exists because a clean run and one whose <assign> failed leave the "
                                "same configuration; if they ever differ, the fixture stopped measuring what it "
                                "claims";
    EXPECT_EQ(counter("pokes"), "1") << "`poke` did not run, so the comparison above compared nothing";
}

/// The boundary, on this engine: an author's unmatched `<raise>` is a discarded
/// internal event and not an error. The generated engines draw the same line.
TEST_F(UnhandledErrorIsObservableTest, AnAuthorsUnmatchedRaiseIsNotAnError) {
    const auto whispered = sm_->processEvent("whisper");
    EXPECT_TRUE(whispered.success) << "`whisper` itself matches a transition in `idle`";
    EXPECT_EQ(counter("caught"), "0")
        << "`unheard` and `retry.error.execution` match nothing and are discarded, exactly as an "
           "unmatched error would be — and neither is an error: the author wrote the raises and "
           "the absent handlers. `retry.error.execution` CONTAINS `error.` without starting with "
           "it, and W3C 3.12.2 reserves the prefix, not the substring";
    EXPECT_EQ(counter("heards"), "1")
        << "`whisper`'s third raise, `heard`, does match — this Interpreter runs the same three "
           "internal events the generated engines do, so the AOT sibling's assertion about the "
           "drain still selecting transitions is measuring the same document";
}

}  // namespace Tests
}  // namespace SCE
