// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// JMH benchmark: Expression evaluation — guard conditions, arithmetic, strings
//
// Maps to C++ JSEngineBenchmark::SimpleExpression.
// Guard condition evaluation is on the CRITICAL PATH of every state transition.
// This is the single most important benchmark for engine selection.

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
open class ExpressionBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private lateinit var sessionId: String

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
        sessionId = "bench_expr"
        scriptEngine.createSession(sessionId)
        scriptEngine.setupSystemVariables(sessionId, "benchmark")

        // Pre-populate datamodel variables used in condition benchmarks
        scriptEngine.setVariable(sessionId, "Var1", 42)
        scriptEngine.setVariable(sessionId, "Var2", 7)
        scriptEngine.setVariable(sessionId, "Var3", "hello")
        scriptEngine.executeScript(sessionId, "var counter = 0;")
    }

    @TearDown(Level.Trial)
    fun teardown() {
        scriptEngine.destroySession(sessionId)
    }

    // -- Guard condition evaluation (SCXML critical path) --

    /** Simple numeric comparison — most common guard pattern in SCXML */
    @Benchmark
    fun conditionSimple(): Boolean =
        scriptEngine.evaluateCondition(sessionId, "Var1 == 42")

    /** Compound boolean — multi-variable guard */
    @Benchmark
    fun conditionCompound(): Boolean =
        scriptEngine.evaluateCondition(sessionId, "Var1 > 0 && Var2 < 100")

    /** Negation + type coercion — tests truthiness handling */
    @Benchmark
    fun conditionNegation(): Boolean =
        scriptEngine.evaluateCondition(sessionId, "!(Var1 == 0)")

    // -- Expression evaluation --

    /** Simple arithmetic — baseline for expression engine overhead */
    @Benchmark
    fun exprArithmetic(): Any? =
        scriptEngine.evaluateExpr(sessionId, "1 + 2 * 3")

    /** Math built-ins — exercises native function dispatch */
    @Benchmark
    fun exprMathBuiltins(): Any? =
        scriptEngine.evaluateExpr(sessionId, "Math.sqrt(144) + Math.pow(2, 10)")

    /** String concatenation — common in event data construction */
    @Benchmark
    fun exprStringConcat(): Any? =
        scriptEngine.evaluateExpr(sessionId, "'hello' + ' ' + 'world'")

    /** Variable reference + arithmetic — typical assign expression */
    @Benchmark
    fun exprVarArithmetic(): Any? =
        scriptEngine.evaluateExpr(sessionId, "Var1 + Var2 * 2")

    /** Ternary / conditional expression */
    @Benchmark
    fun exprTernary(): Any? =
        scriptEngine.evaluateExpr(sessionId, "Var1 > 0 ? Var1 : -Var1")
}
