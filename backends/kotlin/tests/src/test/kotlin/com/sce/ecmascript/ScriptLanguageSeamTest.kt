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

                val refusal = try {
                    engine.evaluateExpr(id, ScriptSource.lua("a[1]", "a[0]"))
                    fail("$name accepted lowered Lua")
                } catch (raised: Exception) {
                    raised.message.orEmpty()
                }
                assertTrue(
                    "lua" in refusal && "ecmascript" in refusal,
                    "$name's refusal must name both languages so a host can act on it; " +
                        "it said: $refusal",
                )

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
        val engines = File("backends/kotlin").walkTopDown()
            .filter { it.isFile && it.extension == "kt" }
            .filterNot { "/build/" in it.path }
            .map { it to it.readText() }
            .filter { (_, body) -> IMPLEMENTS_ENGINE.containsMatchIn(body) }
            .toList()

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

    private companion object {
        /** A class that declares itself an engine, however it spells the supertype list. */
        val IMPLEMENTS_ENGINE = Regex("""class\s+\w+[^{]*:\s*[^{]*\bScxmlScriptEngine\b""")

        /**
         * An `override` of a guarded entry point — the two-argument
         * `ScriptSource` form specifically. The `String` forms are the ones
         * engines are supposed to implement, so the parameter type is what
         * separates a legitimate override from a bypassed contract.
         */
        val GUARDED_OVERRIDE =
            Regex("""override\s+fun\s+(evaluateExpr|executeScript)\s*\([^)]*ScriptSource[^)]*\)""")

        /** Rhino, QuickJS, Lua — plus the Android app's own Rhino. */
        const val ENGINE_FLOOR = 3
    }
}
