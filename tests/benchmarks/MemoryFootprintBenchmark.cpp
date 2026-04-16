// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// Memory Footprint Benchmarks

#include "dispatchers/StdThreadDispatcher.h"
#include "wrappers/AsyncStateMachine.h"
#include "wrappers/TimerManager.h"
#include <benchmark/benchmark.h>
#include <vector>

using namespace SCE::Dispatchers;
using namespace SCE::Wrappers;

// ============================================================================
// Mock State Machines for Memory Measurement
// ============================================================================

/**
 * @brief Minimal state machine (1 state, 1 event)
 */
class MinimalStateMachine {
public:
    enum class State { Single };
    enum class Event { Trigger };

    MinimalStateMachine() : state_(State::Single) {}

    void initialize() {}

    void raiseExternal(Event) {}

    void step() {}

    State getCurrentState() const {
        return state_;
    }

private:
    State state_;
};

/**
 * @brief Small state machine (3 states, 3 events)
 */
class SmallStateMachine {
public:
    enum class State { S1, S2, S3 };
    enum class Event { E1, E2, E3 };

    SmallStateMachine() : state_(State::S1) {}

    void initialize() {
        state_ = State::S1;
    }

    void raiseExternal(Event e) {
        lastEvent_ = e;
    }

    void step() {
        switch (state_) {
        case State::S1:
            if (lastEvent_ == Event::E1) {
                state_ = State::S2;
            }
            break;
        case State::S2:
            if (lastEvent_ == Event::E2) {
                state_ = State::S3;
            }
            break;
        case State::S3:
            if (lastEvent_ == Event::E3) {
                state_ = State::S1;
            }
            break;
        }
    }

    State getCurrentState() const {
        return state_;
    }

private:
    State state_;
    Event lastEvent_{Event::E1};
};

/**
 * @brief Medium state machine (10 states, 10 events)
 */
class MediumStateMachine {
public:
    enum class State { S0, S1, S2, S3, S4, S5, S6, S7, S8, S9 };
    enum class Event { E0, E1, E2, E3, E4, E5, E6, E7, E8, E9 };

    MediumStateMachine() : state_(State::S0) {}

    void initialize() {
        state_ = State::S0;
    }

    void raiseExternal(Event e) {
        lastEvent_ = e;
    }

    void step() {}

    State getCurrentState() const {
        return state_;
    }

private:
    State state_;
    Event lastEvent_{Event::E0};
    int data_[8]{};  // Some internal data
};

// ============================================================================
// Base State Machine Memory Benchmarks
// ============================================================================

/**
 * @brief Minimal state machine size
 *
 * Reports sizeof() for absolute minimal state machine.
 */
static void BM_Memory_MinimalStateMachine(benchmark::State &state) {
    for (auto _ : state) {
        MinimalStateMachine sm;
        benchmark::DoNotOptimize(sm);
    }

    state.counters["Size_bytes"] = sizeof(MinimalStateMachine);
}

BENCHMARK(BM_Memory_MinimalStateMachine);

/**
 * @brief Small state machine size
 *
 * Reports sizeof() for 3-state machine.
 */
static void BM_Memory_SmallStateMachine(benchmark::State &state) {
    for (auto _ : state) {
        SmallStateMachine sm;
        benchmark::DoNotOptimize(sm);
    }

    state.counters["Size_bytes"] = sizeof(SmallStateMachine);
}

BENCHMARK(BM_Memory_SmallStateMachine);

/**
 * @brief Medium state machine size
 *
 * Reports sizeof() for 10-state machine with data.
 */
static void BM_Memory_MediumStateMachine(benchmark::State &state) {
    for (auto _ : state) {
        MediumStateMachine sm;
        benchmark::DoNotOptimize(sm);
    }

    state.counters["Size_bytes"] = sizeof(MediumStateMachine);
}

BENCHMARK(BM_Memory_MediumStateMachine);

// ============================================================================
// Dispatcher Memory Benchmarks
// ============================================================================

/**
 * @brief StdThreadDispatcher size
 *
 * Reports sizeof() for dispatcher instance.
 */
static void BM_Memory_StdThreadDispatcher(benchmark::State &state) {
    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        benchmark::DoNotOptimize(dispatcher);
    }

    // Note: shared_ptr adds overhead, report raw class size
    state.counters["Size_bytes"] = sizeof(StdThreadDispatcher);
}

BENCHMARK(BM_Memory_StdThreadDispatcher);

/**
 * @brief Task queue memory impact (without processing)
 *
 * Measures memory usage of dispatcher task queue with N pending tasks.
 * NOTE: Tasks are NOT processed - this measures queue data structure overhead only.
 * No event loop thread is started, so tasks remain in queue.
 */
static void BM_Memory_DispatcherTaskQueueSize(benchmark::State &state) {
    const int numTasks = state.range(0);

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();

        // Enqueue without processing - measuring queue memory only
        for (int i = 0; i < numTasks; ++i) {
            dispatcher->enqueue([]() {});
        }

        size_t queueSize = dispatcher->pendingTasks();
        state.counters["QueueSize_tasks"] = queueSize;
        state.counters["QueueMemory_estimate_bytes"] = queueSize * sizeof(std::function<void()>);

        dispatcher->stop();
    }
}

BENCHMARK(BM_Memory_DispatcherTaskQueueSize)->Arg(0)->Arg(10)->Arg(100)->Arg(1000);

/**
 * @brief Timer registry memory impact (without firing)
 *
 * Measures memory usage of dispatcher with N registered timers.
 * NOTE: Timers do NOT fire - this measures timer registry overhead only.
 * No event loop thread is started, so timers remain registered but inactive.
 */
static void BM_Memory_DispatcherTimerRegistry(benchmark::State &state) {
    const int numTimers = state.range(0);

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        dispatcher->start();

        // Register timers without event loop - measuring registry memory
        for (int i = 0; i < numTimers; ++i) {
            dispatcher->startTimer(i, 1000, []() {}, true);
        }

        state.counters["RegisteredTimers"] = numTimers;
        // TimerInfo contains: time_point (16B) + milliseconds (8B) + function (32B) + bool (1B) ≈ 64B
        state.counters["TimerRegistryMemory_estimate_bytes"] = numTimers * 64;

        // Cleanup
        for (int i = 0; i < numTimers; ++i) {
            dispatcher->stopTimer(i);
        }

        dispatcher->stop();
    }
}

BENCHMARK(BM_Memory_DispatcherTimerRegistry)->Arg(0)->Arg(10)->Arg(50)->Arg(100);

// ============================================================================
// Wrapper Overhead Benchmarks
// ============================================================================

/**
 * @brief AsyncStateMachine wrapper size
 *
 * Reports sizeof() for async wrapper.
 */
static void BM_Memory_AsyncStateMachineWrapper(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();

    for (auto _ : state) {
        AsyncStateMachine<SmallStateMachine, SmallStateMachine::Event> sm(dispatcher);
        benchmark::DoNotOptimize(sm);
    }

    state.counters["Size_bytes"] = sizeof(AsyncStateMachine<SmallStateMachine, SmallStateMachine::Event>);
    state.counters["BaseSM_bytes"] = sizeof(SmallStateMachine);
    state.counters["Overhead_bytes"] =
        sizeof(AsyncStateMachine<SmallStateMachine, SmallStateMachine::Event>) - sizeof(SmallStateMachine);
}

BENCHMARK(BM_Memory_AsyncStateMachineWrapper);

/**
 * @brief TimerManager wrapper size
 *
 * Reports sizeof() for timer manager wrapper.
 */
static void BM_Memory_TimerManagerWrapper(benchmark::State &state) {
    // Note: TimerManager is template, need concrete type
    // Using SmallStateMachine as example

    struct SmallSMWithTimerID : public SmallStateMachine {
        enum class TimerID { T1, T2 };
        using SmallStateMachine::SmallStateMachine;
    };

    for (auto _ : state) {
        SmallSMWithTimerID sm;
        TimerManager<SmallSMWithTimerID> tm(sm);
        benchmark::DoNotOptimize(tm);
    }

    state.counters["Size_bytes"] = sizeof(TimerManager<SmallSMWithTimerID>);
}

BENCHMARK(BM_Memory_TimerManagerWrapper);

// ============================================================================
// Scalability Benchmarks
// ============================================================================

/**
 * @brief Memory per state machine instance
 *
 * Measures memory for N state machine instances.
 */
static void BM_Memory_MultipleStateMachines(benchmark::State &state) {
    const int numInstances = state.range(0);

    for (auto _ : state) {
        std::vector<SmallStateMachine> machines(numInstances);
        benchmark::DoNotOptimize(machines);
    }

    size_t totalSize = sizeof(SmallStateMachine) * numInstances;
    size_t vectorOverhead = sizeof(std::vector<SmallStateMachine>);

    state.counters["NumInstances"] = numInstances;
    state.counters["TotalSize_bytes"] = totalSize;
    state.counters["PerInstance_bytes"] = sizeof(SmallStateMachine);
    state.counters["VectorOverhead_bytes"] = vectorOverhead;
}

BENCHMARK(BM_Memory_MultipleStateMachines)->Arg(1)->Arg(10)->Arg(100)->Arg(1000);

// ============================================================================
// Comparative Analysis
// ============================================================================

/**
 * @brief Synchronous vs Async memory comparison
 *
 * Compares memory footprint of sync vs async state machines.
 */
static void BM_Memory_SyncVsAsync(benchmark::State &state) {
    auto dispatcher = StdThreadDispatcher::create();

    for (auto _ : state) {
        SmallStateMachine syncSM;
        AsyncStateMachine<SmallStateMachine, SmallStateMachine::Event> asyncSM(dispatcher);

        benchmark::DoNotOptimize(syncSM);
        benchmark::DoNotOptimize(asyncSM);
    }

    size_t syncSize = sizeof(SmallStateMachine);
    size_t asyncSize = sizeof(AsyncStateMachine<SmallStateMachine, SmallStateMachine::Event>);
    size_t overhead = asyncSize - syncSize;
    double overheadPercent = (overhead * 100.0) / syncSize;

    state.counters["Sync_bytes"] = syncSize;
    state.counters["Async_bytes"] = asyncSize;
    state.counters["Overhead_bytes"] = overhead;
    state.counters["Overhead_percent"] = overheadPercent;
}

BENCHMARK(BM_Memory_SyncVsAsync);

/**
 * @brief Realistic async system memory usage
 *
 * Measures memory for 1 shared dispatcher + multiple sync state machines.
 * This reflects actual usage: one dispatcher shared across many state machines.
 */
static void BM_Memory_SharedDispatcherSystem(benchmark::State &state) {
    const int numStateMachines = state.range(0);

    for (auto _ : state) {
        auto dispatcher = StdThreadDispatcher::create();
        std::vector<SmallStateMachine> machines(numStateMachines);

        benchmark::DoNotOptimize(dispatcher);
        benchmark::DoNotOptimize(machines);
    }

    size_t dispatcherSize = sizeof(StdThreadDispatcher);
    size_t perSMSize = sizeof(SmallStateMachine);
    size_t totalSMSize = perSMSize * numStateMachines;
    size_t totalSize = dispatcherSize + totalSMSize;
    size_t avgPerSM = totalSize / numStateMachines;

    state.counters["NumStateMachines"] = numStateMachines;
    state.counters["Dispatcher_bytes"] = dispatcherSize;
    state.counters["AllSMs_bytes"] = totalSMSize;
    state.counters["Total_bytes"] = totalSize;
    state.counters["AvgPerSM_bytes"] = avgPerSM;
}

BENCHMARK(BM_Memory_SharedDispatcherSystem)->Arg(1)->Arg(10)->Arg(50)->Arg(100);

BENCHMARK_MAIN();
