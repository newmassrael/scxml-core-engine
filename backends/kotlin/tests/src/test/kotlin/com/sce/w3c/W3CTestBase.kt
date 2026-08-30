// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// W3C SCXML Conformance Test Harness for Kotlin AOT State Machines

package com.sce.w3c

import com.sce.runtime.Event
import com.sce.runtime.ManualClock
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

        /**
         * Default when nothing asks for one.
         *
         * ⚠ It is not a preference. The committed machines under
         * `com/sce/generated/` are emitted for ONE language — whichever
         * `Language::Kotlin.default_script_engine_target()` answers — and a
         * run with no `-Psce.script.engine` hands them to whatever this
         * names. So this constant has to name an engine that ACCEPTS that
         * language, or the suite's own default run is the mis-supply
         * `ScriptSource` exists to prevent.
         *
         * It said `rhino` until 2026-08-30, correctly, while the backend's
         * default artifact carried the author's ECMAScript. That default
         * moved to Lua, and Rhino refuses Lua — so this moved with it. The
         * pairing is not left to this comment: `scripts/gates/w3c-kotlin.sh`
         * asks the generator's manifest which language the committed machines
         * hold and refuses a run whose default engine does not take it.
         *
         * The Lua engine is the right default for a second reason it is worth
         * naming: it accepts BOTH languages (`acceptsLanguage`), so it is the
         * only one of the three that keeps working whichever way the artifact
         * default moves next.
         */
        const val DEFAULT_ENGINE: String = "lua"

        /**
         * Spelled once, and read by [createEngine]'s refusal and by the test
         * that pins it. A second list would be free to disagree with the
         * `when` above, which is the drift this whole change is about.
         */
        val KNOWN_ENGINES: List<String> = listOf("rhino", "quickjs", "lua")

        /**
         * How many times the drive loop will advance virtual time before it
         * calls the machine stuck.
         *
         * A budget in STEPS rather than in milliseconds, because virtual time
         * does not bound the loop on its own: a `<send>` that re-arms itself
         * with `delay="0s"` is always due now, so the loop would tick forever
         * without the clock ever moving. Generous enough that no conforming
         * document reaches it — the corpus's busiest run advances a handful of
         * times — and finite so a runaway fails with a message instead of
         * being killed by a JUnit timeout, which would put the wall clock back
         * in the verdict.
         */
        const val MAX_VIRTUAL_STEPS: Int = 10_000
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

        // §scxml-6.2: this harness OWNS time rather than racing it.
        //
        // The clock has to be installed before `initialize()`, because entering
        // the initial configuration arms every `<send delay>` in it against
        // whatever clock is there at that moment — the setter refuses a swap
        // afterwards for exactly that reason.
        sm.clock = ManualClock()
        sm.initialize()

        // The drive loop below replaced a wall-clock one
        // (`System.currentTimeMillis() + timeoutMs`, polled with
        // `Thread.sleep(10); tick()`). That loop made the verdict a race
        // between two real-time quantities that have nothing to do with the
        // clause under test: how fast this thread is scheduled, and the
        // document's OWN failure timer.
        //
        // 154 of the corpus's documents arm one — `<send event="timeout"
        // delay="Ns"/>`, the shortest at 2s — so a suite that is descheduled
        // for two seconds reads a conforming engine as failing. Measured
        // 2026-08-25: `availableProcessors()` reports 32 whatever else is
        // running, because this machine sets no cgroup quota, so the suite
        // sizes itself for a machine it does not have and loses that race
        // under load. Raising the budget only moves the mark; it does not
        // stop the document's timer, which is armed in the same real seconds.
        //
        // Virtual time removes the race outright: nothing is due until this
        // loop says so, and the same sequence of calls produces the same
        // configuration on an idle machine and a loaded one.
        if (!sm.isInFinalState) {
            var remaining = timeoutMs
            var steps = 0
            while (!sm.isInFinalState) {
                // Tick before advancing. Not every reason a machine is not
                // finished yet is on the clock: an invoked child that has
                // nothing SCHEDULED still needs its parent's tick to run its
                // queues and to be noticed finishing, and it reports no
                // deadline for that. The wall-clock loop this replaced ticked
                // every 10ms whether anything was due or not, so it never had
                // to say so; a loop that only moves when a deadline says to
                // must.
                sm.tick()
                if (sm.isInFinalState) {
                    break
                }
                // `null` is "nothing is owed" — by this machine or any child
                // it is responsible for ticking. With the tick above already
                // done, no amount of further time can change the answer.
                val due = sm.timeUntilNextScheduledMs() ?: break
                if (due > remaining) break
                sm.advanceTimeMs(due)
                remaining -= due
                if (++steps > MAX_VIRTUAL_STEPS) {
                    throw AssertionError(
                        "the machine asked to be ticked $MAX_VIRTUAL_STEPS times without " +
                            "reaching a final state or consuming its ${timeoutMs}ms budget; " +
                            "a send that re-arms itself with no delay is the usual cause"
                    )
                }
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
