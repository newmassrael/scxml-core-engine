// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.4 autoforward happens at the external dequeue — Interpreter path.
//
// Appendix D's `mainEventLoop` forwards one statement after
// `externalQueue.dequeue()` and before `selectTransitions`, and §6.4.2 says
// the same in prose: the parent forwards "at the point at which it removes it
// from the external event queue". Forwarding where the event is *queued*
// instead breaks run-to-completion — the child sees event N before the parent
// has processed 1..N-1, so the two sessions disagree about what has happened.
//
// This is the half of the contract queue order cannot express. Forwarding at
// the enqueue and forwarding at the dequeue hand the child the same events in
// the same relative order; only a second channel driven from inside a
// transition body — the parent's `<send target="#_inv_probe">` — separates
// them.
//
// Siblings `AutoforwardDoneInvokeTest.cpp` and
// `AutoforwardInternalQueueTest.cpp` pin *which* events are forwarded and are
// deliberately blind to *when*; this one pins the position and nothing else.
//
// Fixture: integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml

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

class AutoforwardDequeuePointTest : public ::testing::Test {
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

TEST_F(AutoforwardDequeuePointTest, AnExternalEventIsForwardedAtTheDequeueNotTheEnqueue) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/autoforward_dequeue_point/autoforward_dequeue_point.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the child's `<send target="#_parent">`, the parent's
    // targetless `<send event="first"/>` and its `<send target="#_inv_probe">`
    // all need the dispatcher chain (scheduler -> target factory ->
    // dispatcher).
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
            (void)event;
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

    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline) {
        if (!sm->isRunning()) {
            break;
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the probe child reported neither "
                                  << "verdict, so `second` never reached it";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the probe child saw `second` before `mark`, so both events were handed over while "
        << "the parent was still executing the transition that queued them. W3C Appendix D "
        << "`mainEventLoop` forwards one statement after `externalQueue.dequeue()`, and §6.4.2 "
        << "puts it \"at the point at which it removes it from the external event queue\" — "
        << "forwarding at the enqueue lets the child run ahead of the parent by a whole event.";
}

}  // namespace Tests
}  // namespace SCE
