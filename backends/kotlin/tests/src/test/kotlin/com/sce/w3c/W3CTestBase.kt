// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML Conformance Test Harness for Kotlin AOT State Machines

package com.sce.w3c

import com.sce.runtime.Event
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/**
 * Abstract base class for W3C SCXML conformance tests.
 *
 * Matches C++ AOT test harness (SimpleAotTest / ScheduledAotTest):
 *   - [createStateMachine]: Factory for the generated StateMachineEngine
 *   - [expectedPassState]: The "pass" final state object
 *
 * Execution model (C++ AOT parity):
 *   - sm.initialize() — synchronous, all microsteps complete immediately
 *   - Simple tests: SM reaches final state during initialize
 *   - Scheduled tests: poll with sm.tick() for delayed sends/invokes
 *
 * Script engine selection via system property "sce.script.engine":
 *   - "rhino" (default): RhinoScriptEngine (JVM, pure Java)
 *   - "quickjs": QuickJSScriptEngine (ECMA-262 via JNI)
 *   - "lua": LuaScriptEngine (Lua 5.4 via JNI)
 *
 * A name outside that set is refused rather than defaulted. The set used to
 * end in `else -> RhinoScriptEngine()`, which made a misspelt engine a green
 * run of the default one — so a CI lane declaring an engine it never got
 * would report that engine passing 226 cases. Measured: `-Psce.script.engine=
 * nonexistent-engine` passed. What a lane claims to cover and what it covers
 * have to be the same thing, and the only place that can be checked is here.
 *
 * @param S State sealed interface type
 * @param E Event sealed interface type
 */
abstract class W3CTestBase<S : State, E : Event> {

    abstract fun createStateMachine(): StateMachineEngine<S, E>
    abstract val expectedPassState: S

    companion object {
        /**
         * Create a ScxmlScriptEngine based on system property "sce.script.engine".
         *
         * Both engines are compile-time dependencies of sce-kotlin-tests.
         * Generated test code calls this:
         *   override fun createStateMachine() = XxxStateMachine(createEngine())
         */
        /** The engine this run was asked for, defaulted but never guessed. */
        fun requestedEngine(): String =
            System.getProperty("sce.script.engine", DEFAULT_ENGINE).lowercase()

        fun createEngine(): ScxmlScriptEngine = engineFor(requestedEngine())

        /**
         * The engine a name selects.
         *
         * Split from [createEngine] so the mapping can be asserted without
         * writing the system property: this suite runs JUnit in parallel, and
         * a test that set `sce.script.engine` to observe the result was read
         * by whichever sibling looked at it next. That was measured, not
         * feared — the first version of `ScriptEngineSelectionTest` failed
         * with `expected: <rhino> but was: <nonexistent-engine>`, one test
         * seeing another's write.
         */
        fun engineFor(engineType: String): ScxmlScriptEngine = when (engineType) {
            "rhino" -> RhinoScriptEngine()
            "lua" -> LuaScriptEngine()
            "quickjs" -> QuickJSScriptEngine()
            else -> throw IllegalArgumentException(
                "sce.script.engine=\"$engineType\" names no engine this suite can build. " +
                    "Known: ${KNOWN_ENGINES.joinToString(", ")}. Refused rather than " +
                    "defaulted, because a run that quietly substituted another engine " +
                    "would report that engine's result under this one's name."
            )
        }

        /** Default when nothing asks for one. */
        const val DEFAULT_ENGINE: String = "rhino"

        /**
         * Spelled once, and read by [createEngine]'s refusal and by the test
         * that pins it. A second list would be free to disagree with the
         * `when` above, which is the drift this whole change is about.
         */
        val KNOWN_ENGINES: List<String> = listOf("rhino", "quickjs", "lua")
    }

    /**
     * How long this harness polls a machine that did not finish during
     * [StateMachineEngine.initialize].
     *
     * Three seconds, which is what the Rust and Go drivers already give the
     * same documents: `generate_test_file` in `sce_codegen.rs` emits 3s for a
     * simple test and 5s for a scheduled one, and that same source emits the
     * `override val timeoutMs = 5000L` on this class's scheduled subclasses.
     * This value was the one half of the Kotlin budget that had not been
     * brought into line, so the lane gave a simple test less room than its
     * siblings did and less than its own scheduled tests got.
     *
     * Why it matters that the two agree: a W3C document that can fail arms its
     * own failure timer with `<send event="timeout" delay="Ns"/>`, and the
     * shortest such timer in the corpus is 2s. A polling budget EQUAL to the
     * document's own timer is a dead heat — whether the run passes depends on
     * how many `tick()`s get scheduled inside that window, which is a property
     * of the machine rather than of the clause under test.
     *
     * Stated plainly because the history is easy to misread: raising this from
     * 2000L did NOT fix test253. That test failed on `main` and went on
     * failing at 3000L, because a longer polling budget cannot help once the
     * document's own timer has already fired. Its cause was the suite running
     * a thousand tests at once on thirty-two processors — see the parallelism
     * comment in `backends/kotlin/tests/build.gradle.kts`, which is what fixed
     * it. This value is parity, and parity is worth having on its own.
     */
    open val timeoutMs: Long = 3000L

    @Test
    open fun testW3CConformance() {
        val sm = createStateMachine()
        sm.initialize()

        // C++ ScheduledAotTest pattern: poll for delayed sends/invokes
        if (!sm.isInFinalState) {
            val deadline = System.currentTimeMillis() + timeoutMs
            while (!sm.isInFinalState && System.currentTimeMillis() < deadline) {
                Thread.sleep(10)
                sm.tick()
            }
        }

        try {
            assertEquals(
                expectedPassState,
                sm.currentState.value,
                "W3C conformance failed: expected Pass but reached ${sm.currentState.value}"
            )
        } finally {
            sm.cleanup()
        }
    }
}
