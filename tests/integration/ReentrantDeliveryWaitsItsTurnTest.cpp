// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML Appendix D: an event handed to a machine in the middle of its own
// macrostep is processed where the main event loop takes it, in turn —
// Interpreter path.
//
// Appendix D processes an event in exactly one place: where `mainEventLoop`
// dequeues it. The algorithm has no way for an event to arrive anywhere else,
// but an engine does. A transport that delivers at once, on the sender's own
// thread, calls straight back into `processEvent` while the macrostep that
// sent the event is still running — and so does a host callback bound into
// the datamodel, or a parent forwarding to a child that is the one calling it.
//
// Such an event is external, and arrives while the interpreter is busy, so the
// clause's answer is the ordinary one: it joins the external queue behind
// whatever is already there, and is taken when its turn comes. Three ways to
// get that wrong, each with a test below:
//
//   - process it on the spot, inside the macrostep: its transitions are then
//     selected against a configuration the running microstep has half exited;
//   - hold it apart from the external queue and take it before, or after,
//     everything queued there: the clause's first-in-first-out order is lost;
//   - take it without being asked to: an interactive host, which steps the
//     queue itself, finds an event processed that it never stepped.
//
// The transport here is the smallest real one: a registered target scheme
// whose `send` hands the event to the machine synchronously.

#include "events/EventDispatcherImpl.h"
#include "events/EventSchedulerImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "events/IEventTarget.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <future>
#include <gtest/gtest.h>

namespace SCE {
namespace Tests {

namespace {

/// Each path sends one event to the session's own external queue and one
/// through the loopback transport, in opposite orders, and the states after
/// it only reach `inOrder` if the two are taken in the order they were sent.
constexpr const char *DOCUMENT = R"(<?xml version="1.0" encoding="UTF-8"?>
<scxml xmlns="http://www.w3.org/2005/07/scxml" version="1.0"
       datamodel="ecmascript" initial="idle">

  <state id="idle">
    <transition event="queuedThenHandedOver" target="expectQueued">
      <send event="queued"/>
      <send event="handedOver" target="loopback://self"/>
    </transition>
    <transition event="handedOverThenQueued" target="expectHandedOver">
      <send event="handedOver" target="loopback://self"/>
      <send event="queued"/>
    </transition>
  </state>

  <state id="expectQueued">
    <transition event="queued" target="thenHandedOver"/>
    <transition event="handedOver" target="outOfOrder"/>
  </state>
  <state id="thenHandedOver">
    <transition event="handedOver" target="inOrder"/>
  </state>

  <state id="expectHandedOver">
    <transition event="handedOver" target="thenQueued"/>
    <transition event="queued" target="outOfOrder"/>
  </state>
  <state id="thenQueued">
    <transition event="queued" target="inOrder"/>
  </state>

  <state id="inOrder"/>
  <state id="outOfOrder"/>
</scxml>
)";

/// An in-process transport that delivers at once, on the sender's thread —
/// which, for a machine sending to itself, is the thread running the
/// macrostep that sent it.
class LoopbackTarget : public IEventTarget {
public:
    explicit LoopbackTarget(std::weak_ptr<StateMachine> machine) : machine_(std::move(machine)) {}

    std::future<SendResult> send(const EventDescriptor &event) override {
        if (const auto machine = machine_.lock()) {
            machine->processEvent(event.eventName, event.data);
        }
        std::promise<SendResult> delivered;
        delivered.set_value(SendResult::success(event.sendId));
        return delivered.get_future();
    }

    std::string getTargetType() const override {
        return "loopback";
    }

    bool canHandle(const std::string &targetUri) const override {
        return targetUri.rfind("loopback:", 0) == 0;
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    std::string getDebugInfo() const override {
        return "loopback target";
    }

private:
    std::weak_ptr<StateMachine> machine_;
};

}  // namespace

class ReentrantDeliveryWaitsItsTurnTest : public ::testing::Test {
protected:
    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();

        sm_ = std::make_shared<StateMachine>(*engine_);
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
        auto targets = std::make_shared<EventTargetFactoryImpl>(raiser_);
        std::weak_ptr<StateMachine> machine = sm_;
        targets->registerTargetType(
            "loopback", [machine](const std::string &) { return std::make_shared<LoopbackTarget>(machine); });
        sm_->setEventRaiser(raiser_);
        sm_->setEventDispatcher(std::make_shared<EventDispatcherImpl>(scheduler_, targets));
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

/// Handed over second, taken second: it joined the queue behind the event
/// already there, rather than going first by some route of its own.
TEST_F(ReentrantDeliveryWaitsItsTurnTest, AnEventHandedOverAfterAQueuedOneIsTakenAfterIt) {
    ASSERT_TRUE(sm_->start());
    ASSERT_TRUE(sm_->processEvent("queuedThenHandedOver").success);

    EXPECT_EQ(sm_->getCurrentState(), "inOrder")
        << "`queued` was put on the external queue first and `handedOver` arrived through the transport "
           "second, so the loop must take them in that order — `outOfOrder` means the handed-over event "
           "was taken ahead of the queue it should have joined";
}

/// Handed over first, taken first — and, since it arrived in the middle of the
/// microstep that sent it, only once that macrostep is over: the transition
/// into `expectHandedOver` has to have finished for it to match anything.
TEST_F(ReentrantDeliveryWaitsItsTurnTest, AnEventHandedOverBeforeAQueuedOneIsTakenBeforeItAndAfterTheMacrostep) {
    ASSERT_TRUE(sm_->start());
    ASSERT_TRUE(sm_->processEvent("handedOverThenQueued").success);

    EXPECT_EQ(sm_->getCurrentState(), "inOrder")
        << "`handedOver` arrived first, so it is taken first; `outOfOrder` means it was held apart and "
           "taken after the queue, and staying in `expectHandedOver` means it was processed on the spot, "
           "against a configuration the microstep had already exited, and matched nothing";
}

/// Interactive mode: what arrived mid-macrostep waits on the queue like
/// anything else, for the host to step.
TEST_F(ReentrantDeliveryWaitsItsTurnTest, InteractiveModeLeavesTheHandedOverEventForTheHost) {
    ASSERT_TRUE(sm_->start(/*autoProcessQueuedEvents=*/false));
    ASSERT_TRUE(sm_->processEvent("handedOverThenQueued").success);

    ASSERT_EQ(sm_->getCurrentState(), "expectHandedOver")
        << "an interactive host steps the queue itself, so nothing the macrostep received may have been "
           "taken for it";
    ASSERT_TRUE(raiser_->hasQueuedEvents()) << "the handed-over event is on the queue, not somewhere else";

    EXPECT_TRUE(raiser_->processNextQueuedEvent());
    EXPECT_EQ(sm_->getCurrentState(), "thenQueued") << "the first step takes `handedOver`";
    EXPECT_TRUE(raiser_->processNextQueuedEvent());
    EXPECT_EQ(sm_->getCurrentState(), "inOrder") << "and the second takes `queued`";
}

}  // namespace Tests
}  // namespace SCE
