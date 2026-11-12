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
// Fixture: backends/kotlin/tests/fixtures/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml
//
// Regeneration (after fixture or template edit):
//   scripts/regen_a_child_takes_a_handed_value_in_place_of_its_data_kotlin.sh

package com.sce.integration

import com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data.AChildTakesAHandedValueInPlaceOfItsDataState
import com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data.AChildTakesAHandedValueInPlaceOfItsDataStateMachine
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
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
    fun `an object a parent hands arrives whole on every engine`() {
        // QuickJS keeps an object it evaluated inside the session that evaluated
        // it, and hands back a handle to it (`EngineHeldValue`), which the child's
        // session refuses; the value is copied out where the parent hands it.
        // Every engine is run whatever the suite selected, so the one that hands
        // a handle is always among them.
        for (name in W3CTestBase.KNOWN_ENGINES) {
            reported(W3CTestBase.engineFor(name)) { sm ->
                assertEquals(
                    "whole",
                    sm.sawRecord(),
                    "$name: the object the parent handed did not arrive whole in the child",
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
