// SPDX-License-Identifier: AGPL-3.0-only WITH AdditionRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-6.4, docs/adr/0098 — Kotlin AOT channel: a value an `<invoke>` hands
// its child is the initial value of the child's top-level `<data>` of that id,
// declared in place of the `<data>`'s own, which is then not evaluated.
//
// Each child reports what it saw to its parent, which keeps each report in a
// variable of its own and judges it, landing a failure in a `<final>` named for
// the sentence that broke; the test reads the verdict, and the kept values
// through the parent's typed readers. The fixture's own comment says what each
// `<data>` of each child is for.
//
// The fixture is run on every engine whatever the suite selected: QuickJS
// hands back a handle to an object it keeps, and Rhino the object itself, so
// both are always among the engines that hand a child an object.
//
// One case runs the same machines over an engine that refuses to hold one
// handed value: the session that cannot hold it raises error.execution for
// that `<data>`, as for an expression of its own that failed, and does not take
// the `<data>`'s own value instead.
//
// The handed values are consumed by the start they were handed for: a machine
// stopped and started again declares its `<data>` from its own value. That case
// runs a hand-written machine, since the fixture's child is not the test's to
// start twice.
//
// Fixture: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/materialize.sh kotlin-integration

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

/** The value the parent hands its child under `refusable`, which [RefusesOneHandedValue] will not hold. */
private const val UNHOLDABLE = "handed refusable"

/** What reaching each verdict `<final>` of the fixture says broke. */
private const val VERDICTS =
    "FailOwnValueRan / FailChildRaised: the own expression of a `<data>` whose value was handed " +
        "was evaluated. FailHandedValueLost / FailNamelistValueLost / FailNamelistOwnValueRaised: " +
        "a handed value did not take its `<data>`'s place. FailStateDataTookTheValue / " +
        "FailLateStateDataTookTheValue: a state's `<data>`, early- or late-bound, took a handed " +
        "value. FailUnmatchedNameAdded / FailNamelistNameAdded: a name no top-level `<data>` " +
        "declares became a variable. FailObjectNotWhole: an object lost its structure. " +
        "FailChildSharedTheParentsObject: the child was handed the parent's object, not a copy. " +
        "FailScriptReadItsOwnValue / FailEntryReadItsOwnValue: the top-level `<script>` or the " +
        "initial `<onentry>` ran before the values were declared. FailParentRaised: the parent " +
        "could not evaluate what it hands. null: a child never reported, or the child of " +
        "`firstMacrostep` read its own value in its first macrostep."

/**
 * The engine the suite runs on, refusing to hold one value: [UNHOLDABLE]
 * written to `refusable`, as a session refuses a value it cannot hold. Any
 * other value written there, the `<data>`'s own `'own refusable'` included, is
 * held, so a start that fell back to the `<data>`'s own value would be seen.
 * The members it does not name are the engine's own; the `ScriptSource` entry
 * points are not overridden
 * (`ScriptLanguageSeamTest.theLanguageContractIsNotAnEngineDetail`).
 */
private class RefusesOneHandedValue(
    private val engine: ScxmlScriptEngine = W3CTestBase.createEngine(),
) : ScxmlScriptEngine by engine {
    override fun setVariable(sessionId: String, name: String, value: Any?) {
        if (name == "refusable" && value == UNHOLDABLE) {
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

/** Run the fixture's parent over [engine] until it ends, and hand it to [check]. */
private fun ended(
    engine: ScxmlScriptEngine,
    check: (AChildTakesAHandedValueInPlaceOfItsDataStateMachine) -> Unit,
) {
    val sm = AChildTakesAHandedValueInPlaceOfItsDataStateMachine(engine)
    try {
        sm.initializeOnVirtualTime()
        sm.driveOnVirtualTime(3000L)
        check(sm)
    } finally {
        sm.cleanup()
    }
}

@DisplayName("A child takes a handed value in place of its data (§scxml-6.4, docs/adr/0098)")
class AChildTakesAHandedValueInPlaceOfItsDataTest {

    @Test
    fun `a handed value is declared in place of the data's own, which is not evaluated`() {
        ended(W3CTestBase.createEngine()) { sm ->
            assertEquals(
                AChildTakesAHandedValueInPlaceOfItsDataState.Pass,
                sm.terminalState,
                "errors_seen=${sm.errorsSeen()}. $VERDICTS",
            )
        }
    }

    @Test
    fun `an object a parent hands arrives whole on every engine, as a copy`() {
        // QuickJS keeps an object it evaluated inside the session that evaluated
        // it, and hands back a handle to it (`EngineHeldValue`), which the child's
        // session refuses; Rhino hands back the object itself, which a child
        // given it would share with its parent. The value is copied out where
        // the parent hands it, so it arrives whole and as the child's own: the
        // fixture lands in `failObjectNotWhole` or
        // `failChildSharedTheParentsObject` otherwise.
        for (name in W3CTestBase.KNOWN_ENGINES) {
            ended(W3CTestBase.engineFor(name)) { sm ->
                assertEquals(
                    AChildTakesAHandedValueInPlaceOfItsDataState.Pass,
                    sm.terminalState,
                    "$name: payload_seen=${sm.payloadSeen()} record_seen=${sm.recordSeen()}. $VERDICTS",
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
        ended(RefusesOneHandedValue()) { sm ->
            // The parent judges every other value of the child before its error
            // count, so this verdict says they were all in their places.
            assertEquals(
                AChildTakesAHandedValueInPlaceOfItsDataState.FailChildRaised,
                sm.terminalState,
                "the session refused the value handed to `refusable`, which is that `<data>`'s " +
                    "initialization error, and every other handed value is still declared. " +
                    VERDICTS,
            )
            assertEquals(
                1L,
                sm.errorsSeen(),
                "the refusal is the one error.execution of `refusable`'s initialization, and no " +
                    "other `<data>` raised",
            )
            assertEquals(
                "unbound",
                sm.refusableSeen(),
                "`refusable` holds a value: a `<data>` whose handed value was refused is left as " +
                    "one whose initialization failed, not given its own value instead",
            )
        }
    }
}
