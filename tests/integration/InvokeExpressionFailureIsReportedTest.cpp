// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4.3: an `<invoke>` naming its target through an expression
// evaluates that expression at invoke-fire time, and a failure to evaluate
// places `error.execution` on the internal event queue.
//
// The clause puts two obligations on the Processor, and the channels do not
// agree about the first one: the Interpreter resolves the evaluated string
// into a document and runs it, while five of the six AOT backends spawn a
// stub fixed at build time (docs/SCE_ACCEPTED_SUBSET.md §2.13). This fixture
// is deliberately blind to that difference. It measures the obligation every
// channel shares, which is also the one a fixture CAN measure across all of
// them: a value that cannot be computed is reported rather than swallowed.
//
// Building it on the failure rather than on the value is not a concession.
// The value is unobservable on five channels — their stub answers
// `done.invoke` whatever the expression said — so a fixture resting on it
// would be green everywhere while proving nothing, which is the shape this
// repository treats as worse than no fixture at all.
//
// Sibling of `InvokeExpressionFailureIsReportedAotTest.cpp` (C++ AOT
// channel). Both engines ship in production, so each is held to the clause
// independently against one canonical fixture.
//
// Fixture: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <chrono>
#include <fstream>
#include <gtest/gtest.h>
#include <memory>
#include <sstream>
#include <string>
#include <thread>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

class InvokeExpressionFailureIsReportedTest : public ::testing::Test {
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

TEST_F(InvokeExpressionFailureIsReportedTest, AnUnevaluatableInvokeExpressionRaisesErrorExecution) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/invoke_expression_failure_is_reported/"
                                "invoke_expression_failure_is_reported.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();
    const std::string scxml = buffer.str();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

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

    ASSERT_TRUE(sm->loadSCXMLFromString(scxml));
    ASSERT_TRUE(sm->start());

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the machine did not reach the `error.execution` handler. §scxml-6.4.3 requires the "
           "Processor to evaluate a `srcexpr` when the `<invoke>` fires and to raise "
           "error.execution when it cannot; resting in `probe` means the failure was swallowed, "
           "and reaching `fail` means a child started on a value nothing could compute.";
}

}  // namespace Tests
}  // namespace SCE
