// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Timer Accuracy and Performance Benchmarks

#include "dispatchers/StdThreadDispatcher.h"
#include <atomic>
#include <benchmark/benchmark.h>
#include <chrono>
#include <cmath>
#include <thread>
#include <vector>

using namespace SCE::Dispatchers;
using namespace std::chrono;

// ============================================================================
// Timer Accuracy Benchmarks
// ============================================================================

/**
 * @brief One-shot timer accuracy
 *
 * Measures accuracy of single timer expiration.
 */
static void BM_Timer_OneShotAccuracy(benchmark::State &state) {
    const int delayMs = state.range(0);
    std::vector<long long> drifts;

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();

        std::mutex mtx;
        std::condition_variable cv;
        std::atomic<bool> fired{false};
        steady_clock::time_point startTime;
        steady_clock::time_point fireTime;

        std::thread eventLoop([&]() { dispatcher->run(); });

        startTime = steady_clock::now();
        dispatcher->startTimer(
            1, delayMs,
            [&]() {
                fireTime = steady_clock::now();
                {
                    std::lock_guard<std::mutex> lock(mtx);
                    fired.store(true);
                }
                cv.notify_one();
            },
            false);

        // Wait for timer with condition_variable (more efficient than busy-wait)
        {
            std::unique_lock<std::mutex> lock(mtx);
            cv.wait(lock, [&]() { return fired.load(); });
        }

        auto elapsed = duration_cast<milliseconds>(fireTime - startTime).count();
        long long drift = elapsed - delayMs;
        drifts.push_back(drift);

        dispatcher->stop();
        eventLoop.join();
    }

    // Calculate statistics
    double avgDrift = 0;
    for (auto d : drifts) {
        avgDrift += d;
    }
    avgDrift /= drifts.size();

    state.counters["AvgDrift_ms"] = avgDrift;
    state.counters["TargetDelay_ms"] = delayMs;
}

BENCHMARK(BM_Timer_OneShotAccuracy)
    ->Unit(benchmark::kMillisecond)
    ->Arg(10)
    ->Arg(50)
    ->Arg(100)
    ->Arg(500)
    ->Iterations(20);

/**
 * @brief Periodic timer drift measurement
 *
 * Measures cumulative drift over multiple timer firings.
 */
static void BM_Timer_PeriodicDrift(benchmark::State &state) {
    const int intervalMs = state.range(0);
    const int numFirings = 10;

    std::vector<long long> totalDrifts;

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();

        std::vector<steady_clock::time_point> fireTimes;
        steady_clock::time_point startTime;

        std::thread eventLoop([&]() { dispatcher->run(); });

        startTime = steady_clock::now();
        dispatcher->startTimer(1, intervalMs, [&]() { fireTimes.push_back(steady_clock::now()); }, true);

        // Wait for all firings
        while (fireTimes.size() < numFirings) {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

        dispatcher->stopTimer(1);

        // Calculate total drift
        auto expectedTime = startTime + milliseconds(intervalMs * (numFirings - 1));
        auto actualTime = fireTimes.back();
        long long totalDrift = duration_cast<milliseconds>(actualTime - expectedTime).count();

        totalDrifts.push_back(std::abs(totalDrift));

        dispatcher->stop();
        eventLoop.join();
    }

    // Calculate statistics
    double avgDrift = 0;
    for (auto d : totalDrifts) {
        avgDrift += d;
    }
    avgDrift /= totalDrifts.size();

    state.counters["AvgTotalDrift_ms"] = avgDrift;
    state.counters["NumFirings"] = numFirings;
    state.counters["Interval_ms"] = intervalMs;
}

BENCHMARK(BM_Timer_PeriodicDrift)->Unit(benchmark::kMillisecond)->Arg(50)->Arg(100)->Iterations(10);

// ============================================================================
// Timer Overhead Benchmarks
// ============================================================================

/**
 * @brief Timer creation overhead
 *
 * Measures time to start a timer.
 */
static void BM_Timer_StartOverhead(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    int timerId = 0;
    for (auto _ : state) {
        dispatcher->startTimer(timerId++, 1000, []() {}, false);
        dispatcher->stopTimer(timerId - 1);
    }

    dispatcher->stop();
    eventLoop.join();
}

// A FIXED iteration count, like every timing case above, because this one is
// also a ctest case (`benchmark_timer_accuracy_quick`) and a ctest case has a
// wall-clock TIMEOUT.
//
// Left adaptive, google-benchmark calibrates from a short sample and then
// commits to however many iterations fill `--benchmark_min_time` — measured
// 2026-08-16 on a quiet machine, 431,461 of them at 436 ns. The count is fixed
// before the run; the per-iteration cost is not. A busy machine multiplies the
// second and the total is unbounded, which is how a case that takes 0.21 s
// alone hit the 300 s timeout and blocked a push twice.
//
// Not an accumulation bug: at 4,000,000 iterations the cost per operation
// measures 243 ns, LOWER than at 431k, so `stopTimer` is releasing what
// `startTimer` took. Only the schedule is unbounded.
//
// 100,000 keeps the sample large enough for a nanosecond figure while bounding
// the work: ~40 ms nominal, and still seconds rather than minutes if the
// machine is a hundred times slower.
BENCHMARK(BM_Timer_StartOverhead)->Unit(benchmark::kNanosecond)->Iterations(100000);

/**
 * @brief Timer stop overhead
 *
 * Measures time to stop a timer.
 */
static void BM_Timer_StopOverhead(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    int timerId = 0;
    for (auto _ : state) {
        dispatcher->startTimer(timerId, 1000, []() {}, false);
        dispatcher->stopTimer(timerId);
        timerId++;
    }

    dispatcher->stop();
    eventLoop.join();
}

// Fixed for the same reason as `BM_Timer_StartOverhead` above, and it is the
// worse of the two: adaptive, this one committed to 1,026,222 iterations.
BENCHMARK(BM_Timer_StopOverhead)->Unit(benchmark::kNanosecond)->Iterations(100000);

/**
 * @brief Timer query overhead
 *
 * Measures time to check if timer is running.
 */
static void BM_Timer_IsRunningOverhead(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    dispatcher->startTimer(1, 1000, []() {}, true);

    for (auto _ : state) {
        benchmark::DoNotOptimize(dispatcher->isTimerRunning(1));
    }

    dispatcher->stop();
    eventLoop.join();
}

BENCHMARK(BM_Timer_IsRunningOverhead)->Unit(benchmark::kNanosecond);

// ============================================================================
// Concurrent Timer Operations
// ============================================================================

/**
 * @brief Multiple concurrent timers
 *
 * Measures performance with many simultaneous timers.
 */
static void BM_Timer_ConcurrentTimers(benchmark::State &state) {
    const int numTimers = state.range(0);
    std::atomic<int> fireCount{0};

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();
        std::thread eventLoop([&]() { dispatcher->run(); });

        fireCount.store(0);

        // Start multiple timers
        auto startTime = steady_clock::now();
        for (int i = 0; i < numTimers; ++i) {
            dispatcher->startTimer(i, 100, [&]() { fireCount.fetch_add(1, std::memory_order_relaxed); }, false);
        }

        // Wait for all timers
        while (fireCount.load(std::memory_order_relaxed) < numTimers) {
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }
        auto endTime = steady_clock::now();

        auto elapsed = duration_cast<milliseconds>(endTime - startTime).count();

        dispatcher->stop();
        eventLoop.join();

        state.counters["Elapsed_ms"] = elapsed;
    }

    state.SetItemsProcessed(state.iterations() * numTimers);
}

BENCHMARK(BM_Timer_ConcurrentTimers)->Unit(benchmark::kMillisecond)->Arg(10)->Arg(50)->Arg(100)->Arg(500);

/**
 * @brief Timer start/stop churn
 *
 * Stress test: rapidly starting and stopping timers.
 */
static void BM_Timer_StartStopChurn(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();
    dispatcher->start();
    std::thread eventLoop([&]() { dispatcher->run(); });

    const int numOperations = state.range(0);

    for (auto _ : state) {
        for (int i = 0; i < numOperations; ++i) {
            dispatcher->startTimer(i, 100, []() {}, false);
            dispatcher->stopTimer(i);
        }
    }

    dispatcher->stop();
    eventLoop.join();

    state.SetItemsProcessed(state.iterations() * numOperations * 2);  // start + stop
}

BENCHMARK(BM_Timer_StartStopChurn)->Unit(benchmark::kMicrosecond)->Arg(10)->Arg(100)->Arg(1000);

// ============================================================================
// Timer Callback Execution
// ============================================================================

/**
 * @brief Timer callback execution overhead
 *
 * Measures time from timer expiry to callback execution.
 */
static void BM_Timer_CallbackLatency(benchmark::State &state) {
    std::vector<long long> latencies;

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();
        std::thread eventLoop([&]() { dispatcher->run(); });

        std::mutex mtx;
        std::condition_variable cv;
        std::atomic<bool> fired{false};
        steady_clock::time_point expectedTime;
        steady_clock::time_point callbackTime;

        expectedTime = steady_clock::now() + milliseconds(100);
        dispatcher->startTimer(
            1, 100,
            [&]() {
                callbackTime = steady_clock::now();
                {
                    std::lock_guard<std::mutex> lock(mtx);
                    fired.store(true);
                }
                cv.notify_one();
            },
            false);

        // Wait for timer with condition_variable (more efficient than busy-wait)
        {
            std::unique_lock<std::mutex> lock(mtx);
            cv.wait(lock, [&]() { return fired.load(); });
        }

        long long latency = duration_cast<microseconds>(callbackTime - expectedTime).count();
        latencies.push_back(latency);

        dispatcher->stop();
        eventLoop.join();
    }

    // Calculate statistics
    double avgLatency = 0;
    for (auto l : latencies) {
        avgLatency += l;
    }
    avgLatency /= latencies.size();

    state.counters["AvgLatency_us"] = avgLatency;
}

BENCHMARK(BM_Timer_CallbackLatency)->Unit(benchmark::kMillisecond)->Iterations(20);

// ============================================================================
// Periodic Timer Performance
// ============================================================================

/**
 * @brief Periodic timer throughput
 *
 * Measures event rate from periodic timer.
 */
static void BM_Timer_PeriodicThroughput(benchmark::State &state) {
    const int intervalMs = state.range(0);

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();
        std::thread eventLoop([&]() { dispatcher->run(); });

        std::atomic<int> fireCount{0};
        auto startTime = steady_clock::now();

        dispatcher->startTimer(1, intervalMs, [&]() { fireCount.fetch_add(1, std::memory_order_relaxed); }, true);

        // Run for 1 second
        std::this_thread::sleep_for(std::chrono::seconds(1));

        dispatcher->stopTimer(1);
        auto endTime = steady_clock::now();

        auto elapsed = duration_cast<milliseconds>(endTime - startTime).count();
        int fires = fireCount.load();

        dispatcher->stop();
        eventLoop.join();

        state.counters["FiresPerSec"] = (fires * 1000.0) / elapsed;
        state.counters["Interval_ms"] = intervalMs;
    }
}

BENCHMARK(BM_Timer_PeriodicThroughput)->Unit(benchmark::kMillisecond)->Arg(10)->Arg(50)->Arg(100)->Iterations(5);

BENCHMARK_MAIN();
