// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// W3C SCXML 6.2 + 6.4 + C.1: a delay postpones a <send>, it does not change
// where the send goes — C++ Interpreter.
//
// The test owns the clock. The fixture reads `early` (10ms) before `inner`
// (20ms), and both are sent on entry to the first state, before the two invoked
// children have started. On the wall clock both fall due while the children are
// still starting whenever the host is busy, and the main loop then takes the
// internal `inner` before the external `early` (W3C SCXML 3.13), so `order`
// reads 13 for a reason that has nothing to do with routing. Measured on an
// 8-core host with ten spinning processes: 13 of 20 runs. The other channels
// step a virtual clock; a scheduler that fires an event only when this test
// advances its clock is that clock here. The engine's own scheduler is not used
// because its MANUAL mode is not one a session with children can run in: the
// builder of a child session puts the shared scheduler back on the wall clock.
//
// Fixture:
// integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names.scxml

#include "events/EventDispatcherImpl.h"
#include "events/EventTargetFactoryImpl.h"
#include "runtime/EventRaiserImpl.h"
#include "runtime/StateMachine.h"
#include "scripting/ScriptEngineProvider.h"

#include <algorithm>
#include <chrono>
#include <future>
#include <gtest/gtest.h>
#include <memory>
#include <mutex>
#include <string>
#include <thread>
#include <utility>
#include <vector>

#ifndef SCE_PROJECT_ROOT
#define SCE_PROJECT_ROOT "."
#endif

namespace SCE {
namespace Tests {

namespace {

/// A scheduler whose clock moves only when `forcePoll()` is called.
///
/// `forcePoll()` advances the clock to the earliest pending deadline and delivers every
/// event due then, in the order they were scheduled, on the calling thread. It reports
/// AUTOMATIC and ignores `setMode`, so a session built for `<invoke>` that resets the mode
/// of the scheduler it shares leaves this one as it is and runs as it does in production.
class VirtualClockScheduler : public IEventScheduler {
public:
    explicit VirtualClockScheduler(EventExecutionCallback deliver) : deliver_(std::move(deliver)) {}

    std::future<std::string> scheduleEvent(const EventDescriptor &event, std::chrono::milliseconds delay,
                                           std::shared_ptr<IEventTarget> target, const std::string &sendId,
                                           const std::string &sessionId) override {
        std::lock_guard<std::mutex> lock(mutex_);
        const std::string id = sendId.empty() ? "virtual_send_" + std::to_string(sequence_) : sendId;
        pending_.erase(std::remove_if(pending_.begin(), pending_.end(), [&](const Entry &e) { return e.sendId == id; }),
                       pending_.end());
        pending_.push_back({event, now_ + delay, sequence_++, std::move(target), id, sessionId});
        std::promise<std::string> assigned;
        assigned.set_value(id);
        return assigned.get_future();
    }

    bool cancelEvent(const std::string &sendId, const std::string &sessionId) override {
        std::lock_guard<std::mutex> lock(mutex_);
        return eraseWhere([&](const Entry &e) {
                   return e.sendId == sendId && (sessionId.empty() || e.sessionId == sessionId);
               }) > 0;
    }

    size_t cancelEventsForSession(const std::string &sessionId) override {
        std::lock_guard<std::mutex> lock(mutex_);
        return eraseWhere([&](const Entry &e) { return e.sessionId == sessionId; });
    }

    bool hasEvent(const std::string &sendId) const override {
        std::lock_guard<std::mutex> lock(mutex_);
        return std::any_of(pending_.begin(), pending_.end(), [&](const Entry &e) { return e.sendId == sendId; });
    }

    size_t getScheduledEventCount() const override {
        std::lock_guard<std::mutex> lock(mutex_);
        return pending_.size();
    }

    void shutdown(bool) override {
        std::lock_guard<std::mutex> lock(mutex_);
        pending_.clear();
        running_ = false;
    }

    bool isRunning() const override {
        std::lock_guard<std::mutex> lock(mutex_);
        return running_;
    }

    /// For the interactive debugger's view of pending sends, which no session of this test reads.
    std::vector<ScheduledEventInfo> getScheduledEvents() const override {
        return {};
    }

    void setMode(SchedulerMode) override {}

    SchedulerMode getMode() const override {
        return SchedulerMode::AUTOMATIC;
    }

    size_t forcePoll() override {
        std::vector<Entry> due;
        {
            std::lock_guard<std::mutex> lock(mutex_);
            if (pending_.empty()) {
                return 0;
            }
            now_ = std::min_element(pending_.begin(), pending_.end(), [](const Entry &a, const Entry &b) {
                       return a.dueAt < b.dueAt;
                   })->dueAt;
            for (auto it = pending_.begin(); it != pending_.end();) {
                if (it->dueAt <= now_) {
                    due.push_back(std::move(*it));
                    it = pending_.erase(it);
                } else {
                    ++it;
                }
            }
        }
        std::sort(due.begin(), due.end(), [](const Entry &a, const Entry &b) { return a.sequence < b.sequence; });
        for (const auto &entry : due) {
            deliver_(entry.event, entry.target, entry.sendId);
        }
        return due.size();
    }

private:
    struct Entry {
        EventDescriptor event;
        std::chrono::milliseconds dueAt;
        uint64_t sequence;
        std::shared_ptr<IEventTarget> target;
        std::string sendId;
        std::string sessionId;
    };

    template <typename Predicate> size_t eraseWhere(Predicate matches) {
        const size_t before = pending_.size();
        pending_.erase(std::remove_if(pending_.begin(), pending_.end(), matches), pending_.end());
        return before - pending_.size();
    }

    EventExecutionCallback deliver_;
    mutable std::mutex mutex_;
    std::vector<Entry> pending_;
    std::chrono::milliseconds now_{0};
    uint64_t sequence_ = 0;
    bool running_ = true;
};

}  // namespace

class ADelayedSendReachesWhatItsTargetNamesTest : public ::testing::Test {
protected:
    /// What the children's threads get between two steps of the clock. Which event is next
    /// is the clock's to say; the pause only lets a child report to its parent.
    static constexpr std::chrono::milliseconds kSettle{20};

    void SetUp() override {
        engine_ = &ScriptEngineProvider::getScriptEngine();
        engine_->reset();
    }

    void TearDown() override {
        sm_.reset();
        if (engine_) {
            engine_->shutdown();
        }
    }

    /// A value the handlers recorded (W3C SCXML 5.3), for the failure message.
    std::string read(const std::string &name) const {
        auto result = ScriptEngineProvider::getScriptEngine().evaluateExpression(sm_->getSessionId(), name).get();
        return result.isSuccess() ? result.getValueAsString() : std::string("<unreadable>");
    }

    IScriptEngine *engine_ = nullptr;
    std::shared_ptr<StateMachine> sm_;
};

TEST_F(ADelayedSendReachesWhatItsTargetNamesTest, ADelayedSendReachesWhatItsTargetNames) {
    const std::string fixture = std::string(SCE_PROJECT_ROOT) +
                                "/integration_resources/a_delayed_send_reaches_what_its_target_names/"
                                "a_delayed_send_reaches_what_its_target_names.scxml";

    sm_ = std::make_shared<StateMachine>(ScriptEngineProvider::getScriptEngine());
    auto scheduler = std::make_shared<VirtualClockScheduler>(
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
    sm_->setEventRaiser(eventRaiser);
    sm_->setEventDispatcher(
        std::make_shared<EventDispatcherImpl>(scheduler, std::make_shared<EventTargetFactoryImpl>(eventRaiser)));

    ASSERT_TRUE(sm_->loadSCXML(fixture)) << "canonical fixture not loadable: " << fixture;
    ASSERT_TRUE(sm_->start());
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::seconds(5);
    while (std::chrono::steady_clock::now() < deadline && sm_->isRunning()) {
        std::this_thread::sleep_for(kSettle);
        eventRaiser->processQueuedEvents();
        scheduler->forcePoll();
        eventRaiser->processQueuedEvents();
    }

    EXPECT_EQ(sm_->terminalState().value_or(""), "done") << "the run must end in `done`";
    const std::vector<std::pair<std::string, int>> expected = {
        {"order", 31}, {"innerInternal", 1}, {"lateOk", 1},      {"lateCount", 1},
        {"pongOk", 1}, {"commErrors", 2},    {"lostArrived", 0}, {"afterStranger", 0}};
    for (const auto &[name, value] : expected) {
        EXPECT_EQ(read(name + " === " + std::to_string(value)), "true")
            << name << " = " << read(name) << ", want " << value;
    }
}

}  // namespace Tests
}  // namespace SCE
