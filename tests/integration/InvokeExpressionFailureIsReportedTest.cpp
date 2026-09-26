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

    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "the machine did not reach the `error.execution` handler. §scxml-6.4.3 requires the "
           "Processor to evaluate a `srcexpr` when the `<invoke>` fires and to raise "
           "error.execution when it cannot; resting in `probe` means the failure was swallowed, "
           "and reaching `fail` means a child started on a value nothing could compute.";
}

// §scxml-3.12.2 lets a platform "include additional information about the
// nature of the error in the 'data' field", and this engine does. What that
// field SAYS is asserted for the six generated backends by
// `sce-build/tests/one_wording_for_an_invoke_expression_failure.rs`, which
// reads their emitted text. That test cannot reach this engine: its literal is
// compiled rather than generated, so the only way to read it is to run a
// machine and look at what arrived.
//
// The document is built here rather than added to the canonical fixture for
// the reason the fixture's own header gives: it is deliberately single-axis
// and is driven by six other channels that would then all carry a `<data>`
// they have no use for.
TEST_F(InvokeExpressionFailureIsReportedTest, TheRaisedErrorNamesTheExpressionInTheOneSharedWording) {
    const std::string scxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="probe" name="invoke_expression_failure_message">
    <datamodel>
        <data id="target" expr="null"/>
        <data id="seen" expr="''"/>
    </datamodel>
    <state id="probe">
        <invoke type="scxml" srcexpr="target.path"/>
        <transition event="error.execution" target="pass">
            <assign location="seen" expr="_event.data"/>
        </transition>
    </state>
    <final id="pass"/>
</scxml>)";

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
    ASSERT_EQ(sm->terminalState().value_or(""), "pass") << "the handler never fired, so there is no message to read";

    const std::string expected = "<invoke srcexpr='target.path'> could not be evaluated";
    const auto seen = engine_->evaluateExpression(sm->getSessionId(), "seen").get();
    ASSERT_TRUE(seen.isSuccess()) << "could not read the captured `_event.data`";
    EXPECT_EQ(seen.getValue<std::string>(), expected)
        << "this engine's §6.4.3 wording drifted from the one the six generated backends emit. "
           "One W3C fact, one wording — the same collapse §6.4.1 already paid for.";
}

// §scxml-6.4.1: "if the value of 'src' ... is invalid, the processor MUST
// place error.execution in the internal event queue". An expression that
// evaluates cleanly to a document nobody can load is that case, one step
// past the one above — and the two used to share a silence: this engine
// logged and returned for both, so a machine whose child never existed sat
// waiting for a `done.invoke` that could not come.
//
// Only this channel can be asked. The AOT backends never load a document at
// run time — their child is fixed at build time (§2.13) — so "the named
// document does not exist" is not a state they can reach, and a fixture
// driven on all seven would be asserting about six machines that cannot fail
// the way this one did.
TEST_F(InvokeExpressionFailureIsReportedTest, ADocumentThatCannotBeLoadedIsReported) {
    const std::string scxml = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0" datamodel="ecmascript"
       initial="probe" name="invoke_src_missing">
    <datamodel>
        <data id="target" expr="'file:no-such-document-8f3a.scxml'"/>
        <data id="seen" expr="''"/>
    </datamodel>
    <state id="probe">
        <invoke type="scxml" srcexpr="target"/>
        <transition event="error.execution" target="pass">
            <assign location="seen" expr="_event.data"/>
        </transition>
        <transition event="done.invoke" target="fail"/>
    </state>
    <final id="pass"/>
    <final id="fail"/>
</scxml>)";

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

    EXPECT_EQ(sm->terminalState().value_or(""), "pass")
        << "resting in `probe` means the missing document was swallowed; §scxml-6.4.1 requires "
           "error.execution when the source is invalid, and a machine that cannot see the failure "
           "waits for a `done.invoke` that will never arrive.";

    const auto seen = engine_->evaluateExpression(sm->getSessionId(), "seen").get();
    ASSERT_TRUE(seen.isSuccess()) << "could not read the captured `_event.data`";
    EXPECT_EQ(seen.getValue<std::string>(), "<invoke> could not load 'file:no-such-document-8f3a.scxml'")
        << "the refusal must name the document that did not load — that is the whole of what an "
           "author can act on here";
}

}  // namespace Tests
}  // namespace SCE
