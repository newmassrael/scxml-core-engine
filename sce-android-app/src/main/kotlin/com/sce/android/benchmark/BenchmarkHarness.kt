// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Android — Custom benchmark harness replacing JMH for Android runtime
//
// JMH depends on javax.annotation and JVM-specific fork/classloading that
// are unavailable on Android. This harness provides equivalent functionality:
// warmup phase (ART JIT compilation), measurement phase, and statistics.

package com.sce.android.benchmark

import com.sce.android.EngineFactory
import com.sce.android.EngineType
import com.sce.runtime.ScxmlScriptEngine

data class BenchmarkResult(
    val scenarioName: String,
    val category: String,
    val engineType: EngineType,
    val meanUs: Double,
    val medianUs: Double,
    val stddevUs: Double,
    val p99Us: Double,
    val minUs: Double,
    val maxUs: Double,
    val opsPerSec: Double
)

data class BenchmarkProgress(
    val currentScenario: String,
    val currentEngine: EngineType,
    val completedCount: Int,
    val totalCount: Int
)

class BenchmarkHarness(
    private val warmupIterations: Int = 500,
    private val measurementIterations: Int = 2000
) {

    /**
     * Run a single benchmark scenario on a given engine, returning timing statistics.
     *
     * Flow: setup -> warmup (discard timing) -> measure (collect nanos) -> teardown -> stats
     */
    fun run(
        scenario: BenchmarkScenario,
        engineType: EngineType
    ): BenchmarkResult {
        val engine = EngineFactory.create(engineType)
        val sessionId = "bench_${scenario.name}_${engineType.name}"

        // Setup
        engine.createSession(sessionId)
        engine.setupSystemVariables(sessionId, "benchmark")
        scenario.setup(engine, sessionId)

        try {
            val body = { scenario.body(engine, sessionId) }

            // Warmup: trigger ART JIT compilation
            repeat(warmupIterations) { body() }

            // Measurement: collect per-invocation timings
            val timingsNs = LongArray(measurementIterations)
            for (i in 0 until measurementIterations) {
                val start = System.nanoTime()
                body()
                timingsNs[i] = System.nanoTime() - start
            }

            // Compute statistics
            return computeStats(scenario.name, scenario.category, engineType, timingsNs)
        } finally {
            scenario.teardown(engine, sessionId)
            engine.destroySession(sessionId)
        }
    }

    private fun computeStats(
        name: String,
        category: String,
        engineType: EngineType,
        timingsNs: LongArray
    ): BenchmarkResult {
        val sorted = timingsNs.sorted()
        val count = sorted.size
        val sum = sorted.sum().toDouble()
        val meanNs = sum / count
        val medianNs = if (count % 2 == 0) {
            (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
        } else {
            sorted[count / 2].toDouble()
        }
        val p99Index = ((count - 1) * 0.99).toInt()
        val p99Ns = sorted[p99Index].toDouble()
        val minNs = sorted.first().toDouble()
        val maxNs = sorted.last().toDouble()

        val variance = sorted.sumOf { (it - meanNs) * (it - meanNs) } / count
        val stddevNs = kotlin.math.sqrt(variance)

        val meanUs = meanNs / 1000.0
        val opsPerSec = if (meanNs > 0) 1_000_000_000.0 / meanNs else 0.0

        return BenchmarkResult(
            scenarioName = name,
            category = category,
            engineType = engineType,
            meanUs = meanUs,
            medianUs = medianNs / 1000.0,
            stddevUs = stddevNs / 1000.0,
            p99Us = p99Ns / 1000.0,
            minUs = minNs / 1000.0,
            maxUs = maxNs / 1000.0,
            opsPerSec = opsPerSec
        )
    }
}
