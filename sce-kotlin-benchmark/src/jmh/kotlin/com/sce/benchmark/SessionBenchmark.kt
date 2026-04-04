// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// JMH benchmark: Session lifecycle — create/setup/destroy overhead
//
// Maps to C++ JSEngineBenchmark::SessionCreation.
// Critical for AOSP/AAOS app launch latency — session creation happens
// once per state machine instantiation.

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
open class SessionBenchmark {

    @Param("rhino", "lua", "quickjs")
    lateinit var engine: String

    private lateinit var scriptEngine: ScxmlScriptEngine
    private var counter = 0

    @Setup(Level.Trial)
    fun setup() {
        scriptEngine = EngineFactory.create(engine)
    }

    /**
     * Full session lifecycle: create + system variable setup + destroy.
     * This is the overhead every StateMachineEngine.initialize() pays.
     */
    @Benchmark
    fun createSetupDestroy(): Boolean {
        val sid = "bench_session_${++counter}"
        scriptEngine.createSession(sid)
        scriptEngine.setupSystemVariables(sid, "benchmark")
        scriptEngine.destroySession(sid)
        return true
    }

    /**
     * Bare session creation without system variable setup.
     * Isolates the native context allocation cost (JNI for Lua/QuickJS).
     */
    @Benchmark
    fun createDestroyOnly(): Boolean {
        val sid = "bench_bare_${++counter}"
        scriptEngine.createSession(sid)
        scriptEngine.destroySession(sid)
        return true
    }
}
