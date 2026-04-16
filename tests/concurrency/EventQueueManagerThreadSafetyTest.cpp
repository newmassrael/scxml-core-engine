// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "core/EventQueueManager.h"
#include <atomic>
#include <chrono>
#include <gtest/gtest.h>
#include <set>
#include <thread>
#include <vector>

namespace SCE::Core {

class EventQueueManagerThreadSafetyTest : public ::testing::Test {
protected:
    static constexpr int NUM_THREADS = 20;
    static constexpr int EVENTS_PER_THREAD = 1000;
};

#ifdef SCE_THREAD_SAFE

/**
 * @brief Test concurrent event raising from multiple threads
 *
 * This test verifies that EventQueueManager correctly handles concurrent
 * raise() operations from multiple threads without data loss or corruption.
 *
 * Expected behavior (with SCE_THREAD_SAFE enabled):
 * - All events are successfully queued
 * - No events are lost
 * - Queue size matches expected count
 */
TEST_F(EventQueueManagerThreadSafetyTest, ConcurrentRaise) {
    EventQueueManager<int> queue;
    std::atomic<bool> startFlag{false};
    std::vector<std::thread> threads;

    // Launch threads that raise events concurrently
    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([&queue, &startFlag, i]() {
            // Wait for start signal to maximize concurrency
            while (!startFlag.load()) {
                std::this_thread::yield();
            }

            // Each thread raises unique events
            for (int j = 0; j < EVENTS_PER_THREAD; ++j) {
                int event = i * EVENTS_PER_THREAD + j;
                queue.raise(event);
            }
        });
    }

    // Start all threads simultaneously
    startFlag.store(true);

    // Wait for all threads to complete
    for (auto &t : threads) {
        t.join();
    }

    // Verify all events were queued
    size_t expectedCount = NUM_THREADS * EVENTS_PER_THREAD;
    EXPECT_EQ(queue.size(), expectedCount) << "Expected " << expectedCount << " events, got " << queue.size();

    // Verify all events are unique (no duplicates from race conditions)
    std::set<int> receivedEvents;
    while (queue.hasEvents()) {
        int event = queue.pop();
        EXPECT_TRUE(receivedEvents.insert(event).second) << "Duplicate event detected: " << event;
    }

    EXPECT_EQ(receivedEvents.size(), expectedCount) << "Some events were lost or duplicated";
}

/**
 * @brief Test concurrent pop operations from multiple threads
 *
 * This test verifies that EventQueueManager correctly handles concurrent
 * pop() operations without double-delivery or exceptions.
 *
 * Expected behavior (with SCE_THREAD_SAFE enabled):
 * - Each event is popped exactly once
 * - No events are lost
 * - No duplicate deliveries
 */
TEST_F(EventQueueManagerThreadSafetyTest, ConcurrentPop) {
    EventQueueManager<int> queue;

    // Pre-populate queue with events
    const int totalEvents = NUM_THREADS * EVENTS_PER_THREAD;
    for (int i = 0; i < totalEvents; ++i) {
        queue.raise(i);
    }

    std::atomic<bool> startFlag{false};
    std::vector<std::thread> threads;
    std::vector<std::set<int>> threadResults(NUM_THREADS);

    // Launch threads that pop events concurrently
    for (int i = 0; i < NUM_THREADS; ++i) {
        threads.emplace_back([&queue, &startFlag, &threadResults, i]() {
            while (!startFlag.load()) {
                std::this_thread::yield();
            }

            // Pop events until queue is empty
            while (queue.hasEvents()) {
                try {
                    int event = queue.pop();
                    threadResults[i].insert(event);
                } catch (const std::runtime_error &) {
                    // Queue became empty (race with other threads)
                    break;
                }
            }
        });
    }

    // Start all threads simultaneously
    startFlag.store(true);

    // Wait for all threads to complete
    for (auto &t : threads) {
        t.join();
    }

    // Verify queue is empty
    EXPECT_FALSE(queue.hasEvents()) << "Queue should be empty after all pops";

    // Collect all popped events
    std::set<int> allEvents;
    for (const auto &threadSet : threadResults) {
        for (int event : threadSet) {
            EXPECT_TRUE(allEvents.insert(event).second) << "Event " << event << " popped multiple times";
        }
    }

    // Verify all events were popped exactly once
    EXPECT_EQ(allEvents.size(), static_cast<size_t>(totalEvents)) << "Some events were lost or duplicated";
}

/**
 * @brief Test mixed concurrent operations (raise + pop + size + hasEvents)
 *
 * This test verifies that EventQueueManager correctly handles concurrent
 * mixed operations without race conditions or deadlocks.
 *
 * Expected behavior (with SCE_THREAD_SAFE enabled):
 * - No deadlocks
 * - No data corruption
 * - Consistent queue state
 */
TEST_F(EventQueueManagerThreadSafetyTest, MixedConcurrentOperations) {
    EventQueueManager<int> queue;
    std::atomic<bool> startFlag{false};
    std::atomic<bool> stopFlag{false};
    std::vector<std::thread> threads;

    // Producer threads
    for (int i = 0; i < NUM_THREADS / 2; ++i) {
        threads.emplace_back([&queue, &startFlag, &stopFlag, i]() {
            while (!startFlag.load()) {
                std::this_thread::yield();
            }

            int eventBase = i * 10000;
            int count = 0;
            while (!stopFlag.load() && count < EVENTS_PER_THREAD) {
                queue.raise(eventBase + count);
                count++;
                std::this_thread::yield();
            }
        });
    }

    // Consumer threads
    std::atomic<int> totalPopped{0};
    for (int i = 0; i < NUM_THREADS / 2; ++i) {
        threads.emplace_back([&queue, &startFlag, &stopFlag, &totalPopped]() {
            while (!startFlag.load()) {
                std::this_thread::yield();
            }

            while (!stopFlag.load()) {
                if (queue.hasEvents()) {
                    try {
                        queue.pop();
                        totalPopped.fetch_add(1);
                    } catch (const std::runtime_error &) {
                        // Queue became empty
                    }
                }
                std::this_thread::yield();
            }
        });
    }

    // Start all threads
    startFlag.store(true);

    // Let threads run for a bit
    std::this_thread::sleep_for(std::chrono::milliseconds(100));

    // Stop all threads
    stopFlag.store(true);

    // Wait for all threads to complete
    for (auto &t : threads) {
        t.join();
    }

    // Drain remaining events
    int remaining = 0;
    while (queue.hasEvents()) {
        queue.pop();
        remaining++;
    }

    int totalProduced = (NUM_THREADS / 2) * EVENTS_PER_THREAD;
    int totalConsumed = totalPopped.load() + remaining;

    // Verify no events were lost
    EXPECT_EQ(totalConsumed, totalProduced)
        << "Events lost: produced=" << totalProduced << ", consumed=" << totalConsumed;
}

/**
 * @brief Test concurrent clear operations
 *
 * This test verifies that clear() is thread-safe and doesn't cause
 * crashes when called concurrently with other operations.
 */
TEST_F(EventQueueManagerThreadSafetyTest, ConcurrentClear) {
    EventQueueManager<int> queue;
    std::atomic<bool> startFlag{false};
    std::atomic<bool> stopFlag{false};
    std::vector<std::thread> threads;

    // Producer thread
    threads.emplace_back([&queue, &startFlag, &stopFlag]() {
        while (!startFlag.load()) {
            std::this_thread::yield();
        }

        int count = 0;
        while (!stopFlag.load()) {
            queue.raise(count++);
            std::this_thread::yield();
        }
    });

    // Clear threads
    for (int i = 0; i < 5; ++i) {
        threads.emplace_back([&queue, &startFlag, &stopFlag]() {
            while (!startFlag.load()) {
                std::this_thread::yield();
            }

            while (!stopFlag.load()) {
                queue.clear();
                std::this_thread::sleep_for(std::chrono::microseconds(100));
            }
        });
    }

    // Start all threads
    startFlag.store(true);

    // Let threads run for a bit
    std::this_thread::sleep_for(std::chrono::milliseconds(50));

    // Stop all threads
    stopFlag.store(true);

    // Wait for all threads to complete
    for (auto &t : threads) {
        t.join();
    }

    // Test passes if no crash occurred
    SUCCEED() << "Concurrent clear operations completed without crash";
}

#else  // SCE_THREAD_SAFE not defined

/**
 * @brief Disabled test when SCE_THREAD_SAFE is not enabled
 *
 * This test verifies that thread safety tests are properly skipped
 * when SCE_THREAD_SAFE build flag is disabled.
 */
TEST_F(EventQueueManagerThreadSafetyTest, ThreadSafetyDisabled) {
    GTEST_SKIP() << "Thread safety tests require SCE_THREAD_SAFE build flag. "
                 << "Rebuild with: cmake -DSCE_THREAD_SAFE=ON ..";
}

#endif  // SCE_THREAD_SAFE

}  // namespace SCE::Core
