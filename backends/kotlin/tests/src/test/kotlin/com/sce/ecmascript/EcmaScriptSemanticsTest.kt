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

/**
 * Paths, relative to the repository root — the suite's `workingDir`, which is
 * also how the shared table is named below and how the C++ and Rust readers
 * name the same files.
 */
private const val DIVERGENCES_PATH = "tests/ecmascript/kotlin_lua_divergences.json"
private const val ENGINE_PATH =
    "backends/kotlin/lua/src/main/kotlin/com/sce/scripting/lua/LuaScriptEngine.kt"

/**
 * The `diverges_on` path this suite is the contract for.
 *
 * There are two routes from `datamodel="ecmascript"` into a Lua engine and they
 * fail differently: the engine's own input adapter rewriting the author's text,
 * which is what this suite exercises, and `sce-build`'s frontend having emitted
 * Lua at build time, which nothing on this backend does yet. Spelled the same
 * on all three readers of these lists — this one,
 * `tests/engine/LoweredEcma262Test.cpp` and
 * `sce-build/tests/ecma262_scoreboard_contract.rs`, which is where the
 * vocabulary is held against what the code generator derives.
 */
private const val RUNTIME_REWRITER_PATH = "runtime-rewriter"

/** What identifies one case, and one declared divergence against it. */
private data class Key(val source: String, val clause: String)

/**
 * One case an engine answered differently from ECMA-262.
 *
 * Carried as a key plus a message rather than a message alone: the message is
 * for a person reading a failure, and the key is what a declared list can be
 * compared against. `source` alone is not the key — the table asks `a && b`
 * under two clauses, and collapsing them would let one declaration cover a
 * divergence nobody looked at.
 */
private data class Divergence(val source: String, val clause: String, val message: String) {
    val key: Key get() = Key(source, clause)
}

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
     * asked politely, so `datamodel="ecmascript"` — a claim about a language
     * — cannot be met by it. The honest landing is not a permanently red
     * suite, which teaches a reader to scroll past red, but a test that pins
     * WHICH cases the answer is no for and fires when that set moves in
     * either direction.
     *
     * "Either direction" is the part this test did not used to have. It
     * asserted only that the failure set was NOT EMPTY, which is satisfied by
     * one disagreement and by fifty, so the rewriter could regress or improve
     * for months without a word. The engine's own KDoc meanwhile carried "27
     * of its 58" under a sentence saying this test held it to the
     * measurement; it held no number at all, and the shared table had since
     * grown to 98 cases. A declared list is what makes both directions
     * visible — the same shape `tests/ecmascript/lua_engine_divergences.json`
     * gives the C++ selection, and for the same reason its header states: a
     * count that lives in prose is a count nobody re-answers.
     *
     * The two lists are separate on purpose. This backend rewrites with its
     * own [com.sce.scripting.lua.EcmaScriptToLuaTransformer] onto a different
     * Lua, so its divergences are its own measurement and neither list may be
     * derived from the other.
     */
    @Test
    fun luaIsNotAnEcmaScriptEngineAndSaysSo() {
        val failures = collectFailures { LuaScriptEngine() }
        val declared = loadDeclaredDivergences()

        // Ordered before the floor deliberately. A list that is empty — the
        // first run of a new one — fails HERE, printing every divergence in
        // the shape the file takes, instead of dying below with nothing to
        // show the person who has to write it.
        val undeclared = failures.filterNot { it.key in declared }
        assertTrue(
            undeclared.isEmpty(),
            "${undeclared.size} expression(s) disagree with ECMA-262 on LuaScriptEngine " +
                "without being declared to. Either the rewriter regressed, or " +
                "$DIVERGENCES_PATH has not caught up with it. If it is the second, these " +
                "are the entries to add:\n" +
                undeclared.joinToString(",\n") {
                    "    { \"source\": ${quoted(it.source)}, \"clause\": ${quoted(it.clause)} }"
                } +
                "\n\nWhat each one answered:\n" +
                undeclared.joinToString("\n") { "  ${it.message}" },
        )

        val stillWrong = failures.map { it.key }.toSet()
        val repaired = declared.filterNot { it in stillWrong }
        assertTrue(
            repaired.isEmpty(),
            "${repaired.size} declared divergence(s) no longer describe this engine. Remove " +
                "them from $DIVERGENCES_PATH — a list that keeps a repaired case is a list " +
                "that cannot be trusted in the other direction either, and it is what a " +
                "reader consults to decide whether their document stays inside what the " +
                "rewriter covers.\n" +
                repaired.joinToString("\n") { "  ${it.source}  (${it.clause})" },
        )

        assertTrue(
            declared.isNotEmpty(),
            "$DIVERGENCES_PATH declares nothing, and nothing disagreed. If Lua now answers " +
                "all ${loadCases().size} ECMA-262 cases that is good news and it makes this " +
                "test wrong: rewrite the doc comment on LuaScriptEngine and turn this into " +
                "`measure(...)` beside the other two.",
        )

        // Refutable, and refuted where a consumer can see it: the engine's own
        // header must not be recommending itself for the datamodel it answers
        // differently from. It said "For AOSP/AAOS production, this replaces
        // Rhino with a faster native engine" while answering a whole class of
        // the table differently from the language that production would run.
        val header = File(ENGINE_PATH)
        assertTrue(header.isFile, "the Lua engine is missing at ${header.absolutePath}")
        val doc = header.readText().substringBefore("class LuaScriptEngine")
        assertTrue(
            "not an ECMAScript engine" in doc,
            "the Lua engine's documentation does not say what this test measures. " +
                "${failures.size} of the shared table's cases are answered differently, so a " +
                "reader who takes that header at its word chooses an engine that runs their " +
                "`datamodel=\"ecmascript\"` document as something else.",
        )

        // The header must point at the list rather than restate its size. The
        // count it used to carry outlived two growths of the shared table, and
        // a number in prose is the one thing here nothing can re-answer.
        assertTrue(
            DIVERGENCES_PATH in doc,
            "the Lua engine's documentation does not name $DIVERGENCES_PATH. That list is " +
                "the answer to the question its header raises — which cases — and a reader " +
                "who cannot reach it from the engine is left to take a count on trust, " +
                "which is how the previous one survived being wrong.",
        )
    }

    /**
     * The declared set, keyed the way the failures are.
     *
     * Read from disk rather than held in this file: the list is a measurement
     * a person maintains and a reader consults, and a Kotlin constant is
     * neither greppable beside the C++ list nor readable by anyone not
     * building this backend.
     */
    private fun loadDeclaredDivergences(): Set<Key> {
        val file = File(DIVERGENCES_PATH)
        assertTrue(
            file.isFile,
            "the declared divergences are missing at ${file.absolutePath}. This test compares " +
                "against them in both directions, so without the file it measures nothing.",
        )
        val root = Json.parseToJsonElement(file.readText()).jsonObject
        return root.getValue("divergences").jsonArray.mapNotNull { entry ->
            val obj = entry.jsonObject
            val key = Key(
                obj.getValue("source").jsonPrimitive.content,
                obj.getValue("clause").jsonPrimitive.content,
            )
            // Only the entries declared on THIS path. `diverges_on` splits the
            // list by which route into the Lua engine still answers the case
            // differently, and this suite reaches the engine through
            // `EcmaScriptToLuaTransformer` — so it is the `runtime-rewriter`
            // contract. Today that is the only path this backend has, and the
            // filter is what keeps that from being an assumption: the day the
            // Kotlin templates cross the seam and gain build-time lowering,
            // an entry only that path gets wrong would otherwise be reported
            // here as a rewriter divergence that has been repaired, and its
            // deletion demanded.
            //
            // Unclassified is RED, not a default. An entry naming no path is
            // exempt from every per-path suite at once.
            val paths = obj["diverges_on"]
            assertTrue(
                paths != null,
                "the entry ${quoted(key.source)} / ${quoted(key.clause)} in $DIVERGENCES_PATH " +
                    "carries no `diverges_on`, so no per-path suite can tell whether it is about it.",
            )
            val onThisPath = paths!!.jsonArray.any { it.jsonPrimitive.content == RUNTIME_REWRITER_PATH }
            if (onThisPath) key else null
        }.toSet()
    }

    /** JSON-quoted, so a failure prints entries that can be pasted as-is. */
    private fun quoted(value: String): String =
        "\"" + value.replace("\\", "\\\\").replace("\"", "\\\"") + "\""

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
                failures.joinToString("\n") { it.message },
        )
    }

    private fun collectFailures(create: () -> ScxmlScriptEngine): List<Divergence> {
        val cases = loadCases()

        // A floor, not an equality: adding a case must not have to touch this
        // number, but a table that stopped being read must not pass either.
        assertTrue(
            cases.size >= 55,
            "the shared ECMA-262 table produced only ${cases.size} case(s), " +
                "so this is not measuring the corpus it claims to",
        )

        val engine = create()
        val failures = mutableListOf<Divergence>()

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
                        failures += Divergence(
                            case.source,
                            case.clause,
                            "[${case.source}] setup did not run: ${failure.message}" +
                                "\n  setup: ${case.setup}",
                        )
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

    private fun evaluate(engine: ScxmlScriptEngine, sessionId: String, case: Case): List<Divergence> {
        fun diverged(message: String) = listOf(Divergence(case.source, case.clause, message))

        if (case.asCondition) {
            val expected = case.expected
            val answered =
                try {
                    engine.evaluateCondition(sessionId, case.source)
                } catch (failure: Exception) {
                    return diverged(
                        "[${case.source}] failed to evaluate as a condition: " +
                            "${failure.message} (${case.clause})",
                    )
                }
            val wanted = (expected as? Answer.Bool)?.value
                ?: return diverged("[${case.source}] is a condition but names a non-boolean answer")
            return if (answered == wanted) {
                emptyList()
            } else {
                diverged(
                    "[${case.source}] answered $answered, ECMAScript says " +
                        "${expected.describe()} (${case.clause})",
                )
            }
        }

        val value =
            try {
                engine.evaluateExpr(sessionId, case.source)
            } catch (failure: Exception) {
                return diverged("[${case.source}] failed to evaluate: ${failure.message} (${case.clause})")
            }
        return if (matches(value, case.expected)) {
            emptyList()
        } else {
            diverged(
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
