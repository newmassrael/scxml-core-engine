// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
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
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachineBuilder.h"
#include "scripting/ScriptEngineProvider.h"

#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <memory>
#include <string>
#include <thread>
#include <vector>

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

namespace {

/// A raiser over `scheduler`.
std::shared_ptr<EventRaiserImpl> raiserOver(const std::shared_ptr<EventSchedulerImpl> &scheduler) {
    auto raiser = std::make_shared<EventRaiserImpl>();
    raiser->setScheduler(scheduler);
    return raiser;
}

}  // namespace

// A child session built for `<invoke>` shares its parent's scheduler, and that is how it
// inherits the parent's mode. The builder used to write AUTOMATIC into whatever scheduler it
// was given, so the first child a stepped parent invoked put the whole tree back on the wall
// clock: from then on a delayed event fell due when the host was slow, not when a step asked
// for it.
TEST(SchedulerManualModeTest, ABuilderNotToldAModeLeavesTheSchedulersModeAlone) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();
    scheduler->setMode(SchedulerMode::MANUAL);

    auto machine = StateMachineBuilder()
                       .withScriptEngine(ScriptEngineProvider::getScriptEngine())
                       .withEventRaiser(raiserOver(scheduler))
                       .build();
    ASSERT_NE(machine, nullptr);

    EXPECT_EQ(scheduler->getMode(), SchedulerMode::MANUAL)
        << "building a state machine over a MANUAL scheduler changed the scheduler's mode";

    scheduler->shutdown();
}

// And the explicit request is still honoured, or the test above would pass against a builder
// that had stopped touching the mode at all.
TEST(SchedulerManualModeTest, ABuilderToldAModeSetsIt) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();

    auto machine = StateMachineBuilder()
                       .withScriptEngine(ScriptEngineProvider::getScriptEngine())
                       .withEventRaiser(raiserOver(scheduler))
                       .withSchedulerMode(SchedulerMode::MANUAL)
                       .build();
    ASSERT_NE(machine, nullptr);

    EXPECT_EQ(scheduler->getMode(), SchedulerMode::MANUAL) << "withSchedulerMode(MANUAL) did not reach the scheduler";

    scheduler->shutdown();
}

// Whether a session is stepped is its raiser's immediate mode. A child shares the scheduler of
// the parent that is being stepped but is not stepped itself, so its raiser must still hand an
// event to its session at once. Reading the scheduler's mode here queued the child's events
// where nothing would ever take them: a stepped parent invoking a child that had to answer
// came to a stop.
TEST(SchedulerManualModeTest, ARaiserInImmediateModeDeliversAtOnceOverAManualScheduler) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();
    scheduler->setMode(SchedulerMode::MANUAL);

    auto raiser = raiserOver(scheduler);
    std::atomic<int> delivered{0};
    raiser->setEventCallback([&](const std::string &, const std::string &) {
        delivered.fetch_add(1);
        return true;
    });
    raiser->setImmediateMode(true);

    raiser->raiseExternalEvent("report", "");

    EXPECT_EQ(delivered.load(), 1) << "an event raised in immediate mode waited for a step the session does not have";

    scheduler->shutdown();
}

// §scxml-6.2.3: a delayed send is dispatched once its delay has elapsed. In MANUAL mode the
// host decides how much time has elapsed, by the amount it names and not by the next deadline
// (`forcePoll()`), so a replay of a recorded run releases exactly what the recording's time
// span covers. The same scenarios are replayed on the generated backends with a manual clock.
TEST(SchedulerManualModeTest, AdvancingTheLogicalClockReleasesExactlyWhatItsSpanCovers) {
    std::vector<std::string> fired;
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [&fired](const EventDescriptor &event, std::shared_ptr<IEventTarget>, const std::string &) -> bool {
            fired.push_back(event.eventName);
            return true;
        });
    scheduler->setMode(SchedulerMode::MANUAL);

    EventDescriptor soon;
    soon.eventName = "soon";
    EventDescriptor later;
    later.eventName = "later";
    scheduler->scheduleEvent(soon, std::chrono::milliseconds(100), std::make_shared<AcceptingTarget>(), "a", "s");
    scheduler->scheduleEvent(later, std::chrono::milliseconds(300), std::make_shared<AcceptingTarget>(), "b", "s");

    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(99)), 0u) << "99ms has not covered a 100ms delay";
    EXPECT_TRUE(fired.empty());
    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(1)), 1u) << "100ms covers the 100ms delay";
    EXPECT_EQ(fired, std::vector<std::string>{"soon"});
    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(199)), 0u) << "the 300ms delay is one ms away";
    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(1)), 1u);
    EXPECT_EQ(fired, (std::vector<std::string>{"soon", "later"}));
    EXPECT_EQ(scheduler->getLogicalTime(), std::chrono::milliseconds(300));

    scheduler->shutdown();
}

// The queue is ordered by logical deadline in MANUAL mode. An event armed after the clock moved
// on, with a shorter delay than one armed earlier, has the later deadline, and ordering by the
// wall reading at the time of arming put it first: the older event then stood behind one that
// was not yet due and was not released when its own time came.
TEST(SchedulerManualModeTest, ADeadlineLaterOnTheLogicalClockDoesNotOvertakeAnEarlierOne) {
    std::vector<std::string> fired;
    auto scheduler = std::make_shared<EventSchedulerImpl>(
        [&fired](const EventDescriptor &event, std::shared_ptr<IEventTarget>, const std::string &) -> bool {
            fired.push_back(event.eventName);
            return true;
        });
    scheduler->setMode(SchedulerMode::MANUAL);

    EventDescriptor first;
    first.eventName = "first";
    EventDescriptor second;
    second.eventName = "second";
    scheduler->scheduleEvent(first, std::chrono::milliseconds(100), std::make_shared<AcceptingTarget>(), "a", "s");
    scheduler->advanceTime(std::chrono::milliseconds(60));
    // Armed at 60ms for 50ms: due at 110ms, after `first` (100ms), though its delay is shorter.
    scheduler->scheduleEvent(second, std::chrono::milliseconds(50), std::make_shared<AcceptingTarget>(), "b", "s");

    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(40)), 1u) << "`first` is due at 100ms";
    EXPECT_EQ(fired, std::vector<std::string>{"first"});
    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(10)), 1u) << "`second` is due at 110ms";
    EXPECT_EQ(fired, (std::vector<std::string>{"first", "second"}));

    scheduler->shutdown();
}

// Time is the wall clock's in AUTOMATIC mode, and the host cannot move it: an advance there is
// refused rather than silently reading as a wait.
TEST(SchedulerManualModeTest, AutomaticModeRefusesToHaveItsClockMoved) {
    ManualModeProbe probe;
    auto scheduler = probe.makeScheduler();

    EXPECT_EQ(scheduler->advanceTime(std::chrono::milliseconds(1000)), 0u);
    EXPECT_EQ(scheduler->getLogicalTime(), std::chrono::milliseconds(0)) << "the refused advance moved the clock";

    scheduler->shutdown();
}

}  // namespace Tests
}  // namespace SCE
