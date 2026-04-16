// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// GameLoopTimer and LogicalTimeScheduler API Tests

#include "wrappers/GameLoopTimer.h"
#include "common/LogicalTimeScheduler.h"
#include <chrono>
#include <gtest/gtest.h>

using namespace std::chrono_literals;

namespace SCE::Tests {

// ============================================================================
// Test Event Enum
// ============================================================================

enum class TestEvent : uint8_t { None = 0, Timeout, Heartbeat, Cooldown, Animation };

// ============================================================================
// LogicalTimeScheduler Tests
// ============================================================================

class LogicalTimeSchedulerTest : public ::testing::Test {
protected:
    SCE::Common::LogicalTimeScheduler<TestEvent> scheduler_;
};

TEST_F(LogicalTimeSchedulerTest, InitialState) {
    EXPECT_FALSE(scheduler_.hasPendingEvents());
    EXPECT_EQ(scheduler_.getPendingCount(), 0);
}

TEST_F(LogicalTimeSchedulerTest, ScheduleEventAt) {
    auto sendId = scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);

    EXPECT_FALSE(sendId.empty());
    EXPECT_TRUE(scheduler_.hasPendingEvents());
    EXPECT_EQ(scheduler_.getPendingCount(), 1);
}

TEST_F(LogicalTimeSchedulerTest, ScheduleEventWithDelay) {
    double currentTime = 50.0;
    auto sendId = scheduler_.scheduleEvent(TestEvent::Timeout, currentTime, 100ms);

    EXPECT_FALSE(sendId.empty());
    EXPECT_TRUE(scheduler_.hasPendingEvents());

    // Event should fire at 50 + 100 = 150ms
    EXPECT_FALSE(scheduler_.hasReadyEvents(100.0));
    EXPECT_FALSE(scheduler_.hasReadyEvents(149.9));
    EXPECT_TRUE(scheduler_.hasReadyEvents(150.0));
    EXPECT_TRUE(scheduler_.hasReadyEvents(200.0));
}

TEST_F(LogicalTimeSchedulerTest, PopReadyEvent) {
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);

    TestEvent event;
    std::string eventData;

    // Not ready yet
    EXPECT_FALSE(scheduler_.popReadyEvent(50.0, event, eventData));

    // Ready now
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event, eventData));
    EXPECT_EQ(event, TestEvent::Timeout);

    // Queue is now empty
    EXPECT_FALSE(scheduler_.hasPendingEvents());
    EXPECT_FALSE(scheduler_.popReadyEvent(200.0, event, eventData));
}

TEST_F(LogicalTimeSchedulerTest, EventOrdering) {
    // Schedule events in reverse order
    scheduler_.scheduleEventAt(TestEvent::Animation, 300.0);
    scheduler_.scheduleEventAt(TestEvent::Cooldown, 200.0);
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);

    EXPECT_EQ(scheduler_.getPendingCount(), 3);

    TestEvent event;
    std::string eventData;

    // Pop in time order
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event, eventData));
    EXPECT_EQ(event, TestEvent::Timeout);

    EXPECT_TRUE(scheduler_.popReadyEvent(200.0, event, eventData));
    EXPECT_EQ(event, TestEvent::Cooldown);

    EXPECT_TRUE(scheduler_.popReadyEvent(300.0, event, eventData));
    EXPECT_EQ(event, TestEvent::Animation);

    EXPECT_FALSE(scheduler_.hasPendingEvents());
}

TEST_F(LogicalTimeSchedulerTest, CancelEvent) {
    auto sendId = scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);

    EXPECT_TRUE(scheduler_.hasPendingEvents());

    // Cancel event
    EXPECT_TRUE(scheduler_.cancelEvent(sendId));
    EXPECT_TRUE(scheduler_.isCancelled(sendId));

    // Event should be skipped when popping
    TestEvent event;
    std::string eventData;
    EXPECT_FALSE(scheduler_.popReadyEvent(100.0, event, eventData));
}

TEST_F(LogicalTimeSchedulerTest, CancelEmptySendId) {
    EXPECT_FALSE(scheduler_.cancelEvent(""));
}

TEST_F(LogicalTimeSchedulerTest, CustomSendId) {
    auto sendId = scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0, "custom_id");

    EXPECT_EQ(sendId, "custom_id");

    // Cancel with custom ID
    EXPECT_TRUE(scheduler_.cancelEvent("custom_id"));

    TestEvent event;
    std::string eventData;
    EXPECT_FALSE(scheduler_.popReadyEvent(100.0, event, eventData));
}

TEST_F(LogicalTimeSchedulerTest, EventData) {
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0, "", "test_data");

    TestEvent event;
    std::string eventData;
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event, eventData));
    EXPECT_EQ(event, TestEvent::Timeout);
    EXPECT_EQ(eventData, "test_data");
}

TEST_F(LogicalTimeSchedulerTest, Clear) {
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);
    scheduler_.scheduleEventAt(TestEvent::Heartbeat, 200.0);

    EXPECT_EQ(scheduler_.getPendingCount(), 2);

    scheduler_.clear();

    EXPECT_EQ(scheduler_.getPendingCount(), 0);
    EXPECT_FALSE(scheduler_.hasPendingEvents());
}

TEST_F(LogicalTimeSchedulerTest, MultipleEventsAtSameTime) {
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);
    scheduler_.scheduleEventAt(TestEvent::Heartbeat, 100.0);

    EXPECT_EQ(scheduler_.getPendingCount(), 2);

    TestEvent event;
    std::string eventData;

    // Both should be ready at time 100
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event, eventData));
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event, eventData));
    EXPECT_FALSE(scheduler_.popReadyEvent(100.0, event, eventData));
}

TEST_F(LogicalTimeSchedulerTest, PopReadyEventSimple) {
    scheduler_.scheduleEventAt(TestEvent::Timeout, 100.0);

    TestEvent event;
    // Use simpler overload without eventData
    EXPECT_FALSE(scheduler_.popReadyEvent(50.0, event));
    EXPECT_TRUE(scheduler_.popReadyEvent(100.0, event));
    EXPECT_EQ(event, TestEvent::Timeout);
}

// ============================================================================
// Mock State Machine for GameLoopTimer Tests
// ============================================================================

/**
 * @brief Minimal mock state machine for testing GameLoopTimer
 *
 * Tracks raiseExternal and step calls for verification.
 */
class MockStateMachine {
public:
    using Event = TestEvent;

    void raiseExternal(Event event) {
        raisedEvents_.push_back({event, ""});
    }

    void raiseExternal(Event event, const std::string &data) {
        raisedEvents_.push_back({event, data});
    }

    void step() {
        ++stepCount_;
    }

    // Test helpers
    size_t getStepCount() const {
        return stepCount_;
    }

    const std::vector<std::pair<Event, std::string>> &getRaisedEvents() const {
        return raisedEvents_;
    }

    void clearRaisedEvents() {
        raisedEvents_.clear();
        stepCount_ = 0;
    }

private:
    std::vector<std::pair<Event, std::string>> raisedEvents_;
    size_t stepCount_ = 0;
};

// ============================================================================
// GameLoopTimer Tests
// ============================================================================

class GameLoopTimerTest : public ::testing::Test {
protected:
    void SetUp() override {
        sm_ = std::make_unique<MockStateMachine>();
        // Default tic rate: 35 Hz (DOOM)
        timer_ = std::make_unique<SCE::Wrappers::GameLoopTimer<MockStateMachine, 35>>(*sm_);
    }

    std::unique_ptr<MockStateMachine> sm_;
    std::unique_ptr<SCE::Wrappers::GameLoopTimer<MockStateMachine, 35>> timer_;
};

TEST_F(GameLoopTimerTest, InitialState) {
    EXPECT_EQ(timer_->getCurrentTic(), 0);
    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), 0.0);
    EXPECT_FALSE(timer_->hasPendingEvents());
    EXPECT_EQ(timer_->getPendingCount(), 0);
}

TEST_F(GameLoopTimerTest, ProcessTicAdvancesTime) {
    // At 35 Hz: 1000ms / 35 = 28.571428... ms per tic
    constexpr double MS_PER_TIC = 1000.0 / 35.0;

    timer_->processTic();
    EXPECT_EQ(timer_->getCurrentTic(), 1);
    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), MS_PER_TIC);

    timer_->processTic();
    EXPECT_EQ(timer_->getCurrentTic(), 2);
    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), 2 * MS_PER_TIC);
}

TEST_F(GameLoopTimerTest, ScheduleByTics) {
    // Schedule event for 10 tics from now (10 * 28.57ms = 285.7ms)
    auto sendId = timer_->scheduleByTics(TestEvent::Cooldown, 10);

    EXPECT_FALSE(sendId.empty());
    EXPECT_TRUE(timer_->hasPendingEvents());

    // Process 9 tics - event should not fire yet
    for (int i = 0; i < 9; ++i) {
        timer_->processTic();
    }
    EXPECT_TRUE(sm_->getRaisedEvents().empty());

    // Process 10th tic - event should fire
    timer_->processTic();
    ASSERT_EQ(sm_->getRaisedEvents().size(), 1);
    EXPECT_EQ(sm_->getRaisedEvents()[0].first, TestEvent::Cooldown);
    EXPECT_EQ(sm_->getStepCount(), 1);  // step() should be called
}

TEST_F(GameLoopTimerTest, ScheduleByMs) {
    // Schedule event for 500ms from now
    auto sendId = timer_->scheduleByMs(TestEvent::Timeout, 500ms);

    EXPECT_FALSE(sendId.empty());
    EXPECT_TRUE(timer_->hasPendingEvents());

    // At 35 Hz: 500ms / 28.57ms = ~17.5 tics needed
    // Process 17 tics (~485.7ms)
    for (int i = 0; i < 17; ++i) {
        timer_->processTic();
    }
    EXPECT_TRUE(sm_->getRaisedEvents().empty());

    // Process 18th tic (~514.3ms) - event should fire
    timer_->processTic();
    ASSERT_EQ(sm_->getRaisedEvents().size(), 1);
    EXPECT_EQ(sm_->getRaisedEvents()[0].first, TestEvent::Timeout);
}

TEST_F(GameLoopTimerTest, CancelScheduledEvent) {
    auto sendId = timer_->scheduleByTics(TestEvent::Timeout, 5);

    // Cancel immediately
    EXPECT_TRUE(timer_->cancel(sendId));

    // Process 10 tics - event should never fire
    for (int i = 0; i < 10; ++i) {
        timer_->processTic();
    }
    EXPECT_TRUE(sm_->getRaisedEvents().empty());
}

TEST_F(GameLoopTimerTest, ProcessTimeVariableTimestep) {
    // Schedule event for 100ms
    timer_->scheduleByMs(TestEvent::Timeout, 100ms);

    // Use variable timestep (like requestAnimationFrame)
    timer_->processTime(40.0);  // 40ms
    EXPECT_TRUE(sm_->getRaisedEvents().empty());

    timer_->processTime(40.0);  // 80ms total
    EXPECT_TRUE(sm_->getRaisedEvents().empty());

    timer_->processTime(40.0);  // 120ms total - event should fire
    ASSERT_EQ(sm_->getRaisedEvents().size(), 1);
    EXPECT_EQ(sm_->getRaisedEvents()[0].first, TestEvent::Timeout);
}

TEST_F(GameLoopTimerTest, ProcessTimeZeroOrNegative) {
    timer_->scheduleByMs(TestEvent::Timeout, 100ms);

    // Zero and negative deltas should be ignored
    EXPECT_EQ(timer_->processTime(0.0), 0);
    EXPECT_EQ(timer_->processTime(-10.0), 0);

    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), 0.0);
}

TEST_F(GameLoopTimerTest, EventWithData) {
    timer_->scheduleByTics(TestEvent::Timeout, 1, "", "custom_data");

    timer_->processTic();

    ASSERT_EQ(sm_->getRaisedEvents().size(), 1);
    EXPECT_EQ(sm_->getRaisedEvents()[0].first, TestEvent::Timeout);
    EXPECT_EQ(sm_->getRaisedEvents()[0].second, "custom_data");
}

TEST_F(GameLoopTimerTest, CustomSendId) {
    auto sendId = timer_->scheduleByTics(TestEvent::Timeout, 5, "my_custom_id");

    EXPECT_EQ(sendId, "my_custom_id");

    // Cancel using custom ID
    EXPECT_TRUE(timer_->cancel("my_custom_id"));
}

TEST_F(GameLoopTimerTest, Reset) {
    // Schedule events and advance time
    timer_->scheduleByTics(TestEvent::Timeout, 5);
    timer_->processTic();
    timer_->processTic();

    EXPECT_EQ(timer_->getCurrentTic(), 2);
    EXPECT_TRUE(timer_->hasPendingEvents());

    // Reset
    timer_->reset();

    EXPECT_EQ(timer_->getCurrentTic(), 0);
    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), 0.0);
    EXPECT_FALSE(timer_->hasPendingEvents());
}

TEST_F(GameLoopTimerTest, SetLogicalTime) {
    timer_->setLogicalTime(1000.0);  // 1 second

    EXPECT_DOUBLE_EQ(timer_->getLogicalTimeMs(), 1000.0);
    // Tic count should be updated based on MS_PER_TIC (1000ms / 28.57ms = 35 tics)
    EXPECT_EQ(timer_->getCurrentTic(), 35);
}

TEST_F(GameLoopTimerTest, MultipleEventsInOneTic) {
    // Schedule multiple events at same time
    timer_->scheduleByTics(TestEvent::Timeout, 1);
    timer_->scheduleByTics(TestEvent::Heartbeat, 1);

    timer_->processTic();

    // Both events should fire
    ASSERT_EQ(sm_->getRaisedEvents().size(), 2);
    // step() called once after all events fired
    EXPECT_EQ(sm_->getStepCount(), 1);
}

TEST_F(GameLoopTimerTest, GamePauseBehavior) {
    // Schedule event for 5 tics
    timer_->scheduleByTics(TestEvent::Timeout, 5);

    // Process 3 tics
    for (int i = 0; i < 3; ++i) {
        timer_->processTic();
    }

    // "Pause" game - don't call processTic for a while
    // (In real code, this would be actual game pause)

    // Event should NOT have fired yet
    EXPECT_TRUE(sm_->getRaisedEvents().empty());

    // Resume - process remaining 2 tics
    timer_->processTic();
    timer_->processTic();

    // Now event should fire (5 total tics)
    ASSERT_EQ(sm_->getRaisedEvents().size(), 1);
}

TEST_F(GameLoopTimerTest, GetSchedulerAccess) {
    // Verify we can access the underlying scheduler
    auto &scheduler = timer_->getScheduler();
    EXPECT_EQ(scheduler.getPendingCount(), 0);

    timer_->scheduleByTics(TestEvent::Timeout, 5);
    EXPECT_EQ(scheduler.getPendingCount(), 1);
}

TEST_F(GameLoopTimerTest, ConstSchedulerAccess) {
    const auto &constTimer = *timer_;
    const auto &scheduler = constTimer.getScheduler();
    EXPECT_EQ(scheduler.getPendingCount(), 0);
}

// ============================================================================
// Different Tic Rates
// ============================================================================

TEST(GameLoopTimerTicRateTest, DifferentTicRates) {
    MockStateMachine sm;

    // 60 Hz (typical game)
    SCE::Wrappers::GameLoopTimer<MockStateMachine, 60> timer60(sm);
    EXPECT_DOUBLE_EQ(timer60.MS_PER_TIC, 1000.0 / 60.0);

    // 30 Hz
    SCE::Wrappers::GameLoopTimer<MockStateMachine, 30> timer30(sm);
    EXPECT_DOUBLE_EQ(timer30.MS_PER_TIC, 1000.0 / 30.0);

    // 120 Hz (high refresh)
    SCE::Wrappers::GameLoopTimer<MockStateMachine, 120> timer120(sm);
    EXPECT_DOUBLE_EQ(timer120.MS_PER_TIC, 1000.0 / 120.0);
}

TEST(GameLoopTimerTicRateTest, TicRateAffectsScheduling) {
    MockStateMachine sm60, sm30;

    SCE::Wrappers::GameLoopTimer<MockStateMachine, 60> timer60(sm60);
    SCE::Wrappers::GameLoopTimer<MockStateMachine, 30> timer30(sm30);

    // Schedule event for 100ms on both
    timer60.scheduleByMs(TestEvent::Timeout, 100ms);
    timer30.scheduleByMs(TestEvent::Timeout, 100ms);

    // At 60 Hz: 100ms needs ~6 tics
    // At 30 Hz: 100ms needs ~3 tics
    for (int i = 0; i < 6; ++i) {
        timer60.processTic();
        timer30.processTic();
    }

    // 60 Hz should fire after ~6 tics (100ms)
    EXPECT_EQ(sm60.getRaisedEvents().size(), 1);
    // 30 Hz should have fired after ~3 tics (100ms) - already done
    EXPECT_EQ(sm30.getRaisedEvents().size(), 1);
}

}  // namespace SCE::Tests
