// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// JMH benchmark: Script execution — varying complexity levels
//
// Maps to C++ JSEngineBenchmark::ScriptComplexity.
// Measures overhead of <script> blocks in onentry/onexit handlers.

package com.sce.benchmark

import com.sce.runtime.ScxmlScriptEngine
import org.openjdk.jmh.annotations.*
import java.util.concurrent.TimeUnit

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.MICROSECONDS)
@State(Scope.Thread)
@Warmup(iterations = 5, time = 1, timeUnit = TimeUnit.SECONDS)
@Measurement(iterations = 10, time = 1, timeUnit = TimeUnit.SECONDS)
@Fork(2)
open class ScriptBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private lateinit var sessionId: String

    // Pre-generated scripts of varying complexity
    private lateinit var tinyScript: String
    private lateinit var smallScript: String
    private lateinit var mediumScript: String
    private lateinit var largeScript: String

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
        sessionId = "bench_script"
        scriptEngine.createSession(sessionId)
        scriptEngine.setupSystemVariables(sessionId, "benchmark")

        // Initialize accumulator
        scriptEngine.executeScript(sessionId, "var result = 0;")

        tinyScript = "result = result + 1;"

        smallScript = buildString {
            appendLine("var sum = 0;")
            for (i in 1..5) appendLine("sum = sum + $i;")
            appendLine("result = sum;")
        }

        mediumScript = buildString {
            appendLine("var arr = [];")
            appendLine("for (var i = 0; i < 20; i++) { arr.push(i * 2); }")
            appendLine("var total = 0;")
            appendLine("for (var j = 0; j < arr.length; j++) { total = total + arr[j]; }")
            appendLine("result = total;")
        }

        largeScript = buildString {
            appendLine("var data = {};")
            for (i in 0 until 50) appendLine("data['key$i'] = $i * $i;")
            appendLine("var sum = 0;")
            appendLine("for (var k in data) { sum = sum + data[k]; }")
            appendLine("result = sum;")
        }
    }

    @TearDown(Level.Trial)
    fun teardown() {
        scriptEngine.destroySession(sessionId)
    }

    /** 1-line script — minimum overhead for <script> execution */
    @Benchmark
    fun tiny(): Unit {
        scriptEngine.executeScript(sessionId, tinyScript)
    }

    /** 7-line script — typical small onentry handler */
    @Benchmark
    fun small(): Unit {
        scriptEngine.executeScript(sessionId, smallScript)
    }

    /** 5-line script with loops and arrays — moderate complexity */
    @Benchmark
    fun medium(): Unit {
        scriptEngine.executeScript(sessionId, mediumScript)
    }

    /** 50+ line script with object manipulation — complex handler */
    @Benchmark
    fun large(): Unit {
        scriptEngine.executeScript(sessionId, largeScript)
    }
}
