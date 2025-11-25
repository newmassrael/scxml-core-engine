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
        threads.emplace_back([this, &counter, tasksPerThread]() {
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

        // Destructor should stop dispatcher
        tempDispatcher.reset();

        // Event loop should exit
        eventLoop.join();
    }

    // Test passes if no hang occurs
    SUCCEED();
}

}  // namespace SCE::Dispatchers
