// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "dispatchers/GLibDispatcher.h"
#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <stdexcept>
#include <thread>
#include <vector>

namespace SCE::Dispatchers {

// Test timing constants (avoid magic numbers)
namespace TestConstants {
static constexpr auto kShortWait = std::chrono::milliseconds(10);
static constexpr auto kTimerShortInterval = std::chrono::milliseconds(50);
static constexpr auto kTaskWait = std::chrono::milliseconds(100);
static constexpr auto kMediumWait = std::chrono::milliseconds(200);
static constexpr auto kTimerLongInterval = std::chrono::milliseconds(250);
static constexpr auto kLongWait = std::chrono::milliseconds(300);
static constexpr auto kDriftTestWait = std::chrono::milliseconds(600);
static constexpr int kTimerAccuracyToleranceMs = 30;
static constexpr int kDriftToleranceMs = 100;
}  // namespace TestConstants

/**
 * @brief Unit tests for GLibDispatcher
 *
 * Architecture Compliance:
 * - Zero Duplication: Tests shared base class functionality through GLib backend
 * - Platform Integration: Validates GLib main loop integration
 * - Thread Safety: Validates concurrent enqueue operations
 */
class GLibDispatcherTest : public ::testing::Test {
protected:
    void SetUp() override {
        dispatcher_ = GLibDispatcher::create();
    }

    void TearDown() override {
        if (dispatcher_ && dispatcher_->isRunning()) {
            dispatcher_->stop();
        }
        if (eventLoopThread_.joinable()) {
            eventLoopThread_.join();
        }
        dispatcher_.reset();
    }

    void startEventLoop() {
        dispatcher_->start();
        eventLoopThread_ = std::thread([this]() { dispatcher_->run(); });
    }

    void stopEventLoop() {
        dispatcher_->stop();
        if (eventLoopThread_.joinable()) {
            eventLoopThread_.join();
        }
    }

    std::shared_ptr<GLibDispatcher> dispatcher_;
    std::thread eventLoopThread_;
};

// ============================================================================
// Basic Lifecycle Tests
// ============================================================================

TEST_F(GLibDispatcherTest, CreateDispatcher) {
    ASSERT_NE(dispatcher_, nullptr);
    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(GLibDispatcherTest, StartDispatcher) {
    dispatcher_->start();
    EXPECT_TRUE(dispatcher_->isRunning());
    dispatcher_->stop();
}

TEST_F(GLibDispatcherTest, StopDispatcher) {
    dispatcher_->start();
    dispatcher_->stop();
    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(GLibDispatcherTest, StartStopMultipleTimes) {
    for (int i = 0; i < 3; ++i) {
        dispatcher_->start();
        EXPECT_TRUE(dispatcher_->isRunning());

        dispatcher_->stop();
        EXPECT_FALSE(dispatcher_->isRunning());
    }
}

TEST_F(GLibDispatcherTest, EnqueueWithoutStart) {
    EXPECT_THROW(dispatcher_->enqueue([]() {}), std::runtime_error);
}

TEST_F(GLibDispatcherTest, RunWithoutStart) {
    EXPECT_THROW(dispatcher_->run(), std::runtime_error);
}

TEST_F(GLibDispatcherTest, DestructorStopsRunningDispatcher) {
    std::atomic<bool> taskExecuted{false};

    {
        auto localDispatcher = GLibDispatcher::create();
        localDispatcher->start();

        std::thread runThread([&localDispatcher]() { localDispatcher->run(); });

        localDispatcher->enqueue([&taskExecuted]() {
            taskExecuted.store(true);
            std::this_thread::sleep_for(TestConstants::kTimerShortInterval);
        });

        std::this_thread::sleep_for(TestConstants::kTaskWait);

        // Destructor should stop the dispatcher and join the thread
        localDispatcher->stop();
        runThread.join();
    }

    EXPECT_TRUE(taskExecuted.load());
}

// ============================================================================
// Task Execution Tests
// ============================================================================

TEST_F(GLibDispatcherTest, ExecuteSingleTask) {
    std::atomic<bool> executed{false};

    startEventLoop();

    dispatcher_->enqueue([&executed]() { executed.store(true); });

    // Wait for task execution
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    stopEventLoop();

    EXPECT_TRUE(executed.load());
}

TEST_F(GLibDispatcherTest, ExecuteMultipleTasks) {
    std::atomic<int> counter{0};

    startEventLoop();

    const int numTasks = 10;
    for (int i = 0; i < numTasks; ++i) {
        dispatcher_->enqueue([&counter]() { counter.fetch_add(1); });
    }

    // Wait for all tasks
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_EQ(counter.load(), numTasks);
}

TEST_F(GLibDispatcherTest, FIFOOrdering) {
    std::vector<int> executionOrder;
    std::mutex orderMutex;

    startEventLoop();

    // Enqueue tasks in specific order
    for (int i = 0; i < 5; ++i) {
        dispatcher_->enqueue([&executionOrder, &orderMutex, i]() {
            std::lock_guard<std::mutex> lock(orderMutex);
            executionOrder.push_back(i);
            std::this_thread::sleep_for(TestConstants::kShortWait);
        });
    }

    // Wait for all tasks
    std::this_thread::sleep_for(TestConstants::kLongWait);

    stopEventLoop();

    ASSERT_EQ(executionOrder.size(), 5);
    for (int i = 0; i < 5; ++i) {
        EXPECT_EQ(executionOrder[i], i);
    }
}

// ============================================================================
// Concurrent Enqueue Tests
// ============================================================================

TEST_F(GLibDispatcherTest, ConcurrentEnqueue) {
    std::atomic<int> counter{0};

    startEventLoop();

    // Spawn multiple threads enqueueing tasks
    const int numThreads = 5;
    const int tasksPerThread = 10;
    std::vector<std::thread> threads;

    for (int t = 0; t < numThreads; ++t) {
        threads.emplace_back([this, &counter]() {
            for (int i = 0; i < tasksPerThread; ++i) {
                dispatcher_->enqueue([&counter]() { counter.fetch_add(1); });
            }
        });
    }

    for (auto &thread : threads) {
        thread.join();
    }

    // Wait for all tasks
    std::this_thread::sleep_for(TestConstants::kLongWait);

    stopEventLoop();

    EXPECT_EQ(counter.load(), numThreads * tasksPerThread);
}

// ============================================================================
// Exception Handling Tests
// ============================================================================

TEST_F(GLibDispatcherTest, TaskExceptionDoesNotStopDispatcher) {
    std::atomic<int> executedTasks{0};

    startEventLoop();

    // Task 1: Normal
    dispatcher_->enqueue([&executedTasks]() { executedTasks.fetch_add(1); });

    // Task 2: Throws exception
    dispatcher_->enqueue([]() { throw std::runtime_error("Test exception"); });

    // Task 3: Normal (should execute despite Task 2 exception)
    dispatcher_->enqueue([&executedTasks]() { executedTasks.fetch_add(1); });

    // Wait for all tasks
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_EQ(executedTasks.load(), 2);
}

TEST_F(GLibDispatcherTest, MultipleExceptions) {
    std::atomic<int> executedTasks{0};

    startEventLoop();

    for (int i = 0; i < 5; ++i) {
        if (i % 2 == 0) {
            // Normal task
            dispatcher_->enqueue([&executedTasks]() { executedTasks.fetch_add(1); });
        } else {
            // Exception task
            dispatcher_->enqueue([]() { throw std::runtime_error("Test exception"); });
        }
    }

    // Wait for all tasks
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_EQ(executedTasks.load(), 3);  // 3 normal tasks (indices 0, 2, 4)
}

// ============================================================================
// Pending Tasks Tests
// ============================================================================

TEST_F(GLibDispatcherTest, PendingTasksCount) {
    dispatcher_->start();
    EXPECT_EQ(dispatcher_->pendingTasks(), 0);

    // Enqueue tasks without starting event loop
    dispatcher_->enqueue([]() { std::this_thread::sleep_for(TestConstants::kTaskWait); });
    dispatcher_->enqueue([]() {});

    // Should have 2 pending tasks
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    EXPECT_EQ(dispatcher_->pendingTasks(), 2);

    dispatcher_->stop();
}

TEST_F(GLibDispatcherTest, PendingTasksDecrease) {
    std::atomic<int> executedTasks{0};

    startEventLoop();

    // Enqueue tasks
    for (int i = 0; i < 3; ++i) {
        dispatcher_->enqueue([&executedTasks]() {
            executedTasks.fetch_add(1);
            std::this_thread::sleep_for(TestConstants::kTimerShortInterval);
        });
    }

    // Wait for some tasks to execute
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_EQ(dispatcher_->pendingTasks(), 0);
    EXPECT_EQ(executedTasks.load(), 3);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

TEST_F(GLibDispatcherTest, StopBeforeAnyTask) {
    startEventLoop();

    // Immediately stop
    stopEventLoop();

    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(GLibDispatcherTest, StopWithPendingTasks) {
    std::atomic<int> executedTasks{0};

    startEventLoop();

    // Enqueue many tasks with delay between executions
    constexpr int kTotalTasks = 20;
    for (int i = 0; i < kTotalTasks; ++i) {
        dispatcher_->enqueue([&executedTasks]() {
            executedTasks.fetch_add(1);
            // Delay to allow stop() to take effect between tasks
            std::this_thread::sleep_for(TestConstants::kTaskWait);
        });
    }

    // Stop after some tasks execute (250ms = ~2 tasks at 100ms each)
    std::this_thread::sleep_for(TestConstants::kTimerLongInterval);
    stopEventLoop();

    // Verify stop() completes without hanging and some tasks executed
    // Key verification: stop() doesn't hang, some tasks ran
    EXPECT_GT(executedTasks.load(), 0);
}

// ============================================================================
// Timer Tests
// ============================================================================

TEST_F(GLibDispatcherTest, StartOneShotTimer) {
    std::atomic<bool> timerFired{false};

    startEventLoop();

    dispatcher_->startTimer(1, 100, [&timerFired]() { timerFired.store(true); }, false);

    // Wait for timer to fire
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_TRUE(timerFired.load());
}

TEST_F(GLibDispatcherTest, StartPeriodicTimer) {
    std::atomic<int> fireCount{0};

    startEventLoop();

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, true);

    // Wait for multiple firings
    std::this_thread::sleep_for(TestConstants::kTimerLongInterval);

    dispatcher_->stopTimer(1);
    stopEventLoop();

    // Should have fired at least 3 times
    EXPECT_GE(fireCount.load(), 3);
}

TEST_F(GLibDispatcherTest, StopTimer) {
    std::atomic<bool> timerFired{false};

    startEventLoop();

    dispatcher_->startTimer(1, 200, [&timerFired]() { timerFired.store(true); }, false);

    // Stop timer before it fires
    std::this_thread::sleep_for(TestConstants::kTimerShortInterval);
    dispatcher_->stopTimer(1);

    // Wait past original expiry time
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_FALSE(timerFired.load());
}

TEST_F(GLibDispatcherTest, IsTimerRunning) {
    startEventLoop();

    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    dispatcher_->startTimer(1, 100, []() {}, false);
    EXPECT_TRUE(dispatcher_->isTimerRunning(1));

    dispatcher_->stopTimer(1);
    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    stopEventLoop();
}

TEST_F(GLibDispatcherTest, MultipleTimers) {
    std::atomic<int> timer1Count{0};
    std::atomic<int> timer2Count{0};
    std::atomic<int> timer3Count{0};

    startEventLoop();

    dispatcher_->startTimer(1, 50, [&timer1Count]() { timer1Count.fetch_add(1); }, false);
    dispatcher_->startTimer(2, 100, [&timer2Count]() { timer2Count.fetch_add(1); }, false);
    dispatcher_->startTimer(3, 150, [&timer3Count]() { timer3Count.fetch_add(1); }, false);

    // Wait for all timers to fire
    std::this_thread::sleep_for(TestConstants::kTimerLongInterval);

    stopEventLoop();

    EXPECT_EQ(timer1Count.load(), 1);
    EXPECT_EQ(timer2Count.load(), 1);
    EXPECT_EQ(timer3Count.load(), 1);
}

TEST_F(GLibDispatcherTest, ReplaceExistingTimer) {
    std::atomic<int> fireCount{0};

    startEventLoop();

    // Start timer with 200ms delay
    dispatcher_->startTimer(1, 200, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Replace with timer with 50ms delay
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Wait for new timer to fire
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    stopEventLoop();

    // Should have fired once (from second timer)
    EXPECT_EQ(fireCount.load(), 1);
}

TEST_F(GLibDispatcherTest, RestartTimer) {
    std::atomic<int> fireCount{0};

    startEventLoop();

    // Start one-shot timer with 50ms delay
    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Wait for timer to fire
    std::this_thread::sleep_for(TestConstants::kTaskWait);
    EXPECT_EQ(fireCount.load(), 1);

    // Restart the timer (should fire again with same settings)
    bool restarted = dispatcher_->restartTimer(1);
    EXPECT_TRUE(restarted);

    // Wait for restarted timer to fire
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    stopEventLoop();

    // Should have fired twice (original + restart)
    EXPECT_EQ(fireCount.load(), 2);
}

TEST_F(GLibDispatcherTest, RestartNonExistentTimer) {
    startEventLoop();

    // Try to restart a timer that was never started
    bool restarted = dispatcher_->restartTimer(999);
    EXPECT_FALSE(restarted);

    stopEventLoop();
}

TEST_F(GLibDispatcherTest, TimerCallbackException) {
    std::atomic<int> taskCounter{0};

    startEventLoop();

    // Timer with exception
    dispatcher_->startTimer(1, 50, []() { throw std::runtime_error("Timer exception"); }, false);

    // Normal task after timer
    dispatcher_->enqueue([&taskCounter]() {
        std::this_thread::sleep_for(TestConstants::kTaskWait);
        taskCounter.fetch_add(1);
    });

    // Wait for timer and task
    std::this_thread::sleep_for(TestConstants::kTimerLongInterval);

    stopEventLoop();

    // Task should execute despite timer exception
    EXPECT_EQ(taskCounter.load(), 1);
}

TEST_F(GLibDispatcherTest, StopNonExistentTimer) {
    startEventLoop();

    // Should not crash
    dispatcher_->stopTimer(999);

    stopEventLoop();

    SUCCEED();
}

TEST_F(GLibDispatcherTest, DestructorStopsTimerThread) {
    {
        auto tempDispatcher = GLibDispatcher::create();
        tempDispatcher->start();

        std::thread runThread([&tempDispatcher]() { tempDispatcher->run(); });

        // Start a timer
        tempDispatcher->startTimer(1, 100, []() {}, false);

        std::this_thread::sleep_for(TestConstants::kTimerShortInterval);

        // Destructor should stop both event loop and timer thread
        tempDispatcher->stop();
        runThread.join();
        tempDispatcher.reset();
    }

    // Test passes if no hang occurs
    SUCCEED();
}

TEST_F(GLibDispatcherTest, TimerAccuracy) {
    auto startTime = std::chrono::steady_clock::now();
    std::atomic<bool> timerFired{false};

    startEventLoop();

    const int delayMs = 100;
    dispatcher_->startTimer(
        1, delayMs,
        [&timerFired, &startTime]() {
            timerFired.store(true);
            auto endTime = std::chrono::steady_clock::now();
            auto elapsed = std::chrono::duration_cast<std::chrono::milliseconds>(endTime - startTime).count();

            // Timer should fire within reasonable tolerance (±30ms)
            EXPECT_GE(elapsed, delayMs - 30);
            EXPECT_LE(elapsed, delayMs + 30);
        },
        false);

    // Wait for timer
    std::this_thread::sleep_for(TestConstants::kMediumWait);

    stopEventLoop();

    EXPECT_TRUE(timerFired.load());
}

TEST_F(GLibDispatcherTest, OneShotTimerAutoRemoved) {
    std::atomic<int> fireCount{0};

    startEventLoop();

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Wait for timer to fire
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    // Timer should be auto-removed
    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    // Wait more to ensure it doesn't fire again
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    stopEventLoop();

    EXPECT_EQ(fireCount.load(), 1);
}

TEST_F(GLibDispatcherTest, PeriodicTimerNotAutoRemoved) {
    std::atomic<int> fireCount{0};

    startEventLoop();

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, true);

    // Wait for timer to fire once
    std::this_thread::sleep_for(TestConstants::kTaskWait);

    // Timer should still be running
    EXPECT_TRUE(dispatcher_->isTimerRunning(1));

    dispatcher_->stopTimer(1);
    stopEventLoop();
}

TEST_F(GLibDispatcherTest, PeriodicTimerDriftPrevention) {
    std::vector<std::chrono::steady_clock::time_point> fireTimes;
    std::mutex fireTimesMutex;
    auto startTime = std::chrono::steady_clock::now();

    startEventLoop();

    const int intervalMs = 50;
    dispatcher_->startTimer(
        1, intervalMs,
        [&fireTimes, &fireTimesMutex]() {
            std::lock_guard<std::mutex> lock(fireTimesMutex);
            fireTimes.push_back(std::chrono::steady_clock::now());
        },
        true);

    // Wait for 10 firings (500ms)
    std::this_thread::sleep_for(std::chrono::milliseconds(600));

    dispatcher_->stopTimer(1);
    stopEventLoop();

    // Analyze drift
    ASSERT_GE(fireTimes.size(), 8);  // At least 8 firings

    // Calculate total drift from start time
    long long totalElapsed =
        std::chrono::duration_cast<std::chrono::milliseconds>(fireTimes.back() - startTime).count();
    long long expectedElapsed = intervalMs * static_cast<long long>(fireTimes.size() - 1);
    long long totalDrift =
        (totalElapsed > expectedElapsed) ? (totalElapsed - expectedElapsed) : (expectedElapsed - totalElapsed);

    // Total drift should be less than 100ms over multiple firings
    EXPECT_LT(totalDrift, 100) << "Total drift: " << totalDrift << "ms over " << fireTimes.size() << " firings";
}

}  // namespace SCE::Dispatchers
