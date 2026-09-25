// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: the outer half of the main event loop, reached from a
// host call — Interpreter path.
//
// Once a macrostep has completed and its invokes have started, `mainEventLoop`
// takes the next external event, and goes on doing so until the external queue
// is empty:
//
//     while running:
//         ... complete the macrostep, start its invokes ...
//         externalEvent = externalQueue.dequeue()
//
// A host call is where this engine runs that loop. So what the macrostep a host
// event opened sent to this very session — a `<send>` with no target, which
// §scxml-6.2.4 puts on the session's own external queue — has been taken by the
// time the call returns. The AOT engine's `processEvent` runs the same loop, and
// so does this engine's `start`.
//
// Measured while this engine moved onto the shared Appendix D core: `start`
// took the external queue and `processEvent` did not. The same document then
// answered differently depending on which of the two had opened the macrostep
// that queued the event, and the event waited for the host's next call — or
// for good, because the raiser hands a newly arrived event straight to the
// machine only while nothing is queued ahead of it.
//
// Interactive mode is the other half of the contract: there the host steps the
// queue itself, so the call must leave the event where it is — and each step
// must say whether it took an event, including one no state answers.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <gtest/gtest.h>

namespace SCE {
namespace Tests {

namespace {

/// `go` sends `echo` to this session and moves to `sent`; only taking `echo`
/// off the external queue reaches `echoed`.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="idle">

  <state id="idle">
    <transition event="go" target="sent">
      <send event="echo"/>
    </transition>
  </state>

  <state id="sent">
    <transition event="echo" target="echoed"/>
  </state>

  <state id="echoed"/>
</scxml>
)";

}  // namespace

class HostCallTakesTheExternalQueueTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        sm_ = std::make_shared<StateMachine>(*engine_);
        // §scxml-6.2: a `<send>` needs the dispatcher chain (scheduler ->
        // target factory -> dispatcher) to reach this session's own queue.
        scheduler_ = std::make_shared<EventSchedulerImpl>(
            [](const EventDescriptor &event, std::shared_ptr<IEventTarget> target, const std::string &) -> bool {
                try {
                    return target->send(event).get().isSuccess;
                } catch (...) {
                    return false;
                }
            });
        raiser_ = std::make_shared<EventRaiserImpl>();
        raiser_->setScheduler(scheduler_);
        sm_->setEventRaiser(raiser_);
        sm_->setEventDispatcher(
            std::make_shared<EventDispatcherImpl>(scheduler_, std::make_shared<EventTargetFactoryImpl>(raiser_)));
        ASSERT_TRUE(sm_->loadSCXMLFromString(DOCUMENT));
    }

    void TearDown() override {
        sm_.reset();
        if (scheduler_) {
            scheduler_->shutdown(true);
        }
        if (engine_) {
            engine_->shutdown();
        }
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<EventSchedulerImpl> scheduler_;
    std::shared_ptr<EventRaiserImpl> raiser_;
    std::shared_ptr<StateMachine> sm_;
};

/// The axis: the call that opened the macrostep also takes what it queued.
TEST_F(HostCallTakesTheExternalQueueTest, WhatTheMacrostepSentItselfIsTakenBeforeTheCallReturns) {
    ASSERT_TRUE(sm_->start());
    ASSERT_EQ(sm_->getCurrentState(), "idle");

    ASSERT_TRUE(sm_->processEvent("go").success) << "`go` matches `idle`'s transition";

    EXPECT_EQ(sm_->getCurrentState(), "echoed")
        << "`go`'s transition sent `echo` to this session's external queue, and the main event loop "
           "takes the next external event once the macrostep is over — an engine left in `sent` "
           "returned from the host call with its own event still queued";
    EXPECT_FALSE(raiser_->hasQueuedEvents()) << "nothing the macrostep sent is left waiting";
}

/// The other half: in interactive mode the call opens the macrostep and stops
/// there, and the event it queued is the host's to step.
TEST_F(HostCallTakesTheExternalQueueTest, InteractiveModeLeavesItForTheHostToStep) {
    ASSERT_TRUE(sm_->start(/*autoProcessQueuedEvents=*/false));
    ASSERT_EQ(sm_->getCurrentState(), "idle");

    ASSERT_TRUE(sm_->processEvent("go").success) << "`go` matches `idle`'s transition";

    EXPECT_EQ(sm_->getCurrentState(), "sent") << "an interactive host steps the queue itself, so the call that "
                                                 "delivered `go` must not have taken `echo` too";
    ASSERT_TRUE(raiser_->hasQueuedEvents()) << "`echo` is waiting for the host";

    EXPECT_TRUE(raiser_->processNextQueuedEvent()) << "the host steps the one event that is queued";
    EXPECT_EQ(sm_->getCurrentState(), "echoed") << "and that step takes `echo`";
}

/// A step whose event no active state answers is still a step (§scxml-3.1.2
/// discards the event). A host stepping the queue reads the step's answer as
/// "was there one", so reporting the discard as an empty queue loses the step
/// and the event with it — the interactive runner's forward run on W3C test 240
/// stopped short of `pass` exactly that way.
TEST_F(HostCallTakesTheExternalQueueTest, AStepWhoseEventNothingAnswersStillReportsTakingIt) {
    ASSERT_TRUE(sm_->start(/*autoProcessQueuedEvents=*/false));
    ASSERT_EQ(sm_->getCurrentState(), "idle");

    ASSERT_TRUE(raiser_->raiseExternalEvent("echo", "")) << "`idle` answers only `go`";
    ASSERT_TRUE(raiser_->hasQueuedEvents()) << "an interactive host steps `echo` itself";

    EXPECT_TRUE(raiser_->processNextQueuedEvent())
        << "the step took `echo` off the queue; that no state answered it is the discard, not an empty queue";
    EXPECT_EQ(sm_->getCurrentState(), "idle") << "and the discard moved nothing";
    EXPECT_FALSE(raiser_->hasQueuedEvents()) << "the event is gone, so the next step has nothing to take";
    EXPECT_FALSE(raiser_->processNextQueuedEvent()) << "an empty queue is the one step that answers false";
}

}  // namespace Tests
}  // namespace SCE
