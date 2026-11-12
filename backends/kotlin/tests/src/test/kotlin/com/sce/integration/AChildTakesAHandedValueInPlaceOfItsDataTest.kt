// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4, docs/adr/0098 — Kotlin AOT channel: a value an `<invoke>` hands
// its child is the initial value of the child's top-level `<data>` of that id,
// declared in place of the `<data>`'s own, which is then not evaluated.
//
// The child reports what it saw to its parent, which keeps each report in a
// variable of its own; the test reads them through the parent's typed readers.
// The fixture's own comment says what each `<data>` of the child is for.
//
// The second case runs the same machines over an engine that refuses to hold
// one handed value: the session that cannot hold it raises error.execution for
// that `<data>`, as for an expression of its own that failed, and does not take
// the `<data>`'s own value instead.
//
// The handed values are consumed by the start they were handed for: a machine
// stopped and started again declares its `<data>` from its own value. That case
// runs a hand-written machine, since the fixture's child is not the test's to
// start twice.
//
// Fixture: backends/kotlin/tests/fixtures/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_takes_a_handed_value_in_place_of_its_data_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data.AChildTakesAHandedValueInPlaceOfItsDataState
import com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data.AChildTakesAHandedValueInPlaceOfItsDataStateMachine
import com.sce.runtime.EnabledTransition
import com.sce.runtime.Event
import com.sce.runtime.HistoryId
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScriptSessionFailure
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.State
import com.sce.runtime.StateMachineEngine
import com.sce.w3c.W3CTestBase
import com.sce.w3c.driveOnVirtualTime
import com.sce.w3c.initializeOnVirtualTime
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.DisplayName
import org.junit.jupiter.api.Test

/** The value the parent hands its child under `refused`, which [RefusesOneHandedValue] will not hold. */
private const val UNHOLDABLE = "unholdable"

/**
 * The engine the suite runs on, refusing to hold one value: [UNHOLDABLE]
 * written to `refused`, as a session refuses a value it cannot hold. Any other
 * value written there, the `<data>`'s own `'own'` included, is held, so a start
 * that fell back to the `<data>`'s own value would be seen. The members it does
 * not name are the engine's own; the `ScriptSource` entry points are not
 * overridden (`ScriptLanguageSeamTest.theLanguageContractIsNotAnEngineDetail`).
 */
private class RefusesOneHandedValue(
    private val engine: ScxmlScriptEngine = W3CTestBase.createEngine(),
) : ScxmlScriptEngine by engine {
    override fun setVariable(sessionId: String, name: String, value: Any?) {
        if (name == "refused" && value == UNHOLDABLE) {
            throw ScriptEngineException("the double holds no '$name' of '$value'")
        }
        engine.setVariable(sessionId, name, value)
    }
}

private object SeedState : State

private sealed interface SeedEvent : Event {
    /** The `error.execution` a handed value the session cannot hold raises. */
    data object Failed : SeedEvent
}

/**
 * The smallest machine whose start declares a top-level `<data>` as a generated
 * one does: one state, and `<data id="seed" expr="'own'"/>`, which asks first
 * for a value its invoke handed it ([declareHandedValue]). It can hand another
 * machine values as a parent's `<invoke>` does, and read its own `seed`.
 */
private class SeedMachine(engine: ScxmlScriptEngine) : StateMachineEngine<SeedState, SeedEvent>(engine) {
    override val initialState: SeedState = SeedState

    override fun firstEnabledTransition(state: SeedState, event: SeedEvent?): EnabledTransition<SeedState, HistoryId>? = null

    override fun onEntry(state: SeedState, isDefaultEntry: Boolean) {}

    override fun onExit(state: SeedState) {}

    override fun executeTransitionContent(source: SeedState, transitionIndex: Int) {}

    override fun establishScriptSession(): ScriptSessionFailure? =
        makeScriptSession(scriptEngine ?: error("the machine is built with an engine"), "seed_machine")

    override fun declareDatamodel() {
        val engine = scriptEngine ?: error("the machine is built with an engine")
        val sid = scriptSessionId ?: error("the start established a session")
        if (!declareHandedValue(engine, sid, "seed", SeedEvent.Failed)) {
            engine.setVariable(sid, "seed", "own")
        }
    }

    /** Hand [child] [values] before it starts, as a parent's `<invoke>` does. */
    fun hand(child: SeedMachine, values: Map<String, Any?>) = setInvokeParams(child, values)

    /** What `seed` holds in the session of this machine's latest start. */
    fun seed(): Any? {
        val sid = scriptSessionId ?: return null
        return scriptEngine?.getVariable(sid, "seed")
    }
}

/** Run the fixture's parent over [engine] until its child has reported, and hand it to [check]. */
private fun reported(
    engine: ScxmlScriptEngine,
    check: (AChildTakesAHandedValueInPlaceOfItsDataStateMachine) -> Unit,
) {
    val sm = AChildTakesAHandedValueInPlaceOfItsDataStateMachine(engine)
    try {
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(3000L)
        assertEquals(
            AChildTakesAHandedValueInPlaceOfItsDataState.Reported,
            sm.terminalState,
            "the parent ends when its child reports, and its child reports once its " +
                "start has run: a parent left in `invoking` was told nothing",
        )
        check(sm)
    } finally {
        sm.cleanup()
    }
}

@DisplayName("A child takes a handed value in place of its data (§scxml-6.4, docs/adr/0098)")
class AChildTakesAHandedValueInPlaceOfItsDataTest {

    @Test
    fun `a handed value is declared in place of the data's own, which is not evaluated`() {
        reported(W3CTestBase.createEngine()) { sm ->
            assertEquals(
                "handed",
                sm.sawFailing(),
                "`failing` takes the value its invoke handed it",
            )
            assertEquals(
                "none",
                sm.sawErrors(),
                "the child raised error.execution in its start: `failing`'s own expression, " +
                    "which fails, was evaluated although a value was handed in its place",
            )
            assertEquals(
                "handed",
                sm.sawEffectful(),
                "`effectful` takes the value its invoke handed it",
            )
            assertEquals(
                "untouched",
                sm.sawEffects(),
                "`effects` was changed: `effectful`'s own expression, which assigns it, was " +
                    "evaluated although a value was handed in its place",
            )
            assertEquals(
                "own",
                sm.sawStateLocal(),
                "a state's `<data>` took a handed value: only the top-level `<data>` take one",
            )
            assertEquals(
                "undefined",
                sm.sawNowhere(),
                "a handed name the child declares nowhere became a variable of the child",
            )
            assertEquals(
                "whole",
                sm.sawRecord(),
                "the object the parent handed did not arrive whole, its inner object and " +
                    "array included",
            )
            assertEquals(
                "handed",
                sm.sawByScript(),
                "the top-level `<script>` read `failing` before the handed value was declared",
            )
            assertEquals(
                "handed",
                sm.sawByEntry(),
                "the initial `<onentry>` read `effectful` before the handed value was declared",
            )
            assertEquals(
                "string",
                sm.sawRefused(),
                "`refused` takes the value its invoke handed it",
            )
        }
    }

    @Test
    fun `an object a parent hands arrives whole on every engine, as a copy`() {
        // QuickJS keeps an object it evaluated inside the session that evaluated
        // it, and hands back a handle to it (`EngineHeldValue`), which the child's
        // session refuses; Rhino hands back the object itself, which a child
        // given it would share with its parent. The value is copied out where
        // the parent hands it, so it arrives whole and as the child's own. Every
        // engine is run whatever the suite selected, so both are always among
        // them.
        for (name in W3CTestBase.KNOWN_ENGINES) {
            reported(W3CTestBase.engineFor(name)) { sm ->
                assertEquals(
                    "whole",
                    sm.sawRecord(),
                    "$name: the object the parent handed did not arrive whole in the child",
                )
                assertEquals(
                    "whole",
                    sm.sawParentRecord(),
                    "$name: the name the child wrote into the object it was handed is what the " +
                        "parent's `record` holds, so the child was handed the parent's object " +
                        "rather than a copy of it",
                )
                assertEquals(
                    "none",
                    sm.sawErrors(),
                    "$name: the child raised error.execution in its start, so a handed value " +
                        "it could not hold, or a `<data>`'s own expression, failed",
                )
            }
        }
    }

    @Test
    fun `a later start of the same machine declares the data's own value`() {
        // The values an invoke hands are the arguments of the invoke's start of
        // the child, consumed by it: a host that stops the same object and
        // starts it again starts it from the document's own values, as the
        // Swift and Python children do.
        val engine = W3CTestBase.createEngine()
        val parent = SeedMachine(engine)
        val child = SeedMachine(engine)
        try {
            parent.hand(child, mapOf("seed" to "handed"))
            child.initialize()
            assertEquals("handed", child.seed(), "the invoke's start of the child takes the handed value")
            child.stop()
            child.initialize()
            assertEquals(
                "own",
                child.seed(),
                "a second start of the same machine took the value handed for the first: the " +
                    "start that declared it did not consume it",
            )
        } finally {
            child.cleanup()
            parent.cleanup()
        }
    }

    @Test
    fun `a handed value the session cannot hold is the error of its data`() {
        reported(RefusesOneHandedValue()) { sm ->
            assertEquals(
                "one",
                sm.sawErrors(),
                "the session refused the value handed to `refused`, which is that `<data>`'s " +
                    "initialization error: one error.execution, and no other",
            )
            assertEquals(
                "undefined",
                sm.sawRefused(),
                "`refused` holds a value: a `<data>` whose handed value was refused is left as " +
                    "one whose own expression failed, not given its own value instead",
            )
            assertEquals(
                "handed",
                sm.sawFailing(),
                "the refusal of one handed value reached another `<data>`",
            )
        }
    }
}
