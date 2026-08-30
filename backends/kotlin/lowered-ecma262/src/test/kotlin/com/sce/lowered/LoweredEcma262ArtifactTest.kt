// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// A Lua-lowered KOTLIN artifact, compiled and RUN, answers ECMA-262.
//
// The twin of `tests/engine/LoweredEcma262Test.cpp`, and it closes the gap
// that file's own header names for the other backend: until a generated
// artifact is compiled with the build-time lowering in it and driven to its
// final state, "the frontend answers the shared table" is a statement about
// `sce-build` plus somebody else's Lua.
//
// ── What is measured, and how it differs from its neighbours ─────
//
// `EcmaScriptSemanticsTest` hands `LuaScriptEngine` the author's ECMAScript
// directly and measures what comes back. `LoweredEcma262Test` (the one beside
// it, not the C++ file) hands the engine the Lua the frontend emitted, read
// out of a committed table. Neither runs an ARTIFACT: both call an engine
// entry point with text a test chose.
//
// This one runs two generated state machines. The subject is emitted with
// `--script-engine lua`, so every expression reaches the engine as
// `ScriptSource.lua(lowered, source)` and `LuaScriptEngine` passes the lowered
// text through untouched — the build-time route, end to end, with no run-time
// translation of any kind in it. The control is emitted the way this backend
// emits by default, `ScriptSource.ecmascript(...)`, which the SAME engine
// offers to the SAME frontend at RUN time.
//
// Two routes, one answer expected of both, and
// `tests/ecmascript/kotlin_lua_divergences.json` names them exactly:
// `build-time-lowering` and `runtime-rewriter`. That file has held both names
// and had a reader for only one of them.
//
// ── The population is the shared table, in full ──────────────────
//
// Not the divergence list. A path's divergences cannot be enumerated by a list
// built from a different path's failures, and a fixture shaped by the list the
// harness then checks can only ask questions the list has already chosen. The
// list is read here as the EXPECTATION — which cases each route gets wrong —
// and held in both directions, because a list that only forbids surprises can
// never empty.
//
// ⚠ Both directions matter even while the list is empty. The direction that is
// live today is "an undeclared wrong answer is red": 98 cases, each answered
// per ECMA-262 or the suite fails. The other direction — "a declared case
// answered correctly is red" — is what lets the list reach zero and stay
// honest, and it is vacuous only in the state the project is trying to be in.

package com.sce.lowered

import com.sce.generated.ecma262_lowered.Ecma262LoweredStateMachine
import com.sce.generated.ecma262_source.Ecma262SourceStateMachine
import com.sce.runtime.StateMachineEngine
import com.sce.scripting.lua.LuaScriptEngine
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.doubleOrNull
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.io.File

/** What the shared table says the language answers. */
private const val CASES_PATH = "tests/ecmascript/ecma262_semantics.json"

/** Which of those cases each route into the Lua engine still gets wrong. */
private const val DIVERGENCES_PATH = "tests/ecmascript/kotlin_lua_divergences.json"

/**
 * Which of those cases a generated ARTIFACT gets wrong for a reason that is
 * not about the language.
 *
 * A separate file from the divergence list, and separate on purpose: folding a
 * code-generation defect into a language-divergence list would make that list
 * non-empty and would say the frontend gets a case wrong, which is false.
 */
private const val DEFECTS_PATH = "tests/ecmascript/kotlin_lowered_artifact_defects.json"

/**
 * The ceiling on [DEFECTS_PATH].
 *
 * An exclusion list with no ceiling is a way to make any lane green. Every
 * entry there is a defect someone is expected to remove, so the list is small
 * by construction and this refuses it growing into a second population.
 */
private const val MAX_DEFECTS = 3

/**
 * The two routes the list may name.
 *
 * Read back out of the list rather than assumed, in [joinPopulation]: an entry
 * naming a route nothing measures is a claim no lane can fault.
 */
private const val PATH_BUILD_TIME_LOWERING = "build-time-lowering"
private const val PATH_RUNTIME_REWRITER = "runtime-rewriter"

/**
 * The fixture's encoding of a guard's two outcomes, and of an expression the
 * engine would not evaluate.
 *
 * Spelled in `tools/generate_lowered_ecma262_fixture.py` and repeated here
 * rather than imported, because the two files are the two halves of the gate:
 * a harness that read its protocol out of the generator could not disagree
 * with it, and a gate whose two halves share a source is not a gate.
 */
private const val COND_HELD = 1L
private const val COND_NOT_HELD = 2L
private const val UNEVALUATED = "<unevaluated>"

/** The probe controls the fixture opens every run with. */
private const val CONTROL_REFUSED = "ctlRefused"
private const val CONTROL_EVALUABLE = "ctlEvaluable"

/**
 * The floor.
 *
 * A table that shrank to nothing would score every route perfectly. Same
 * number as `MIN_CASES` in the fixture generator, in `ecma262_scoreboard_contract`
 * and in the C++ twin — one population, one floor.
 */
private const val MIN_CASES = 55

/** One case of the shared table, joined with whatever the list says about it. */
private class Case(
    /**
     * Index into the table's `cases`, which is the fixture's own key: state
     * `dN` asks case N. The alignment is by POSITION in one file rather than
     * by a name either side could spell.
     */
    val index: Int,
    val source: String,
    val clause: String,
    val asCondition: Boolean,
    val expect: JsonObject,
    val declared: Boolean,
    val paths: Set<String>,
    /**
     * Declared in [DEFECTS_PATH] — a generated artifact gets this wrong for a
     * reason that is not the language.
     *
     * Route-independent by construction: a code-generation defect affects
     * every artifact, which is exactly how this one was told apart from a
     * lowering divergence. A divergence of a LOWERING route cannot show up on
     * the route that does no lowering, and this case shows up on both.
     */
    val codegenDefect: Boolean,
) {
    val divergesOnLowering: Boolean get() = PATH_BUILD_TIME_LOWERING in paths
    val divergesOnRewriter: Boolean get() = PATH_RUNTIME_REWRITER in paths
    override fun toString(): String = "[$source] ($clause)"
}

/** The four readings one answer slot can carry, which are four findings. */
private enum class Reading {
    /**
     * `answers.rN` absent — the case's state was never entered. Never a pass,
     * and not the same finding as a wrong answer: it says the fixture and this
     * harness disagree about the population, or the machine stopped early.
     */
    NotReached,

    /** The slot still holds the sentinel — the engine refused the expression. */
    Refused,

    /**
     * Reached, and the slot is absent: the expression evaluated to
     * null/undefined, which the engine's JSON encoding omits. This is what the
     * shared table spells `{"empty": true}`.
     */
    Empty,

    /** The slot holds a value. */
    Value,
}

private class Answer(
    val reading: Reading,
    val value: JsonElement? = null,
    /**
     * Could the engine evaluate the case's expression at all?
     *
     * For a condition case this is the PROBE's answer and it is load-bearing
     * rather than diagnostic: §scxml-5.9.1 makes a guard the engine refused
     * evaluate to false, so without it every case whose ECMA-262 answer is
     * `false` would pass on an expression that could not be parsed.
     */
    val evaluable: Boolean = false,
) {
    val isValue: Boolean get() = reading == Reading.Value
}

/** One artifact's whole run: the answers it recorded, and what it is called. */
private class Run(val label: String, val answers: JsonObject?)

class LoweredEcma262ArtifactTest {

    private companion object {
        val JSON = Json { ignoreUnknownKeys = true }

        fun readJson(path: String, what: String): JsonObject {
            val file = File(path)
            check(file.isFile) {
                "cannot read $what at $path — this suite's workingDir is the repository root"
            }
            return JSON.parseToJsonElement(file.readText()).jsonObject
        }

        /**
         * The shared table, each case carrying what the divergence list claims.
         *
         * `(source, clause)` identifies a case; `source` alone does not, since
         * the table asks `a && b` under two clauses. The join is computed from
         * the two committed files rather than imported from the generator.
         */
        fun joinPopulation(): List<Case> {
            val table = readJson(CASES_PATH, "the shared ECMA-262 table")
            val list = readJson(DIVERGENCES_PATH, "the divergence list")
            val defectList = readJson(DEFECTS_PATH, "the artifact code-generation defect list")

            val defects = defectList["defects"]!!.jsonArray.map {
                val obj = it.jsonObject
                obj["source"]!!.jsonPrimitive.content to obj["clause"]!!.jsonPrimitive.content
            }.toSet()
            assertTrue(
                defects.size <= MAX_DEFECTS,
                "$DEFECTS_PATH names ${defects.size} defect(s), over the ceiling of " +
                    "$MAX_DEFECTS. An exclusion list with no ceiling is a way to make any " +
                    "lane green; every entry here is meant to be removed, not accumulated."
            )

            val declarable = list["paths"]?.jsonArray
                ?.map { it.jsonPrimitive.content }
                ?.toSet()
                ?: emptySet()
            assertTrue(
                PATH_BUILD_TIME_LOWERING in declarable,
                "$DIVERGENCES_PATH does not list `$PATH_BUILD_TIME_LOWERING` among the " +
                    "paths its entries may name, yet this suite is that path's contract. " +
                    "Either the list stopped tracking the route this lane measures, or " +
                    "this lane is measuring a backend the list is not about."
            )

            val declaredBy = HashMap<Pair<String, String>, Set<String>>()
            for (entry in list["divergences"]?.jsonArray.orEmpty()) {
                val obj = entry.jsonObject
                val key = obj["source"]!!.jsonPrimitive.content to obj["clause"]!!.jsonPrimitive.content
                val paths = obj["diverges_on"]?.jsonArray?.map { it.jsonPrimitive.content }?.toSet()
                assertTrue(
                    !paths.isNullOrEmpty(),
                    "$DIVERGENCES_PATH entry $key has an EMPTY or absent `diverges_on`. " +
                        "Every route answers it, so it is not a divergence of any of them."
                )
                declaredBy[key] = paths!!
            }

            val joined = table["cases"]!!.jsonArray.mapIndexed { index, element ->
                val c = element.jsonObject
                val source = c["source"]!!.jsonPrimitive.content
                val clause = c["clause"]!!.jsonPrimitive.content
                val paths = declaredBy[source to clause]
                Case(
                    index = index,
                    source = source,
                    clause = clause,
                    asCondition = c["form"]?.jsonPrimitive?.content == "condition",
                    expect = c["expect"]!!.jsonObject,
                    declared = paths != null,
                    paths = paths ?: emptySet(),
                    codegenDefect = (source to clause) in defects,
                )
            }

            // An entry that excuses NOTHING is a claim no lane can fault, and
            // it is the way an exclusion list rots: the "remove it when the
            // artifact answers it" direction can never fire on a case that is
            // not in the population, so a phantom entry sits there for ever
            // being cited as a known defect. The same reasoning the divergence
            // list's empty-`diverges_on` check is built on.
            val unmatched = defects - joined.map { it.source to it.clause }.toSet()
            assertTrue(
                unmatched.isEmpty(),
                "${unmatched.size} entry/entries in $DEFECTS_PATH name no case in " +
                    "$CASES_PATH, so nothing here can ever answer them correctly and " +
                    "nothing can ever remove them:\n  " +
                    unmatched.joinToString("\n  ") { "[${it.first}] (${it.second})" }
            )
            return joined
        }

        /**
         * Drive one artifact to its final state and read the answers back.
         *
         * ⚠ The read itself goes through `JSON.stringify(answers)`, which the
         * generated accessor asks the engine as ECMAScript. That is a
         * run-time translation, and it is deliberately OUTSIDE what this lane
         * measures: it is one fixed expression, identical on both artifacts,
         * and it reads the recording rather than producing it. Every answer in
         * the object was already computed by the route under test before this
         * call is made.
         */
        fun drive(label: String, sm: StateMachineEngine<*, *>, answers: () -> String?): Run {
            sm.initialize()
            assertTrue(
                sm.isInFinalState,
                "the $label artifact did not reach its final state, so the answers below " +
                    "would be a partial run reported as a full one"
            )
            val recorded = answers()
            assertTrue(
                recorded != null,
                "the $label artifact's `answers` datamodel object could not be read back — " +
                    "no engine, no session, or the engine refused `JSON.stringify(answers)`"
            )
            return Run(label, JSON.parseToJsonElement(recorded!!).jsonObject)
        }

        fun probeSaysEvaluable(answers: JsonObject, c: Case): Boolean {
            val probe = answers["v${c.index}"]
                // The probe assignment yielded null/undefined, which the
                // engine's JSON encoding omits. It evaluated; it just produced
                // nothing to hold.
                ?: return answers.containsKey("r${c.index}")
            val primitive = probe as? JsonPrimitive ?: return true
            return !(primitive.isString && primitive.content == UNEVALUATED)
        }

        fun readAnswer(answers: JsonObject, c: Case): Answer {
            if (!answers.containsKey("r${c.index}")) {
                return Answer(Reading.NotReached)
            }
            val slot = answers["d${c.index}"]
                ?: return Answer(Reading.Empty, evaluable = !c.asCondition)
            val primitive = slot as? JsonPrimitive
            if (primitive != null && primitive.isString && primitive.content == UNEVALUATED) {
                return Answer(Reading.Refused)
            }
            return Answer(
                Reading.Value,
                value = slot,
                evaluable = if (c.asCondition) probeSaysEvaluable(answers, c) else true,
            )
        }

        /**
         * Does one recorded answer agree with ECMA-262?
         *
         * Numbers are compared as doubles because the engine families hold
         * them differently and the shared table says so. Everything else is
         * compared by type as well as by value: `0`, `false` and `""` are
         * three different answers, and truthiness is exactly where a
         * Lua-shaped translation confuses them.
         */
        fun agrees(answer: Answer, c: Case): Boolean {
            if (c.asCondition) {
                val primitive = (answer.value as? JsonPrimitive)?.takeIf { !it.isString }
                val held = primitive?.longOrNull ?: return false
                if (!answer.isValue) return false
                // §scxml-5.9.1 makes a `cond` the engine refused evaluate to
                // FALSE, so a guard that did not hold is two findings wearing
                // one verdict. A refusal is never an answer about the
                // language, however well it matches the expectation.
                if (!answer.evaluable) return false
                val wanted = c.expect["bool"]?.jsonPrimitive?.booleanOrNull ?: return false
                return held == (if (wanted) COND_HELD else COND_NOT_HELD)
            }
            if (c.expect.containsKey("empty")) {
                // The one shape that is an ABSENCE: a pass only when the case
                // was reached and the slot went from the sentinel to nothing.
                return answer.reading == Reading.Empty
            }
            if (!answer.isValue) return false
            val primitive = answer.value as? JsonPrimitive ?: return false
            c.expect["bool"]?.jsonPrimitive?.booleanOrNull?.let { wanted ->
                return !primitive.isString && primitive.booleanOrNull == wanted
            }
            c.expect["number"]?.jsonPrimitive?.doubleOrNull?.let { wanted ->
                return !primitive.isString && primitive.doubleOrNull == wanted
            }
            c.expect["string"]?.jsonPrimitive?.let { wanted ->
                return primitive.isString && primitive.content == wanted.content
            }
            return false
        }

        /** What one probe control recorded, spelled the way the census prints it. */
        fun controlReading(answers: JsonObject, slot: String): String {
            val value = answers[slot] ?: return "<absent>"
            val primitive = value as? JsonPrimitive ?: return value.toString()
            return if (primitive.isString) primitive.content else primitive.content
        }

        val population: List<Case> by lazy { joinPopulation() }

        val lowered: Run by lazy {
            val sm = Ecma262LoweredStateMachine(LuaScriptEngine())
            drive("lowered", sm) { sm.answers() }
        }

        val control: Run by lazy {
            val sm = Ecma262SourceStateMachine(LuaScriptEngine())
            drive("source-passing control", sm) { sm.answers() }
        }
    }

    /**
     * The population is the shared table in full, and large enough to mean
     * something.
     *
     * A floor rather than an exact count: cases are added to the table and the
     * lane must not have to be edited for that. What the floor stops is the
     * table shrinking to a size where "answers every case" is cheap.
     */
    @Test
    fun the_population_is_the_shared_table_in_full() {
        assertTrue(
            population.size >= MIN_CASES,
            "the shared table joined ${population.size} case(s), under the floor of " +
                "$MIN_CASES. A population this small would score both routes perfectly " +
                "without either of them being right."
        )
    }

    /**
     * The refusal probe reported BOTH outcomes, on both artifacts, on this run.
     *
     * [agrees] refuses to read a condition verdict whose probe says the engine
     * could not evaluate the expression, so a probe stuck on one answer
     * decides the whole measurement by itself: stuck on "refused" makes every
     * condition case a divergence, stuck on "evaluated" makes a genuine
     * §scxml-5.9.1 refusal read as the answer `false`.
     *
     * The fixture opens every run with two controls for exactly this — one
     * expression that cannot be evaluated (a member of an absent object) and
     * one that plainly can (a literal) — and this asserts both came back as
     * themselves. Held here rather than only in the gate because the gate
     * reads these off the census line this suite prints.
     */
    @Test
    fun the_refusal_probe_distinguished_both_outcomes_on_both_artifacts() {
        for (run in listOf(lowered, control)) {
            val answers = run.answers!!
            val refused = controlReading(answers, CONTROL_REFUSED)
            val evaluable = controlReading(answers, CONTROL_EVALUABLE)
            assertTrue(
                refused == UNEVALUATED,
                "the ${run.label} artifact's refusal control read '$refused', not the " +
                    "unevaluated sentinel — the probe is not reporting §scxml-5.9.1 " +
                    "refusals, so a guard the engine would not parse reads as the answer false"
            )
            assertTrue(
                evaluable != UNEVALUATED,
                "the ${run.label} artifact's evaluable control read the unevaluated " +
                    "sentinel over a literal — the probe is stuck on refusal, so every " +
                    "condition case is a divergence by construction"
            )
        }
    }

    /**
     * BUILD-TIME lowering answers the language exactly where it is not
     * declared to diverge.
     *
     * The subject. Every expression in this artifact reached the engine as Lua
     * the frontend produced at BUILD time, so nothing in this run translated
     * anything — which is the claim `docs/SCE_LUA_TRANSLATION_SEAM.md` needs
     * before `Language::Kotlin.default_script_engine_target()` can move.
     */
    @Test
    fun build_time_lowering_answers_ecma262_where_it_is_not_declared_to_diverge() {
        assertRoute(lowered, PATH_BUILD_TIME_LOWERING) { it.divergesOnLowering }
    }

    /**
     * The RUN-TIME route answers the language exactly where it is not declared
     * to diverge.
     *
     * The control, and it is a control in the sense that matters: it is the
     * artifact this backend emits by DEFAULT today, so a run in which the two
     * agree is a run in which the flip would change no answer. It is not a
     * control in the C++ sense of bringing back a rewriter's divergences —
     * this backend's Lua engine reaches the same frontend at run time, and
     * `kotlin_lua_divergences.json` declares nothing for either route.
     *
     * ⚠ Which is why this cannot be the lane's only assertion, and is not: a
     * suite in which two artifacts agree proves nothing about either unless
     * something says they were DIFFERENT artifacts. That is
     * `scripts/gates/ecma262-lowered-kotlin.sh`, which counts
     * `ScriptSource.lua(` in each emitted machine and refuses a run where the
     * subject carries none or the control carries any.
     */
    @Test
    fun the_run_time_route_answers_ecma262_where_it_is_not_declared_to_diverge() {
        assertRoute(control, PATH_RUNTIME_REWRITER) { it.divergesOnRewriter }
    }

    /**
     * The census, printed so a GREEN run states what it measured.
     *
     * A number that only exists on a red run is a number nobody can cite from
     * a green one. The gate lifts this line out of the log and reads the probe
     * controls off it.
     */
    @Test
    fun the_run_reports_what_it_measured() {
        val answers = lowered.answers!!
        val controlAnswers = control.answers!!
        val declared = population.count { it.declared }
        println(
            "LoweredEcma262Kotlin census: cases=${population.size} declared=$declared " +
                "codegen-defects=${population.count { it.codegenDefect }} " +
                "lowered-control-refused=${controlReading(answers, CONTROL_REFUSED)} " +
                "lowered-control-evaluable=${controlReading(answers, CONTROL_EVALUABLE)} " +
                "source-control-refused=${controlReading(controlAnswers, CONTROL_REFUSED)} " +
                "source-control-evaluable=${controlReading(controlAnswers, CONTROL_EVALUABLE)}"
        )
    }

    /**
     * One route, held in BOTH directions.
     *
     * A case the route gets wrong without being declared is red — that is the
     * live direction. A case declared for this route and answered CORRECTLY is
     * red too, and that direction is what lets the list empty: without it a
     * list can only grow, and every entry that stopped being true would sit
     * there unfalsifiable.
     */
    private fun assertRoute(run: Run, path: String, declaresThisRoute: (Case) -> Boolean) {
        val answers = run.answers!!
        val unexpected = mutableListOf<String>()
        val fixed = mutableListOf<String>()

        for (case in population) {
            val answer = readAnswer(answers, case)
            val ok = agrees(answer, case)
            if (case.codegenDefect) {
                // Held in the same two directions, and the direction that
                // matters is this one: the day the generator stops
                // re-evaluating a guard, this case starts passing and the
                // entry has to go. Without it the list could only grow.
                if (ok) {
                    fixed += "$case — declared in $DEFECTS_PATH as a code-generation " +
                        "defect, and the ${run.label} artifact answers it correctly. The " +
                        "defect is fixed: remove the entry."
                }
                continue
            }
            if (declaresThisRoute(case)) {
                if (ok) {
                    fixed += "$case — declared to diverge on $path, and answered correctly"
                }
            } else if (!ok) {
                unexpected += "$case — expected ${case.expect}, " +
                    "read ${answer.reading}${answer.value?.let { " $it" } ?: ""}" +
                    (if (case.asCondition && !answer.evaluable) " (probe: the engine could not evaluate it)" else "")
            }
        }

        assertTrue(
            unexpected.isEmpty(),
            "${unexpected.size} case(s) the ${run.label} artifact answers differently from " +
                "ECMA-262 without $DIVERGENCES_PATH declaring `$path` for them:\n  " +
                unexpected.joinToString("\n  ")
        )
        assertTrue(
            fixed.isEmpty(),
            "${fixed.size} case(s) are declared to diverge on `$path` and the " +
                "${run.label} artifact answers them correctly. Remove the entry — a list " +
                "that only ever grows cannot reach zero, which is this seam's finish line:" +
                "\n  " + fixed.joinToString("\n  ")
        )
    }
}
