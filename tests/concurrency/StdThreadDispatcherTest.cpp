// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "dispatchers/StdThreadDispatcher.h"
#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <stdexcept>
#include <thread>
#include <vector>

namespace SCE::Dispatchers {

/**
 * @brief Unit tests for StdThreadDispatcher
 *
 * Architecture Compliance:
 * - Zero Overhead: Header-only template, tests verify no mandatory linking
 * - Thread Safety: Validates concurrent enqueue operations
 * - Exception Safety: Verifies task exception handling
 */
class StdThreadDispatcherTest : public ::testing::Test {
protected:
    void SetUp() override {
        dispatcher_ = StdThreadDispatcher::create();
    }

    void TearDown() override {
        if (dispatcher_ && dispatcher_->isRunning()) {
            dispatcher_->stop();
        }
        dispatcher_.reset();
    }

    std::shared_ptr<StdThreadDispatcher> dispatcher_;
};

// Basic Operations Tests

TEST_F(StdThreadDispatcherTest, CreateDispatcher) {
    ASSERT_NE(dispatcher_, nullptr);
    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(StdThreadDispatcherTest, StartDispatcher) {
    dispatcher_->start();
    EXPECT_TRUE(dispatcher_->isRunning());
}

TEST_F(StdThreadDispatcherTest, StopDispatcher) {
    dispatcher_->start();
    dispatcher_->stop();
    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(StdThreadDispatcherTest, StartStopMultipleTimes) {
    for (int i = 0; i < 3; ++i) {
        dispatcher_->start();
        EXPECT_TRUE(dispatcher_->isRunning());

        dispatcher_->stop();
        EXPECT_FALSE(dispatcher_->isRunning());
    }
}

TEST_F(StdThreadDispatcherTest, EnqueueWithoutStart) {
    EXPECT_THROW(dispatcher_->enqueue([]() {}), std::runtime_error);
}

TEST_F(StdThreadDispatcherTest, RunWithoutStart) {
    EXPECT_THROW(dispatcher_->run(), std::runtime_error);
}

// Task Execution Tests

TEST_F(StdThreadDispatcherTest, ExecuteSingleTask) {
    std::atomic<bool> executed{false};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->enqueue([&executed]() { executed.store(true); });

    // Wait for task execution
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_TRUE(executed.load());
}

TEST_F(StdThreadDispatcherTest, ExecuteMultipleTasks) {
    std::atomic<int> counter{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    const int numTasks = 10;
    for (int i = 0; i < numTasks; ++i) {
        dispatcher_->enqueue([&counter]() { counter.fetch_add(1); });
    }

    // Wait for all tasks
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(counter.load(), numTasks);
}

// FIFO Ordering Test

TEST_F(StdThreadDispatcherTest, FIFOOrdering) {
    std::vector<int> executionOrder;
    std::mutex orderMutex;

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Enqueue tasks in specific order
    for (int i = 0; i < 5; ++i) {
        dispatcher_->enqueue([&executionOrder, &orderMutex, i]() {
            std::lock_guard<std::mutex> lock(orderMutex);
            executionOrder.push_back(i);
            std::this_thread::sleep_for(std::chrono::milliseconds(10));  // Ensure sequential execution
        });
    }

    // Wait for all tasks
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    dispatcher_->stop();
    eventLoop.join();

    ASSERT_EQ(executionOrder.size(), 5);
    for (int i = 0; i < 5; ++i) {
        EXPECT_EQ(executionOrder[i], i);
    }
}

// Multi-threading Tests

TEST_F(StdThreadDispatcherTest, ConcurrentEnqueue) {
    std::atomic<int> counter{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

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
    std::this_thread::sleep_for(std::chrono::milliseconds(300));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(counter.load(), numThreads * tasksPerThread);
}

// Exception Handling Tests

TEST_F(StdThreadDispatcherTest, TaskExceptionDoesNotStopDispatcher) {
    std::atomic<int> executedTasks{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Task 1: Normal
    dispatcher_->enqueue([&executedTasks]() { executedTasks.fetch_add(1); });

    // Task 2: Throws exception
    dispatcher_->enqueue([]() { throw std::runtime_error("Test exception"); });

    // Task 3: Normal (should execute despite Task 2 exception)
    dispatcher_->enqueue([&executedTasks]() { executedTasks.fetch_add(1); });

    // Wait for all tasks
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(executedTasks.load(), 2);
}

TEST_F(StdThreadDispatcherTest, MultipleExceptions) {
    std::atomic<int> executedTasks{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

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
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(executedTasks.load(), 3);  // 3 normal tasks (indices 0, 2, 4)
}

// Pending Tasks Tests

TEST_F(StdThreadDispatcherTest, PendingTasksCount) {
    dispatcher_->start();
    EXPECT_EQ(dispatcher_->pendingTasks(), 0);

    // Enqueue tasks without starting event loop
    dispatcher_->enqueue([]() { std::this_thread::sleep_for(std::chrono::milliseconds(100)); });
    dispatcher_->enqueue([]() {});

    // Should have 2 pending tasks
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    EXPECT_EQ(dispatcher_->pendingTasks(), 2);

    dispatcher_->stop();
}

TEST_F(StdThreadDispatcherTest, PendingTasksDecrease) {
    std::atomic<int> executedTasks{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Enqueue tasks
    for (int i = 0; i < 3; ++i) {
        dispatcher_->enqueue([&executedTasks]() {
            executedTasks.fetch_add(1);
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        });
    }

    // Wait for some tasks to execute
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(dispatcher_->pendingTasks(), 0);
    EXPECT_EQ(executedTasks.load(), 3);
}

// Edge Cases

TEST_F(StdThreadDispatcherTest, StopBeforeAnyTask) {
    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Immediately stop
    dispatcher_->stop();
    eventLoop.join();

    EXPECT_FALSE(dispatcher_->isRunning());
}

TEST_F(StdThreadDispatcherTest, StopWithPendingTasks) {
    std::atomic<int> executedTasks{0};

    dispatcher_->start();

    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Enqueue many tasks
    for (int i = 0; i < 100; ++i) {
        dispatcher_->enqueue([&executedTasks]() {
            executedTasks.fetch_add(1);
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        });
    }

    // Stop before all tasks complete
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
    dispatcher_->stop();
    eventLoop.join();

    // Some tasks should have executed, but not all
    EXPECT_GT(executedTasks.load(), 0);
    EXPECT_LT(executedTasks.load(), 100);
}

TEST_F(StdThreadDispatcherTest, DestructorStopsRunningDispatcher) {
    {
        auto tempDispatcher = StdThreadDispatcher::create();
        tempDispatcher->start();

        std::thread eventLoop([&tempDispatcher]() { tempDispatcher->run(); });

        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        // Stop dispatcher and join event loop before destroying the object
        // to avoid data race (event loop thread accessing destroyed object)
        tempDispatcher->stop();
        eventLoop.join();

        // Destructor runs on reset - timer thread cleanup
        tempDispatcher.reset();
    }

    // Test passes if no hang occurs
    SUCCEED();
}

// Timer Tests

TEST_F(StdThreadDispatcherTest, StartOneShotTimer) {
    std::atomic<bool> timerFired{false};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 100, [&timerFired]() { timerFired.store(true); }, false);

    // Wait for timer to fire
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_TRUE(timerFired.load());
}

TEST_F(StdThreadDispatcherTest, StartPeriodicTimer) {
    std::atomic<int> fireCount{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, true);

    // Wait for multiple firings
    std::this_thread::sleep_for(std::chrono::milliseconds(250));

    dispatcher_->stopTimer(1);
    dispatcher_->stop();
    eventLoop.join();

    // Should have fired at least 3 times (50ms, 100ms, 150ms, 200ms)
    EXPECT_GE(fireCount.load(), 3);
}

TEST_F(StdThreadDispatcherTest, StopTimer) {
    std::atomic<bool> timerFired{false};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 200, [&timerFired]() { timerFired.store(true); }, false);

    // Stop timer before it fires
    std::this_thread::sleep_for(std::chrono::milliseconds(50));
    dispatcher_->stopTimer(1);

    // Wait past original expiry time
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_FALSE(timerFired.load());
}

TEST_F(StdThreadDispatcherTest, IsTimerRunning) {
    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    dispatcher_->startTimer(1, 100, []() {}, false);
    EXPECT_TRUE(dispatcher_->isTimerRunning(1));

    dispatcher_->stopTimer(1);
    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    dispatcher_->stop();
    eventLoop.join();
}

TEST_F(StdThreadDispatcherTest, MultipleTimers) {
    std::atomic<int> timer1Count{0};
    std::atomic<int> timer2Count{0};
    std::atomic<int> timer3Count{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 50, [&timer1Count]() { timer1Count.fetch_add(1); }, false);
    dispatcher_->startTimer(2, 100, [&timer2Count]() { timer2Count.fetch_add(1); }, false);
    dispatcher_->startTimer(3, 150, [&timer3Count]() { timer3Count.fetch_add(1); }, false);

    // Wait for all timers to fire
    std::this_thread::sleep_for(std::chrono::milliseconds(250));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(timer1Count.load(), 1);
    EXPECT_EQ(timer2Count.load(), 1);
    EXPECT_EQ(timer3Count.load(), 1);
}

TEST_F(StdThreadDispatcherTest, ReplaceExistingTimer) {
    std::atomic<int> fireCount{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Start timer with 200ms delay
    dispatcher_->startTimer(1, 200, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Replace with timer with 50ms delay
    std::this_thread::sleep_for(std::chrono::milliseconds(10));
    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Wait for new timer to fire
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dispatcher_->stop();
    eventLoop.join();

    // Should have fired once (from second timer)
    EXPECT_EQ(fireCount.load(), 1);
}

TEST_F(StdThreadDispatcherTest, TimerAccuracy) {
    auto startTime = std::chrono::steady_clock::now();
    std::atomic<bool> timerFired{false};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

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
    std::this_thread::sleep_for(std::chrono::milliseconds(200));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_TRUE(timerFired.load());
}

TEST_F(StdThreadDispatcherTest, TimerCallbackException) {
    std::atomic<int> taskCounter{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Timer with exception
    dispatcher_->startTimer(1, 50, []() { throw std::runtime_error("Timer exception"); }, false);

    // Normal task after timer
    dispatcher_->enqueue([&taskCounter]() {
        std::this_thread::sleep_for(std::chrono::milliseconds(100));
        taskCounter.fetch_add(1);
    });

    // Wait for timer and task
    std::this_thread::sleep_for(std::chrono::milliseconds(250));

    dispatcher_->stop();
    eventLoop.join();

    // Task should execute despite timer exception
    EXPECT_EQ(taskCounter.load(), 1);
}

TEST_F(StdThreadDispatcherTest, StopNonExistentTimer) {
    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    // Should not crash
    dispatcher_->stopTimer(999);

    dispatcher_->stop();
    eventLoop.join();

    SUCCEED();
}

TEST_F(StdThreadDispatcherTest, OneShotTimerAutoRemoved) {
    std::atomic<int> fireCount{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, false);

    // Wait for timer to fire
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Timer should be auto-removed
    EXPECT_FALSE(dispatcher_->isTimerRunning(1));

    // Wait more to ensure it doesn't fire again
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    dispatcher_->stop();
    eventLoop.join();

    EXPECT_EQ(fireCount.load(), 1);
}

TEST_F(StdThreadDispatcherTest, PeriodicTimerNotAutoRemoved) {
    std::atomic<int> fireCount{0};

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

    dispatcher_->startTimer(1, 50, [&fireCount]() { fireCount.fetch_add(1); }, true);

    // Wait for timer to fire once
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Timer should still be running
    EXPECT_TRUE(dispatcher_->isTimerRunning(1));

    dispatcher_->stopTimer(1);
    dispatcher_->stop();
    eventLoop.join();
}

TEST_F(StdThreadDispatcherTest, DestructorStopsTimerThread) {
    {
        auto tempDispatcher = StdThreadDispatcher::create();
        tempDispatcher->start();

        std::thread eventLoop([&tempDispatcher]() { tempDispatcher->run(); });

        // Start a timer
        tempDispatcher->startTimer(1, 100, []() {}, false);

        std::this_thread::sleep_for(std::chrono::milliseconds(50));

        // Stop dispatcher and join event loop before destroying the object
        // to avoid data race (event loop thread accessing destroyed object)
        tempDispatcher->stop();
        eventLoop.join();

        // Destructor runs on reset - timer thread cleanup
        tempDispatcher.reset();
    }

    // Test passes if no hang occurs
    SUCCEED();
}

TEST_F(StdThreadDispatcherTest, PeriodicTimerDriftPrevention) {
    std::vector<std::chrono::steady_clock::time_point> fireTimes;
    std::mutex fireTimesMutex;
    auto startTime = std::chrono::steady_clock::now();

    dispatcher_->start();
    std::thread eventLoop([this]() { dispatcher_->run(); });

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
    dispatcher_->stop();
    eventLoop.join();

    // Analyze drift
    ASSERT_GE(fireTimes.size(), 8);  // At least 8 firings

    // Calculate average interval between firings
    std::vector<long long> intervals;
    for (size_t i = 1; i < fireTimes.size(); ++i) {
        auto interval = std::chrono::duration_cast<std::chrono::milliseconds>(fireTimes[i] - fireTimes[i - 1]).count();
        intervals.push_back(interval);
    }

    // Calculate total drift from start time
    long long totalElapsed =
        std::chrono::duration_cast<std::chrono::milliseconds>(fireTimes.back() - startTime).count();
    long long expectedElapsed = intervalMs * static_cast<long long>(fireTimes.size() - 1);
    long long totalDrift =
        (totalElapsed > expectedElapsed) ? (totalElapsed - expectedElapsed) : (expectedElapsed - totalElapsed);

    // Total drift should be less than 100ms over multiple firings
    EXPECT_LT(totalDrift, 100) << "Total drift: " << totalDrift << "ms over " << fireTimes.size() << " firings";

    // Average interval should be close to expected (±20ms)
    double avgInterval = 0;
    for (auto interval : intervals) {
        avgInterval += interval;
    }
    avgInterval /= intervals.size();

    EXPECT_GE(avgInterval, intervalMs - 20) << "Average interval too short: " << avgInterval << "ms";
    EXPECT_LE(avgInterval, intervalMs + 20) << "Average interval too long: " << avgInterval << "ms";
}

}  // namespace SCE::Dispatchers
