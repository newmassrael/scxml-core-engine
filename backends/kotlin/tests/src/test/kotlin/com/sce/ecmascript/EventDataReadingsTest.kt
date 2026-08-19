// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-8-1 — which reading an arriving payload gets.
//
// The clause names four readings and orders them: key-value pairs become named
// properties; otherwise JSON becomes the corresponding object; otherwise, "if
// the Processor can interpret the content as a valid XML document, it MUST
// create the corresponding DOM structure"; and then the sentence that closes
// it — "Otherwise, the Processor MUST treat the content as a space-normalized
// string literal".
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_data_readings.json`, one payload per case with the
// sentence of the clause that decides it, and the two C++ engines, the Rust,
// Go and Python bindings read the same file — a per-backend copy drifts toward
// the backend that reads it, which is the blindness that let nine engines give
// four different answers to one clause.
//
// Why all three engines rather than a selected one: a generated Kotlin state
// machine takes its engine as a constructor argument, so this backend ships
// three equal options and "which of the things we hand people reads the clause"
// can only be answered by asking each. It is also how the divergence survived:
// measured 2026-08-19, Rhino and QuickJS read every case, and the Lua engine
// answered 5 for a payload of `2 + 3` — it still had the
// `load("return " .. payload)` rung that its four siblings across the
// repository had removed on 2026-08-17, because nothing selected it.
//
// Which spelling each is asked is the one it is handed. Rhino and QuickJS run
// the author's ECMAScript, so they get `source`; the Lua engine is handed what
// the frontend lowered, so it gets `lua`.

package com.sce.ecmascript

import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import java.io.File
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.double
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/** The single answer a row names: exactly one of these shapes. */
private sealed interface ReadingAnswer {
    fun describe(): String

    class Bool(val value: Boolean) : ReadingAnswer {
        override fun describe() = value.toString()
    }

    class Num(val value: Double) : ReadingAnswer {
        override fun describe() = value.toString()
    }

    class Text(val value: String) : ReadingAnswer {
        override fun describe() = "\"$value\""
    }

    object Empty : ReadingAnswer {
        override fun describe() = "null/undefined"
    }
}

/** One row of the shared table: a payload and what reading it must get. */
private class ReadingCase(
    val payload: String,
    val source: String,
    val lua: String,
    val clause: String,
    val expected: ReadingAnswer,
)

class EventDataReadingsTest {

    @Test
    fun rhinoReadsEveryPayloadTheClauseNames() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsReadsEveryPayloadTheClauseNames() = measure("QuickJS") { QuickJSScriptEngine() }

    @Test
    fun luaReadsEveryPayloadTheClauseNames() = measure("Lua") { LuaScriptEngine() }

    /**
     * The sharper half of the expression case, which the shared table cannot
     * ask because the side effect is spelled in the receiver's own language.
     *
     * Reading the payload gives back its own text; running it gives back `x`
     * and, on the way, whatever else the sender named. `_event.data` is the one
     * field a document takes from outside itself.
     */
    @Test
    fun aPayloadThatIsACallLeavesTheLuaSessionAlone() {
        val engine = LuaScriptEngine()
        val sessionId = "lua_payload_call"
        engine.createSession(sessionId)
        try {
            engine.setupSystemVariables(sessionId, "payload_call")
            // Bound through the engine's own variable API rather than a script:
            // reading a name this session never declared raises rather than
            // answering nil, so a test that only caught the raise would pass
            // for a payload that ran and one that was never delivered alike.
            engine.setVariable(sessionId, "breached", false)
            engine.setCurrentEvent(
                sessionId,
                SetCurrentEventArgs(
                    name = "brief",
                    data = "(function() breached = true return 'x' end)()",
                    type = "external",
                ),
            )
            assertTrue(
                engine.getVariable(sessionId, "breached") == false,
                "the payload ran: a host, a peer session or an HTTP sender could write " +
                    "this session's globals by naming them in event data",
            )
        } finally {
            engine.destroySession(sessionId)
        }
    }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val cases = loadCases()
        val engine = create()
        // The Lua engine is handed what the frontend lowered; the two
        // ECMAScript engines are handed what the author wrote.
        val lowered = engineName == "Lua"
        val failures = mutableListOf<String>()

        cases.forEachIndexed { index, case ->
            val sessionId = "event_data_reading_$index"
            val source = if (lowered) case.lua else case.source
            engine.createSession(sessionId)
            try {
                engine.setupSystemVariables(sessionId, "event_data_reading")
                engine.setCurrentEvent(
                    sessionId,
                    SetCurrentEventArgs(name = "brief", data = case.payload, type = "external"),
                )
                val answered =
                    try {
                        engine.evaluateExpr(sessionId, source)
                    } catch (failure: Exception) {
                        failures += "payload \"${case.payload}\": [$source] failed to evaluate: " +
                            "${failure.message} (${case.clause})"
                        return@forEachIndexed
                    }
                if (!matches(answered, case.expected)) {
                    failures += "payload \"${case.payload}\": [$source] answered " +
                        "${describe(answered)}, ${case.clause} says ${case.expected.describe()}"
                }
            } finally {
                engine.destroySession(sessionId)
            }
        }

        // Every case is reported, not just the first: an engine that drops the
        // fall-through is a different defect from one that runs the payload,
        // and the first failure alone cannot tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${cases.size} readings disagree with W3C SCXML B.2.8.1, " +
                "evaluated by $engineName.\n" +
                failures.joinToString("\n"),
        )
    }

    /**
     * An engine may hold a whole number as an integer or as a double, and
     * ECMA-262 has one Number type — so both spellings answer a `number` case.
     */
    private fun matches(actual: Any?, expected: ReadingAnswer): Boolean = when (expected) {
        is ReadingAnswer.Bool -> actual is Boolean && actual == expected.value
        is ReadingAnswer.Num -> actual is Number && abs(actual.toDouble() - expected.value) < 1e-9
        is ReadingAnswer.Text -> actual is String && actual == expected.value
        ReadingAnswer.Empty -> actual == null || isUndefined(actual)
    }

    /**
     * Rhino hands back its own singleton for `undefined` rather than a Kotlin
     * `null`, and the table treats null and undefined as one answer because
     * SCXML's datamodel cannot tell an absent property from a null one.
     */
    private fun isUndefined(value: Any): Boolean =
        value.javaClass.name == "org.mozilla.javascript.Undefined" || value.toString() == "undefined"

    private fun describe(value: Any?): String = when (value) {
        null -> "null"
        is String -> "\"$value\""
        else -> "$value (${value.javaClass.simpleName})"
    }

    private fun loadCases(): List<ReadingCase> {
        // The tests run from the repository root (`tasks.test { workingDir }`),
        // so the shared table is named by the same path its other readers use.
        val file = File("tests/ecmascript/event_data_readings.json")
        assertTrue(
            file.isFile,
            "the shared reading table is missing at ${file.absolutePath}; " +
                "this test measures nothing without it",
        )
        val table = Json.parseToJsonElement(file.readText()).jsonObject
        val cases = table.getValue("cases").jsonArray.map { element ->
            val row = element.jsonObject
            ReadingCase(
                payload = row.getValue("payload").jsonPrimitive.content,
                source = row.getValue("source").jsonPrimitive.content,
                lua = row.getValue("lua").jsonPrimitive.content,
                clause = row.getValue("clause").jsonPrimitive.content,
                expected = readAnswer(row.getValue("expect").jsonObject),
            )
        }
        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            cases.size >= 8,
            "the shared reading table produced only ${cases.size} case(s), " +
                "so this is not measuring the surface it claims to",
        )
        return cases
    }

    private fun readAnswer(expect: JsonObject): ReadingAnswer = when {
        expect.containsKey("bool") -> ReadingAnswer.Bool(expect.getValue("bool").jsonPrimitive.boolean)
        expect.containsKey("number") -> ReadingAnswer.Num(expect.getValue("number").jsonPrimitive.double)
        expect.containsKey("string") ->
            ReadingAnswer.Text(expect.getValue("string").jsonPrimitive.contentOrNull ?: "")
        expect.containsKey("empty") -> ReadingAnswer.Empty
        // A case whose expectation cannot be read is not a case that passes:
        // reading it as "no answer" would let a typo in a key retire a case
        // silently.
        else -> error("case has no readable expectation: $expect")
    }
}
