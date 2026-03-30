#include "scripting/LuaEngine.h"
#include <algorithm>
#include <atomic>
#include <benchmark/benchmark.h>
#include <memory>
#include <random>
#include <sstream>
#include <vector>

using namespace SCE;

// ============================================================================
// Thread-safe counter for unique session IDs
// ============================================================================
static std::atomic<uint64_t> globalLuaSessionCounter{0};

static std::string generateUniqueLuaSessionId() {
    uint64_t id = globalLuaSessionCounter.fetch_add(1, std::memory_order_relaxed);
    return "lua_bench_" + std::to_string(id);
}

// ECMAScript script generator (same workload as JSEngineBenchmark)
// LuaEngine's EcmaScriptToLuaTransformer handles conversion automatically
static std::string generateEcmaScript(int complexity) {
    std::stringstream ss;
    ss << "var result = 0;\n";
    for (int i = 0; i < complexity; ++i) {
        ss << "result = result + " << i << ";\n";
    }
    ss << "result;";
    return ss.str();
}

// Native Lua script generator (bypasses ECMAScript transformation)
static std::string generateNativeLuaScript(int complexity) {
    std::stringstream ss;
    ss << "local result = 0\n";
    for (int i = 0; i < complexity; ++i) {
        ss << "result = result + " << i << "\n";
    }
    ss << "return result";
    return ss.str();
}

// ============================================================================
// Benchmark Fixtures
// ============================================================================
class LuaEngineFixture : public benchmark::Fixture {
protected:
    LuaEngine *engine_;
    std::mt19937 rng_{42};

    void SetUp(const ::benchmark::State & /*state*/) override {
        engine_ = &LuaEngine::instance();
    }

    void TearDown(const ::benchmark::State & /*state*/) override {
        // Singleton cleanup not needed between tests
    }

    int randomComplexity(int min_ops = 1, int max_ops = 50) {
        std::uniform_int_distribution<> dist(min_ops, max_ops);
        return dist(rng_);
    }
};

// ============================================================================
// Micro-Benchmarks: Session Management
// (Mirrors JSEngineBenchmark for direct comparison)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, SessionCreation)(benchmark::State &state) {
    for (auto _ : state) {
        std::string sessionId = generateUniqueLuaSessionId();
        bool created = engine_->createSession(sessionId);
        benchmark::DoNotOptimize(created);

        if (created) {
            engine_->destroySession(sessionId);
        }
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Single-threaded session create/destroy");
}

BENCHMARK_F(LuaEngineFixture, SessionLookup)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    for (auto _ : state) {
        bool exists = engine_->hasSession(sessionId);
        benchmark::DoNotOptimize(exists);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Read-only session lookup");
}

// ============================================================================
// Micro-Benchmarks: ECMAScript Expression Evaluation
// (Same expressions as JSEngineBenchmark — includes transformation overhead)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, SimpleExpression)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    std::string script = "1 + 2 * 3";

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("ECMAScript arithmetic (via transformer)");
}

BENCHMARK_F(LuaEngineFixture, VariableOperations)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    for (auto _ : state) {
        engine_->setVariable(sessionId, "testVar", "42");
        auto result = engine_->getVariable(sessionId, "testVar");
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Set and get variable");
}

BENCHMARK_F(LuaEngineFixture, ScriptComplexity)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    const int complexity = state.range(0);
    std::string script = generateEcmaScript(complexity);

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("ECMAScript complexity=" + std::to_string(complexity));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ScriptComplexity)->Arg(1)->Arg(10)->Arg(50)->Arg(100);

// ============================================================================
// Micro-Benchmarks: Native Lua Expression Evaluation
// (Pure engine performance — no ECMAScript-to-Lua transformation)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, NativeSimpleExpression)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    std::string script = "return 1 + 2 * 3";

    for (auto _ : state) {
        auto result = engine_->executeScript(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Native Lua arithmetic (no transform)");
}

BENCHMARK_F(LuaEngineFixture, NativeMathExpression)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    std::string script = "return math.sqrt(1234567) + math.sin(0.5)";

    for (auto _ : state) {
        auto result = engine_->executeScript(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Native Lua math functions");
}

BENCHMARK_F(LuaEngineFixture, NativeScriptComplexity)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    const int complexity = state.range(0);
    std::string script = generateNativeLuaScript(complexity);

    for (auto _ : state) {
        auto result = engine_->executeScript(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Native Lua complexity=" + std::to_string(complexity));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, NativeScriptComplexity)->Arg(1)->Arg(10)->Arg(50)->Arg(100);

// ============================================================================
// Scalability Benchmarks: Concurrent Operations
// (Mirrors JSEngineBenchmark for direct comparison)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, ConcurrentSessionCreation)(benchmark::State &state) {
    for (auto _ : state) {
        std::string sessionId = generateUniqueLuaSessionId();
        bool created = engine_->createSession(sessionId);
        benchmark::DoNotOptimize(created);

        if (created) {
            engine_->destroySession(sessionId);
        }
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("threads=" + std::to_string(state.threads()));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ConcurrentSessionCreation)
    ->Threads(1)
    ->Threads(2)
    ->Threads(4)
    ->Threads(8)
    ->UseRealTime();

BENCHMARK_F(LuaEngineFixture, ConcurrentScriptExecution)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();

    if (state.thread_index() == 0) {
        engine_->createSession(sessionId);
    }

    // Same ECMAScript expression as JSEngineBenchmark (transformer converts Math→math)
    std::string script = "Math.sqrt(1234567) + Math.sin(0.5)";

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    if (state.thread_index() == 0) {
        engine_->destroySession(sessionId);
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("threads=" + std::to_string(state.threads()));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ConcurrentScriptExecution)
    ->Threads(1)
    ->Threads(2)
    ->Threads(4)
    ->Threads(8)
    ->UseRealTime();

BENCHMARK_F(LuaEngineFixture, ConcurrentSameSession)(benchmark::State &state) {
    static std::string sharedSessionId;
    static std::once_flag initFlag;

    std::call_once(initFlag, [this]() {
        sharedSessionId = generateUniqueLuaSessionId();
        engine_->createSession(sharedSessionId);
    });

    std::string script = "1 + 2 + 3";

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sharedSessionId, script);
        benchmark::DoNotOptimize(result);
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("shared session | threads=" + std::to_string(state.threads()));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ConcurrentSameSession)
    ->Threads(1)
    ->Threads(2)
    ->Threads(4)
    ->Threads(8)
    ->UseRealTime();

// ============================================================================
// Mixed Workload Benchmarks
// (Mirrors JSEngineBenchmark for direct comparison)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, MixedWorkload)(benchmark::State &state) {
    std::vector<std::string> sessionPool;
    for (int i = 0; i < 5; ++i) {
        std::string sid = generateUniqueLuaSessionId();
        engine_->createSession(sid);
        sessionPool.push_back(sid);
    }

    std::uniform_int_distribution<> op_dist(0, 9);
    std::uniform_int_distribution<> session_dist(0, sessionPool.size() - 1);

    for (auto _ : state) {
        int op = op_dist(rng_);
        std::string sessionId = sessionPool[session_dist(rng_)];

        if (op < 2) {
            // 20% session creation/destruction
            std::string newSession = generateUniqueLuaSessionId();
            bool created = engine_->createSession(newSession);
            benchmark::DoNotOptimize(created);
            if (created) {
                engine_->destroySession(newSession);
            }
        } else if (op < 4) {
            // 20% session lookup
            bool exists = engine_->hasSession(sessionId);
            benchmark::DoNotOptimize(exists);
        } else if (op < 8) {
            // 40% script execution
            auto result = engine_->evaluateExpression(sessionId, "42 * 2");
            benchmark::DoNotOptimize(result);
        } else {
            // 20% variable operations
            engine_->setVariable(sessionId, "v", "10");
            auto result = engine_->getVariable(sessionId, "v");
            benchmark::DoNotOptimize(result);
        }
    }

    for (const auto &sid : sessionPool) {
        engine_->destroySession(sid);
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("20% create, 20% lookup, 40% exec, 20% var | threads=" + std::to_string(state.threads()));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, MixedWorkload)->Threads(1)->Threads(2)->Threads(4)->Threads(8)->UseRealTime();

// ============================================================================
// Latency Benchmarks
// (Mirrors JSEngineBenchmark for direct comparison)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, ScriptExecutionLatency)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Same ECMAScript expression as JSEngineBenchmark
    std::string script = "Math.pow(2, 10) + Math.sqrt(256)";
    std::vector<double> latencies;

    for (auto _ : state) {
        auto start = std::chrono::steady_clock::now();

        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);

        auto end = std::chrono::steady_clock::now();
        auto latency_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();

        latencies.push_back(latency_us);
    }

    engine_->destroySession(sessionId);

    if (!latencies.empty()) {
        std::sort(latencies.begin(), latencies.end());
        double p50 = latencies[latencies.size() * 50 / 100];
        double p95 = latencies[latencies.size() * 95 / 100];
        double p99 = latencies[latencies.size() * 99 / 100];

        state.counters["p50_us"] = p50;
        state.counters["p95_us"] = p95;
        state.counters["p99_us"] = p99;
    }
    state.SetLabel("Latency percentiles (us)");
}

// Native Lua latency (no transformation overhead)
BENCHMARK_F(LuaEngineFixture, NativeScriptExecutionLatency)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    std::string script = "return 2^10 + math.sqrt(256)";
    std::vector<double> latencies;

    for (auto _ : state) {
        auto start = std::chrono::steady_clock::now();

        auto result = engine_->executeScript(sessionId, script);
        benchmark::DoNotOptimize(result);

        auto end = std::chrono::steady_clock::now();
        auto latency_us = std::chrono::duration_cast<std::chrono::microseconds>(end - start).count();

        latencies.push_back(latency_us);
    }

    engine_->destroySession(sessionId);

    if (!latencies.empty()) {
        std::sort(latencies.begin(), latencies.end());
        double p50 = latencies[latencies.size() * 50 / 100];
        double p95 = latencies[latencies.size() * 95 / 100];
        double p99 = latencies[latencies.size() * 99 / 100];

        state.counters["p50_us"] = p50;
        state.counters["p95_us"] = p95;
        state.counters["p99_us"] = p99;
    }
    state.SetLabel("Native Lua latency percentiles (us)");
}

// ============================================================================
// Stress Tests
// (Mirrors JSEngineBenchmark for direct comparison)
// ============================================================================

BENCHMARK_F(LuaEngineFixture, ManySessionsStress)(benchmark::State &state) {
    const int num_sessions = state.range(0);

    for (auto _ : state) {
        state.PauseTiming();

        std::vector<std::string> sessionIds;
        sessionIds.reserve(num_sessions);
        for (int i = 0; i < num_sessions; ++i) {
            std::string sessionId = generateUniqueLuaSessionId();
            sessionIds.push_back(sessionId);
            engine_->createSession(sessionId);
        }

        state.ResumeTiming();

        std::string sessionId = sessionIds[rng_() % sessionIds.size()];
        auto result = engine_->evaluateExpression(sessionId, "42");
        benchmark::DoNotOptimize(result);

        state.PauseTiming();

        for (const auto &sid : sessionIds) {
            engine_->destroySession(sid);
        }

        state.ResumeTiming();
    }

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("sessions=" + std::to_string(num_sessions));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ManySessionsStress)->Arg(10)->Arg(50)->Arg(100)->Arg(500);

BENCHMARK_MAIN();
