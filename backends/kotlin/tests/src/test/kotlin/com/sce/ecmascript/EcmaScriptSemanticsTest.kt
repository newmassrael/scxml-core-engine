// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// Every engine this backend offers for `datamodel="ecmascript"`, measured
// against what ECMA-262 says.
//
// The expectations are not ours. They live in
// `tests/ecmascript/ecma262_semantics.json`, one claim per case with the
// clause that backs it, and the C++ engine test and the Rust frontend test
// read the same file. A per-backend copy would drift toward whichever engine
// reads it, which is the blindness this table was written to end: the same
// document answered differently depending on the engine, and no fixture in
// the repository could tell, because a W3C suite that is green end to end
// never asks `0 && x`.
//
// Why all three engines rather than the selected one. The C++ build picks an
// engine at configure time, so there the question is "did this build choose
// an ECMAScript engine". Kotlin has no such choice to make: a generated state
// machine takes its engine as a constructor argument, and this backend ships
// three — Rhino, QuickJS and Lua — each offered by `EngineFactory` as an
// equal option. So the question here is "which of the things we hand people
// are ECMAScript", and it can only be answered by asking all of them.
//
// The evaluation goes through `evaluateCondition` / `evaluateExpr`, which is
// what generated code calls (`scriptengine_helpers.kt.jinja2`), not a
// reimplementation of it.

package com.sce.ecmascript

import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.boolean
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.double
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.math.abs
import kotlin.test.Test
import kotlin.test.assertTrue

/** One row of the shared table. */
private class Case(
    val setup: String,
    val source: String,
    val asCondition: Boolean,
    val clause: String,
    val expected: Answer,
)

/** The single answer a row names: exactly one of these shapes. */
private sealed interface Answer {
    fun describe(): String

    class Bool(val value: Boolean) : Answer {
        override fun describe() = value.toString()
    }

    class Num(val value: Double) : Answer {
        override fun describe() = value.toString()
    }

    class Text(val value: String) : Answer {
        override fun describe() = "\"$value\""
    }

    object Empty : Answer {
        override fun describe() = "null or undefined"
    }
}

class EcmaScriptSemanticsTest {

    @Test
    fun rhinoAnswersWhatEcmaScriptAnswers() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsAnswersWhatEcmaScriptAnswers() = measure("QuickJS") { QuickJSScriptEngine() }

    /**
     * Lua is measured too, and the assertion runs the other way.
     *
     * It is not an ECMAScript engine and it does not become one by being
     * asked politely: 27 of these 58 cases answered differently when this was
     * first run, the same class of disagreement the C++ build measured at 26.
     * `datamodel="ecmascript"` is a claim about a language, so the honest
     * landing is not a permanently red suite — which teaches a reader to
     * scroll past red — but a test that pins WHY the answer is no, and fires
     * if that ever stops being true.
     *
     * If this fails because Lua now conforms, nothing is broken: the
     * documentation on `LuaScriptEngine` is what needs rewriting, and this
     * comment with it.
     */
    @Test
    fun luaIsNotAnEcmaScriptEngineAndSaysSo() {
        val failures = collectFailures { LuaScriptEngine() }
        assertTrue(
            failures.isNotEmpty(),
            "Lua answered all ${loadCases().size} ECMA-262 cases correctly. That is good " +
                "news and it makes this test wrong: the engine's own documentation says it " +
                "is not an ECMAScript engine, and a reader choosing between the three " +
                "deserves the current answer. Rewrite the doc comment on LuaScriptEngine " +
                "and turn this into `measure(...)` beside the other two.",
        )

        // Refutable, and refuted where a consumer can see it: the engine's own
        // header must not be recommending itself for the datamodel it answers
        // differently from. It said "For AOSP/AAOS production, this replaces
        // Rhino with a faster native engine" while answering 27 of 58 cases
        // differently from the language that production would be running.
        val header = File(
            "backends/kotlin/lua/src/main/kotlin/com/sce/scripting/lua/LuaScriptEngine.kt",
        )
        assertTrue(header.isFile, "the Lua engine is missing at ${header.absolutePath}")
        val doc = header.readText().substringBefore("class LuaScriptEngine")
        assertTrue(
            "not an ECMAScript engine" in doc,
            "the Lua engine's documentation does not say what this test measures. " +
                "${failures.size} of the shared table's cases are answered differently, so a " +
                "reader who takes that header at its word chooses an engine that runs their " +
                "`datamodel=\"ecmascript\"` document as something else.",
        )
    }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val failures = collectFailures(create)
        // Every case is reported, not just the first: a build that answers one
        // group wrong and another right is a different problem from one that
        // answers nothing, and the first failure alone cannot tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${loadCases().size} expressions disagree with ECMA-262, " +
                "evaluated by $engineName.\n" +
                "An engine offered for `datamodel=\"ecmascript\"` answers what that " +
                "language answers; one that does not is not a choice a consumer can " +
                "make safely, whatever else it is good at.\n" +
                failures.joinToString("\n"),
        )
    }

    private fun collectFailures(create: () -> ScxmlScriptEngine): List<String> {
        val cases = loadCases()

        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            cases.size >= 55,
            "the shared ECMA-262 table produced only ${cases.size} case(s), " +
                "so this is not measuring the corpus it claims to",
        )

        val engine = create()
        val failures = mutableListOf<String>()

        cases.forEachIndexed { index, case ->
            val sessionId = "ecma262_case_$index"
            engine.createSession(sessionId)
            try {
                engine.setupSystemVariables(sessionId, "ecma262")
                var setupOk = true
                if (case.setup.isNotEmpty()) {
                    try {
                        engine.executeScript(sessionId, case.setup)
                    } catch (failure: Exception) {
                        failures += "[${case.source}] setup did not run: ${failure.message}" +
                            "\n  setup: ${case.setup}"
                        setupOk = false
                    }
                }
                if (setupOk) {
                    failures += evaluate(engine, sessionId, case)
                }
            } finally {
                engine.destroySession(sessionId)
            }
        }
        return failures
    }

    private fun evaluate(engine: ScxmlScriptEngine, sessionId: String, case: Case): List<String> {
        if (case.asCondition) {
            val expected = case.expected
            val answered =
                try {
                    engine.evaluateCondition(sessionId, case.source)
                } catch (failure: Exception) {
                    return listOf(
                        "[${case.source}] failed to evaluate as a condition: " +
                            "${failure.message} (${case.clause})",
                    )
                }
            val wanted = (expected as? Answer.Bool)?.value
                ?: return listOf("[${case.source}] is a condition but names a non-boolean answer")
            return if (answered == wanted) {
                emptyList()
            } else {
                listOf(
                    "[${case.source}] answered $answered, ECMAScript says " +
                        "${expected.describe()} (${case.clause})",
                )
            }
        }

        val value =
            try {
                engine.evaluateExpr(sessionId, case.source)
            } catch (failure: Exception) {
                return listOf("[${case.source}] failed to evaluate: ${failure.message} (${case.clause})")
            }
        return if (matches(value, case.expected)) {
            emptyList()
        } else {
            listOf(
                "[${case.source}] answered ${describe(value)}, ECMAScript says " +
                    "${case.expected.describe()} (${case.clause})",
            )
        }
    }

    /**
     * An engine may hold a whole number as an integer or as a double, and
     * ECMA-262 has one Number type — so both spellings answer a `number` case.
     * The same rule the C++ reader applies, for the same reason.
     */
    private fun matches(actual: Any?, expected: Answer): Boolean = when (expected) {
        is Answer.Bool -> actual is Boolean && actual == expected.value
        is Answer.Num -> actual is Number && abs(actual.toDouble() - expected.value) < 1e-9
        is Answer.Text -> actual is String && actual == expected.value
        Answer.Empty -> actual == null || isUndefined(actual)
    }

    /**
     * Rhino hands back its own singleton for `undefined` rather than a Kotlin
     * `null`, and the table treats null and undefined as one answer because
     * ECMAScript's `==` equates them and SCXML's datamodel cannot tell an
     * absent property from a null one.
     */
    private fun isUndefined(value: Any): Boolean =
        value.javaClass.name == "org.mozilla.javascript.Undefined" || value.toString() == "undefined"

    private fun describe(value: Any?): String = when (value) {
        null -> "null"
        is String -> "\"$value\""
        else -> "$value (${value.javaClass.simpleName})"
    }

    private fun loadCases(): List<Case> {
        // The tests run from the repository root (`tasks.test { workingDir }`),
        // so the shared table is named by the same path its other readers use.
        val table = File("tests/ecmascript/ecma262_semantics.json")
        assertTrue(
            table.isFile,
            "the shared ECMA-262 table is missing at ${table.absolutePath}; " +
                "this test measures nothing without it",
        )
        val root = Json.parseToJsonElement(table.readText()).jsonObject
        return root.getValue("cases").jsonArray.map { entry ->
            val obj = entry.jsonObject
            Case(
                setup = obj["setup"]?.jsonPrimitive?.contentOrNull.orEmpty(),
                source = obj.getValue("source").jsonPrimitive.content,
                asCondition = obj.getValue("form").jsonPrimitive.content == "condition",
                clause = obj.getValue("clause").jsonPrimitive.content,
                expected = parseAnswer(obj.getValue("expect").jsonObject, obj),
            )
        }
    }

    private fun parseAnswer(expect: JsonObject, row: JsonObject): Answer {
        expect["bool"]?.let { return Answer.Bool(it.jsonPrimitive.boolean) }
        expect["number"]?.let { return Answer.Num(it.jsonPrimitive.double) }
        expect["string"]?.let { return Answer.Text(it.jsonPrimitive.content) }
        // `empty` carries no value of its own — its presence IS the answer.
        expect["empty"]?.let { return Answer.Empty }
        // A case whose expectation cannot be read is not a case that passes.
        throw IllegalStateException(
            "case ${row.getValue("source").jsonPrimitive.content} names no expected answer",
        )
    }
}
