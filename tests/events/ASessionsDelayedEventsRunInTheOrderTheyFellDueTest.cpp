// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The delayed events of one session run one at a time, in the order they fell due.
//
// The scheduler wakes a timer thread when the first deadline arrives and hands every event
// that is due to a pool of callback workers. Two events with deadlines a few microseconds
// apart are not due at the same wake-up: the first is handed over alone, the timer sleeps
// again, and the second is handed over on the next wake-up. When the pool gave each hand-over
// to whichever worker was free, the second could be delivered before the first, or while the
// first was still being delivered. A statechart that sends `beat` and then `echo` with the
// same delay sees `echo` first, and only on a machine slow enough to split the wake-up.
// That is what `static_timers` measured (2026-10-09): `trace` was 142 where 124 is the
// order the document sent them in.
//
// What these pin is the guarantee that has to hold on any machine: one session's events never
// run at once and never run out of order, and a second session is not made to wait for the
// first.

#include "events/EventSchedulerImpl.h"
#include "events/IEventDispatcher.h"
#include "events/IEventTarget.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <future>
#include <gtest/gtest.h>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <vector>

namespace SCE {
namespace Tests {

namespace {

/// A target that accepts anything. `scheduleEvent` rejects a null target outright, so a probe
/// without one queues nothing and every assertion below would be about an empty schedule.
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

/// The session an event belongs to is the part of its name before the colon, so the callback,
/// which is given the event and not its session, can tell the sessions apart.
std::string sessionOf(const std::string &eventName) {
    return eventName.substr(0, eventName.find(':'));
}

/// Records, per session, the order events started and finished in and the most that ran at
/// once. An event named `<session>:slow` stays in its callback for `hold`, which is what lets a
/// later event of the same session be handed to a second worker while it is still running.
class OrderProbe {
public:
    explicit OrderProbe(std::chrono::milliseconds hold) : hold_(hold) {}

    std::shared_ptr<EventSchedulerImpl> makeScheduler() {
        return std::make_shared<EventSchedulerImpl>(
            [this](const EventDescriptor &event, std::shared_ptr<IEventTarget>, const std::string &) -> bool {
                run(event.eventName);
                return true;
            });
    }

    bool waitForFinished(size_t count, std::chrono::milliseconds limit) {
        std::unique_lock<std::mutex> lock(mutex_);
        return changed_.wait_for(lock, limit, [&] { return finished_.size() >= count; });
    }

    std::vector<std::string> started() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return started_;
    }

    std::vector<std::string> finished() const {
        std::lock_guard<std::mutex> lock(mutex_);
        return finished_;
    }

    int mostAtOnce(const std::string &session) const {
        std::lock_guard<std::mutex> lock(mutex_);
        auto it = most_.find(session);
        return it == most_.end() ? 0 : it->second;
    }

private:
    void run(const std::string &name) {
        const std::string session = sessionOf(name);
        {
            std::lock_guard<std::mutex> lock(mutex_);
            started_.push_back(name);
            int &now = inFlight_[session];
            ++now;
            int &most = most_[session];
            most = std::max(most, now);
        }

        if (name.size() >= 5 && name.compare(name.size() - 5, 5, ":slow") == 0) {
            std::this_thread::sleep_for(hold_);
        }

        {
            std::lock_guard<std::mutex> lock(mutex_);
            --inFlight_[session];
            finished_.push_back(name);
        }
        changed_.notify_all();
    }

    const std::chrono::milliseconds hold_;
    mutable std::mutex mutex_;
    std::condition_variable changed_;
    std::vector<std::string> started_;
    std::vector<std::string> finished_;
    std::map<std::string, int> inFlight_;
    std::map<std::string, int> most_;
};

void schedule(EventSchedulerImpl &scheduler, const std::string &name, std::chrono::milliseconds delay,
              const std::string &sendId, const std::string &session) {
    EventDescriptor event;
    event.eventName = name;
    scheduler.scheduleEvent(event, delay, std::make_shared<AcceptingTarget>(), sendId, session);
}

}  // namespace

TEST(ASessionsDelayedEventsRunInTheOrderTheyFellDueTest, TwoEventsOfOneSessionNeverRunAtOnce) {
    OrderProbe probe(std::chrono::milliseconds(300));
    auto scheduler = probe.makeScheduler();

    // The deadlines are far enough apart that each is handed over on a wake-up of its own,
    // which is the split a slow machine makes by accident. The first holds its callback long
    // enough for the second to be handed to the other worker.
    schedule(*scheduler, "s:slow", std::chrono::milliseconds(20), "first", "session-order");
    schedule(*scheduler, "s:quick", std::chrono::milliseconds(80), "second", "session-order");

    ASSERT_TRUE(probe.waitForFinished(2, std::chrono::milliseconds(5000)))
        << "both events have to be delivered for anything below to be measuring the scheduler";

    EXPECT_EQ(probe.mostAtOnce("s"), 1) << "the session's second event was executed while its first was still "
                                           "being executed: two workers took two events of one session, so "
                                           "what they do to the session's state can interleave";
    EXPECT_EQ(probe.started(), (std::vector<std::string>{"s:slow", "s:quick"}))
        << "events were started in a different order than they fell due in";
    EXPECT_EQ(probe.finished(), (std::vector<std::string>{"s:slow", "s:quick"}))
        << "the later event finished before the earlier one: it was delivered ahead of it";

    scheduler->shutdown();
}

TEST(ASessionsDelayedEventsRunInTheOrderTheyFellDueTest, ALongBatchOfOneSessionKeepsItsOrder) {
    OrderProbe probe(std::chrono::milliseconds(0));
    auto scheduler = probe.makeScheduler();

    // Many deadlines spread over a few milliseconds, so wake-ups split them in whatever way the
    // machine does. The order the deadlines are in is the only order that may come out.
    std::vector<std::string> expected;
    for (int i = 0; i < 60; ++i) {
        const std::string name = "s:e" + std::to_string(1000 + i);
        expected.push_back(name);
        schedule(*scheduler, name, std::chrono::milliseconds(30 + i), "id" + std::to_string(i), "session-batch");
    }

    ASSERT_TRUE(probe.waitForFinished(expected.size(), std::chrono::milliseconds(8000)));

    EXPECT_EQ(probe.mostAtOnce("s"), 1);
    EXPECT_EQ(probe.finished(), expected);

    scheduler->shutdown();
}

TEST(ASessionsDelayedEventsRunInTheOrderTheyFellDueTest, ASecondSessionIsNotMadeToWaitForTheFirst) {
    OrderProbe probe(std::chrono::milliseconds(600));
    auto scheduler = probe.makeScheduler();

    // The first session's event holds its worker for 600ms. The second session's event is due
    // well inside that, and has to be delivered on the other worker: ordering is per session,
    // and a scheduler that serialised everything would also pass the two tests above.
    schedule(*scheduler, "a:slow", std::chrono::milliseconds(10), "slow-a", "session-a");
    schedule(*scheduler, "b:quick", std::chrono::milliseconds(100), "quick-b", "session-b");

    ASSERT_TRUE(probe.waitForFinished(2, std::chrono::milliseconds(5000)));

    EXPECT_EQ(probe.finished(), (std::vector<std::string>{"b:quick", "a:slow"}))
        << "the second session's event waited for the first session's to finish";

    scheduler->shutdown();
}

}  // namespace Tests
}  // namespace SCE
