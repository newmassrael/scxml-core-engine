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
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
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

/**
 * The text rewriter that used to be this engine's only answer, then its
 * fallback behind `sce-build`'s ECMAScript frontend, and is now retired.
 *
 * Two spellings because two questions are asked of them: whether the engine
 * still CALLS it (read from the class body, so a mention in a comment is not a
 * call) and whether the engine's documentation NAMES it.
 *
 * ⚠ They are kept after the retirement rather than deleted with it, because
 * the check they drive is what tells a rewriter that CAME BACK from one that
 * never left — and the arm they select is the one that would then have to be
 * satisfied. The tree-wide claim that no Kotlin file reaches the rewriter is
 * `retirement:kotlin-rewriter-deleted`'s, in
 * `sce-build/tests/lowering_decision_ledger.rs`; this pair is about the
 * engine's own documentation.
 */
private const val REWRITER_NAME = "EcmaScriptToLuaTransformer"
private const val REWRITER_CALL = "transformer."

/**
 * What stands where the fallback stood, and the clause that makes it an answer.
 *
 * `refusedToLower` is read from the class BODY and the clause from the header,
 * which is the same split the pair above uses and for the same reason: one asks
 * what the code does, the other what a reader is told.
 */
private const val REFUSAL_CALL = "refusedToLower("
private const val REFUSAL_CLAUSE = "§scxml-5.9.1"

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

class EcmaScriptSemanticsTest {

    @Test
    fun rhinoAnswersWhatEcmaScriptAnswers() = measure("Rhino") { RhinoScriptEngine() }

    @Test
    fun quickJsAnswersWhatEcmaScriptAnswers() = measure("QuickJS") { QuickJSScriptEngine() }

    /**
     * Lua is measured too, and the assertion runs in both directions.
     *
     * "Both directions" is the part this test did not used to have. It
     * asserted only that the failure set was NOT EMPTY, which is satisfied by
     * one disagreement and by fifty, so the engine could regress or improve
     * for months without a word. Its own KDoc meanwhile carried "27 of its
     * 58" under a sentence saying this test held it to the measurement; it
     * held no number at all, and the shared table had since grown to 98 cases.
     * A declared list is what makes both directions visible — the same shape
     * `tests/ecmascript/lua_engine_divergences.json` gives the C++ selection,
     * and for the same reason its header states: a count that lives in prose
     * is a count nobody re-answers.
     *
     * ⚠ EMPTY IS A LEGAL ANSWER, and forbidding it was this test's own defect.
     * A third assertion here required the declared list to be non-empty, to
     * catch a list this suite had stopped reading — a real failure — but it
     * also failed on a list with nothing left in it, which is the terminal
     * state the whole seam is working towards. A counter whose zero is
     * forbidden is not a counter. What "was the list actually read" now means
     * is that the FILE was read, which [loadDeclaredDivergences] asserts, and
     * `ecma262_scoreboard_contract`'s `readers_of` is what still fails if
     * nothing opens the list at all.
     *
     * The two lists are separate on purpose, and that both are now empty is a
     * measurement rather than a definition: they were 23 and 44 while each
     * backend had its own text rewriter, and neither may be derived from the
     * other.
     */
    @Test
    fun theLuaEngineDivergesExactlyWhereItIsDeclaredTo() {
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
                "without being declared to. Either the engine lost an answer it used to " +
                "give, or $DIVERGENCES_PATH has not caught up with it. If it is the " +
                "second, these are the entries to add:\n" +
                undeclared.joinToString(",\n") {
                    "    { \"source\": ${jsonQuoted(it.source)}, \"clause\": ${jsonQuoted(it.clause)} }"
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
                "reader consults to decide whether their document stays inside what this " +
                "engine covers.\n" +
                repaired.joinToString("\n") { "  ${it.source}  (${it.clause})" },
        )

        // Refutable, and refuted where a consumer can see it: what happens to
        // an expression the frontend REFUSES has to be readable from the
        // engine's own header. An empty divergence list says the frontend
        // answers the shared table's 98 cases; it says nothing about an
        // expression outside that table which the frontend refuses, and that is
        // the one a reader choosing this engine for `datamodel="ecmascript"`
        // has to know the fate of. A header claiming ECMAScript without saying
        // is the shape this file already paid for once, when it read "For
        // AOSP/AAOS production, this replaces Rhino with a faster native
        // engine" over a whole class of the table answered differently.
        //
        // ⚠ TWO STATES, BOTH WITH A REQUIREMENT — and that is the correction
        // this block carries. It used to be one-sided: while the fallback was
        // called the header had to name it, and it asked NOTHING otherwise, so
        // "derived, it retires itself" meant a retirement left the engine free
        // to document nothing about refusal at all. A check with an unasked arm
        // reads exactly like a live one. Now the retired state has its own
        // requirement: the call sites must reach `refusedToLower` and the
        // header must cite the clause that makes refusal the answer.
        val header = File(ENGINE_PATH)
        assertTrue(header.isFile, "the Lua engine is missing at ${header.absolutePath}")
        val engineSource = header.readText()
        val doc = engineSource.substringBefore("class LuaScriptEngine")
        val body = engineSource.substringAfter("class LuaScriptEngine")
        if (REWRITER_CALL in body) {
            assertTrue(
                REWRITER_NAME in doc,
                "$ENGINE_PATH still calls `$REWRITER_CALL` — the text rewriter is the " +
                    "fallback behind every lowering entry point — and its documentation does " +
                    "not name $REWRITER_NAME. An empty $DIVERGENCES_PATH means the frontend " +
                    "answers the ${loadSharedTable().size} cases of the shared table; it does " +
                    "not mean there is no second answer left. A reader choosing this engine " +
                    "for `datamodel=\"ecmascript\"` has to be able to see that an expression " +
                    "the frontend refuses is rewritten rather than refused.",
            )
        } else {
            assertTrue(
                REFUSAL_CALL in body,
                "$ENGINE_PATH no longer calls `$REWRITER_CALL`, so an expression the " +
                    "frontend refuses has to be REFUSED — and nothing in the class body " +
                    "calls `$REFUSAL_CALL`. Neither a rewrite nor a refusal means the text " +
                    "is going somewhere this test cannot see, which is the state a retiring " +
                    "round must not leave behind.",
            )
            assertTrue(
                REFUSAL_CLAUSE in doc,
                "$ENGINE_PATH refuses what the frontend refuses and its documentation does " +
                    "not cite $REFUSAL_CLAUSE. The clause is what makes the refusal an " +
                    "ANSWER rather than a defect — the processor places `error.execution` on " +
                    "the internal queue — and a reader choosing this engine has to be able " +
                    "to see that an expression outside what the frontend parses fails " +
                    "loudly rather than being guessed at.",
            )
        }

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
            // differently, and this suite hands the engine the author's
            // ECMAScript and has it lowered AT RUN TIME against the session's
            // scope — so it is the `runtime-rewriter` contract. The path keeps
            // that name because `EcmaScriptToLuaTransformer` is what used to do
            // that lowering; since it was retired the frontend does, and the
            // route is still the one that can be refused by a scope. The filter
            // is what keeps this from being an assumption: an entry that only
            // build-time lowering gets wrong would otherwise be reported here
            // as a repaired divergence and its deletion demanded.
            //
            // Unclassified is RED, not a default. An entry naming no path is
            // exempt from every per-path suite at once.
            val paths = obj["diverges_on"]
            assertTrue(
                paths != null,
                "the entry ${jsonQuoted(key.source)} / ${jsonQuoted(key.clause)} in $DIVERGENCES_PATH " +
                    "carries no `diverges_on`, so no per-path suite can tell whether it is about it.",
            )
            val onThisPath = paths!!.jsonArray.any { it.jsonPrimitive.content == RUNTIME_REWRITER_PATH }
            if (onThisPath) key else null
        }.toSet()
    }

    private fun measure(engineName: String, create: () -> ScxmlScriptEngine) {
        val failures = collectFailures(create)
        // Every case is reported, not just the first: a build that answers one
        // group wrong and another right is a different problem from one that
        // answers nothing, and the first failure alone cannot tell them apart.
        assertTrue(
            failures.isEmpty(),
            "${failures.size} of ${loadSharedTable().size} expressions disagree with ECMA-262, " +
                "evaluated by $engineName.\n" +
                "An engine offered for `datamodel=\"ecmascript\"` answers what that " +
                "language answers; one that does not is not a choice a consumer can " +
                "make safely, whatever else it is good at.\n" +
                failures.joinToString("\n") { it.message },
        )
    }

    private fun collectFailures(create: () -> ScxmlScriptEngine): List<Divergence> {
        // `loadSharedTable` carries the arity floor: a table that stopped
        // being read must not pass, and asserting it in one place keeps this
        // suite and `LoweredEcma262Test` on the same corpus by construction.
        val cases = loadSharedTable()

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

    private fun evaluate(engine: ScxmlScriptEngine, sessionId: String, case: Ecma262Case): List<Divergence> {
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
        return if (answerMatches(value, case.expected)) {
            emptyList()
        } else {
            diverged(
                "[${case.source}] answered ${describeValue(value)}, ECMAScript says " +
                    "${case.expected.describe()} (${case.clause})",
            )
        }
    }

    /**
     * The row, the answer shapes, the number comparison and the shared
     * table's own floor live in `SharedEcma262Table.kt` beside this file:
     * `LoweredEcma262Test` asks the same table a different question — what
     * this backend's Lua answers for what the FRONTEND emitted — and a copy
     * of the reading in each suite would drift toward whichever one edits it,
     * exactly when a disagreement between the two is the interesting result.
     */
}
