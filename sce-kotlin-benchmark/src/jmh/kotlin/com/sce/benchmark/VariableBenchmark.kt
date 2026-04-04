// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// JMH benchmark: Variable operations — set, get, assign
//
// Maps to C++ JSEngineBenchmark::VariableOperations.
// Data model manipulation throughput for data-intensive state machines.

package com.sce.benchmark

import com.sce.runtime.ScxmlScriptEngine
import org.openjdk.jmh.annotations.*
import java.util.concurrent.TimeUnit

@BenchmarkMode(Mode.AverageTime, Mode.Throughput)
@OutputTimeUnit(TimeUnit.MICROSECONDS)
@State(Scope.Thread)
@Warmup(iterations = 5, time = 1, timeUnit = TimeUnit.SECONDS)
@Measurement(iterations = 10, time = 1, timeUnit = TimeUnit.SECONDS)
@Fork(2)
open class VariableBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private lateinit var sessionId: String

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
        sessionId = "bench_var"
        scriptEngine.createSession(sessionId)
        scriptEngine.setupSystemVariables(sessionId, "benchmark")

        // Pre-declare variables
        scriptEngine.setVariable(sessionId, "target", 0)
        scriptEngine.setVariable(sessionId, "counter", 100)
    }

    @TearDown(Level.Trial)
    fun teardown() {
        scriptEngine.destroySession(sessionId)
    }

    /** Set a single variable — W3C SCXML <data> initialization */
    @Benchmark
    fun setVariable(): Unit {
        scriptEngine.setVariable(sessionId, "target", 42)
    }

    /** Get a single variable — reading datamodel state */
    @Benchmark
    fun getVariable(): Any? =
        scriptEngine.getVariable(sessionId, "counter")

    /** Assign with expression — W3C SCXML <assign> with expr */
    @Benchmark
    fun assignExpression(): Unit {
        scriptEngine.assign(sessionId, "target", "counter + 1")
    }

    /** Check variable existence — W3C SCXML invoke param validation */
    @Benchmark
    fun hasVariable(): Boolean =
        scriptEngine.hasVariable(sessionId, "counter")

    /** Batch: initialize 5 variables — typical SCXML <datamodel> setup */
    @Benchmark
    fun initializeDataModel(): Unit {
        scriptEngine.setVariable(sessionId, "v1", 0)
        scriptEngine.setVariable(sessionId, "v2", "text")
        scriptEngine.setVariable(sessionId, "v3", true)
        scriptEngine.setVariable(sessionId, "v4", 3.14)
        scriptEngine.setVariable(sessionId, "v5", null)
    }
}
