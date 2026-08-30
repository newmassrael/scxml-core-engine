// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2026 newmassrael
//
// The language seam on `ScxmlScriptEngine`: does handing an engine Lua the
// frontend already produced actually SKIP the run-time lowering, and does an
// engine that cannot read Lua refuse it?
//
// The Kotlin sibling of `tests/engine/ScriptLanguageSeamTest.cpp` (ctest
// `ScriptLanguageSeam`), and it is built the same way for the same reason:
// every case carries its own CONTROL, so none of them can pass by measuring
// nothing. A test that only asserted "lowered `a[1]` answers 10" would pass on
// an engine that ignored the tag entirely and lowered everything — the control
// is the same characters tagged ECMAScript, which must answer something
// DIFFERENT.
//
// ⚠ The controls survived `EcmaScriptToLuaTransformer`'s retirement because
// none of them depended on the rewriter EXISTING — only on the two arms
// answering differently, which a zero-based author index and a one-based
// lowered one do whichever half performs the lowering. The one that narrowed
// says so in its own KDoc.

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
     * when tagged `lua`. An engine that ignored the tag and lowered everything
     * would answer the same value twice, and that is what
     * [assertNotEquals] refuses.
     *
     * This is the shape a lowered artifact actually hits: `sce-build`'s
     * frontend emits one-based Lua indices, and an engine that lowered them
     * again would shift a correct expression off by one — silently, because
     * both readings are valid Lua.
     *
     * ⚠ The control arm depends on the session's scope knowing `a`, and the
     * setup declares it in LUA. What tells the frontend about a name a Lua
     * chunk introduced is `offerDocumentGlobalsToScope`, reading Lua's own
     * global table — the ECMAScript parser cannot see through that door. Before
     * that reader existed the control was answered by the text rewriter
     * instead, which is the same 20 by a route that no longer exists.
     */
    @Test
    fun theLoweredArmSkipsTheRunTimeLoweringAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_index") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            val lowered = engine.evaluateExpr(id, ScriptSource.lua("a[1]", "a[0]"))
            val authored = engine.evaluateExpr(id, ScriptSource.ecmascript("a[1]"))

            assertEquals(
                10L,
                (lowered as Number).toLong(),
                "lowered `a[1]` must read the author's FIRST element: the frontend already " +
                    "emitted a one-based index, and an engine that lowered it again would " +
                    "shift a correct expression off by one",
            )
            assertEquals(
                20L,
                (authored as Number).toLong(),
                "the author's `a[1]` is zero-based, so lowering it must produce the SECOND " +
                    "element — this is the control",
            )
            assertNotEquals(
                lowered.toLong(),
                authored.toLong(),
                "the two arms answered the same value, so the tag changed nothing and this " +
                    "suite is measuring one path twice",
            )
        }
    }

    /**
     * The script arm, and its control, on the same defect shape.
     *
     * `executeScript` is the other half of what `LoweredEcma262Test` needs: a
     * case's setup is a script, and a setup that got lowered a second time
     * would leave the session in a state no lowered artifact would produce.
     *
     * ⚠ **What the control observes narrowed when the rewriter was retired,
     * and it got stronger rather than weaker.** The text below is a Lua table
     * constructor with a bracketed key — not ECMAScript. Tagged `ecmascript`
     * it used to be handed to `EcmaScriptToLuaTransformer`, which mangled it
     * into something that ran and left no readable `o.k`; it is now REFUSED,
     * which the same assertion reads as the same "not 7". A wrong answer and a
     * refusal are both "the tag was not ignored", and only the refusal is what
     * §scxml-5.9.1 asks for.
     */
    @Test
    fun theLoweredScriptArmSkipsTheRunTimeLoweringToo() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_script") { id ->
            engine.executeScript(id, ScriptSource.lua("""o = {["k"] = 7}""", """var o = {k: 7};"""))
            val lowered = engine.evaluateExpr(id, ScriptSource.lua("o.k", "o.k"))
            assertEquals(
                7L,
                (lowered as Number).toLong(),
                "a lowered script must reach the interpreter as written: an engine that " +
                    "offered this table constructor to an ECMAScript parser would be refused " +
                    "by it",
            )
        }

        // The control, in its own session so the first cannot have set it up:
        // the same Lua handed over as if the author had written it is offered
        // to a parser that reads ECMAScript, which refuses it — so there is no
        // readable `o.k` afterwards.
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
                "the same characters tagged `ecmascript` came back readable, so the tag " +
                    "changed nothing and this case's control proves nothing about the seam",
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
     * control is what proves the tag reached the lowering's door and stopped
     * it.
     *
     * Not folded into
     * [theLoweredArmSkipsTheRunTimeLoweringAndTheControlProvesIt] because a
     * guard is a different route through this engine: §scxml-5.9 truthiness is
     * not Lua's, so it reaches a SEPARATE frontend entry point
     * (`lowerCondition`, the `to_lua_guard` wrapping) that the general path
     * never uses — and a seam that covered `evaluateExpr` alone would leave
     * this one lowering unconditionally with nothing to notice.
     */
    @Test
    fun theLoweredGuardArmSkipsTheRunTimeLoweringAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_guard") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            val lowered = engine.evaluateCondition(id, ScriptSource.lua("a[1] == 10", "a[0] == 10"))
            val authored = engine.evaluateCondition(id, ScriptSource.ecmascript("a[1] == 10"))

            assertTrue(
                lowered,
                "a lowered guard must read the author's FIRST element: the frontend already " +
                    "emitted a one-based index, and a guard lowered again compares the " +
                    "wrong element",
            )
            assertTrue(
                !authored,
                "the author's `a[1]` is zero-based, so lowering it must compare the SECOND " +
                    "element and answer false — this is the control",
            )
            assertNotEquals(
                lowered,
                authored,
                "both arms answered the same, so the tag changed nothing on the guard route",
            )
        }
    }

    /**
     * The assign arm, and its control.
     *
     * §scxml-5.4 `<assign expr="…">` reaches the engine through its own entry
     * point — the engine evaluates AND stores — so a lowered artifact whose
     * assignments were lowered a second time would write the wrong value into
     * the datamodel with nothing raised.
     */
    @Test
    fun theLoweredAssignArmSkipsTheRunTimeLoweringAndTheControlProvesIt() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_lowered_assign") { id ->
            engine.executeScript(id, ScriptSource.lua("a = {10, 20}", "var a = [10, 20];"))

            engine.assign(
                id,
                ScriptSource.lua("fromLowered", "fromLowered"),
                ScriptSource.lua("a[1]", "a[0]"),
            )
            engine.assign(
                id,
                ScriptSource.ecmascript("fromAuthor"),
                ScriptSource.ecmascript("a[1]"),
            )

            val lowered = (engine.getVariable(id, "fromLowered") as Number).toLong()
            val authored = (engine.getVariable(id, "fromAuthor") as Number).toLong()

            assertEquals(10L, lowered, "a lowered assign must store the author's FIRST element")
            assertEquals(
                20L,
                authored,
                "the author's zero-based `a[1]` is the SECOND element — this is the control",
            )
            assertNotEquals(
                lowered,
                authored,
                "both arms stored the same value, so the tag changed nothing on the assign route",
            )
        }
    }

    /**
     * The foreach arm, and its control.
     *
     * §scxml-4.6 `<foreach array="…">` is the one entry point whose expression
     * must evaluate to a COLLECTION, and the engine reports a non-collection as
     * `error.execution`. An array expression lowered a second time therefore
     * does not merely iterate the wrong elements — it can iterate a different
     * object entirely, which is what this case measures: the same characters
     * select the first row of a table of rows when tagged `lua` and the second
     * when tagged `ecmascript`.
     */
    @Test
    fun theLoweredForeachArmSkipsTheRunTimeLoweringAndTheControlProvesIt() {
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
            val authored = collect(ScriptSource.ecmascript("m[1]"))

            assertEquals(
                listOf(1L, 2L),
                lowered,
                "a lowered foreach must iterate the author's FIRST row: the frontend already " +
                    "emitted a one-based index",
            )
            assertEquals(
                listOf(3L, 4L),
                authored,
                "the author's zero-based `m[1]` is the SECOND row — this is the control",
            )
            assertNotEquals(
                lowered,
                authored,
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
                    "assign" to { s -> engine.assign(s, ScriptSource.ecmascript("v"), lua) },
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
            "`sce-build`'s ECMAScript frontend is this engine's adapter — linked into " +
                "`sce_lua_jni` and reached through `LoweringScope` — so it accepts the " +
                "author's text as well, and that is what the divergence list measures",
        )
        assertTrue(engine.acceptsLanguage(ScriptLanguage.Lua))
    }

    /**
     * `sce-build`'s ECMAScript frontend really is linked into `sce_lua_jni`.
     *
     * ⚠ **This test is named in two comments that predate it by four rounds,
     * and until 2026-08-30 it did not exist.**
     * `backends/kotlin/lua/src/main/cpp/CMakeLists.txt` says the link is "not
     * optional" and that *"`theFrontendIsLinked` refuses a build that reached
     * here without it"*; nothing in the tree carried that name. A check named
     * in prose is a promise that something re-derives the sentence beside it,
     * and this one was keeping a sentence nobody could fault — the same defect
     * `lowering_decision_ledger`'s own sweep exists to catch one document over.
     *
     * ⚠⚠ **The retirement is what makes the predicate exact**, which is why it
     * lands with it rather than earlier. While `EcmaScriptToLuaTransformer`
     * stood behind the frontend, an ECMAScript expression could be answered by
     * either half, so "this engine answered ECMAScript" said nothing about the
     * link. With the fallback gone the frontend is the ONLY thing that can
     * answer one: a library built without it hands every expression to
     * `refusedToLower`. So an answer here is the link, and a refusal is its
     * absence.
     *
     * `1 + 1` deliberately names nothing. A scope with nothing declared admits
     * exactly the closed expressions, so this asks the frontend the one
     * question that cannot fail for a reason of its own.
     */
    @Test
    fun theFrontendIsLinked() {
        val engine = LuaScriptEngine()
        withSession(engine, "seam_frontend_linked") { id ->
            val answered = engine.evaluateExpr(id, ScriptSource.ecmascript("1 + 1"))
            assertEquals(
                2L,
                (answered as Number).toLong(),
                "the Lua engine did not answer a closed ECMAScript expression. Since the " +
                    "rewriter was retired the frontend is the only thing that can, so this " +
                    "says `SCE::Lowering` is not in `sce_lua_jni` — a build that answers " +
                    "`datamodel=\"ecmascript\"` by refusing every expression in it",
            )
        }
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
     * Every lowering this engine performs is reached through the branch that
     * asks what language the text is in.
     *
     * This is the predicate that replaces a count in a comment, and it is the
     * one that would have been red for the day `evaluateCondition`, `assign`
     * and `executeForeach` still lowered unconditionally while the interface's
     * own header said the seam existed. A sentence claiming "the seam is
     * landed" is not measurable; "no lowering happens outside the branch that
     * decides the language" is.
     *
     * ⚠ **Its subject moved when `EcmaScriptToLuaTransformer` was retired, and
     * the property did not.** It used to count calls into that rewriter; it
     * counts calls into the frontend now, because the frontend is what stands
     * behind the same branch. The defect is the same shape and the diagnostic
     * is what changed: an entry point that lowers unconditionally used to
     * rewrite already-lowered Lua and shift a correct index off by one, and now
     * offers it to a parser that refuses to read it as ECMAScript. Both are
     * silent about the cause at the call site, which is why the branch — not
     * the callee — is what this holds.
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
     * the seam. Deriving the population as *engines* also keeps the suites
     * out by WHAT THEY ARE rather than by an exemption entry, which is the
     * escape hatch this gate exists to refuse.
     *
     * Within each engine the split is at the class-level `fun` declarations,
     * and the sweep carries two floors: the engine count, and the lowering
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
    fun everyLoweringIsReachedThroughTheSeamBranch() {
        val engines = engineSources()
        assertTrue(
            engines.size >= ENGINE_FLOOR,
            "found ${engines.size} implementation(s) of ScxmlScriptEngine under " +
                "backends/kotlin (from ${File(".").absolutePath}) and this backend ships " +
                "at least $ENGINE_FLOOR. A sweep that stopped finding the tree would pass " +
                "this test by examining nothing.",
        )

        val lowerings = engines.sumOf { (_, body) -> LOWER_CALL.findAll(body).count() }
        assertTrue(
            lowerings >= LOWER_FLOOR,
            "found $lowerings frontend lowering call(s) across ${engines.size} engine(s) " +
                "and the seam branch alone holds $LOWER_FLOOR (`loweredTextOf`, " +
                "`loweredConditionOf`, `loweredLocationOf`, `loweredScriptOf`). Fewer means " +
                "the lowering moved out of the engines and this sweep is no longer looking " +
                "at it.",
        )

        val unguarded = engines.flatMap { (file, body) ->
            val members = mutableListOf<Pair<String, StringBuilder>>()
            for (line in body.lineSequence()) {
                val declaration = MEMBER_DECLARATION.find(line)
                if (declaration != null) members.add(declaration.groupValues[1] to StringBuilder())
                members.lastOrNull()?.second?.append(line)?.append('\n')
            }
            members
                .filter { (_, text) -> LOWER_CALL.containsMatchIn(text) }
                .filterNot { (_, text) -> LANGUAGE_BRANCH.containsMatchIn(text) }
                .map { (name, _) -> "${file.path}: $name" }
        }

        assertTrue(
            unguarded.isEmpty(),
            "${unguarded.size} engine member(s) lower text through the ECMAScript frontend " +
                "without asking what language the text is in: " +
                "${unguarded.joinToString(", ")}. An entry point that lowers " +
                "unconditionally cannot be handed Lua the build-time frontend already " +
                "produced — it offers it to a parser that reads ECMAScript, which refuses " +
                "text that is already correct. Take a `ScriptSource` and go through " +
                "`loweredTextOf` / `loweredConditionOf` / `loweredLocationOf` / " +
                "`loweredScriptOf`.",
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

        /** A call into `sce-build`'s ECMAScript frontend, whichever entry point. */
        val LOWER_CALL = Regex("""\bloweringScope\.lower\w+\s*\(""")

        /** The branch that decides whether the text needs lowering at all. */
        val LANGUAGE_BRANCH = Regex("""==\s*ScriptLanguage\.Lua""")

        /** A member declaration at class indentation, and its name. */
        val MEMBER_DECLARATION = Regex("""^\s{4}(?:\w+\s+)*fun\s+(\w+)""")

        /**
         * `loweredTextOf`, `loweredConditionOf`, `loweredLocationOf` and
         * `loweredScriptOf` — the branch is four calls wide.
         *
         * A FLOOR and not an equality, so a fifth arm does not fail this on
         * the day it lands; what it defends is the sweep still finding the
         * lowering at all. It moved 2 → 3 on 2026-08-29 when an `<assign>`
         * LOCATION started carrying its language too, which it has to: the
         * Lua arm splices the location in front of `=` and runs the result.
         * It moved 3 → 4 when the rewriter was retired, because until then
         * the `cond` arm reached its fallback rather than a separate frontend
         * entry point and the sweep was counting the fallback.
         */
        const val LOWER_FLOOR = 4

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
