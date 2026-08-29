// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The language seam on `ScxmlScriptEngine`: does handing an engine Lua the
// frontend already produced actually SKIP the rewriter, and does an engine
// that cannot read Lua refuse it?
//
// The Kotlin sibling of `tests/engine/ScriptLanguageSeamTest.cpp` (ctest
// `ScriptLanguageSeam`), and it is built the same way for the same reason:
// every case carries its own CONTROL, so none of them can pass by measuring
// nothing. A test that only asserted "lowered `a[1]` answers 10" would pass on
// an engine that ignored the tag entirely and got there by rewriting — the
// control is the same characters tagged ECMAScript, which must answer
// something DIFFERENT.

package com.sce.ecmascript

import com.sce.runtime.ScriptLanguage
import com.sce.runtime.ScriptSource
import com.sce.runtime.ScxmlScriptEngine
import com.sce.scripting.RhinoScriptEngine
import com.sce.scripting.lua.LuaScriptEngine
import com.sce.scripting.quickjs.QuickJSScriptEngine
import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class ScriptLanguageSeamTest {

    private fun <T> withSession(engine: ScxmlScriptEngine, id: String, body: (String) -> T): T {
        engine.createSession(id)
        try {
            engine.setupSystemVariables(id, "script_language_seam")
            return body(id)
        } finally {
            engine.destroySession(id)
        }
    }

    /**
     * The one that proves the skip is real.
     *
     * `a[1]` means different elements in the two languages — ECMAScript is
     * zero-based and Lua is one-based — so the SAME CHARACTERS answer the
     * author's second element when tagged `ecmascript` and the author's first
     * when tagged `lua`. An engine that ignored the tag and rewrote everything
     * would answer the same value twice, and that is what
     * [assertNotEquals] refuses.
     *
     * This is the shape a lowered artifact actually hits: `sce-build`'s
     * frontend emits one-based Lua indices, and an engine that rewrote them
     * again would shift a correct expression off by one — silently, because
     * both readings are valid Lua.
     */
    @Test
    fun theLoweredArmSkipsTheRewriterAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_index") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            val lowered = engine.evaluateExpr(id, ScriptSource.lua("a[1]", "a[0]"))
            val rewritten = engine.evaluateExpr(id, ScriptSource.ecmascript("a[1]"))

            assertEquals(
                10L,
                (lowered as Number).toLong(),
                "lowered `a[1]` must read the author's FIRST element: the frontend already " +
                    "emitted a one-based index, and an engine that rewrote it again would " +
                    "shift a correct expression off by one",
            )
            assertEquals(
                20L,
                (rewritten as Number).toLong(),
                "the author's `a[1]` is zero-based, so the rewriter must turn it into the " +
                    "SECOND element — this is the control",
            )
            assertNotEquals(
                lowered.toLong(),
                rewritten.toLong(),
                "the two arms answered the same value, so the tag changed nothing and this " +
                    "suite is measuring one path twice",
            )
        }
    }

    /**
     * The script arm, and its control, on the same defect shape.
     *
     * `executeScript` is the other half of what `LoweredEcma262Test` needs: a
     * case's setup is a script, and a setup that got rewritten would leave the
     * session in a state no lowered artifact would produce.
     */
    @Test
    fun theLoweredScriptArmSkipsTheRewriterToo() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_script") { id ->
            // A Lua table constructor with a bracketed key. The rewriter
            // mangles this shape — it is one of the twelve cases
            // `kotlin_lowered_ecma262.json` declares unreachable — so a run
            // that skipped the rewriter can read the key back and a run that
            // did not cannot.
            engine.executeScript(id, ScriptSource.lua("""o = {["k"] = 7}""", """var o = {k: 7};"""))
            val lowered = engine.evaluateExpr(id, ScriptSource.lua("o.k", "o.k"))
            assertEquals(
                7L,
                (lowered as Number).toLong(),
                "a lowered script must reach the interpreter as written: this table " +
                    "constructor is one of the shapes the rewriter destroys",
            )
        }

        // The control, in its own session so the first cannot have set it up:
        // the same Lua handed over as if the author had written it comes back
        // through the rewriter, and does NOT produce a readable `o.k`.
        val control = LuaScriptEngine()
        withSession(control, "seam_control_script") { id ->
            val readBack = try {
                control.executeScript(id, ScriptSource.ecmascript("""o = {["k"] = 7}"""))
                control.evaluateExpr(id, ScriptSource.lua("o.k", "o.k"))
            } catch (expected: Exception) {
                null
            }
            assertNotEquals(
                7L,
                (readBack as? Number)?.toLong(),
                "the same characters tagged `ecmascript` came back readable, so the rewriter " +
                    "left them alone and this case's control proves nothing about the seam",
            )
        }
    }

    /**
     * The guard arm, and its control.
     *
     * `cond` is the entry point a generated machine reaches most often — every
     * transition carries one — and it was the last of the five to get an arm.
     * The same characters answer differently in the two languages for the same
     * reason the expression arm does, so a `true` here against a `false` in the
     * control is what proves the tag reached the rewriter's door and stopped
     * it.
     *
     * Not folded into [theLoweredArmSkipsTheRewriterAndTheControlProvesIt]
     * because a guard is a different route through this engine: it hands the
     * rewriter `ExpressionContext.Guard`, a cache and a wrapping the general
     * path never uses, so a seam that covered `evaluateExpr` alone would leave
     * this one rewriting and nothing would have noticed.
     */
    @Test
    fun theLoweredGuardArmSkipsTheRewriterAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_guard") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            val lowered = engine.evaluateCondition(id, ScriptSource.lua("a[1] == 10", "a[0] == 10"))
            val rewritten = engine.evaluateCondition(id, ScriptSource.ecmascript("a[1] == 10"))

            assertTrue(
                lowered,
                "a lowered guard must read the author's FIRST element: the frontend already " +
                    "emitted a one-based index, and a guard rewritten again compares the " +
                    "wrong element",
            )
            assertTrue(
                !rewritten,
                "the author's `a[1]` is zero-based, so the rewriter must compare the SECOND " +
                    "element and answer false — this is the control",
            )
            assertNotEquals(
                lowered,
                rewritten,
                "both arms answered the same, so the tag changed nothing on the guard route",
            )
        }
    }

    /**
     * The assign arm, and its control.
     *
     * §scxml-5.4 `<assign expr="…">` reaches the engine through its own entry
     * point — the engine evaluates AND stores — so a lowered artifact whose
     * assignments still went through the rewriter would write the wrong value
     * into the datamodel with nothing raised.
     */
    @Test
    fun theLoweredAssignArmSkipsTheRewriterAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_assign") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            engine.assign(id, "fromLowered", ScriptSource.lua("a[1]", "a[0]"))
            engine.assign(id, "fromAuthor", ScriptSource.ecmascript("a[1]"))

            val lowered = (engine.getVariable(id, "fromLowered") as Number).toLong()
            val rewritten = (engine.getVariable(id, "fromAuthor") as Number).toLong()

            assertEquals(10L, lowered, "a lowered assign must store the author's FIRST element")
            assertEquals(
                20L,
                rewritten,
                "the author's zero-based `a[1]` is the SECOND element — this is the control",
            )
            assertNotEquals(
                lowered,
                rewritten,
                "both arms stored the same value, so the tag changed nothing on the assign route",
            )
        }
    }

    /**
     * The foreach arm, and its control.
     *
     * §scxml-4.6 `<foreach array="…">` is the one entry point whose expression
     * must evaluate to a COLLECTION, and the engine reports a non-collection as
     * `error.execution`. An array expression rewritten a second time therefore
     * does not merely iterate the wrong elements — it can iterate a different
     * object entirely, which is what this case measures: the same characters
     * select the first row of a table of rows when tagged `lua` and the second
     * when tagged `ecmascript`.
     */
    @Test
    fun theLoweredForeachArmSkipsTheRewriterAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_foreach") { id ->
            engine.executeScript(
                id,
                ScriptSource.lua("m = {{1, 2}, {3, 4}}", "var m = [[1, 2], [3, 4]];"),
            )

            fun collect(array: ScriptSource): List<Long> {
                val seen = mutableListOf<Long>()
                engine.executeForeach(id, array, "elem", "") {
                    seen.add((engine.getVariable(id, "elem") as Number).toLong())
                }
                return seen
            }

            val lowered = collect(ScriptSource.lua("m[1]", "m[0]"))
            val rewritten = collect(ScriptSource.ecmascript("m[1]"))

            assertEquals(
                listOf(1L, 2L),
                lowered,
                "a lowered foreach must iterate the author's FIRST row: the frontend already " +
                    "emitted a one-based index",
            )
            assertEquals(
                listOf(3L, 4L),
                rewritten,
                "the author's zero-based `m[1]` is the SECOND row — this is the control",
            )
            assertNotEquals(
                lowered,
                rewritten,
                "both arms iterated the same row, so the tag changed nothing on the foreach route",
            )
        }
    }

    /**
     * The diagnostic names what the AUTHOR wrote, not what the engine ran.
     *
     * The two-string requirement, asserted rather than described. This message
     * travels out on `_event.data` of `error.execution`, so an engine that
     * reported the lowered text would tell a document about a language nobody
     * wrote in it.
     */
    @Test
    fun aReferenceErrorNamesTheAuthorsTextAndNotTheLoweredOne() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_reference_error") { id ->
            val failure = try {
                engine.evaluateExpr(id, ScriptSource.lua("nosuchvar", "nosuchvar_as_written"))
                fail("reading an undeclared name must raise, the way ECMAScript's ReferenceError does")
            } catch (raised: Exception) {
                raised.message.orEmpty()
            }
            assertTrue(
                "nosuchvar_as_written" in failure,
                "the diagnostic must name the author's text; it said: $failure",
            )
        }
    }

    /**
     * An ECMAScript engine refuses Lua — and evaluates ECMAScript in the same
     * session, so the refusal is about the LANGUAGE and not about the engine
     * being broken.
     *
     * Both engines, because "the ECMAScript ones refuse" is a claim about the
     * default [ScxmlScriptEngine.acceptsLanguage], and an engine that had
     * quietly overridden it would be invisible if only its sibling were asked.
     */
    @Test
    fun anEcmaScriptEngineRefusesLoweredLuaAndStillAnswersEcmaScript() {
        for ((name, engine) in listOf<Pair<String, ScxmlScriptEngine>>(
            "Rhino" to RhinoScriptEngine(),
            "QuickJS" to QuickJSScriptEngine(),
        )) {
            withSession(engine, "seam_refusal_${name.lowercase()}") { id ->
                assertEquals(
                    ScriptLanguage.ECMAScript,
                    engine.nativeLanguage(),
                    "$name is offered for `datamodel=\"ecmascript\"`, so it must say so",
                )
                assertTrue(
                    !engine.acceptsLanguage(ScriptLanguage.Lua),
                    "$name owns no Lua adapter, so it must refuse lowered Lua rather than " +
                        "report a syntax error in a language the author never wrote",
                )

                // Every guarded entry point, not the one that is easiest to
                // call. The refusal lives on the entry point rather than in
                // the engine, so an entry point added without it is exactly
                // the defect this case exists to catch — and three of these
                // five were added a day after the other two.
                val lua = ScriptSource.lua("a[1]", "a[0]")
                val entryPoints = listOf<Pair<String, (String) -> Unit>>(
                    "evaluateExpr" to { s -> engine.evaluateExpr(s, lua) },
                    "executeScript" to { s -> engine.executeScript(s, lua) },
                    "evaluateCondition" to { s -> engine.evaluateCondition(s, lua) },
                    "assign" to { s -> engine.assign(s, "v", lua) },
                    "executeForeach" to { s -> engine.executeForeach(s, lua, "elem", "") {} },
                )
                for ((entry, call) in entryPoints) {
                    val refusal = try {
                        call(id)
                        fail("$name accepted lowered Lua through $entry")
                    } catch (raised: Exception) {
                        raised.message.orEmpty()
                    }
                    assertTrue(
                        "lua" in refusal && "ecmascript" in refusal,
                        "$name.$entry must refuse by naming both languages so a host can act " +
                            "on it; it said: $refusal",
                    )
                }

                // The control: the same session answers its own language, so
                // the refusal above was not the engine failing to work at all.
                assertEquals(
                    3L,
                    (engine.evaluateExpr(id, ScriptSource.ecmascript("1 + 2")) as Number).toLong(),
                    "$name must still evaluate ECMAScript in the session that refused Lua",
                )
            }
        }
    }

    /**
     * The Lua engine says what it is.
     *
     * Small, and it is the fact `EcmaScriptSemanticsTest` spends a whole test
     * documenting in prose: the engine this backend offers beside Rhino and
     * QuickJS is not an ECMAScript engine. Now it is also a value a host can
     * read.
     */
    @Test
    fun theLuaEngineDeclaresItsOwnLanguageAndItsAdapter() {
        val engine = LuaScriptEngine()
        assertEquals(ScriptLanguage.Lua, engine.nativeLanguage())
        assertTrue(
            engine.acceptsLanguage(ScriptLanguage.ECMAScript),
            "`EcmaScriptToLuaTransformer` is this engine's adapter, so it accepts the " +
                "author's text as well — that is what the divergence list measures",
        )
        assertTrue(engine.acceptsLanguage(ScriptLanguage.Lua))
    }

    /**
     * The contract is not an engine detail: no engine overrides the entry
     * points that carry the refusal.
     *
     * C++ says this with `non-virtual`. Kotlin cannot say `final` on an
     * interface member, so it is said here — an engine that declared
     * `evaluateExpr(sessionId, ScriptSource)` would bypass
     * [ScxmlScriptEngine.acceptsLanguage] for itself and nothing else would
     * notice. What engines override is `doEvaluateExpr` / `doExecuteScript`.
     *
     * ⚠ **Read from the SOURCE, not by reflection, and that is a correction.**
     * This test first asked `Class.declaredMethods`, and every engine failed
     * it — including two that override nothing. Kotlin emits a forwarding
     * method into each implementing class for an interface member with a body,
     * so `declaredMethods` cannot tell a compiler's forwarder from an author's
     * `override`. The question is about what somebody WROTE, so the answer has
     * to come from what somebody wrote.
     *
     * The population is derived rather than listed: every Kotlin source under
     * `backends/kotlin` that implements [ScxmlScriptEngine]. A hard-coded list
     * would silently exempt the next engine, which is the shape of escape
     * hatch this repository keeps paying for — so the sweep also carries a
     * floor, because a walk that found nothing would pass by measuring
     * nothing.
     */
    @Test
    fun theLanguageContractIsNotAnEngineDetail() {
        val engines = engineSources()

        assertTrue(
            engines.size >= ENGINE_FLOOR,
            "found ${engines.size} implementation(s) of ScxmlScriptEngine under " +
                "backends/kotlin and this backend ships at least $ENGINE_FLOOR (Rhino, " +
                "QuickJS, Lua). A sweep that stopped finding the tree would pass this test " +
                "by examining nothing, which is the failure it is here to prevent.",
        )

        val offenders = engines.flatMap { (file, body) ->
            GUARDED_OVERRIDE.findAll(body).map { "  ${file.path}: ${it.value.trim()}" }
        }
        assertTrue(
            offenders.isEmpty(),
            "${offenders.size} engine declaration(s) override an entry point that carries " +
                "the language refusal. That refusal is the contract every engine owes a " +
                "host — an engine handed a language it cannot read must say so, rather " +
                "than report a syntax error in a language its author never wrote — and an " +
                "engine that implements the entry point can forget it. Override " +
                "`doEvaluateExpr` / `doExecuteScript` instead.\n" +
                offenders.joinToString("\n"),
        )
    }

    /**
     * Every rewrite this engine performs is reached through the branch that
     * asks what language the text is in.
     *
     * This is the predicate that replaces a count in a comment, and it is the
     * one that would have been red for the day `evaluateCondition`, `assign`
     * and `executeForeach` still called the rewriter unconditionally while the
     * interface's own header said the seam existed. A sentence claiming "the
     * seam is landed" is not measurable; "no rewrite happens outside the branch
     * that decides the language" is.
     *
     * It is deliberately not a list of approved call sites. The failure this
     * repository keeps paying for is the SIXTH entry point — the one nobody
     * adds to the list — so what is asserted is a property every member must
     * have, and an unclassified member is red rather than exempt.
     *
     * **The population is ENGINES, derived the same way
     * [theLanguageContractIsNotAnEngineDetail] derives its own** — every Kotlin
     * source under `backends/kotlin` declaring a [ScxmlScriptEngine]
     * implementation — and then the members of each. It named one file by path
     * until 2026-08-30, which would have missed a second engine that adopted
     * the rewriter. ⚠ The obvious widening, "every file that calls the
     * rewriter", is wrong and was measured to be: `LoweredEcma262Test` calls it
     * on purpose, because comparing the rewritten arm against the lowered one
     * IS its measurement. Deriving the population as *engines* keeps that file
     * out by what it is, rather than by an exemption entry — which is the
     * escape hatch this gate exists to refuse.
     *
     * Within each engine the split is at the class-level `fun` declarations,
     * and the sweep carries two floors: the engine count, and the rewriter
     * calls the seam branch itself holds. A walk that stopped finding either
     * would pass this test by examining nothing.
     *
     * ⚠ What it deliberately does NOT catch, measured 2026-08-30: a branch that
     * is still written and no longer decides anything. Turning the condition
     * into `expr.language == ScriptLanguage.Lua && false` leaves this test green
     * and turns the four behavioural arms above red — expression, guard, assign
     * and foreach, each of which answers a different value under the break. The
     * two kinds of witness are complements, not duplicates: this one names the
     * member that has no arm at all, and those name the arm that stopped
     * working.
     */
    @Test
    fun everyRewriteIsReachedThroughTheSeamBranch() {
        val engines = engineSources()
        assertTrue(
            engines.size >= ENGINE_FLOOR,
            "found ${engines.size} implementation(s) of ScxmlScriptEngine under " +
                "backends/kotlin (from ${File(".").absolutePath}) and this backend ships " +
                "at least $ENGINE_FLOOR. A sweep that stopped finding the tree would pass " +
                "this test by examining nothing.",
        )

        val rewrites = engines.sumOf { (_, body) -> REWRITE_CALL.findAll(body).count() }
        assertTrue(
            rewrites >= REWRITE_FLOOR,
            "found $rewrites rewriter call(s) across ${engines.size} engine(s) and the " +
                "seam branch alone holds $REWRITE_FLOOR (`loweredTextOf`, " +
                "`loweredScriptOf`). Fewer means the rewriter moved out of the engines and " +
                "this sweep is no longer looking at it.",
        )

        val unguarded = engines.flatMap { (file, body) ->
            val members = mutableListOf<Pair<String, StringBuilder>>()
            for (line in body.lineSequence()) {
                val declaration = MEMBER_DECLARATION.find(line)
                if (declaration != null) members.add(declaration.groupValues[1] to StringBuilder())
                members.lastOrNull()?.second?.append(line)?.append('\n')
            }
            members
                .filter { (_, text) -> REWRITE_CALL.containsMatchIn(text) }
                .filterNot { (_, text) -> LANGUAGE_BRANCH.containsMatchIn(text) }
                .map { (name, _) -> "${file.path}: $name" }
        }

        assertTrue(
            unguarded.isEmpty(),
            "${unguarded.size} engine member(s) call the ECMAScript-to-Lua rewriter " +
                "without asking what language the text is in: " +
                "${unguarded.joinToString(", ")}. An entry point that rewrites " +
                "unconditionally cannot be handed Lua the build-time frontend already " +
                "produced — it rewrites it a second time, which shifts a correct index off " +
                "by one with no diagnostic. Take a `ScriptSource` and go through " +
                "`loweredTextOf` / `loweredScriptOf`.",
        )
    }

    /**
     * Every Kotlin source under `backends/kotlin` that declares itself a
     * [ScxmlScriptEngine], with its text.
     *
     * Shared by the two source-reading cases so the population they judge
     * cannot become two different answers to "which engines does this backend
     * ship" — the same reason the seam has one branch and not two.
     */
    private fun engineSources(): List<Pair<File, String>> =
        File("backends/kotlin").walkTopDown()
            .filter { it.isFile && it.extension == "kt" }
            .filterNot { "/build/" in it.path }
            .map { it to it.readText() }
            .filter { (_, body) -> IMPLEMENTS_ENGINE.containsMatchIn(body) }
            .toList()

    private companion object {
        /** A class that declares itself an engine, however it spells the supertype list. */
        val IMPLEMENTS_ENGINE = Regex("""class\s+\w+[^{]*:\s*[^{]*\bScxmlScriptEngine\b""")

        /** A call into `EcmaScriptToLuaTransformer`, whichever of its two entry points. */
        val REWRITE_CALL = Regex("""\btransformer\.transform(Script)?\s*\(""")

        /** The branch that decides whether the text needs rewriting at all. */
        val LANGUAGE_BRANCH = Regex("""==\s*ScriptLanguage\.Lua""")

        /** A member declaration at class indentation, and its name. */
        val MEMBER_DECLARATION = Regex("""^\s{4}(?:\w+\s+)*fun\s+(\w+)""")

        /** `loweredTextOf` and `loweredScriptOf` — the branch is two calls wide. */
        const val REWRITE_FLOOR = 2

        /**
         * An `override` of a guarded entry point — the two-argument
         * `ScriptSource` form specifically. The `String` forms are the ones
         * engines are supposed to implement, so the parameter type is what
         * separates a legitimate override from a bypassed contract.
         */
        val GUARDED_OVERRIDE = Regex(
            """override\s+fun\s+""" +
                """(evaluateExpr|executeScript|evaluateCondition|assign|executeForeach)""" +
                """\s*\([^)]*ScriptSource[^)]*\)"""
        )

        /** Rhino, QuickJS, Lua — plus the Android app's own Rhino. */
        const val ENGINE_FLOOR = 3
    }
}
