// SPDX-License-Identifier: AGPL-3.0-only WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-5.10, §scxml-B-2-8 — which writes to a system variable an engine
// refuses.
//
// "The Processor MUST cause any attempt to change the value of a system
// variable to fail and MUST place the error 'error.execution' on the internal
// event queue." A refused write is a script that FAILS, which every engine
// reports as a `ScriptEngineException` and the Processor turns into
// `error.execution`; so each case runs a write, asks whether the script
// failed, and then reads back the value the write would have changed, so an
// engine cannot pass by reporting a failure while the write landed. W3C tests
// 322, 326 and 346 only `<assign>` to a whole system variable, which the
// Processor refuses before any engine is asked, so no W3C test can see a
// write into `_event` go through.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/system_variable_writes.json`, which every SCE script
// engine reads: the Lua engines bound `_event` as a plain table and answered
// `_event.name = 'x'` by changing it, while the C++ QuickJS binding refused
// it, and no test could tell the two answers apart.
//
// Rhino and QuickJS run the author's ECMAScript, so they get `write` and
// `after`; the Lua engine is handed what the frontend lowered, so it gets
// `write_lua` and `after_lua`.

package com.sce.ecmascript

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScriptSource
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

/** One row of the shared table: a write, whether it is refused, and what must hold afterwards. */
private class SystemVariableWriteCase(
    val group: String,
    val write: String,
    val writeLua: String,
    val refused: Boolean,
    val after: String,
    val afterLua: String,
    val clause: String,
)

/** The session every case runs in, the event bound in it, and the cases. */
private class SystemVariableWritesTable(
    val sessionId: String,
    val machineName: String,
    val ioProcessors: List<IoProcessorDescriptor>,
    val event: SetCurrentEventArgs,
    val cases: List<SystemVariableWriteCase>,
)

class SystemVariableWritesTest {

    @Test
    fun rhinoRefusesEveryWriteToASystemVariable() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsRefusesEveryWriteToASystemVariable() = measure("QuickJS") { QuickJSScriptEngine() }

    @Test
    fun luaRefusesEveryWriteToASystemVariable() = measure("Lua") { LuaScriptEngine() }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val table = loadTable()
        val engine = create()
        val lowered = engineName == "Lua"
        val failures = mutableListOf<String>()
        var asked = 0

        table.cases.forEachIndexed { index, case ->
            val write = if (lowered) case.writeLua else case.write
            val after = if (lowered) case.afterLua else case.after
            val label = "case $index (${case.group}): [$write]"
            // The session id is the table's, because `_sessionid` is bound to
            // the id a session is created under and a case reads it back.
            engine.createSession(table.sessionId)
            try {
                engine.setupSystemVariables(table.sessionId, table.machineName, table.ioProcessors)
                engine.setCurrentEvent(table.sessionId, table.event)
                asked++

                // A refusal is the engine's script failure and nothing else: an
                // exception of another type is a binding that broke, not one
                // that refused.
                val refusal: ScriptEngineException? =
                    try {
                        if (lowered) {
                            engine.executeScript(table.sessionId, ScriptSource.lua(case.writeLua, case.write))
                        } else {
                            engine.executeScript(table.sessionId, case.write)
                        }
                        null
                    } catch (failure: ScriptEngineException) {
                        failure
                    } catch (failure: Exception) {
                        failures += "$label failed with ${failure.javaClass.simpleName} " +
                            "rather than the engine's script failure: ${failure.message} (${case.clause})"
                        return@forEachIndexed
                    }
                if (case.refused && refusal == null) {
                    failures += "$label was not refused; ${case.clause} says it must fail"
                }
                if (!case.refused && refusal != null) {
                    failures += "$label was refused (${refusal.message}); ${case.clause} says it is allowed"
                }

                val script =
                    if (lowered) ScriptSource.lua(case.afterLua, case.after) else ScriptSource.ecmascript(case.after)
                val held =
                    try {
                        engine.evaluateExpr(table.sessionId, script)
                    } catch (failure: Exception) {
                        failures += "$label: [$after] failed to evaluate afterwards: " +
                            "${failure.message} (${case.clause})"
                        return@forEachIndexed
                    }
                if (held != true) {
                    failures += "$label: afterwards [$after] answered ${describe(held)}, " +
                        "so the write changed the value (${case.clause})"
                }
            } finally {
                engine.destroySession(table.sessionId)
            }
        }

        // A case that was never asked is not a case that passed.
        assertTrue(
            asked == table.cases.size,
            "$engineName was asked $asked of the ${table.cases.size} cases the shared table holds",
        )
        // Every case is reported, not just the first: a binding that leaves
        // `_event` writable and one that leaves only its new properties
        // writable are different defects, and the first failure alone cannot
        // tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${table.cases.size} system-variable writes disagree with " +
                "W3C SCXML 5.10 / B.2.8, evaluated by $engineName.\n" +
                failures.joinToString("\n"),
        )
    }

    private fun describe(value: Any?): String = when (value) {
        null -> "null"
        is String -> "\"$value\""
        else -> "$value (${value.javaClass.simpleName})"
    }

    private fun loadTable(): SystemVariableWritesTable {
        // The tests run from the repository root (`tasks.test { workingDir }`),
        // so the shared table is named by the same path its other readers use.
        val file = File("tests/ecmascript/system_variable_writes.json")
        assertTrue(
            file.isFile,
            "the shared system-variable write table is missing at ${file.absolutePath}; " +
                "this test measures nothing without it",
        )
        val root = Json.parseToJsonElement(file.readText()).jsonObject
        val session = root.getValue("session").jsonObject
        val rows = root.getValue("cases").jsonArray
        val cases = rows.map { element ->
            val row = element.jsonObject
            fun text(key: String): String = row.getValue(key).jsonPrimitive.content
            SystemVariableWriteCase(
                group = text("group"),
                write = text("write"),
                writeLua = text("write_lua"),
                refused = row.getValue("refused").jsonPrimitive.boolean,
                after = text("after"),
                afterLua = text("after_lua"),
                clause = text("clause"),
            )
        }
        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            rows.size >= 18,
            "the shared system-variable write table holds only ${rows.size} case(s), " +
                "so this is not measuring the surface it claims to",
        )
        // Both kinds, or the measurement is one-sided: an engine that refused
        // every write would pass a table of refusals, and one that refused none
        // would pass a table of controls.
        assertTrue(
            cases.any { it.refused } && cases.any { !it.refused },
            "the shared system-variable write table must hold refused and allowed cases alike",
        )
        return SystemVariableWritesTable(
            sessionId = session.getValue("sessionid").jsonPrimitive.content,
            machineName = session.getValue("name").jsonPrimitive.content,
            ioProcessors = session.getValue("ioprocessors").jsonObject.map { (name, location) ->
                IoProcessorDescriptor(name, location.jsonPrimitive.content)
            },
            event = readEvent(root.getValue("event").jsonObject),
            cases = cases,
        )
    }

    /** The event as the engine interface carries it; `data` is the payload text. */
    private fun readEvent(event: JsonObject): SetCurrentEventArgs {
        fun field(key: String): String = event[key]?.jsonPrimitive?.content ?: ""
        return SetCurrentEventArgs(
            name = event.getValue("name").jsonPrimitive.content,
            data = field("data"),
            type = field("type"),
            sendId = field("sendid"),
            origin = field("origin"),
            originType = field("origintype"),
            invokeId = field("invokeid"),
        )
    }
}
