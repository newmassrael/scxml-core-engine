// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// What this backend's Lua answers for what `sce-build`'s frontend EMITTED,
// as opposed to for what the author wrote.
//
// `EcmaScriptSemanticsTest` beside this one measures the other route: the
// author's ECMAScript handed to `LuaScriptEngine`, rewritten on the spot by
// `EcmaScriptToLuaTransformer`, answering 46 of the shared table's cases
// differently. That measurement says nothing about the route this file is
// about, because the two share no code past the session: one runs a text
// rewriter, the other runs Lua the frontend already produced.
//
// Until this file existed nothing on this backend asked the second question,
// and `docs/SCE_LUA_TRANSLATION_SEAM.md` recorded the consequence: the
// sentence "the frontend answers all 98 cases" was a statement about
// `sce-build` plus somebody else's Lua — Go's, Python's, the one `sce-build`
// links — and not about the Lua this backend ships. Go, Python and the Rust
// frontend suite all read `tests/ecmascript/ecma262_emitted_lua.json` for
// exactly this reason and each found defects their W3C suite could not see;
// Go answered four cases differently with two natives that had no Array
// implementation at all.
//
// THE EMISSION IS NOT HANDWRITTEN. It comes from the frontend itself and is
// drift-gated in `sce-build/tests/ecmascript_semantics.rs`. Spelling the Lua
// here would measure a translation nobody ships.

package com.sce.ecmascript

import com.sce.scripting.lua.EcmaScriptToLuaTransformer
import com.sce.scripting.lua.LuaScriptEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.contentOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import java.io.File
import kotlin.test.Test
import kotlin.test.assertTrue

/** The frontend's own output, one row per row of the shared table. */
private const val EMISSION_PATH = "tests/ecmascript/ecma262_emitted_lua.json"

/** What this backend cannot yet answer on the lowered route, enumerated. */
private const val LOWERED_PATH = "tests/ecmascript/kotlin_lowered_ecma262.json"

/** One row of the emission: the Lua a generated machine would actually run. */
private class Emission(val source: String, val setup: String, val expression: String)

class LoweredEcma262Test {

    /**
     * The precondition the measurement rests on, asserted instead of assumed.
     *
     * ⚠ This backend has no way to hand its Lua engine text that is ALREADY
     * Lua. Every entry point on `LuaScriptEngine` — `evaluateExpr`,
     * `evaluateCondition`, `executeScript`, `executeForeach` — runs
     * `EcmaScriptToLuaTransformer` over its argument first, because the
     * generated Kotlin hands it the author's ECMAScript at every site
     * (`Language::Kotlin.supports_script_engine_target(Lua)` is false, and
     * `sce-codegen --script-engine lua -l kotlin` refuses, naming the sites
     * still to move). So a suite that feeds the emission through
     * `evaluateExpr` is running `rewriter(lowered)`, and where the rewriter
     * changes anything it would report ITS answer while claiming to measure
     * the frontend's.
     *
     * That is not hypothetical carelessness. Two fixtures beside this one —
     * `DomReadSurfaceTest` and `EventDataReadingsTest` — say in their own
     * comments that the Lua engine "is handed what the frontend lowered", and
     * neither could be: both pass through the same rewriting entry point.
     * What makes their result mean anything is exactly the property asserted
     * here, and nothing was asserting it.
     *
     * So the set of cases the rewriter CHANGES is enumerated rather than
     * counted, and held in both directions. Where it leaves the emission
     * alone, `rewriter(lowered) == lowered` and the measurement below is
     * about the Lua interpreter and the runtime library underneath it —
     * which is what a `--script-engine lua` artifact would run.
     */
    @Test
    fun theRewriterChangesExactlyTheCasesDeclaredUnreachable() {
        val cases = loadSharedTable()
        val emitted = loadEmission(cases)
        val transformer = EcmaScriptToLuaTransformer()

        val changed = mutableMapOf<Key, String>()
        cases.forEachIndexed { index, case ->
            val emission = emitted[index]
            val notes = mutableListOf<String>()
            val rewrittenExpr = transformer.transform(emission.expression)
            if (rewrittenExpr != emission.expression) {
                notes += "expression\n      emitted:   ${emission.expression}" +
                    "\n      rewritten: $rewrittenExpr"
            }
            if (emission.setup.isNotEmpty()) {
                val rewrittenSetup = transformer.transformScript(emission.setup)
                if (rewrittenSetup != emission.setup) {
                    notes += "setup\n      emitted:   ${emission.setup}" +
                        "\n      rewritten: $rewrittenSetup"
                }
            }
            if (notes.isNotEmpty()) {
                changed[case.key] = notes.joinToString("\n    ")
            }
        }

        val declared = loadDeclaredKeys("unreachable")

        // Ordered before anything else: a run where the rewriter started
        // touching a case prints it in the shape the file takes, rather than
        // failing further down with nothing for the person who has to write
        // the entry.
        val undeclared = changed.keys.filterNot { it in declared }
        assertTrue(
            undeclared.isEmpty(),
            "${undeclared.size} emitted case(s) are CHANGED by " +
                "`EcmaScriptToLuaTransformer` without being declared unreachable in " +
                "$LOWERED_PATH. Whatever the engine answers for them is an answer about " +
                "the rewriter's handling of Lua text, not about the frontend's output, " +
                "so they cannot be counted as a lowering result. These are the entries " +
                "to add:\n" +
                undeclared.joinToString(",\n") {
                    "    { \"source\": ${jsonQuoted(it.source)}, " +
                        "\"clause\": ${jsonQuoted(it.clause)} }"
                } +
                "\n\nWhat the rewriter did:\n" +
                undeclared.joinToString("\n") { "  [${it.source}] ${changed[it]}" },
        )

        val nowReachable = declared.filterNot { it in changed.keys }
        assertTrue(
            nowReachable.isEmpty(),
            "${nowReachable.size} case(s) declared unreachable in $LOWERED_PATH now pass " +
                "through the rewriter unchanged, so this suite CAN ask them. Remove them " +
                "from `unreachable` — a list that keeps a case it no longer describes is " +
                "an exemption nothing can fault, and every entry it keeps is one fewer " +
                "case this backend is measured on.\n" +
                nowReachable.joinToString("\n") { "  ${it.source}  (${it.clause})" },
        )
    }

    /**
     * The measurement: the emission, run on the engine this backend ships.
     *
     * Every case is reported rather than the first, for the reason every
     * reader of this table states — a backend that answers one group wrong
     * and another right is a different defect from one that answers nothing,
     * and the first failure alone cannot tell them apart.
     *
     * Held in both directions like every list beside it. A declared entry
     * that starts answering correctly fails here, because the entry is what a
     * reader consults to decide whether the lowered route is usable, and a
     * list that keeps a repaired case cannot be trusted in the other
     * direction either.
     */
    @Test
    fun theFrontendsLuaAnswersEcma262OnThisBackend() {
        val cases = loadSharedTable()
        val emitted = loadEmission(cases)
        val unreachable = loadDeclaredKeys("unreachable")

        // The escape hatch is bounded by what it leaves behind. `unreachable`
        // is an exemption from the measurement, so a list that grew to cover
        // the table would leave a suite that passes by asking nothing — the
        // same floor the shared table itself carries, applied to the cases
        // actually put to the engine rather than to the ones on disk.
        val asked = cases.filterNot { it.key in unreachable }
        assertTrue(
            asked.size >= SHARED_TABLE_FLOOR,
            "only ${asked.size} of ${cases.size} case(s) are still asked on the lowered " +
                "route; ${unreachable.size} are declared unreachable in $LOWERED_PATH and " +
                "the floor is $SHARED_TABLE_FLOOR. An exemption list this wide is not a " +
                "measurement with a residue, it is a suite that has stopped measuring.",
        )

        val engine = LuaScriptEngine()
        val failures = mutableMapOf<Key, String>()

        cases.forEachIndexed { index, case ->
            if (case.key in unreachable) return@forEachIndexed
            val emission = emitted[index]
            val sessionId = "lowered_ecma262_case_$index"
            engine.createSession(sessionId)
            try {
                engine.setupSystemVariables(sessionId, "lowered_ecma262")
                if (emission.setup.isNotEmpty()) {
                    try {
                        engine.executeScript(sessionId, emission.setup)
                    } catch (failure: Exception) {
                        failures[case.key] = "setup did not run: ${failure.message}" +
                            "\n    emitted: ${emission.setup}"
                        return@forEachIndexed
                    }
                }
                val answered =
                    try {
                        engine.evaluateExpr(sessionId, emission.expression)
                    } catch (failure: Exception) {
                        failures[case.key] = "failed to evaluate: ${failure.message}" +
                            "\n    emitted: ${emission.expression}"
                        return@forEachIndexed
                    }
                if (!answerMatches(answered, case.expected)) {
                    failures[case.key] = "answered ${describeValue(answered)}, " +
                        "ECMAScript says ${case.expected.describe()}" +
                        "\n    emitted: ${emission.expression}"
                }
            } finally {
                engine.destroySession(sessionId)
            }
        }

        val declared = loadDeclaredKeys("divergences")

        val undeclared = failures.keys.filterNot { it in declared }
        assertTrue(
            undeclared.isEmpty(),
            "${undeclared.size} of the ${asked.size} emitted expression(s) this suite asks " +
                "disagree with ECMA-262 on this backend's Lua without being declared in " +
                "$LOWERED_PATH.\n" +
                "A document that declares `datamodel=\"ecmascript\"` answers what that " +
                "language answers, whichever backend it was generated for. What is " +
                "measured here is this backend's Lua interpreter and the runtime library " +
                "beside it — the layer a `--script-engine lua` artifact would run, and " +
                "the layer the W3C suite cannot see, because a suite that is green end to " +
                "end never asks `0 && x`. These are the entries to add:\n" +
                undeclared.joinToString(",\n") {
                    "    { \"source\": ${jsonQuoted(it.source)}, " +
                        "\"clause\": ${jsonQuoted(it.clause)} }"
                } +
                "\n\nWhat each one answered:\n" +
                undeclared.joinToString("\n") { "  [${it.source}] ${failures[it]}" },
        )

        val repaired = declared.filterNot { it in failures.keys }
        assertTrue(
            repaired.isEmpty(),
            "${repaired.size} declared divergence(s) no longer describe this backend's " +
                "Lua on the lowered route. Remove them from $LOWERED_PATH — this is the " +
                "direction that lets the list empty, and it is the whole point of writing " +
                "it down instead of counting it in prose.\n" +
                repaired.joinToString("\n") { "  ${it.source}  (${it.clause})" },
        )
    }

    /**
     * One declared array of the lowered file, keyed the way the measurements
     * are.
     *
     * Read from disk rather than held in this file: these are measurements a
     * person maintains and a reader consults, and a Kotlin constant is
     * neither greppable beside the other lists in `tests/ecmascript/` nor
     * readable by anyone not building this backend.
     */
    private fun loadDeclaredKeys(array: String): Set<Key> {
        val file = File(LOWERED_PATH)
        assertTrue(
            file.isFile,
            "the lowered-route declarations are missing at ${file.absolutePath}. This " +
                "suite compares against them in both directions, so without the file it " +
                "measures nothing.",
        )
        val root = Json.parseToJsonElement(file.readText()).jsonObject
        val entries = root[array]
        assertTrue(
            entries != null,
            "$LOWERED_PATH has no `$array` array. Each of its two arrays is held by one " +
                "assertion here, and a missing one is an exemption nothing can fault " +
                "rather than an empty one.",
        )
        return entries!!.jsonArray.map { entry ->
            val obj = entry.jsonObject
            Key(
                obj.getValue("source").jsonPrimitive.content,
                obj.getValue("clause").jsonPrimitive.content,
            )
        }.toSet()
    }

    /**
     * The emission, paired with the table and cross-checked against it.
     *
     * Two files rather than one because they answer different questions — the
     * table says what ECMAScript answers, the emission says what this backend
     * is handed — and the pairing is asserted rather than assumed: an
     * emission regenerated from a table that has since gained a row would
     * otherwise line case 40 up against case 41's answer and report a defect
     * that is really a stale file.
     */
    private fun loadEmission(cases: List<Ecma262Case>): List<Emission> {
        val file = File(EMISSION_PATH)
        assertTrue(
            file.isFile,
            "the frontend's emission is missing at ${file.absolutePath}. Regenerate it " +
                "with `UPDATE_EXPECT=1 cargo test -p sce-build --test ecmascript_semantics`.",
        )
        val root = Json.parseToJsonElement(file.readText()).jsonObject
        val emitted = root.getValue("cases").jsonArray.map { entry ->
            val obj = entry.jsonObject
            Emission(
                source = obj.getValue("source").jsonPrimitive.content,
                setup = obj["setup"]?.jsonPrimitive?.contentOrNull.orEmpty(),
                expression = obj.getValue("expression").jsonPrimitive.content,
            )
        }

        assertTrue(
            emitted.size == cases.size,
            "the shared table holds ${cases.size} case(s) and $EMISSION_PATH ${emitted.size} " +
                "— regenerate the emission with `UPDATE_EXPECT=1 cargo test -p sce-build " +
                "--test ecmascript_semantics`",
        )
        cases.forEachIndexed { index, case ->
            assertTrue(
                case.source == emitted[index].source,
                "case $index is ${jsonQuoted(case.source)} in the shared table and " +
                    "${jsonQuoted(emitted[index].source)} in $EMISSION_PATH — the two files " +
                    "are out of step, so every answer below would be compared against the " +
                    "wrong claim",
            )
        }
        return emitted
    }
}
