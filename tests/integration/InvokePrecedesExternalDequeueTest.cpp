// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: pending invokes start before the external dequeue — Interpreter path.
//
// `mainEventLoop` completes the macrostep on eventless and internal
// transitions alone, then runs `invoke(inv)` for every state entered on the
// last iteration, and only then reaches `externalQueue.dequeue()`:
//
//   while running and not macrostepDone:
//       ... selectEventlessTransitions() / internalQueue.dequeue() ...
//   for state in statesToInvoke.sort(entryOrder):
//       for inv in state.invoke.sort(documentOrder):
//           invoke(inv)
//   statesToInvoke.clear()
//   if not internalQueue.isEmpty(): continue
//   externalEvent = externalQueue.dequeue()
//
// The external queue is named exactly once in that loop and it is after the
// invokes. An engine that folds the external drain into its macrostep
// completion loop consumes whatever `<onentry>` queued for the parent itself
// while the invoked children do not yet exist, so an `autoforward` child
// misses every event the parent queued on the way in — a lost event, not a
// reordered one.
//
// The sibling `AutoforwardDequeuePointTest.cpp` pins *where in the loop* the
// forward sits and is deliberately blind to this axis: there the child opens
// the exchange, so it is alive before anything is queued. Here the parent
// queues first and the child starts second.
//
// Fixture: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml

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

class InvokePrecedesExternalDequeueTest : public ::testing::Test {
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

TEST_F(InvokePrecedesExternalDequeueTest, PendingInvokesStartBeforeTheExternalDequeue) {
    const std::string fixture =
        std::string(SCE_PROJECT_ROOT) +
        "/integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue.scxml";
    std::ifstream in(fixture);
    ASSERT_TRUE(in.is_open()) << "canonical fixture not readable: " << fixture;
    std::ostringstream buffer;
    buffer << in.rdbuf();

    auto sm = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());

    // §scxml-6.2: the parent's targetless `<send event="kick"/>`, its
    // `<send target="#_inv_watch">` and the child's `<send target="#_parent">`
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

    ASSERT_FALSE(sm->isRunning()) << "parent did not halt within 5s — the watching child answered neither "
                                  << "verdict, so `probe` never reached it";

    EXPECT_EQ(sm->getCurrentState(), "pass")
        << "the watching child answered `probe` from `waiting`, so it never saw `kick`. The parent "
        << "drained its external queue before starting the invoke, and the event `<onentry>` had "
        << "queued for itself was consumed while no child existed. W3C Appendix D `mainEventLoop` "
        << "runs `invoke(inv)` for every state entered on the last iteration before it reaches "
        << "`externalQueue.dequeue()`, so an autoforward child is live for the whole external queue.";
}

}  // namespace Tests
}  // namespace SCE
