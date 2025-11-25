// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// TimerManager API Tests

#include "wrappers/TimerManager.h"
#include "timer_test_sm.h"
#include <chrono>
#include <gtest/gtest.h>
#include <thread>

using namespace std::chrono_literals;
using namespace SCE::Generated::timer_test;

/**
 * @brief Timer identifiers for test state machine
 */
enum class TimerID : uint8_t {
    HEARTBEAT,  // Periodic timer
    TIMEOUT,    // One-shot timer
    UNUSED      // For error testing
};

/**
 * @brief Extend generated SM with TimerID for TimerManager compatibility
 */
struct TimerTestSM : public timer_test {
    using TimerID = ::TimerID;
    using timer_test::timer_test;
};

/**
 * @brief Test fixture for TimerManager API tests
 *
 * Tests the high-level timer API built on W3C SCXML 6.2 delayed send.
 */
class TimerManagerTest : public ::testing::Test {
protected:
    void SetUp() override {
        sm_ = std::make_unique<TimerTestSM>();
        timers_ = std::make_unique<SCE::Wrappers::TimerManager<TimerTestSM>>(*sm_);

        // Initialize state machine
        sm_->initialize();

        // Start in running state for timer tests
        sm_->raiseExternal(Event::Start);
        sm_->tick();

        ASSERT_EQ(sm_->getCurrentState(), State::Running);
    }

    void TearDown() override {
        // Stop all timers before cleanup
        if (timers_) {
            timers_->stopAllTimers();
        }
        timers_.reset();
        sm_.reset();
    }

    std::unique_ptr<TimerTestSM> sm_;
    std::unique_ptr<SCE::Wrappers::TimerManager<TimerTestSM>> timers_;
};

// ============================================================================
// Basic Timer Operations
// ============================================================================

/**
 * @brief Test timer registration
 */
TEST_F(TimerManagerTest, RegisterTimer) {
    // Should not throw
    EXPECT_NO_THROW(timers_->registerTimer(TimerID::HEARTBEAT, Event::Heartbeat));
    EXPECT_NO_THROW(timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout));
}

/**
 * @brief Test starting unregistered timer throws error
 */
TEST_F(TimerManagerTest, StartUnregisteredTimerThrows) {
    // Should throw runtime_error
    EXPECT_THROW(timers_->startTimer(TimerID::HEARTBEAT, 100ms, false), std::runtime_error);
}

/**
 * @brief Test one-shot timer start
 */
TEST_F(TimerManagerTest, StartOneShotTimer) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);

    // Start one-shot timer
    EXPECT_NO_THROW(timers_->startTimer(TimerID::TIMEOUT, 100ms, false));

    // Timer should be running
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));
    EXPECT_EQ(timers_->getActiveTimerCount(), 1);

    // Verify timer properties
    auto interval = timers_->getTimerInterval(TimerID::TIMEOUT);
    ASSERT_TRUE(interval.has_value());
    EXPECT_EQ(*interval, 100ms);
    EXPECT_FALSE(timers_->isTimerPeriodic(TimerID::TIMEOUT));
}

/**
 * @brief Test periodic timer start
 */
TEST_F(TimerManagerTest, StartPeriodicTimer) {
    timers_->registerTimer(TimerID::HEARTBEAT, Event::Heartbeat);

    // Start periodic timer
    EXPECT_NO_THROW(timers_->startTimer(TimerID::HEARTBEAT, 50ms, true));

    // Timer should be running
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::HEARTBEAT));
    EXPECT_EQ(timers_->getActiveTimerCount(), 1);

    // Verify timer properties
    auto interval = timers_->getTimerInterval(TimerID::HEARTBEAT);
    ASSERT_TRUE(interval.has_value());
    EXPECT_EQ(*interval, 50ms);
    EXPECT_TRUE(timers_->isTimerPeriodic(TimerID::HEARTBEAT));
}

/**
 * @brief Test stopping timer
 */
TEST_F(TimerManagerTest, StopTimer) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);

    ASSERT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Stop timer
    EXPECT_TRUE(timers_->stopTimer(TimerID::TIMEOUT));
    EXPECT_FALSE(timers_->isTimerRunning(TimerID::TIMEOUT));
    EXPECT_EQ(timers_->getActiveTimerCount(), 0);

    // Stopping again should return false
    EXPECT_FALSE(timers_->stopTimer(TimerID::TIMEOUT));
}

/**
 * @brief Test restarting timer
 */
TEST_F(TimerManagerTest, RestartTimer) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);

    ASSERT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Restart timer
    EXPECT_TRUE(timers_->restartTimer(TimerID::TIMEOUT));
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Verify properties preserved
    auto interval = timers_->getTimerInterval(TimerID::TIMEOUT);
    ASSERT_TRUE(interval.has_value());
    EXPECT_EQ(*interval, 100ms);
}

/**
 * @brief Test restarting non-existent timer
 */
TEST_F(TimerManagerTest, RestartNonExistentTimer) {
    // Should return false
    EXPECT_FALSE(timers_->restartTimer(TimerID::UNUSED));
}

/**
 * @brief Test stopping all timers
 */
TEST_F(TimerManagerTest, StopAllTimers) {
    timers_->registerTimer(TimerID::HEARTBEAT, Event::Heartbeat);
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);

    timers_->startTimer(TimerID::HEARTBEAT, 50ms, true);
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);

    ASSERT_EQ(timers_->getActiveTimerCount(), 2);

    // Stop all
    timers_->stopAllTimers();

    EXPECT_EQ(timers_->getActiveTimerCount(), 0);
    EXPECT_FALSE(timers_->isTimerRunning(TimerID::HEARTBEAT));
    EXPECT_FALSE(timers_->isTimerRunning(TimerID::TIMEOUT));
}

// ============================================================================
// Timer Expiration and Event Delivery
// ============================================================================

/**
 * @brief Test one-shot timer fires and delivers event
 */
TEST_F(TimerManagerTest, OneShotTimerFires) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);

    // State should still be Running
    EXPECT_EQ(sm_->getCurrentState(), State::Running);

    // Wait for timer to expire
    std::this_thread::sleep_for(150ms);

    // Process event
    sm_->tick();

    // Should transition to Timeout_state
    EXPECT_EQ(sm_->getCurrentState(), State::Timeout_state);
}

/**
 * @brief Test periodic timer fires multiple times
 */
TEST_F(TimerManagerTest, PeriodicTimerFiresMultipleTimes) {
    timers_->registerTimer(TimerID::HEARTBEAT, Event::Heartbeat);
    timers_->startTimer(TimerID::HEARTBEAT, 50ms, true);

    int tickCount = 0;
    auto startTime = std::chrono::steady_clock::now();
    auto endTime = startTime + 250ms;  // Run for 250ms

    while (std::chrono::steady_clock::now() < endTime) {
        if (sm_->hasReadyEvents()) {
            auto beforeState = sm_->getCurrentState();
            sm_->tick();
            timers_->processExpiredTimers();  // Re-schedule periodic timers
            auto afterState = sm_->getCurrentState();

            // Heartbeat is targetless transition (stays in Running)
            if (beforeState == State::Running && afterState == State::Running) {
                ++tickCount;
            }
        }
        std::this_thread::sleep_for(10ms);
    }

    // Should have fired at least 3-4 times (250ms / 50ms = 5 theoretical)
    // Allow some tolerance for timing variations
    EXPECT_GE(tickCount, 3);
    EXPECT_LE(tickCount, 6);
}

/**
 * @brief Test processExpiredTimers re-schedules periodic timer
 */
TEST_F(TimerManagerTest, ProcessExpiredTimersReschedulesPeriodicTimer) {
    timers_->registerTimer(TimerID::HEARTBEAT, Event::Heartbeat);
    timers_->startTimer(TimerID::HEARTBEAT, 50ms, true);

    // Wait for first expiration
    std::this_thread::sleep_for(60ms);
    sm_->tick();

    // Timer should still be running (periodic)
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::HEARTBEAT));

    // Process expired timers to re-schedule
    timers_->processExpiredTimers();

    // Timer still running
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::HEARTBEAT));

    // Should fire again after another interval
    std::this_thread::sleep_for(60ms);

    bool hasEvent = sm_->hasReadyEvents();
    EXPECT_TRUE(hasEvent);
}

/**
 * @brief Test stopping timer prevents state transition
 */
TEST_F(TimerManagerTest, StoppedTimerDoesNotFire) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);
    timers_->startTimer(TimerID::TIMEOUT, 200ms, false);

    // Stop immediately
    bool stopped = timers_->stopTimer(TimerID::TIMEOUT);
    EXPECT_TRUE(stopped);
    EXPECT_FALSE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Clear any ready events
    while (sm_->hasReadyEvents()) {
        sm_->tick();
    }

    // Wait longer than timer interval
    std::this_thread::sleep_for(250ms);

    // Process any events
    while (sm_->hasReadyEvents()) {
        sm_->tick();
    }

    // State should still be Running (not transitioned to Timeout_state)
    EXPECT_EQ(sm_->getCurrentState(), State::Running);
}

// ============================================================================
// Edge Cases
// ============================================================================

/**
 * @brief Test getting interval of non-existent timer
 */
TEST_F(TimerManagerTest, GetIntervalOfNonExistentTimer) {
    auto interval = timers_->getTimerInterval(TimerID::UNUSED);
    EXPECT_FALSE(interval.has_value());
}

/**
 * @brief Test checking if non-existent timer is periodic
 */
TEST_F(TimerManagerTest, IsNonExistentTimerPeriodic) {
    // Should return false
    EXPECT_FALSE(timers_->isTimerPeriodic(TimerID::UNUSED));
}

/**
 * @brief Test timer lifecycle: start → stop → start again
 */
TEST_F(TimerManagerTest, TimerLifecycle) {
    timers_->registerTimer(TimerID::TIMEOUT, Event::Timeout);

    // Start
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Stop
    timers_->stopTimer(TimerID::TIMEOUT);
    EXPECT_FALSE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Start again (not restart - stopped timer was removed from activeTimers)
    timers_->startTimer(TimerID::TIMEOUT, 100ms, false);
    EXPECT_TRUE(timers_->isTimerRunning(TimerID::TIMEOUT));

    // Verify still works after re-starting
    std::this_thread::sleep_for(150ms);
    sm_->tick();
    EXPECT_EQ(sm_->getCurrentState(), State::Timeout_state);
}
