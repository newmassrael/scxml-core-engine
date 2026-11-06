// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-8 — what an `_event` field the Processor left blank reads as.
//
// §scxml-5.10.1 leaves `sendid`, `origin`, `origintype` and `invokeid` blank
// for some events, and §scxml-B-2-8 makes blank ECMAScript undefined, not the
// empty string. The engine interface carries a blank field as the empty string
// (`SetCurrentEventArgs`), so the answer is decided where each engine builds
// `_event`, and each of the three is asked.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/event_blank_fields.json`, which every SCE script engine
// reads: the C++ QuickJS binding bound `''` below a comment stating the rule,
// and no test could tell the two answers apart, because a condition reads both
// as false.
//
// Rhino and QuickJS run the author's ECMAScript, so they get `source`; the Lua
// engine is handed what the frontend lowered, so it gets `lua`.

package com.sce.ecmascript

import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import java.io.File
import kotlin.test.Test
import kotlin.test.assertTrue
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

/** One row of the shared table: the event a binding is handed and what one read of it answers. */
private class BlankFieldCase(
    val event: SetCurrentEventArgs,
    val source: String,
    val lua: String,
    val clause: String,
    val expected: Boolean,
)

class EventBlankFieldsTest {

    @Test
    fun rhinoReadsEveryBlankFieldAsUndefined() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsReadsEveryBlankFieldAsUndefined() = measure("QuickJS") { QuickJSScriptEngine() }

    @Test
    fun luaReadsEveryBlankFieldAsNil() = measure("Lua") { LuaScriptEngine() }

    /**
     * The sharper half of the undefined case, which the shared table cannot ask
     * because Lua has no spelling for it: in ECMAScript a blank field is still a
     * property of `_event`, whose value is undefined, so a document asking
     * whether the field exists is answered yes for every event.
     */
    @Test
    fun aBlankFieldIsStillAPropertyOfTheEventInRhino() = blankFieldIsAProperty { RhinoScriptEngine() }

    @Test
    fun aBlankFieldIsStillAPropertyOfTheEventInQuickJs() = blankFieldIsAProperty { QuickJSScriptEngine() }

    private fun blankFieldIsAProperty(create: () -> ScxmlScriptEngine) {
        val engine = create()
        val sessionId = "event_blank_field_property"
        engine.createSession(sessionId)
        try {
            engine.setupSystemVariables(sessionId, "event_blank_field_property")
            engine.setCurrentEvent(sessionId, SetCurrentEventArgs(name = "brief", type = "internal"))
            val answered = engine.evaluateExpr(
                sessionId,
                "'sendid' in _event && 'origin' in _event && 'origintype' in _event && " +
                    "'invokeid' in _event && typeof _event.sendid === 'undefined'",
            )
            assertTrue(
                answered == true,
                "a blank field must be a property of _event whose value is undefined; answered $answered",
            )
        } finally {
            engine.destroySession(sessionId)
        }
    }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val table = loadTable()
        val engine = create()
        val lowered = engineName == "Lua"
        val failures = mutableListOf<String>()
        var asked = 0

        table.cases.forEachIndexed { index, case ->
            val sessionId = "event_blank_field_$index"
            val source = if (lowered) case.lua else case.source
            engine.createSession(sessionId)
            try {
                engine.setupSystemVariables(sessionId, "event_blank_field")
                engine.setCurrentEvent(sessionId, case.event)
                asked++
                val answered =
                    try {
                        engine.evaluateExpr(sessionId, source)
                    } catch (failure: Exception) {
                        failures += "case $index: [$source] failed to evaluate: " +
                            "${failure.message} (${case.clause})"
                        return@forEachIndexed
                    }
                if (!(answered is Boolean && answered == case.expected)) {
                    failures += "case $index: [$source] answered ${describe(answered)}, " +
                        "${case.clause} says ${case.expected}"
                }
            } finally {
                engine.destroySession(sessionId)
            }
        }

        // A case that was never asked is not a case that passed.
        assertTrue(
            asked == table.held,
            "$engineName was asked $asked of the ${table.held} cases the shared table holds",
        )
        // Every case is reported, not just the first: a binding that drops a
        // field always is a different defect from one that binds `''`, and the
        // first failure alone cannot tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${table.held} blank-field readings disagree with W3C SCXML B.2.8, " +
                "evaluated by $engineName.\n" +
                failures.joinToString("\n"),
        )
    }

    private fun describe(value: Any?): String = when (value) {
        null -> "null"
        is String -> "\"$value\""
        else -> "$value (${value.javaClass.simpleName})"
    }

    private class Table(val cases: List<BlankFieldCase>, val held: Int)

    private fun loadTable(): Table {
        // The tests run from the repository root (`tasks.test { workingDir }`),
        // so the shared table is named by the same path its other readers use.
        val file = File("tests/ecmascript/event_blank_fields.json")
        assertTrue(
            file.isFile,
            "the shared blank-field table is missing at ${file.absolutePath}; " +
                "this test measures nothing without it",
        )
        val rows = Json.parseToJsonElement(file.readText()).jsonObject.getValue("cases").jsonArray
        val cases = rows.map { element ->
            val row = element.jsonObject
            BlankFieldCase(
                event = readEvent(row.getValue("event").jsonObject),
                source = row.getValue("source").jsonPrimitive.content,
                lua = row.getValue("lua").jsonPrimitive.content,
                clause = row.getValue("clause").jsonPrimitive.content,
                expected = readAnswer(row.getValue("expect").jsonObject),
            )
        }
        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            rows.size >= 11,
            "the shared blank-field table holds only ${rows.size} case(s), " +
                "so this is not measuring the surface it claims to",
        )
        return Table(cases, rows.size)
    }

    /**
     * The event as the engine interface carries it: a member the table leaves
     * out is a field the Processor left blank, which is the empty string here.
     */
    private fun readEvent(event: JsonObject): SetCurrentEventArgs {
        fun field(key: String): String = event[key]?.jsonPrimitive?.content ?: ""
        return SetCurrentEventArgs(
            name = event.getValue("name").jsonPrimitive.content,
            type = event.getValue("type").jsonPrimitive.content,
            sendId = field("sendid"),
            origin = field("origin"),
            originType = field("origintype"),
            invokeId = field("invokeid"),
        )
    }

    private fun readAnswer(expect: JsonObject): Boolean {
        // A case whose expectation cannot be read is not a case that passes:
        // every row of this table names a `bool`, and any other key is a typo
        // that would otherwise retire the case silently.
        val bool = expect["bool"] ?: error("case has no readable expectation: $expect")
        return bool.jsonPrimitive.boolean
    }
}
