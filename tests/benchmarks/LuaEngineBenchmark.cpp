// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael

#include "SCXMLTypes.h"
#include "common/EventDataHelper.h"
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

BENCHMARK_DEFINE_F(LuaEngineFixture, ScriptComplexity)(benchmark::State &state) {
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

BENCHMARK_DEFINE_F(LuaEngineFixture, NativeScriptComplexity)(benchmark::State &state) {
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

BENCHMARK_DEFINE_F(LuaEngineFixture, ConcurrentSessionCreation)(benchmark::State &state) {
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

BENCHMARK_DEFINE_F(LuaEngineFixture, ConcurrentScriptExecution)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Same ECMAScript expression as JSEngineBenchmark (transformer converts Math→math)
    std::string script = "Math.sqrt(1234567) + Math.sin(0.5)";

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);

    state.SetItemsProcessed(state.iterations());
    state.SetLabel("threads=" + std::to_string(state.threads()));
}

BENCHMARK_REGISTER_F(LuaEngineFixture, ConcurrentScriptExecution)
    ->Threads(1)
    ->Threads(2)
    ->Threads(4)
    ->Threads(8)
    ->UseRealTime();

BENCHMARK_DEFINE_F(LuaEngineFixture, ConcurrentSameSession)(benchmark::State &state) {
    const std::string sessionId = "lua_shared_session";

    if (state.thread_index() == 0) {
        engine_->createSession(sessionId);
    }

    std::string script = "1 + 2 + 3";

    for (auto _ : state) {
        auto result = engine_->evaluateExpression(sessionId, script);
        benchmark::DoNotOptimize(result);
    }

    if (state.thread_index() == 0) {
        engine_->destroySession(sessionId);
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

BENCHMARK_DEFINE_F(LuaEngineFixture, MixedWorkload)(benchmark::State &state) {
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

BENCHMARK_DEFINE_F(LuaEngineFixture, ManySessionsStress)(benchmark::State &state) {
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

// ============================================================================
// mesh_open_issues.md Issue 4 measurement — setCurrentEvent receive-path cost
//
// Isolates the per-event cost of the 8-arg `setCurrentEvent` overload's
// `luaL_dostring("return " + eventData)` compile attempt, and the
// downstream `EventDataHelper::jsonStringToScriptValue` fallback, against
// the typedData-overlay path that runs in `EventRaiserImpl` 's
// pipeline-level pre-parse + the shared_ptr `setCurrentEvent` overlay.
//
// Threshold-driven decision rule:
//   < 100 ns/event waste → AUTO-CLOSE
//   100-500 ns/event     → DOCUMENT-ONLY
//   > 500 ns/event       → AUTO-FIX
//
// The delta of interest is:
//   (8ArgJsonShape - 8ArgEmpty) = compile-attempt + JSON-parse cost per event
//   (EventOverloadWithTypedData - 8ArgJsonShape) = redundant overlay cost
// ============================================================================

BENCHMARK_F(LuaEngineFixture, SetCurrentEvent8ArgEmpty)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    for (auto _ : state) {
        auto fut = engine_->setCurrentEvent(sessionId, SCE::SetCurrentEventArgs{"evt", "", "internal", "", "", "", ""});
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Baseline: SetCurrentEventArgs overload, empty data (no parse path)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEvent8ArgJsonShapeShort)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // 7 chars — minimal JSON shape (always fails luaL_dostring, succeeds JSON parse)
    const std::string jsonData = R"({"a":1})";

    for (auto _ : state) {
        auto fut =
            engine_->setCurrentEvent(sessionId, SCE::SetCurrentEventArgs{"evt", jsonData, "internal", "", "", "", ""});
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("SetCurrentEventArgs overload, JSON {\"a\":1} (compile-fail + JSON-parse)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEvent8ArgJsonShapeRealistic)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Representative event payload — params merged via EventDataHelper::buildJsonFromParams
    const std::string jsonData = R"({"counter":42,"name":"hello","flag":true})";

    for (auto _ : state) {
        auto fut =
            engine_->setCurrentEvent(sessionId, SCE::SetCurrentEventArgs{"evt", jsonData, "internal", "", "", "", ""});
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("SetCurrentEventArgs overload, JSON ~40 chars (compile-fail + JSON-parse)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEvent8ArgLuaTable)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Lua table syntax — compile succeeds on L1 path
    const std::string luaData = "{a=1,b=2}";

    for (auto _ : state) {
        auto fut =
            engine_->setCurrentEvent(sessionId, SCE::SetCurrentEventArgs{"evt", luaData, "internal", "", "", "", ""});
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("SetCurrentEventArgs overload, Lua {a=1,b=2} (compile-success path)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEvent8ArgPlainText)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Plain text — fails L1 (Lua) and L2 (JSON), falls to L3 (normalize)
    const std::string plainData = "hello world";

    for (auto _ : state) {
        auto fut =
            engine_->setCurrentEvent(sessionId, SCE::SetCurrentEventArgs{"evt", plainData, "internal", "", "", "", ""});
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("SetCurrentEventArgs overload, plain text (compile-fail + JSON-fail + normalize)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEventWithTypedData)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Mirrors the production path: EventRaiserImpl pipeline-level pre-parses
    // JSON into typedData; LuaEngine 8-arg overload still runs full parse on
    // eventData string, then the shared_ptr overlay overwrites _event.data
    // with the typedData ScriptValue — the redundant work is what Issue 4
    // describes.
    const std::string jsonData = R"({"counter":42,"name":"hello","flag":true})";
    auto typedData = EventDataHelper::jsonStringToScriptValue(jsonData);

    for (auto _ : state) {
        auto event = std::make_shared<Event>("evt", "internal");
        event->setRawJsonData(jsonData);
        if (typedData.has_value()) {
            event->setTypedData(typedData.value());
        }
        auto fut = engine_->setCurrentEvent(sessionId, event);
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    // After Issue 4 AUTO-FIX this is the fast path: metadata + direct
    // ScriptValue push, skipping the 8-arg string-parse path entirely.
    // Pre-fix cost was ~3.1us/event (8-arg parse + overlay overwrite); the
    // delta against the JsonShapeRealistic 8-arg benchmark is the regression
    // guard for the typedData bypass.
    state.SetLabel("Event overload + typedData (fast path: metadata + direct push)");
}

BENCHMARK_F(LuaEngineFixture, SetCurrentEventWithoutTypedData)(benchmark::State &state) {
    std::string sessionId = generateUniqueLuaSessionId();
    engine_->createSession(sessionId);

    // Same flow as above but typedData absent — measures just the
    // Event-allocation + 8-arg overload (no overlay). Delta against the
    // typedData variant isolates the overlay cost; delta against the raw
    // 8-arg JSON benchmark isolates the shared_ptr<Event> allocation cost.
    const std::string jsonData = R"({"counter":42,"name":"hello","flag":true})";

    for (auto _ : state) {
        auto event = std::make_shared<Event>("evt", "internal");
        event->setRawJsonData(jsonData);
        auto fut = engine_->setCurrentEvent(sessionId, event);
        auto result = fut.get();
        benchmark::DoNotOptimize(result);
    }

    engine_->destroySession(sessionId);
    state.SetItemsProcessed(state.iterations());
    state.SetLabel("Event overload, no typedData (8-arg parse only)");
}

BENCHMARK_MAIN();
