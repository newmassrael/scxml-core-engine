// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// §scxml-B-2-1 / §scxml-B-2-8-1 — XML in the data model is a DOM
// structure, not three method names.
//
// The expectations are not this file's. They live in
// `tests/ecmascript/dom_read_surface.json`, one claim per case with the
// DOM clause that backs it, and the two C++ engines, the Rust, Go and
// Python bindings and the frontend read the same file. Measured
// 2026-08-18, every read in it answered undefined on all three engines
// here: what they carried was `getElementsByTagName` and `getAttribute`,
// which are exactly the two names the W3C IRP suite reads — this
// backend's Rhino engines did not even carry the third one the frontend
// lowers — so a document that walked the tree the way DOM Level 1 Core
// spells it got nothing, with every W3C fixture green.
//
// Why all three engines rather than a selected one: a generated Kotlin
// state machine takes its engine as a constructor argument, so this
// backend ships three equal options and "which of the things we hand
// people implement the DOM" can only be answered by asking each.
//
// Which spelling each is asked is the one it is handed. Rhino and QuickJS
// run the author's ECMAScript, so they get `source`; the Lua engine is
// handed what the frontend lowered, so it gets `lua`. Both spellings are
// bound to each other by `sce-build/tests/dom_read_surface_table.rs`,
// which asserts the frontend's own lowering of `source` IS `lua`.

package com.sce.ecmascript

import com.sce.runtime.ScriptSource
import com.sce.runtime.ScxmlScriptEngine
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
private sealed interface DomAnswer {
    fun describe(): String

    class Bool(val value: Boolean) : DomAnswer {
        override fun describe() = value.toString()
    }

    class Num(val value: Double) : DomAnswer {
        override fun describe() = value.toString()
    }

    class Text(val value: String) : DomAnswer {
        override fun describe() = "\"$value\""
    }

    object Empty : DomAnswer {
        override fun describe() = "null/undefined"
    }
}

/** One row of the shared table, with the document it is asked of. */
private class DomCase(
    val xml: String,
    val source: String,
    val lua: String,
    val clause: String,
    val expected: DomAnswer,
)

class DomReadSurfaceTest {

    @Test
    fun rhinoAnswersTheDomReadSurface() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsAnswersTheDomReadSurface() = measure("QuickJS") { QuickJSScriptEngine() }

    @Test
    fun luaAnswersTheDomReadSurface() = measure("Lua") { LuaScriptEngine() }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val cases = loadCases()
        val engine = create()
        // The Lua engine is handed what the frontend lowered; the two
        // ECMAScript engines are handed what the author wrote.
        val lowered = engineName == "Lua"
        val failures = mutableListOf<String>()

        cases.forEachIndexed { index, case ->
            val sessionId = "dom_surface_$index"
            val source = if (lowered) case.lua else case.source
            // Tagged with the language it is in, which is the whole reason
            // `ScriptSource` exists. Handing the Lua spelling through the
            // `String` door would claim it is ECMAScript, and the engine would
            // offer it to a parser that correctly refuses to read it.
            val unit =
                if (lowered) ScriptSource.lua(case.lua, case.source) else ScriptSource.ecmascript(case.source)
            engine.createSession(sessionId)
            try {
                engine.setupSystemVariables(sessionId, "dom_surface")
                val bound = engine.parseDataValue(sessionId, case.xml)
                if (bound == null || bound is String) {
                    failures += "[$source] the XML did not become a DOM (got ${describe(bound)})"
                    return@forEachIndexed
                }
                engine.setVariable(sessionId, "var1", bound)
                val answered =
                    try {
                        engine.evaluateExpr(sessionId, unit)
                    } catch (failure: Exception) {
                        failures += "[$source] failed to evaluate: ${failure.message} (${case.clause})"
                        return@forEachIndexed
                    }
                if (!matches(answered, case.expected)) {
                    failures += "[$source] answered ${describe(answered)}, ${case.clause} says " +
                        case.expected.describe()
                }
            } finally {
                engine.destroySession(sessionId)
            }
        }

        // Every case is reported, not just the first: a binding that answers
        // the methods and none of the properties is a different defect from
        // one that cannot parse the document, and the first failure alone
        // cannot tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${cases.size} reads disagree with DOM Level 1 Core, " +
                "evaluated by $engineName.\n" +
                "W3C SCXML B.2.1 obliges the Processor to create \"the corresponding DOM " +
                "structure\"; an engine that answers two of its members supplies a pair of " +
                "methods, not that structure.\n" +
                failures.joinToString("\n"),
        )
    }

    /**
     * An engine may hold a whole number as an integer or as a double, and
     * ECMA-262 has one Number type — so both spellings answer a `number`
     * case. The same rule the C++ and Go readers apply, for the same reason.
     */
    private fun matches(actual: Any?, expected: DomAnswer): Boolean = when (expected) {
        is DomAnswer.Bool -> actual is Boolean && actual == expected.value
        is DomAnswer.Num -> actual is Number && abs(actual.toDouble() - expected.value) < 1e-9
        is DomAnswer.Text -> actual is String && actual == expected.value
        DomAnswer.Empty -> actual == null || isUndefined(actual)
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

    private fun loadCases(): List<DomCase> {
        // The tests run from the repository root (`tasks.test { workingDir }`),
        // so the shared table is named by the same path its other readers use.
        val file = File("tests/ecmascript/dom_read_surface.json")
        assertTrue(
            file.isFile,
            "the shared DOM table is missing at ${file.absolutePath}; " +
                "this test measures nothing without it",
        )
        val table = Json.parseToJsonElement(file.readText()).jsonObject
        val documents = table.getValue("documents").jsonObject
        val cases = table.getValue("cases").jsonArray.map { element ->
            val row = element.jsonObject
            val documentName = row.getValue("document").jsonPrimitive.content
            val xml = documents.getValue(documentName).jsonPrimitive.content
            DomCase(
                xml = xml,
                source = row.getValue("source").jsonPrimitive.content,
                lua = row.getValue("lua").jsonPrimitive.content,
                clause = row.getValue("clause").jsonPrimitive.content,
                expected = readDomAnswer(row.getValue("expect").jsonObject),
            )
        }
        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            cases.size >= 30,
            "the shared DOM table produced only ${cases.size} case(s), " +
                "so this is not measuring the surface it claims to",
        )
        return cases
    }

    private fun readDomAnswer(expect: JsonObject): DomAnswer = when {
        expect.containsKey("bool") -> DomAnswer.Bool(expect.getValue("bool").jsonPrimitive.boolean)
        expect.containsKey("number") -> DomAnswer.Num(expect.getValue("number").jsonPrimitive.double)
        expect.containsKey("string") ->
            DomAnswer.Text(expect.getValue("string").jsonPrimitive.contentOrNull ?: "")
        expect.containsKey("empty") -> DomAnswer.Empty
        // A case whose expectation cannot be read is not a case that passes:
        // reading it as "no answer" would let a typo in a key retire a case
        // silently.
        else -> error("case has no readable expectation: $expect")
    }
}
