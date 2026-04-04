// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// JMH benchmark: Data parsing — JSON, XML DOM, plain text
//
// Measures parseDataValue() performance for different content types.
// Used by <data>, <content>, <send>, and <invoke> elements.

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
open class DataParseBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private lateinit var sessionId: String

    // Test data payloads
    private val jsonSimple = """{"name": "test", "value": 42}"""
    private val jsonComplex = """{"users": [{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}], "count": 2}"""
    private val xmlSimple = """<data><item key="a">1</item><item key="b">2</item></data>"""
    private val plainText = "  hello   world   this  is  a  test  "

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
        sessionId = "bench_parse"
        scriptEngine.createSession(sessionId)
        scriptEngine.setupSystemVariables(sessionId, "benchmark")
    }

    @TearDown(Level.Trial)
    fun teardown() {
        scriptEngine.destroySession(sessionId)
    }

    /** JSON object parsing — W3C SCXML B.2 detection path 2 */
    @Benchmark
    fun parseJsonSimple(): Any? =
        scriptEngine.parseDataValue(sessionId, jsonSimple)

    /** Nested JSON with arrays — complex event data */
    @Benchmark
    fun parseJsonComplex(): Any? =
        scriptEngine.parseDataValue(sessionId, jsonComplex)

    /** XML DOM parsing — W3C SCXML B.2 detection path 1 */
    @Benchmark
    fun parseXml(): Any? =
        scriptEngine.parseDataValue(sessionId, xmlSimple)

    /** Plain text normalization — W3C SCXML B.2 detection path 3 */
    @Benchmark
    fun parsePlainText(): Any? =
        scriptEngine.parseDataValue(sessionId, plainText)
}
