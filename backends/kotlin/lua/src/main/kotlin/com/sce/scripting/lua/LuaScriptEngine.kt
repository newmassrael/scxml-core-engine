// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Lua — Lua 5.4 implementation of ScxmlScriptEngine
//
// Drop-in replacement for RhinoScriptEngine using Lua 5.4 via JNI.
// ECMAScript expressions are lowered to Lua by sce-build's ECMAScript frontend,
// with EcmaScriptToLuaTransformer as the fallback for what it refuses.
// Each session gets an isolated lua_State for full variable isolation.
//
// C++ parity: sce/src/scripting/LuaEngine.cpp

package com.sce.scripting.lua

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.SceXmlDom
import com.sce.runtime.PayloadReading
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScriptLanguage
import com.sce.runtime.ScriptSource
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.w3c.dom.Node

/**
 * Lua 5.4, running SCXML expressions that `sce-build`'s ECMAScript frontend
 * PARSED and lowered.
 *
 * **What answers ECMAScript here is a parser, not a pass over text.** The
 * author's expression goes to the frontend — the same one every backend's
 * build-time lowering uses, reached at run time through `sce_lua_jni` — and
 * what comes back is Lua the parse produced. That is the whole reason the
 * disagreements this header used to enumerate are gone: `0 && x`, `1 == '1'`,
 * `-7 % 3` and a computed array index were all cases a rewriting pass could
 * not reach, because no pass that replaces text can say where an operand ends.
 *
 * **Which cases it still answers differently** is not written here, because a
 * count in prose is the one thing nothing can re-answer. It is enumerated,
 * clause by clause, in `tests/ecmascript/kotlin_lua_divergences.json`, and
 * `EcmaScriptSemanticsTest.theLuaEngineDivergesExactlyWhereItIsDeclaredTo`
 * holds this engine to that list in BOTH directions: a case that starts
 * disagreeing without being declared is red, and a declared case that has been
 * repaired is red too. The C++ selection has its own list beside it
 * (`lua_engine_divergences.json`) — two measurements of two engines that now
 * share a frontend, neither derivable from the other.
 *
 * That list is what this paragraph used to lack. It carried two counts —
 * "27 of its 58" here and "26 of 58" for C++ — under a sentence claiming the
 * test held them to the measurement. It held neither; it asserted only that
 * the failure set was not empty, which one disagreement satisfies as well as
 * fifty. The shared table had meanwhile grown to 98 cases, so both
 * denominators named a table that no longer existed.
 *
 * ⚠ **[EcmaScriptToLuaTransformer] is still the FALLBACK.** An expression the
 * frontend refuses — one naming something the session's [LoweringScope] has
 * not been told about, or text its parser will not read — is handed to the old
 * rewriter and guessed at rather than refused. An empty divergence list says
 * the frontend answers the shared table; it does not say there is no second
 * answer left. That second claim is `retire-rewriter`'s, which
 * `docs/SCE_LUA_TRANSLATION_SEAM.md` records for C++ and has open for this
 * backend.
 *
 * This header used to read "For AOSP/AAOS production, this replaces Rhino
 * with a faster native engine". It is faster, and while a rewriting pass was
 * the only answer that sentence sent a reader building an AAOS product to an
 * engine that answered their guards wrong. Rhino and QuickJS remain the
 * engines that answer ECMA-262 by BEING ECMAScript implementations; this one
 * answers it by lowering, and what it can lower is decided by the scope.
 *
 * Each session gets its own lua_State (variable isolation) and its own
 * [LoweringScope] — the names the frontend may resolve, fed by `<data id>`,
 * by `<assign>`, by `<foreach>` and by what a `<script>` chunk's top level
 * introduces.
 */
class LuaScriptEngine : ScxmlScriptEngine {

    /**
     * Opaque handle for Lua values (functions, metatabled tables) that cannot survive
     * Kotlin round-trip. Stored in Lua registry via luaL_ref, retrieved via lua_rawgeti.
     * Must be consumed via [setVariable] or explicitly released via [unrefIfNeeded].
     *
     * `originHandle` records the lua_State the ref was created in so that
     * cross-session reuse can be rejected; the registry is per-state, so reading
     * `rawgeti(otherState, registryIndex, key)` would silently return whatever
     * happens to live at that key in the other state (or nil).
     */
    private data class LuaRef(val registryKey: Int, val originHandle: Long)

    private data class Session(
        val handle: Long,  // lua_State pointer
        var stateQueryCallback: ((String) -> Boolean)? = null,
        val declaredVars: MutableSet<String> = mutableSetOf(),
        val activeRefs: MutableSet<Int> = mutableSetOf(),
        /**
         * The names this session holds, as `sce-build`'s ECMAScript frontend
         * sees them (W3C SCXML §scxml-5.3, §scxml-5.8).
         *
         * Per session rather than per engine, because it is the set of names
         * one session's datamodel holds: two sessions of the same document may
         * legitimately differ, and one session's names must never answer for
         * another's.
         */
        val loweringScope: LoweringScope = LoweringScope(),
        /**
         * The names already offered to [loweringScope].
         *
         * The frontend accepts a re-declaration, so this is not for
         * correctness; it keeps a `<data id>` that is assigned on every
         * macrostep from crossing the JNI boundary once per assignment.
         */
        val offeredToScope: MutableSet<String> = mutableSetOf()
    )

    private val sessions = mutableMapOf<String, Session>()
    private val transformer = EcmaScriptToLuaTransformer()

    override fun createSession(sessionId: String) {
        val handle = LuaNative.newState()
        if (handle == 0L) throw ScriptEngineException("Failed to create Lua state")
        sessions[sessionId] = Session(handle)
        registerBuiltins(handle)
    }

    override fun destroySession(sessionId: String) {
        val session = sessions.remove(sessionId) ?: return
        val regIdx = LuaNative.registryIndex()
        for (ref in session.activeRefs) {
            LuaNative.unref(session.handle, regIdx, ref)
        }
        LuaNative.closeState(session.handle)
        // The scope is native memory with the session's lifetime, like the Lua
        // state beside it. `close()` is idempotent, so a teardown that runs
        // twice is a second no-op rather than a double free.
        session.loweringScope.close()
    }

    override fun setupSystemVariables(
        sessionId: String,
        machineName: String,
        ioProcessors: List<IoProcessorDescriptor>,
    ) {
        val session = sessions[sessionId] ?: return
        val L = session.handle

        // W3C SCXML 5.10: _sessionid
        LuaNative.pushString(L, sessionId)
        LuaNative.setGlobal(L, "_sessionid")

        // W3C SCXML 5.10: _name
        LuaNative.pushString(L, machineName)
        LuaNative.setGlobal(L, "_name")

        // §scxml-C-1-1 / §scxml-C-2-3: one entry per processor the deployment
        // supports, each with a 'location' field holding the address that
        // reaches this session through it. Names and locations are decided by
        // IoProcessors.build, so this engine's view of `_ioprocessors` matches
        // every other backend's.
        LuaNative.createTable(L, 0, ioProcessors.size)
        for (processor in ioProcessors) {
            LuaNative.createTable(L, 0, 1)
            LuaNative.pushString(L, processor.location)
            LuaNative.setField(L, -2, "location")
            LuaNative.setField(L, -2, processor.name)
        }
        LuaNative.setGlobal(L, "_ioprocessors")

        session.declaredVars.addAll(listOf("_sessionid", "_name", "_ioprocessors", "_event"))
        // §scxml-5.10: the system variables are part of the datamodel a guard
        // may name, so the frontend has to be able to resolve them too. Without
        // this every `cond="_event.data.x"` in the corpus would be refused and
        // fall back to the rewriter, which is most of what a W3C document asks.
        for (name in session.declaredVars) {
            offerToScope(session, name)
        }
    }

    override fun evaluateCondition(sessionId: String, expr: String): Boolean =
        doEvaluateCondition(sessionId, ScriptSource.ecmascript(expr))

    override fun doEvaluateCondition(sessionId: String, expr: ScriptSource): Boolean {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaExpr = loweredConditionOf(expr, session)
        return evaluateLuaBoolean(session, luaExpr, expr.source)
    }

    /**
     * This engine's own language, stated rather than documented.
     *
     * Lua is what it evaluates. What lets it accept the author's ECMAScript as
     * well is `sce-build`'s frontend, linked beside `lua54` and reached through
     * [LoweringScope], with [EcmaScriptToLuaTransformer] behind it for what the
     * frontend refuses — which is why [acceptsLanguage] is true for both, and
     * why a case this engine answers differently from ECMA-262 is a fact about
     * that pair, enumerated in `tests/ecmascript/kotlin_lua_divergences.json`,
     * not a fact about Lua.
     */
    override fun nativeLanguage(): ScriptLanguage = ScriptLanguage.Lua

    override fun acceptsLanguage(language: ScriptLanguage): Boolean = true

    /**
     * The seam, and it is ONE BRANCH — everything after it is the tail both
     * routes run.
     *
     * Lua the frontend already emitted passes through untouched. The author's
     * ECMAScript is offered to `sce-build`'s ECMAScript frontend, which PARSES
     * it, and only text the frontend refuses reaches
     * [EcmaScriptToLuaTransformer], which replaces text without a parse and so
     * cannot say where an operand ends.
     *
     * What the frontend can answer is decided by the SESSION'S SCOPE: it
     * refuses any expression naming something the scope has not been told
     * about, so an empty scope admits exactly the CLOSED expressions and a
     * session that has declared its `<data id>`s admits the ones that name
     * them. `LoweringScope` carries why.
     *
     * The fallback is deliberate and temporary. The C++ engine passed through
     * this same state — frontend first, rewriter behind it — before its
     * `retire-rewriter` row closed and refusal became the final answer
     * (§scxml-5.9.1). Keeping it here means the frontend is adopted one class
     * of expression at a time, and an expression it refuses answers exactly
     * what it answered before rather than newly failing.
     *
     * The `ReferenceError` below is built from [ScriptSource.source] while the
     * check runs on the lowered text, which is the two-string requirement
     * `ScriptSource` exists for: a document that wrote `nosuchvar[0]` must not
     * be told about `nosuchvar[1]`.
     */
    private fun loweredTextOf(
        expr: ScriptSource,
        session: Session,
        context: EcmaScriptToLuaTransformer.ExpressionContext =
            EcmaScriptToLuaTransformer.ExpressionContext.General,
    ): String =
        if (expr.language == ScriptLanguage.Lua) {
            expr.text
        } else {
            session.loweringScope.lowerValue(expr.text)
                ?: transformer.transform(expr.text, context)
        }

    /**
     * As [loweredTextOf], for a `cond` rather than a value.
     *
     * A separate frontend entry point rather than [loweredTextOf] with a flag:
     * §scxml-5.9 truthiness is not Lua's — `0`, `''` and `NaN` are false in
     * ECMAScript and true in Lua — so a guard has to arrive already wrapped in
     * the shared truthiness helper, which is what `to_lua_guard` does and
     * `to_lua_expr` does not. The rewriter's own answer to the same question is
     * its `Guard` context, which is why that is what the fallback passes.
     */
    private fun loweredConditionOf(expr: ScriptSource, session: Session): String =
        if (expr.language == ScriptLanguage.Lua) {
            expr.text
        } else {
            session.loweringScope.lowerCondition(expr.text)
                ?: transformer.transform(expr.text, EcmaScriptToLuaTransformer.ExpressionContext.Guard)
        }

    /**
     * As [loweredTextOf], for an assignment TARGET rather than a value.
     *
     * A location is not a smaller expression: `sce-build` lowers one with
     * `to_lua_location` rather than `to_lua_expr`, because a write is how this
     * datamodel's globals come into existence and the target is therefore not
     * resolved against what the document declares — which is why the frontend
     * entry point for it takes no scope. The runtime rewriter has no separate
     * location arm, so its fallback goes through the general one.
     */
    private fun loweredLocationOf(location: ScriptSource, session: Session): String =
        if (location.language == ScriptLanguage.Lua) {
            location.text
        } else {
            session.loweringScope.lowerLocation(location.text)
                ?: transformer.transform(location.text)
        }

    /**
     * As [loweredTextOf], for a whole script rather than one expression.
     *
     * A chunk asks LESS of the scope than an expression does — `var` bindings
     * are hoisted into the chunk's own frame before anything resolves — so a
     * self-contained body is answered even by an empty scope. What it still
     * asks the scope for is the names it only READS: a `<data id>` the document
     * declared, or a variable an earlier `<script>` introduced.
     */
    private fun loweredScriptOf(script: ScriptSource, session: Session): String =
        if (script.language == ScriptLanguage.Lua) {
            script.text
        } else {
            session.loweringScope.lowerScript(script.text)
                ?: transformer.transformScript(script.text)
        }

    override fun evaluateExpr(sessionId: String, expr: String): Any? =
        doEvaluateExpr(sessionId, ScriptSource.ecmascript(expr))

    override fun doEvaluateExpr(sessionId: String, expr: ScriptSource): Any? {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaExpr = loweredTextOf(expr, session)

        // Check undeclared simple variable (W3C SCXML compliance: JS throws ReferenceError)
        if (isUndeclaredSimpleVariable(luaExpr, session)) {
            throw ScriptEngineException("ReferenceError: ${expr.source} is not defined")
        }

        val L = session.handle

        // Try "return expr" first (to get value)
        val wrapped = "return $luaExpr"
        val status = LuaNative.loadAndCall(L, wrapped, 1)
        if (status == 0) {
            return wrapLuaResult(L, session)
        }
        LuaNative.pop(L, 1)  // pop error

        // Fallback: try as assignment statement
        if ('=' in luaExpr && "==" !in luaExpr && "~=" !in luaExpr &&
            "<=" !in luaExpr && ">=" !in luaExpr) {
            val assignStatus = LuaNative.loadAndCall(L, luaExpr, 0)
            if (assignStatus == 0) return null
            val err = LuaNative.getError(L)
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Expression evaluation failed: '${expr.source}' (Lua: $err)")
        }

        throw ScriptEngineException("Expression evaluation failed: '${expr.source}'")
    }

    override fun executeScript(sessionId: String, script: String) =
        doExecuteScript(sessionId, ScriptSource.ecmascript(script))

    override fun doExecuteScript(sessionId: String, script: ScriptSource) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaScript = loweredScriptOf(script, session)
        val err = LuaNative.doString(session.handle, luaScript)
        if (err != null) {
            throw ScriptEngineException("Script execution failed: $err")
        }

        // W3C SCXML §scxml-5.8: a `<script>` that ran has introduced its
        // top-level declarations into the datamodel, so the frontend is told
        // about them — the `declare_chunk` half of the scope, and the half that
        // reaches the variables no `<data id>` names.
        //
        // Only after a successful run: a chunk that raised declared whatever it
        // reached and this cannot say where it stopped.
        //
        // Only for ECMAScript, because this asks the frontend's own PARSER to
        // read the chunk's top level and Lua text has no ECMAScript parse to
        // ask. A name a Lua-language `<script>` introduced is therefore one the
        // frontend will not resolve, and an ECMAScript expression naming it
        // falls back to the rewriter — the answer it had before this seam
        // existed. Codegen does not produce that mixture (an artifact is
        // generated for one language and hands over that language everywhere),
        // so it is a residue of the engine accepting both rather than a case
        // any document reaches; C++ closes it by sweeping Lua's global table,
        // which needs a name for every global this engine installs and is its
        // own piece of work.
        if (script.language == ScriptLanguage.ECMAScript) {
            session.loweringScope.declareChunk(script.text)
        }
    }

    override fun setVariable(sessionId: String, name: String, value: Any?) {
        val session = sessions[sessionId] ?: return
        val L = session.handle
        pushKotlinValue(L, value)
        LuaNative.setGlobal(L, name)
        session.declaredVars.add(name)
        offerToScope(session, name)
        unrefIfNeeded(session, value)
    }

    /**
     * Tell the frontend about one name this session now holds.
     *
     * Every door that puts a name in a session's namespace comes through here,
     * because the frontend refuses an expression naming anything it has not
     * been told about: a name that reaches the datamodel without reaching the
     * scope is an expression that silently keeps the rewriter's answer.
     */
    private fun offerToScope(session: Session, name: String) {
        if (name.isEmpty() || !session.offeredToScope.add(name)) {
            return
        }
        session.loweringScope.declare(name)
    }

    override fun getVariable(sessionId: String, name: String): Any? {
        val session = sessions[sessionId] ?: return null
        val L = session.handle
        LuaNative.getGlobal(L, name)
        val result = luaToKotlin(L, -1)
        LuaNative.pop(L, 1)
        return result
    }

    override fun hasVariable(sessionId: String, name: String): Boolean {
        val session = sessions[sessionId] ?: return false
        return name in session.declaredVars
    }

    override fun assign(sessionId: String, location: String, expr: String) =
        doAssign(sessionId, ScriptSource.ecmascript(location), ScriptSource.ecmascript(expr))

    override fun doAssign(sessionId: String, location: ScriptSource, expr: ScriptSource) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        // W3C SCXML B.2: System variables are read-only — asked of the
        // author's spelling, because §scxml-5.10 names `_event` and the rest
        // in the document's language and not in this engine's.
        if (location.source.startsWith("_")) {
            throw ScriptEngineException("Cannot assign to system variable: ${location.source}")
        }

        val luaExpr = loweredTextOf(expr, session)
        // The WRITE TARGET, in this engine's language: it is spliced in front
        // of `=` below and run, so `Var1[0]` has to have become `Var1[1]`
        // before it gets here.
        val luaLocation = loweredLocationOf(location, session)
        val L = session.handle

        // C++ AssignmentExecutionHelper 3-path strategy.
        //
        // W3C SCXML 5.10 names `_event`, `_sessionid` and the rest — it names
        // them in the AUTHOR'S language, so the question "is this expression a
        // system variable reference" is asked of `source()` and not of whatever
        // a lowering spells them as. C++ states the same rule in
        // `docs/SCE_LUA_TRANSLATION_SEAM.md`: shape questions stay on the
        // author's text.
        if (isSystemVariableReference(expr.source)) {
            // Path 1: System variable reference — use script execution
            val err = LuaNative.doString(L, "$luaLocation = $luaExpr")
            if (err != null) {
                throw ScriptEngineException("Assignment failed: ${location.source} = ${expr.source} ($err)")
            }
        } else if (isSimpleVariableName(luaLocation)) {
            // Path 2: Simple variable — evaluate + setGlobal
            val status = LuaNative.loadAndCall(L, "return $luaExpr", 1)
            if (status != 0) {
                val err = LuaNative.getError(L)
                LuaNative.pop(L, 1)
                throw ScriptEngineException("Assignment failed: ${location.source} = ${expr.source} ($err)")
            }
            LuaNative.setGlobal(L, luaLocation)
        } else {
            // Path 3: Complex path — use script execution
            val err = LuaNative.doString(L, "$luaLocation = ($luaExpr)")
            if (err != null) {
                throw ScriptEngineException("Assignment failed: ${location.source} = ${expr.source} ($err)")
            }
        }
        session.declaredVars.add(luaLocation.split('.')[0].substringBefore('['))
        // §scxml-5.4: an assignment is how this datamodel's globals come into
        // existence, so the target's root name is one the frontend must be able
        // to resolve afterwards. Taken from the AUTHOR'S spelling rather than
        // from `luaLocation`, for the reason the system-variable check above
        // states: a name is a name in the document's language.
        offerToScope(session, location.source.split('.')[0].substringBefore('['))
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs): PayloadReading {
        val session = sessions[sessionId] ?: return PayloadReading.Absent
        var reading = PayloadReading.Absent
        val L = session.handle

        LuaNative.createTable(L, 0, 8)
        pushField(L, "name", args.name)
        pushField(L, "type", args.type.ifEmpty { "external" })
        pushField(L, "sendid", args.sendId)
        pushField(L, "origin", args.origin)
        pushField(L, "origintype", args.originType)
        pushField(L, "invokeid", args.invokeId)

        if (args.data.isNotEmpty()) {
            // Every rung of the ladder leaves a value on the stack, so the
            // reading is what it answers with rather than whether it managed
            // to push one — the `false` this used to return could only mean
            // "nothing pushed", which no rung produces.
            reading = parseDataValueInternal(L, args.data)
            LuaNative.setField(L, -2, "data")
        } else {
            LuaNative.pushNil(L)
            LuaNative.setField(L, -2, "data")
        }

        LuaNative.setGlobal(L, "_event")
        return reading
    }

    override fun clearCurrentEvent(sessionId: String) {
        val session = sessions[sessionId] ?: return
        LuaNative.pushNil(session.handle)
        LuaNative.setGlobal(session.handle, "_event")
    }

    override fun setStateQueryCallback(sessionId: String, callback: ((String) -> Boolean)?) {
        sessions[sessionId]?.stateQueryCallback = callback
    }

    override fun executeForeach(
        sessionId: String, array: String, item: String,
        index: String, body: () -> Unit
    ) = doExecuteForeach(sessionId, ScriptSource.ecmascript(array), item, index, body)

    override fun doExecuteForeach(
        sessionId: String, array: ScriptSource, item: String,
        index: String, body: () -> Unit
    ) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        if (!isLegalVariableName(item))
            throw ScriptEngineException("Illegal foreach item variable name: '$item'")
        if (index.isNotEmpty() && !isLegalVariableName(index))
            throw ScriptEngineException("Illegal foreach index variable name: '$index'")

        val L = session.handle
        val luaArray = loweredTextOf(array, session)

        // Evaluate array expression
        val status = LuaNative.loadAndCall(L, "return $luaArray", 1)
        if (status != 0) {
            val err = LuaNative.getError(L)
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Foreach array evaluation failed: ${array.source} ($err)")
        }

        if (!LuaNative.isTable(L, -1)) {
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Foreach expression is not an array: ${array.source}")
        }

        // W3C SCXML 4.6: foreach auto-declares item/index variables
        session.declaredVars.add(item)
        if (index.isNotEmpty()) session.declaredVars.add(index)
        // The same declaration, told to the frontend: the body's expressions
        // name `item` and `index`, and one the scope has not been told about is
        // refused.
        offerToScope(session, item)
        if (index.isNotEmpty()) offerToScope(session, index)

        val length = LuaNative.rawLen(L, -1)
        for (i in 1..length) {
            LuaNative.rawGetI(L, -1, i)
            val element = luaToKotlin(L, -1)
            LuaNative.pop(L, 1)

            // Set item variable
            pushKotlinValue(L, element)
            LuaNative.setGlobal(L, item)

            // Set index variable (0-based for ECMAScript compatibility)
            if (index.isNotEmpty()) {
                LuaNative.pushInteger(L, i - 1)
                LuaNative.setGlobal(L, index)
            }

            body()
        }
        LuaNative.pop(L, 1)  // pop array table
    }

    override fun loadDataFromSrc(src: String, basePath: String): String? {
        val filename = if (src.startsWith("file:")) src.substring(5) else src
        val file = java.io.File(basePath, filename)
        return try {
            file.readText(Charsets.UTF_8).trim()
        } catch (_: Exception) {
            null
        }
    }

    override fun parseDataValue(sessionId: String, data: String): Any? {
        val session = sessions[sessionId] ?: return data
        val L = session.handle
        // Every rung of §scxml-B-2-8-1 pushes a value — the third one pushes
        // the space-normalized string — so the branch that used to return the
        // raw `data` when nothing was pushed was unreachable. What the helper
        // reports now is WHICH rung, which is the fact `setCurrentEvent` needs
        // and this caller does not: a `<data>` element's value is whatever the
        // ladder made of it either way.
        parseDataValueInternal(L, data)
        return wrapLuaResult(L, session)
    }

    // === Private Helpers ===

    /**
     * Convert Lua stack top to Kotlin, using LuaRef only for values
     * that cannot survive the round-trip (functions, metatabled tables).
     * Plain tables are converted to Kotlin List/Map directly.
     */
    private fun wrapLuaResult(L: Long, session: Session): Any? {
        if (LuaNative.isFunction(L, -1)) {
            val ref = LuaNative.ref(L, LuaNative.registryIndex())
            session.activeRefs.add(ref)
            return LuaRef(ref, session.handle)
        }
        if (LuaNative.isTable(L, -1) && LuaNative.getMetatable(L, -1)) {
            // Sentinel tables (_NULL, _UNDEFINED) have metatables but are safe to convert
            val isSentinel = LuaNative.rawLen(L, -1) == 0L && !hasNonMetaKeys(L, -1)
            if (!isSentinel) {
                // DOM object or similar — preserve via registry
                val ref = LuaNative.ref(L, LuaNative.registryIndex())
                session.activeRefs.add(ref)
                return LuaRef(ref, session.handle)
            }
        }
        val result = luaToKotlin(L, -1)
        LuaNative.pop(L, 1)
        return result
    }

    /** Check if table at index has any non-metatable keys (i.e., has actual data). */
    private fun hasNonMetaKeys(L: Long, index: Int): Boolean {
        LuaNative.pushNil(L)
        val tableIndex = if (index < 0) index - 1 else index
        val hasKey = LuaNative.next(L, tableIndex) != 0
        if (hasKey) LuaNative.pop(L, 2) // pop key+value
        return hasKey
    }

    /** Release a consumed LuaRef from the registry and session tracking. */
    private fun unrefIfNeeded(session: Session, value: Any?) {
        if (value is LuaRef) {
            if (value.originHandle != session.handle) {
                throw ScriptEngineException(
                    "Cross-session LuaRef rejected: registryKey=${value.registryKey} " +
                        "origin=0x${value.originHandle.toString(16)} " +
                        "target=0x${session.handle.toString(16)}")
            }
            session.activeRefs.remove(value.registryKey)
            LuaNative.unref(session.handle, LuaNative.registryIndex(), value.registryKey)
        }
    }

    private fun registerBuiltins(L: Long) {
        // Sandbox — remove dangerous libraries before any user code runs
        LuaNative.doString(L, """
            os = nil
            io = nil
            loadfile = nil
            dofile = nil
            require = nil
        """.trimIndent())

        // Null/undefined sentinels — C++ parity: lightuserdata with tags
        // Using tables with unique metatables as sentinels, checked by rawequal
        LuaNative.doString(L, """
            _NULL = setmetatable({}, {__tostring = function() return "null" end})
            _UNDEFINED = setmetatable({}, {__tostring = function() return "undefined" end})
        """.trimIndent())

        // `_scxml_truthy`, `_typeof`, `_isArray`, `_indexOf`, `_concat`,
        // `parseInt` and `parseFloat` were written out here too, one of six
        // implementations of one meaning. They come from
        // `loadEcmaSemantics()` below now — the shared file every backend
        // loads — because the six drifted: measured 2026-08-16 against
        // `tests/ecmascript/ecma262_semantics.json`, this copy's `_indexOf`
        // ignored its `from` argument and refused a non-string needle, and
        // its `parseInt` answered 0 where the clause says NaN.

        // String metatable __add for + operator coercion
        LuaNative.doString(L, """
            local mt = getmetatable("")
            if mt then
                mt.__add = function(a, b)
                    return tostring(a) .. tostring(b)
                end
            end
        """.trimIndent())

        // debug.setmetatable for non-object property access — C++ parity
        // Allows (1).bar and true.foo to return nil instead of erroring
        LuaNative.doString(L, """
            debug.setmetatable(0, {__index = function() return nil end})
            debug.setmetatable(true, {__index = function() return nil end})
        """.trimIndent())

        // In() predicate — delegates to Kotlin callback via global lookup
        // Since JNI callbacks are expensive, we store a lookup table that Kotlin updates
        LuaNative.createTable(L, 0, 0)
        LuaNative.setGlobal(L, "__sce_active_states")
        LuaNative.doString(L, """
            function In(stateId)
                return __sce_active_states[stateId] == true
            end
        """.trimIndent())

        // W3C SCXML B.2: the ECMAScript operators Lua does not share — `+`,
        // `==` and the bitwise family. Single Source of Truth at
        // sce/include/scripting/ecma_semantics.lua; the code sce-build emits
        // calls these by name on every backend.
        LuaNative.doString(L, loadEcmaSemantics())

        // W3C SCXML B.2: JSON.stringify / JSON.parse (Single Source of Truth)
        // Shared with C++ LuaEngine (CMake header) and Rust sce-rust-lua (include_str!)
        // via sce/include/scripting/json_builtins.lua — see ARCHITECTURE.md
        LuaNative.doString(L, loadJsonBuiltins())

        // `Object.keys` comes from `loadEcmaSemantics()` above with the rest
        // of the engine vocabulary. Defining it HERE also put it after that
        // load, so this copy is what a document reached — which is the shape
        // that lets a per-engine copy outlive the shared definition silently.

        // W3C DOM: Register metatable once at session init, not per-element
        LuaNative.doString(L, DOM_METATABLE_SETUP)
    }

    /**
     * Update the In() predicate state table for a session.
     * Called by StateMachineEngine before processing events.
     */
    fun updateActiveStates(sessionId: String, activeStateIds: Set<String>) {
        val session = sessions[sessionId] ?: return
        val L = session.handle

        LuaNative.createTable(L, 0, activeStateIds.size)
        for (stateId in activeStateIds) {
            LuaNative.pushBoolean(L, true)
            LuaNative.setField(L, -2, stateId)
        }
        LuaNative.setGlobal(L, "__sce_active_states")
    }

    private fun evaluateLuaBoolean(session: Session, luaExpr: String, originalExpr: String): Boolean {
        val L = session.handle
        val status = LuaNative.loadAndCall(L, "return $luaExpr", 1)
        if (status != 0) {
            val err = LuaNative.getError(L)
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Guard evaluation failed: '$originalExpr' (Lua: $err)")
        }
        val result = LuaNative.toBoolean(L, -1)
        LuaNative.pop(L, 1)
        return result
    }

    private fun pushKotlinValue(L: Long, value: Any?) {
        when (value) {
            null -> LuaNative.pushNil(L)
            is LuaRef -> {
                if (value.originHandle != L) {
                    throw ScriptEngineException(
                        "Cross-session LuaRef rejected: registryKey=${value.registryKey} " +
                            "origin=0x${value.originHandle.toString(16)} " +
                            "target=0x${L.toString(16)}")
                }
                LuaNative.rawGetI(L, LuaNative.registryIndex(), value.registryKey.toLong())
            }
            is Boolean -> LuaNative.pushBoolean(L, value)
            is Int -> LuaNative.pushInteger(L, value.toLong())
            is Long -> LuaNative.pushInteger(L, value)
            is Double -> LuaNative.pushNumber(L, value)
            is Float -> LuaNative.pushNumber(L, value.toDouble())
            is String -> LuaNative.pushString(L, value)
            is List<*> -> {
                LuaNative.createTable(L, value.size, 0)
                for (i in value.indices) {
                    pushKotlinValue(L, value[i])
                    LuaNative.rawSetI(L, -2, (i + 1).toLong())
                }
            }
            is Map<*, *> -> {
                LuaNative.createTable(L, 0, value.size)
                for ((k, v) in value) {
                    pushKotlinValue(L, v)
                    LuaNative.setField(L, -2, k.toString())
                }
            }
            else -> LuaNative.pushString(L, value.toString())
        }
    }

    private fun luaToKotlin(L: Long, index: Int): Any? {
        return when {
            LuaNative.isNil(L, index) -> null
            LuaNative.isBoolean(L, index) -> LuaNative.toBoolean(L, index)
            // A string is asked about BEFORE a number, and by its type
            // rather than by `lua_isnumber`. Lua's `isnumber` answers true
            // for a string that could be converted to one, so the numeric
            // arms used to swallow strings: measured 2026-08-18, a DOM
            // attribute `count="2"` reached the datamodel as the number 2
            // on this backend and as the string "2" on the other six, and
            // a `cond` comparing it to "2" was false.
            LuaNative.type(L, index) == LuaNative.typeString() -> LuaNative.toJString(L, index)
            LuaNative.isInteger(L, index) -> LuaNative.toInteger(L, index)
            LuaNative.isNumber(L, index) -> LuaNative.toNumber(L, index)
            LuaNative.isTable(L, index) -> {
                val len = LuaNative.rawLen(L, index)
                if (len > 0) {
                    val list = mutableListOf<Any?>()
                    for (i in 1..len) {
                        LuaNative.rawGetI(L, index, i)
                        list.add(luaToKotlin(L, -1))
                        LuaNative.pop(L, 1)
                    }
                    list
                } else {
                    val map = mutableMapOf<String, Any?>()
                    LuaNative.pushNil(L)
                    val tableIndex = if (index < 0) index - 1 else index
                    while (LuaNative.next(L, tableIndex) != 0) {
                        val key = LuaNative.toJString(L, -2)
                        val value = luaToKotlin(L, -1)
                        if (key != null) map[key] = value
                        LuaNative.pop(L, 1)
                    }
                    map
                }
            }
            else -> null
        }
    }

    private fun pushField(L: Long, key: String, value: String) {
        LuaNative.pushString(L, value)
        LuaNative.setField(L, -2, key)
    }

    private fun isSystemVariableReference(expr: String): Boolean {
        return expr == "_sessionid" || expr == "_event" || expr == "_name" ||
                expr == "_ioprocessors" || expr == "_x"
    }

    private fun isSimpleVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        if (!name[0].isLetter() && name[0] != '_') return false
        return name.all { it.isLetterOrDigit() || it == '_' }
    }

    private fun isLegalVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        if (name[0] == '\'' || name[0] == '"') return false
        if (!name[0].isLetter() && name[0] != '_' && name[0] != '$') return false
        return name.all { it.isLetterOrDigit() || it == '_' || it == '$' }
    }

    /**
     * §scxml-5.9: does this expression read a name ECMAScript would call
     * undeclared? JavaScript throws `ReferenceError`; Lua answers `nil`.
     *
     * ⚠ **Three questions, not one, and this asked only the second.** C++'s
     * `isUndeclaredIdentifier` (`sce/src/scripting/LuaEngine.cpp`) asks
     * whether the name is a Lua keyword, whether the session DECLARED it, and
     * whether it is a live Lua GLOBAL — undeclared only if all three say no.
     * This copy had the middle one and a hard-coded list standing in for the
     * third, so a name that exists in the interpreter but never went through
     * [setVariable] read as undeclared.
     *
     * That is not a corner. `executeScript` files nothing in `declaredVars` —
     * only [setVariable], [assign] and [executeForeach] do — so a document
     * whose `<script>` assigns `Var1 = 1` and then reads `Var1` got a
     * `ReferenceError` for a variable sitting in the global table. Two of the
     * shared table's cases are exactly that shape (`13.15.2 compound
     * assignment`, `14.7.4 the for statement`), and they were declared
     * divergences on BOTH routes into this engine — the rewriter's and the
     * frontend's — because the defect was never on that seam at all.
     *
     * The global lookup also RETIRES the list. Every non-keyword name it held
     * — `math`, `In`, `JSON`, `_scxml_truthy`, `Object` … — is registered as a
     * global by [registerBuiltins], so the interpreter already knew them and a
     * second copy of that knowledge could only drift. What a lookup cannot
     * answer is a keyword: `true`, `false` and `nil` are literals rather than
     * globals, so those stay, spelled as Lua's own keyword set the way C++
     * spells it.
     */
    private fun isUndeclaredSimpleVariable(luaExpr: String, session: Session): Boolean {
        val trimmed = luaExpr.trim()
        if (trimmed.isEmpty()) return false
        if (!isSimpleVariableName(trimmed)) return false
        if (trimmed.startsWith("_")) return false
        if (trimmed in LUA_KEYWORDS) return false
        if (trimmed in session.declaredVars) return false
        return !isLuaGlobal(session.handle, trimmed)
    }

    /** Whether @p name currently holds a non-nil value in the global table. */
    private fun isLuaGlobal(handle: Long, name: String): Boolean {
        LuaNative.getGlobal(handle, name)
        val present = !LuaNative.isNil(handle, -1)
        LuaNative.pop(handle, 1)
        return present
    }

    /**
     * §scxml-B-2-8-1: parse a raw data value as an XML DOM, as JSON, or as a
     * space-normalized string — those three readings and no fourth.
     *
     * C++ parity: `SCE::LuaEngine::setCurrentEvent`, which lost the same
     * fourth rung on the same day.
     *
     * Returns true if a value was pushed onto the Lua stack, false otherwise.
     */
    private fun parseDataValueInternal(L: Long, data: String): PayloadReading {
        // Step 1: XML detection
        val firstNonWs = data.indexOfFirst { !it.isWhitespace() }
        if (firstNonWs >= 0 && data[firstNonWs] == '<') {
            if (pushDOMObject(L, data)) return PayloadReading.Dom
        }

        // Step 2: JSON.parse via atomic chunk — cleanup guaranteed inside chunk
        //
        // There used to be a rung above this one — `loadAndCall("return " +
        // data)`, running the payload as this engine's own source language
        // before anything looked at it. §scxml-B-2-8-1 gives the payload three
        // readings and no fourth: XML becomes a DOM, JSON becomes the value,
        // anything else becomes a space-normalized string. The 2026-08-17 round
        // removed the rung from the four engines that had a test lane and left
        // it in the two that did not: this one and the C++ Lua engine.
        // Measured 2026-08-19, it still decided all three of the following
        // here:
        //
        //   * `2 + 3` from a host arrived as the number 5, and as the string
        //     "2 + 3" on this backend's OWN Rhino and QuickJS engines, which
        //     read the clause. One payload, two answers, from three engines
        //     behind one backend — and a generated machine takes its engine as
        //     a constructor argument, so which answer a document gets was the
        //     embedder's choice rather than the clause's.
        //   * a payload that is a call RAN, in the session's own globals.
        //     `_event.data` is the one field a document takes from outside
        //     itself.
        //   * the payload was read in whatever language the receiver happened
        //     to be built from.
        //
        // The sender ships JSON (§scxml-B-2-9), so the two rungs the clause
        // names are the two that are here.
        LuaNative.pushString(L, data)
        LuaNative.setGlobal(L, "__sce_tmp")
        val jsonStatus = LuaNative.loadAndCall(L,
            "local d = __sce_tmp; __sce_tmp = nil; if d then return JSON.parse(d) end", 1)
        if (jsonStatus != 0) {
            LuaNative.pop(L, 1)
            LuaNative.pushNil(L); LuaNative.setGlobal(L, "__sce_tmp")
        } else if (!LuaNative.isNil(L, -1)) {
            return PayloadReading.Structured
        } else {
            LuaNative.pop(L, 1)
        }

        // Step 3: Space-normalized string (W3C SCXML B.2, test 562)
        LuaNative.pushString(L, normalizeWhitespace(data))
        // Which of the two third-rung readings this is. The clause treats them
        // the same and a host does not: prose arriving as text is the ladder
        // working, and a payload that opened with a brace and would not parse
        // is a payload whose fields have just stopped existing. Only here is
        // that difference still visible, because only here was the structured
        // read attempted.
        return PayloadReading.ofText(data)
    }

    /**
     * §scxml-B-2-1 — push "the corresponding DOM structure" for [xmlContent].
     *
     * Built as one Lua expression and evaluated, the way the QuickJS
     * backend builds one JS expression: the raw stack API here has no
     * `lua_pushvalue`, so a child cannot be handed a reference to the
     * parent table while both are on the stack — and `parentNode` needs
     * exactly that. `__sce_dom_node` links the parents as it constructs.
     */
    private fun pushDOMObject(L: Long, xmlContent: String): Boolean {
        val document = SceXmlDom.parse(xmlContent) ?: return false
        val root = document.documentElement ?: return false
        val expression = "return __sce_dom_document(${nodeExpression(root)})"
        return LuaNative.loadAndCall(L, expression, 1) == 0
    }

    /**
     * One node as a Lua table constructor, children in document order.
     *
     * Whitespace-only text, comments and processing instructions are not
     * nodes: the cpp reference backend parses with pugixml's
     * `parse_default`, which omits `parse_ws_pcdata`, `parse_comments`
     * and `parse_pi`, and `javax.xml` keeps all three. While
     * `getElementsByTagName` was the only reader the difference could not
     * be seen — that call collects elements — and it decides every
     * traversal now that `childNodes` and `firstChild` are readable.
     */
    private fun nodeExpression(node: Node): String {
        val sb = StringBuilder()
        sb.append("__sce_dom_node({__type=").append(SceXmlDom.nodeType(node))
        sb.append(",__name=").append(luaStringLiteral(SceXmlDom.nodeName(node)))
        if (SceXmlDom.hasNodeValue(node)) {
            sb.append(",__value=").append(luaStringLiteral(node.nodeValue ?: ""))
        } else {
            sb.append(",__tagName=").append(luaStringLiteral(node.nodeName ?: ""))
            sb.append(",__attrs={")
            val attrs = node.attributes
            if (attrs != null) {
                for (i in 0 until attrs.length) {
                    if (i > 0) sb.append(",")
                    val attr = attrs.item(i)
                    sb.append("[").append(luaStringLiteral(attr.nodeName)).append("]=")
                        .append(luaStringLiteral(attr.nodeValue ?: ""))
                }
            }
            sb.append("}")
            val kids = SceXmlDom.children(node)
            if (kids.isNotEmpty()) {
                sb.append(",__kids={")
                for ((index, child) in kids.withIndex()) {
                    if (index > 0) sb.append(",")
                    sb.append(nodeExpression(child))
                }
                sb.append("}")
            }
        }
        sb.append("})")
        return sb.toString()
    }

    private fun luaStringLiteral(value: String): String {
        val sb = StringBuilder(value.length + 2)
        sb.append('"')
        for (c in value) {
            when (c) {
                '\\' -> sb.append("\\\\")
                '"' -> sb.append("\\\"")
                '\n' -> sb.append("\\n")
                '\r' -> sb.append("\\r")
                '\t' -> sb.append("\\t")
                else -> if (c.code < 0x20) sb.append("\\").append(c.code) else sb.append(c)
            }
        }
        sb.append('"')
        return sb.toString()
    }

    private fun normalizeWhitespace(data: String): String {
        val sb = StringBuilder(data.length)
        var inWhitespace = false
        var hasContent = false
        for (c in data) {
            if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
                if (hasContent && !inWhitespace) { sb.append(' '); inWhitespace = true }
            } else { sb.append(c); inWhitespace = false; hasContent = true }
        }
        if (sb.isNotEmpty() && sb.last() == ' ') sb.deleteCharAt(sb.length - 1)
        return sb.toString()
    }

    companion object {
        // W3C SCXML B.2: Load canonical json_builtins.lua from resources (Single Source of Truth)
        // Shared with C++ (CMake header) and Rust (include_str!) — see ARCHITECTURE.md
        private val JSON_BUILTINS: String by lazy {
            LuaScriptEngine::class.java.getResourceAsStream("/scripting/json_builtins.lua")
                ?.bufferedReader()?.readText()
                ?: error("json_builtins.lua not found in resources — check Gradle copyJsonBuiltins task")
        }

        private fun loadJsonBuiltins(): String = JSON_BUILTINS

        // W3C SCXML B.2 operator semantics, copied into resources by the
        // Gradle `copyEcmaSemantics` task from the one shared source.
        private val ECMA_SEMANTICS: String by lazy {
            LuaScriptEngine::class.java.getResourceAsStream("/scripting/ecma_semantics.lua")
                ?.bufferedReader()?.readText()
                ?: error("ecma_semantics.lua not found in resources — check Gradle copyEcmaSemantics task")
        }

        private fun loadEcmaSemantics(): String = ECMA_SEMANTICS

        /**
         * Lua's own keywords — the names a global lookup cannot answer for.
         *
         * This replaced a list that mixed keywords with globals (`math`,
         * `In`, `JSON`, `Object`, `_scxml_truthy` …). Those are registered by
         * [registerBuiltins], so the interpreter is the one place that knows
         * them and [isUndeclaredSimpleVariable] now asks it. `true`, `false`
         * and `nil` are literals rather than globals, and neither is `and` or
         * `end`, so the keyword set is what stays behind — the same 22 names
         * `SCE::isUndeclaredIdentifier` spells in C++, kept as one list rather
         * than the subset this engine happened to meet.
         */
        private val LUA_KEYWORDS = setOf(
            "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if",
            "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until", "while"
        )

        // §scxml-B-2-1's DOM read surface — DOM Level 1 Core, not the two
        // calls the W3C IRP suite happens to read. `__index` is a function
        // because the surface is properties and `parentNode`/`childNodes`
        // point at each other: any eager materialisation of either walks
        // the tree until it runs out of stack.
        //
        // A document handle holds `__root` and answers the Node interface
        // as the document it is, while answering the Element vocabulary
        // for its document element — the delegation `getAttribute` and
        // `getTagName` have always performed.
        private val DOM_METATABLE_SETUP = """
            __sce_dom_mt = {}
            local ELEMENT, TEXT, CDATA, DOCUMENT = 1, 3, 4, 9
            local function resolve(self)
                local root = rawget(self, "__root")
                if root ~= nil then return root, true end
                return self, false
            end
            local function collectByTagName(node, tagName, result)
                local kids = rawget(node, "__kids")
                if not kids then return end
                for i = 1, #kids do
                    local kid = kids[i]
                    if rawget(kid, "__type") == ELEMENT then
                        if rawget(kid, "__tagName") == tagName then
                            result[#result+1] = kid
                        end
                        collectByTagName(kid, tagName, result)
                    end
                end
            end
            local function hasNodeValue(node)
                local t = rawget(node, "__type")
                return t == TEXT or t == CDATA
            end
            local function textContent(node)
                if hasNodeValue(node) then return rawget(node, "__value") or "" end
                local kids = rawget(node, "__kids")
                if not kids then return "" end
                local parts = {}
                for i = 1, #kids do parts[#parts+1] = textContent(kids[i]) end
                return table.concat(parts)
            end
            local function siblingOf(node, step)
                local parent = rawget(node, "__parent")
                if parent == nil then return nil end
                local kids = rawget(parent, "__kids")
                if not kids then return nil end
                for i = 1, #kids do
                    if kids[i] == node then return kids[i + step] end
                end
                return nil
            end
            local methods = {
                getElementsByTagName = function(self, tagName)
                    local node, isDocument = resolve(self)
                    local result = {}
                    -- A document matches its root inclusively, an element
                    -- only descends: DOM Level 1 Core 1.2's split.
                    if isDocument and rawget(node, "__tagName") == tagName then
                        result[1] = node
                    end
                    collectByTagName(node, tagName, result)
                    return result
                end,
                getAttribute = function(self, attrName)
                    local node = resolve(self)
                    local attrs = rawget(node, "__attrs")
                    if attrs then return attrs[attrName] or "" end
                    return ""
                end,
                hasAttribute = function(self, attrName)
                    local node = resolve(self)
                    local attrs = rawget(node, "__attrs")
                    return attrs ~= nil and attrs[attrName] ~= nil
                end,
                getTagName = function(self)
                    local node = resolve(self)
                    return rawget(node, "__tagName") or ""
                end,
                hasChildNodes = function(self)
                    local node, isDocument = resolve(self)
                    if isDocument then return true end
                    local kids = rawget(node, "__kids")
                    return kids ~= nil and #kids > 0
                end
            }
            __sce_dom_mt.__index = function(self, key)
                local method = methods[key]
                if method ~= nil then return method end
                local node, isDocument = resolve(self)
                if key == "nodeType" then
                    if isDocument then return DOCUMENT end
                    return rawget(node, "__type")
                elseif key == "nodeName" then
                    if isDocument then return "#document" end
                    return rawget(node, "__name")
                elseif key == "nodeValue" or key == "data" then
                    if isDocument or not hasNodeValue(node) then return nil end
                    return rawget(node, "__value")
                elseif key == "tagName" then
                    if not isDocument and hasNodeValue(node) then return nil end
                    return rawget(node, "__tagName")
                elseif key == "textContent" then
                    return textContent(node)
                elseif key == "childNodes" then
                    if isDocument then return { node } end
                    local kids = rawget(node, "__kids")
                    if kids == nil then return {} end
                    local copy = {}
                    for i = 1, #kids do copy[i] = kids[i] end
                    return copy
                elseif key == "firstChild" then
                    if isDocument then return node end
                    local kids = rawget(node, "__kids")
                    return kids and kids[1] or nil
                elseif key == "lastChild" then
                    if isDocument then return node end
                    local kids = rawget(node, "__kids")
                    return kids and kids[#kids] or nil
                elseif key == "nextSibling" then
                    if isDocument then return nil end
                    return siblingOf(node, 1)
                elseif key == "previousSibling" then
                    if isDocument then return nil end
                    return siblingOf(node, -1)
                elseif key == "parentNode" then
                    if isDocument then return nil end
                    local parent = rawget(node, "__parent")
                    -- The document element's parent is the document — DOM
                    -- Level 1 Core 1.3 — which is the handle the variable
                    -- already holds.
                    if parent == nil then return rawget(node, "__doc") end
                    return parent
                elseif key == "documentElement" then
                    -- Only the document handle carries this, which is how
                    -- a document tells the two kinds apart.
                    if isDocument then return node end
                    return nil
                end
                return nil
            end
            function __sce_dom_node(spec)
                setmetatable(spec, __sce_dom_mt)
                local kids = rawget(spec, "__kids")
                if kids then
                    for i = 1, #kids do rawset(kids[i], "__parent", spec) end
                end
                return spec
            end
            function __sce_dom_document(root)
                local doc = setmetatable({ __root = root }, __sce_dom_mt)
                rawset(root, "__doc", doc)
                return doc
            end
        """.trimIndent()
    }
}
