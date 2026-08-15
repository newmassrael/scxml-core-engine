// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The one expression every backend's structured `<data>` reader builds,
// measured against every engine that reader can be handed.
//
// A `<data id="rules" expr="[…]"/>` gets a typed accessor like any other
// declaration, and the value crosses to the host as the text the session's own
// `JSON.stringify` produces (W3C SCXML B.2). So each of the six runtimes emits
// the same thing — `JSON.stringify(rules)` — and hands it to whichever engine
// the deployment injected.
//
// That is a claim across two languages, and this file is where it is put to
// the test. `evaluateExpr` takes the ENGINE's language, not the document's: a
// Lua-backed session is handed Lua, which is why generated initialisers carry
// Lua table syntax. `JSON.stringify(x)` happens to be spelled the same in
// both — member access and a call — in a language where §scxml-B-2 requires
// that exact name to exist. If that stops being true for any engine, every
// backend's reader stops answering, and it would otherwise be found by
// whichever backend a consumer happened to be on.
//
// Kotlin is where the claim is measurable, for the reason
// `EcmaScriptSemanticsTest` gives: a generated machine takes its engine as a
// constructor argument and this backend ships three, so all three can be
// asked in one place. No single runtime can reach that many.
//
// Lua is included here and deliberately excluded from the ECMA-262 table
// beside it. The two are not in tension: that table asks whether an engine
// answers what ECMAScript answers, and Lua does not. This asks whether the
// reader's expression reaches the shared JSON builtin, and it must, because
// SCE offers Lua and a document read through it has to be readable.

package com.sce.ecmascript

import com.sce.runtime.DatamodelRead
import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNull
import kotlin.test.assertTrue

class DatamodelReadJsonTest {

    /**
     * How a session is given the value, per engine — the setup, not the
     * measurement.
     *
     * This half IS engine-specific, and saying so is the point: codegen
     * translates a document's ECMAScript initialiser into the engine's
     * language before it ever reaches a session. What must not be
     * engine-specific is the reader, which is what the assertions below are
     * about.
     */
    private class Engine(
        val name: String,
        val arrayLiteral: String,
        val create: () -> ScxmlScriptEngine,
    )

    private val engines = listOf(
        Engine("Rhino", "[{ when: 'design-decision', keys: 'Escape' }]") { RhinoScriptEngine() },
        Engine("QuickJS", "[{ when: 'design-decision', keys: 'Escape' }]") { QuickJSScriptEngine() },
        Engine("Lua", "{{ when = 'design-decision', keys = 'Escape' }}") { LuaScriptEngine() },
    )

    /**
     * Every engine serves the expression the readers build.
     *
     * Asserted per engine rather than once over a chosen one: the reader does
     * not know which engine it is talking to, so a claim about "the engine"
     * is only true if it is true of each of them.
     */
    @Test
    fun everyEngineServesTheExpressionTheReadersBuild() {
        for (engine in engines) {
            withSession(engine) { instance, sessionId ->
                instance.executeScript(sessionId, "rules = ${engine.arrayLiteral}")

                val json = DatamodelRead.readJson(instance, sessionId, "rules")
                assertTrue(
                    json != null,
                    "${engine.name}: the reader's expression `JSON.stringify(rules)` did not " +
                        "come back as JSON text. Every backend emits that one expression for a " +
                        "structured `<data>`, so an engine that does not serve it makes the " +
                        "declaration unreadable on all six.",
                )
                assertTrue(
                    json!!.startsWith("["),
                    "${engine.name}: an authored array must come back as one: $json",
                )
                assertTrue(
                    "design-decision" in json && "Escape" in json,
                    "${engine.name}: the value the session holds is missing from what the " +
                        "reader answered: $json",
                )
            }
        }
    }

    /**
     * Every engine refuses a value of another type, the way the scalar
     * readers do.
     *
     * `5` stringifies to `5`, which is valid JSON — so a reader that forwarded
     * whatever the serialiser produced would hand a consumer a shape the
     * document no longer has. The refusal is decided by the first character of
     * the output, and that has to hold on each engine because each has its own
     * serialiser.
     */
    @Test
    fun everyEngineRefusesAValueThatIsNoLongerStructured() {
        for (engine in engines) {
            withSession(engine) { instance, sessionId ->
                instance.executeScript(sessionId, "rules = 5")
                assertNull(
                    DatamodelRead.readJson(instance, sessionId, "rules"),
                    "${engine.name}: a variable declared structured and now holding a number " +
                        "must report that the machine cannot answer",
                )

                instance.executeScript(sessionId, "rules = 'not a table'")
                assertNull(
                    DatamodelRead.readJson(instance, sessionId, "rules"),
                    "${engine.name}: a string stringifies to a quoted string, which is JSON " +
                        "and is not the declared shape",
                )
            }
        }
    }

    /**
     * A name the session does not hold reads as "cannot answer" on every
     * engine, rather than as an error the host has to catch.
     *
     * This is the `binding="late"` case a consumer meets in practice: the
     * variable exists only once its state is entered, and until then the
     * machine genuinely cannot say.
     */
    @Test
    fun everyEngineAnswersNullForAVariableTheSessionDoesNotHold() {
        for (engine in engines) {
            withSession(engine) { instance, sessionId ->
                assertNull(
                    DatamodelRead.readJson(instance, sessionId, "never_declared"),
                    "${engine.name}: an undeclared name must read as null, not raise",
                )
            }
        }
    }

    /**
     * The engines do not have to agree on the bytes, and this pins what they
     * do have to agree on.
     *
     * The Lua family's shared builtin sorts object keys; Rhino and QuickJS
     * emit property order. Both are stable for their engine, which is what a
     * consumer diffing two reads needs — and it is the same shape of promise
     * `readInt` makes about numeric width. Asserting a single byte string
     * across all three would be asserting something SCE does not offer.
     */
    @Test
    fun eachEngineAnswersTheSameWayTwice() {
        for (engine in engines) {
            withSession(engine) { instance, sessionId ->
                instance.executeScript(sessionId, "rules = ${engine.arrayLiteral}")
                val first = DatamodelRead.readJson(instance, sessionId, "rules")
                val second = DatamodelRead.readJson(instance, sessionId, "rules")
                assertTrue(first != null, "${engine.name}: nothing to compare")
                assertEquals(
                    first,
                    second,
                    "${engine.name}: two reads of an unchanged variable disagreed. A consumer " +
                        "diffing what the document holds would see a change that did not " +
                        "happen — which is why the reader asks the engine instead of walking " +
                        "the value into JSON itself.",
                )
            }
        }
    }

    private fun withSession(engine: Engine, body: (ScxmlScriptEngine, String) -> Unit) {
        val instance = engine.create()
        // Named per engine so a parallel runner cannot have two of these
        // sharing one session's globals — the failure mode that made an
        // earlier cross-engine test refute itself.
        val sessionId = "datamodel_read_json_${engine.name}"
        instance.createSession(sessionId)
        try {
            instance.setupSystemVariables(sessionId, "datamodel_read_json")
            body(instance, sessionId)
        } finally {
            instance.destroySession(sessionId)
        }
    }
}
