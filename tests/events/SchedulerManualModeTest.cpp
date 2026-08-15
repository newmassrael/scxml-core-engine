// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// `SchedulerMode::MANUAL` means the wall clock does not fire anything.
//
// The mode exists for time-travel debugging: `InteractiveTestRunner` sets it so
// that "events only execute on explicit stepForward()" — its own words — and
// then advances a LOGICAL clock with `forcePoll()`. Every snapshot the
// interactive suite compares rests on that: a step is reproducible only if the
// same events landed in it.
//
// What this file pins is that the mode holds while real time passes. The
// scheduler keeps a timer thread that waits on `steady_clock` deadlines and
// calls `processReadyEvents()` when they elapse; that thread does not stop for
// MANUAL mode, and `processReadyEvents()` decides readiness against the logical
// clock. An event whose logical deadline has already arrived is therefore ready
// to BOTH — the stepper that is about to poll, and a timer thread that woke on
// its own. Which one gets it depends on how busy the machine is.
//
// That is why `ComprehensiveInteractiveTests` was red only under load, and why
// it named a different case each time (measured 2026-08-15: Test193, then
// Test175, then a timeout, then green on a quiet machine). A test whose failing
// case moves between runs is not being failed by the tree.

#include "events/EventSchedulerImpl.h"
#include "events/IEventDispatcher.h"
#include "events/IEventTarget.h"

#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>

namespace SCE {
namespace Tests {

namespace {

/// A target that accepts anything.
///
/// Not decoration: `scheduleEvent` rejects a null target outright, so a probe
/// without one measures nothing at all — it queues nothing, and every later
/// assertion reads as "the scheduler held the event" when the scheduler was
/// never given one. Measured 2026-08-15 while writing this file.
class AcceptingTarget : public IEventTarget {
public:
    std::future<SendResult> send(const EventDescriptor &) override {
        std::promise<SendResult> promise;
        SendResult result;
        result.isSuccess = true;
        promise.set_value(result);
        return promise.get_future();
    }

    std::string getTargetType() const override {
        return "test";
    }

    bool canHandle(const std::string &) const override {
        return true;
    }

    std::vector<std::string> validate() const override {
        return {};
    }

    std::string getDebugInfo() const override {
        return "AcceptingTarget";
    }
};

/// Counts executions. The scheduler's callback is the execution point, so a
/// count here is exactly "how many events the scheduler let out".
class ManualModeProbe {
public:
    std::shared_ptr<EventSchedulerImpl> makeScheduler() {
        return std::make_shared<EventSchedulerImpl>(
            [this](const EventDescriptor &, std::shared_ptr<IEventTarget>, const std::string &) -> bool {
                fired_.fetch_add(1, std::memory_order_relaxed);
                return true;
            });
    }

    int fired() const {
        return fired_.load(std::memory_order_relaxed);
    }

private:
    std::atomic<int> fired_{0};
};

}  // namespace

TEST(SchedulerManualModeTest, ManualModeFiresNothingWhileRealTimePasses) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();
    scheduler->setMode(SchedulerMode::MANUAL);

    EventDescriptor event;
    event.eventName = "tick";

    // A zero delay is the sharpest form of the question: its LOGICAL deadline
    // is the logical clock's current value, so the event is ready the moment it
    // is queued — while its wall-clock deadline is now, which is when a timer
    // thread waiting on `steady_clock` wakes up. Nothing has polled.
    scheduler->scheduleEvent(event, std::chrono::milliseconds(0), std::make_shared<AcceptingTarget>(), "s0",
                             "session-manual");
    ASSERT_EQ(scheduler->getScheduledEventCount(), 1u)
        << "the event has to be queued for anything below to be measuring the scheduler";

    std::this_thread::sleep_for(std::chrono::milliseconds(250));

    EXPECT_EQ(probe.fired(), 0) << "the scheduler executed an event in MANUAL mode without an explicit poll. "
                                   "MANUAL exists so a stepper owns when events land — a timer thread that "
                                   "fires them on its own makes the step an event lands in depend on how "
                                   "busy the machine is, which is what a replay comparison cannot survive";
    EXPECT_EQ(scheduler->getScheduledEventCount(), 1u)
        << "the event should still be waiting for the poll that has not happened yet";

    // And the poll is what releases it — otherwise this test would pass against
    // a scheduler that had simply lost the event.
    const size_t polled = scheduler->forcePoll();
    EXPECT_EQ(polled, 1u) << "forcePoll() is the explicit poll MANUAL mode is waiting for";
    EXPECT_EQ(probe.fired(), 1) << "the polled event must actually execute";

    scheduler->shutdown();
}

TEST(SchedulerManualModeTest, ManualModeHoldsADelayedEventPastItsWallClockDeadline) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();
    scheduler->setMode(SchedulerMode::MANUAL);

    EventDescriptor event;
    event.eventName = "timeout";

    // The shape the W3C interactive suite trips over: `<send delay="...">` in a
    // document that is being stepped. Real time passes the deadline while the
    // stepper is still working, and the logical clock has not moved.
    scheduler->scheduleEvent(event, std::chrono::milliseconds(50), std::make_shared<AcceptingTarget>(), "s1",
                             "session-manual");
    ASSERT_EQ(scheduler->getScheduledEventCount(), 1u)
        << "the event has to be queued for anything below to be measuring the scheduler";

    std::this_thread::sleep_for(std::chrono::milliseconds(250));

    EXPECT_EQ(probe.fired(), 0) << "a delayed event fired on the wall clock in MANUAL mode. The logical clock "
                                   "had not reached its deadline, so no step had asked for it";

    const size_t polled = scheduler->forcePoll();
    EXPECT_EQ(polled, 1u) << "forcePoll() advances the logical clock to the event and releases it";
    EXPECT_EQ(probe.fired(), 1) << "the polled event must actually execute";

    scheduler->shutdown();
}

}  // namespace Tests
}  // namespace SCE
