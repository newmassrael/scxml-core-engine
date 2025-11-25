// SPDX-License-Identifier: LGPL-2.1-or-later OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Async Dispatcher Performance Benchmarks

#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"
#include <atomic>
#include <benchmark/benchmark.h>
#include <chrono>
#include <thread>
#include <vector>

using namespace SCE::Dispatchers;
using namespace SCE::Wrappers;

// ============================================================================
// Mock State Machine for Benchmarking
// ============================================================================

class BenchStateMachine {
public:
    enum class State { Idle, Active };
    enum class Event { Start, Stop, Reset };

    BenchStateMachine() : state_(State::Idle), eventCount_(0) {}

    void initialize() {
        state_ = State::Idle;
        eventCount_ = 0;
    }

    void raiseExternal(Event event) {
        lastEvent_ = event;
        eventCount_.fetch_add(1, std::memory_order_relaxed);
    }

    void step() {
        // Simple state machine logic
        switch (state_) {
        case State::Idle:
            if (lastEvent_ == Event::Start) {
                state_ = State::Active;
            }
            break;
        case State::Active:
            if (lastEvent_ == Event::Stop) {
                state_ = State::Idle;
            }
            break;
        }
    }

    State getCurrentState() const {
        return state_;
    }

    int getEventCount() const {
        return eventCount_.load(std::memory_order_relaxed);
    }

private:
    State state_;
    Event lastEvent_{Event::Reset};
    std::atomic<int> eventCount_;
};

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * @brief Wait for dispatcher queue to drain
 *
 * Polls pendingTasks() until queue is empty or timeout occurs.
 * More reliable than arbitrary sleep_for() in CI/CD environments.
 *
 * @param dispatcher StdThreadDispatcher to monitor
 * @param timeoutMs Maximum wait time in milliseconds (default: 5000ms)
 * @return true if queue drained, false if timeout
 */
bool waitForQueueDrain(const std::shared_ptr<StdThreadDispatcher> &dispatcher, int timeoutMs = 5000) {
    auto start = std::chrono::steady_clock::now();
    while (dispatcher->pendingTasks() > 0) {
        auto elapsed =
            std::chrono::duration_cast<std::chrono::milliseconds>(std::chrono::steady_clock::now() - start).count();
        if (elapsed >= timeoutMs) {
            return false;  // Timeout
        }
        std::this_thread::sleep_for(std::chrono::milliseconds(10));
    }
    return true;
}

// ============================================================================
// Event Throughput Benchmarks
// ============================================================================

/**
 * @brief Single-threaded event throughput
 *
 * Measures maximum event processing rate with one producer thread.
 */
static void BM_AsyncDispatcher_SingleThreadThroughput(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
    sm.initialize();

    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    for (auto _ : state) {
        sm.postEvent(BenchStateMachine::Event::Reset);
    }

    // Wait for queue to drain (with timeout)
    waitForQueueDrain(dispatcher);

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations());
}

BENCHMARK(BM_AsyncDispatcher_SingleThreadThroughput)->Unit(benchmark::kMicrosecond)->Iterations(10000);

/**
 * @brief Multi-threaded event throughput
 *
 * Measures event processing rate with multiple producer threads.
 */
static void BM_AsyncDispatcher_MultiThreadThroughput(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
    sm.initialize();

    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    const int numProducers = state.range(0);
    const int eventsPerProducer = 100;

    for (auto _ : state) {
        std::vector<std::thread> producers;
        for (int i = 0; i < numProducers; ++i) {
            producers.emplace_back([&]() {
                for (int j = 0; j < eventsPerProducer; ++j) {
                    sm.postEvent(BenchStateMachine::Event::Reset);
                }
            });
        }

        for (auto &t : producers) {
            t.join();
        }
    }

    // Wait for queue to drain (with timeout)
    waitForQueueDrain(dispatcher);

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations() * numProducers * eventsPerProducer);
}

BENCHMARK(BM_AsyncDispatcher_MultiThreadThroughput)->Unit(benchmark::kMicrosecond)->Arg(2)->Arg(4)->Arg(8);

// ============================================================================
// Dispatcher Overhead Benchmarks
// ============================================================================

/**
 * @brief Dispatcher creation overhead
 *
 * Measures time to create and destroy dispatcher instance.
 */
static void BM_AsyncDispatcher_Creation(benchmark::State &state) {
    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        benchmark::DoNotOptimize(dispatcher);
    }
}

BENCHMARK(BM_AsyncDispatcher_Creation)->Unit(benchmark::kNanosecond);

/**
 * @brief Dispatcher start/stop overhead
 *
 * Measures time to start and stop event loop.
 */
static void BM_AsyncDispatcher_StartStop(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();

    for (auto _ : state) {
        dispatcher->start();
        std::thread eventLoop([&]() { dispatcher->run(); });

        dispatcher->stop();
        eventLoop.join();
    }
}

BENCHMARK(BM_AsyncDispatcher_StartStop)->Unit(benchmark::kMicrosecond);

/**
 * @brief Event enqueue latency
 *
 * Measures time to enqueue event to dispatcher queue.
 */
static void BM_AsyncDispatcher_EnqueueLatency(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    for (auto _ : state) {
        dispatcher->enqueue([]() {
            // Empty task
        });
    }

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations());
}

BENCHMARK(BM_AsyncDispatcher_EnqueueLatency)->Unit(benchmark::kNanosecond);

// ============================================================================
// AsyncStateMachine Wrapper Overhead
// ============================================================================

/**
 * @brief AsyncStateMachine initialization overhead
 *
 * Measures wrapper creation and initialization time.
 */
static void BM_AsyncStateMachine_Initialization(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();

    for (auto _ : state) {
        AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
        sm.initialize();
        benchmark::DoNotOptimize(sm);
    }
}

BENCHMARK(BM_AsyncStateMachine_Initialization)->Unit(benchmark::kNanosecond);

/**
 * @brief Event posting overhead (wrapper layer)
 *
 * Measures overhead of AsyncStateMachine::postEvent() wrapper.
 */
static void BM_AsyncStateMachine_PostEventOverhead(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
    sm.initialize();

    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    for (auto _ : state) {
        sm.postEvent(BenchStateMachine::Event::Reset);
    }

    // Wait for queue to drain (with timeout)
    waitForQueueDrain(dispatcher);

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations());
}

BENCHMARK(BM_AsyncStateMachine_PostEventOverhead)->Unit(benchmark::kNanosecond);

// ============================================================================
// Concurrent Operations Benchmarks
// ============================================================================

/**
 * @brief Concurrent event posting from multiple threads
 *
 * Stress test: multiple threads posting events simultaneously.
 */
static void BM_AsyncDispatcher_ConcurrentPosting(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
    sm.initialize();

    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    const int numThreads = state.range(0);
    const int eventsPerIteration = 50;

    for (auto _ : state) {
        std::vector<std::thread> threads;
        for (int i = 0; i < numThreads; ++i) {
            threads.emplace_back([&]() {
                for (int j = 0; j < eventsPerIteration; ++j) {
                    sm.postEvent(BenchStateMachine::Event::Reset);
                }
            });
        }

        for (auto &t : threads) {
            t.join();
        }
    }

    // Wait for queue to drain (with timeout)
    waitForQueueDrain(dispatcher);

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations() * numThreads * eventsPerIteration);
}

BENCHMARK(BM_AsyncDispatcher_ConcurrentPosting)->Unit(benchmark::kMicrosecond)->Arg(2)->Arg(4)->Arg(8)->Arg(16);

// ============================================================================
// Event Loop Performance
// ============================================================================

/**
 * @brief Empty event loop overhead
 *
 * Measures baseline overhead of event loop with no events.
 */
static void BM_AsyncDispatcher_EmptyEventLoop(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();

    std::atomic<bool> running{true};
    std::thread eventLoop([&]() {
        while (running.load()) {
            // Empty iteration
            std::this_thread::sleep_for(std::chrono::microseconds(10));
        }
    });

    for (auto _ : state) {
        std::this_thread::sleep_for(std::chrono::milliseconds(1));
    }

    running.store(false);
    dispatcher->stop();
    eventLoop.join();
}

BENCHMARK(BM_AsyncDispatcher_EmptyEventLoop)->Unit(benchmark::kMillisecond)->Iterations(100);

/**
 * @brief Event processing batch size impact
 *
 * Measures throughput with different batch sizes.
 */
static void BM_AsyncDispatcher_BatchProcessing(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    AsyncStateMachine<BenchStateMachine, BenchStateMachine::Event> sm(dispatcher);
    sm.initialize();

    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    const int batchSize = state.range(0);

    for (auto _ : state) {
        for (int i = 0; i < batchSize; ++i) {
            sm.postEvent(BenchStateMachine::Event::Reset);
        }
    }

    // Wait for queue to drain (with timeout)
    waitForQueueDrain(dispatcher);

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations() * batchSize);
}

BENCHMARK(BM_AsyncDispatcher_BatchProcessing)->Unit(benchmark::kMicrosecond)->Arg(1)->Arg(10)->Arg(100)->Arg(1000);

BENCHMARK_MAIN();
