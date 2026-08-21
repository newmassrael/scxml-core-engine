// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.7.1 under §scxml-6.4: a `<param>` of an `<invoke>` whose expression
// will not evaluate — Interpreter path.
//
// Two clauses meet here and only one governs. §scxml-6.4.2 terminates the
// element when "the evaluation of its arguments produces an error", and the
// sentence after it — "Otherwise the Processor MUST start a new logical
// instance" — makes the alternative explicit. §scxml-5.7.1 says a failing
// `<param>` costs `error.execution` on the internal queue and "MUST ignore the
// name and value", then delegates only the SUCCESSFUL name and value to the
// context: "See 5.5 <donedata>, 6.2 <send> and 6.4 <invoke> for details."
//
// 5.7.1 governs, because it has already said what the failure costs in this
// context by name, and reading 6.4.2 over it would leave "ignore the name and
// value" with no invoked session for the name to be absent from. W3C test343
// settles the same clause from the `<donedata>` side; no IRP document asks it
// of `<invoke>`, which is why this fixture exists.
//
// Sibling of `InvokeParamErrorStartsTheChildAotTest.cpp`. Both engines ship,
// so each is held to the clause independently against one canonical fixture.
//
// Fixture: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <sstream>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class InvokeParamErrorStartsTheChildTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_;
};

TEST_F(InvokeParamErrorStartsTheChildTest, AnInvokeParamThatWillNotEvaluateCostsItsPairAndNothingElse) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/invoke_param_error_starts_the_child/"
                                "invoke_param_error_starts_the_child.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child answers with `<send target="#_parent">` and the
    // parent's own verdict rides a delayed `<send>`, so the Interpreter needs
    // the dispatcher chain (scheduler -> target factory -> dispatcher) wired.
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            try {
                return target->send(event).get().isSuccess;
            } catch (...) {
                return false;
            }
        });
    auto eventRaiser = std::make_shared<EventRaiserImpl>();
    eventRaiser->setScheduler(scheduler);
    eventRaiser->setImmediateMode(false);
    sm->setEventRaiser(eventRaiser);
    sm->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm->loadSCXMLFromString(buffer.str()));
    ASSERT_TRUE(sm->start());

    // §scxml-3.13: reaching a top-level `<final>` halts processing, so the
    // state name freezes on the verdict. The fixture's own `timeout` is 3s, so
    // the deadline has to outlast it or a never-started child reads as a hang
    // rather than as `failInvokeNotStarted`.
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(10);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 10s — neither the child's `childUp` "
                                  << "nor the delayed `timeout` that judges a never-started child arrived";

    const std::string reached = sm->getCurrentState();
    EXPECT_NE(reached, "failNoParamError")
        << "`childUp` arrived with no `error.execution` before it: §scxml-5.7.1 puts that error on "
        << "the internal queue while the `<invoke>` is being evaluated, so it is dequeued before "
        << "the child's first word.";
    EXPECT_NE(reached, "failInvokeNotStarted")
        << "the child never started: this engine read §scxml-6.4.2's \"terminate the processing of "
        << "the element\" over 5.7.1's per-item rule. One `<param>` that will not evaluate costs "
        << "its own pair, not the session.";
    EXPECT_NE(reached, "failGoodParamLost")
        << "the child's `kept` did not arrive as 'here': §scxml-6.4.3 seeds the child's matching "
        << "`<data>` from the param's value, and one sibling that failed does not cost the others.";
    EXPECT_NE(reached, "failBrokenParamSeeded")
        << "the child found the empty string under `broken`: 5.7.1 says ignore the name AND the "
        << "value, so the child must find its own declaration untouched rather than a placeholder "
        << "the author never wrote.";
    EXPECT_EQ(reached, "pass");
}

}  // namespace Tests
}  // namespace SCE
