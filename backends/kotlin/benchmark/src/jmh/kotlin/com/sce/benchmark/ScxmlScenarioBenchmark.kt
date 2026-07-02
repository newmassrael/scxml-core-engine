// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// JMH benchmark: Realistic SCXML scenarios — end-to-end operation patterns
//
// Simulates actual state machine runtime patterns:
// - Event processing (setCurrentEvent → guards → actions → clear)
// - foreach iteration
// - In() predicate evaluation
// These are integration-level benchmarks reflecting real AOSP/AAOS workloads.

package com.sce.benchmark

import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.openjdk.jmh.annotations.*
import java.util.concurrent.TimeUnit

@BenchmarkMode(Mode.AverageTime, Mode.Throughput)
@OutputTimeUnit(TimeUnit.MICROSECONDS)
@State(Scope.Thread)
@Warmup(iterations = 5, time = 1, timeUnit = TimeUnit.SECONDS)
@Measurement(iterations = 10, time = 1, timeUnit = TimeUnit.SECONDS)
@Fork(2)
open class ScxmlScenarioBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private lateinit var sessionId: String

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
        sessionId = "bench_scxml"
        scriptEngine.createSession(sessionId)
        scriptEngine.setupSystemVariables(sessionId, "benchmark")

        // Register In() predicate — simulates active state configuration
        val activeStates = setOf("s0", "s1", "running")
        scriptEngine.setStateQueryCallback(sessionId) { stateId -> stateId in activeStates }

        // Initialize datamodel — typical SCXML <datamodel> block
        scriptEngine.setVariable(sessionId, "Var1", 0)
        scriptEngine.setVariable(sessionId, "Var2", 10)
        scriptEngine.setVariable(sessionId, "Var3", "hello")
        scriptEngine.executeScript(sessionId, "var items = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];")
        scriptEngine.executeScript(sessionId, "var itemResult = 0;")
    }

    @TearDown(Level.Trial)
    fun teardown() {
        scriptEngine.setStateQueryCallback(sessionId, null)
        scriptEngine.destroySession(sessionId)
    }

    /**
     * Event processing cycle — the core SCXML microstep pattern.
     *
     * Every external event triggers: setCurrentEvent → evaluate guards →
     * execute actions → clearCurrentEvent. This is the hottest path.
     */
    @Benchmark
    fun eventProcessingCycle(): Boolean {
        // 1. Set event metadata (W3C SCXML 5.10)
        scriptEngine.setCurrentEvent(
            sessionId,
            SetCurrentEventArgs(
                name = "user.click",
                data = "",
                type = "external",
                sendId = "",
                origin = "#_scxml_bench",
                originType = "http://www.w3.org/TR/scxml/#SCXMLEventProcessor",
                invokeId = ""
            )
        )

        // 2. Evaluate guard conditions (W3C SCXML 5.9)
        val guard = scriptEngine.evaluateCondition(sessionId, "_event.name == 'user.click'")

        // 3. Execute onentry action (W3C SCXML 3.8)
        if (guard) {
            scriptEngine.assign(sessionId, "Var1", "Var1 + 1")
        }

        // 4. Clear event (before next macrostep)
        scriptEngine.clearCurrentEvent(sessionId)

        return guard
    }

    /**
     * Guard + transition — just the condition evaluation part of a microstep.
     * Isolates guard overhead from event setup.
     */
    @Benchmark
    fun guardWithVariables(): Boolean =
        scriptEngine.evaluateCondition(sessionId, "Var1 >= 0 && Var2 < 100 && Var3 == 'hello'")

    /**
     * In() predicate — tests whether a state is in the active configuration.
     * W3C SCXML 5.9.2: used in compound guard conditions.
     */
    @Benchmark
    fun inPredicate(): Boolean =
        scriptEngine.evaluateCondition(sessionId, "In('running')")

    /**
     * Foreach iteration — W3C SCXML 4.6 <foreach> over 10 elements.
     * Common pattern for processing event data arrays.
     */
    @Benchmark
    fun foreachIteration(): Unit {
        scriptEngine.executeForeach(
            sessionId,
            array = "items",
            item = "x",
            index = "idx",
            body = {
                // Typical loop body: accumulate into a variable
                scriptEngine.assign(sessionId, "itemResult", "itemResult + x")
            }
        )
    }

    /**
     * Full microstep simulation — realistic SCXML transition processing.
     *
     * Pattern: event arrives → try multiple transitions → first match wins →
     * execute exit handlers → execute transition actions → execute entry handlers.
     */
    @Benchmark
    fun fullMicrostep(): Unit {
        // Event arrives
        scriptEngine.setCurrentEvent(
            sessionId,
            SetCurrentEventArgs(
                name = "timer.elapsed",
                data = "{\"count\": 5}",
                type = "external"
            )
        )

        // Try transition 1 (not matching)
        scriptEngine.evaluateCondition(sessionId, "_event.name == 'error'")

        // Try transition 2 (matching)
        val matched = scriptEngine.evaluateCondition(sessionId, "_event.name == 'timer.elapsed' && Var2 > 0")

        if (matched) {
            // Exit handler: save state
            scriptEngine.executeScript(sessionId, "var _saved = Var1;")

            // Transition action: update datamodel
            scriptEngine.assign(sessionId, "Var1", "Var1 + 1")
            scriptEngine.assign(sessionId, "Var2", "Var2 - 1")

            // Entry handler: log
            scriptEngine.executeScript(sessionId, "var _entered = true;")
        }

        scriptEngine.clearCurrentEvent(sessionId)
    }

    /**
     * Data model initialization — simulates <datamodel> processing at start.
     * Includes variable declaration, expression evaluation, and script execution.
     */
    @Benchmark
    fun dataModelInit(): Unit {
        scriptEngine.setVariable(sessionId, "d1", 0)
        scriptEngine.setVariable(sessionId, "d2", null)
        scriptEngine.assign(sessionId, "d1", "42 * 2")
        scriptEngine.executeScript(sessionId, "var d3 = [1, 2, 3]; var d4 = {'key': 'value'};")
        scriptEngine.evaluateExpr(sessionId, "d1 + 1")
    }
}
