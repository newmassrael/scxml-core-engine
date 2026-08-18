// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Lua — Lua 5.4 implementation of ScxmlScriptEngine
//
// Drop-in replacement for RhinoScriptEngine using Lua 5.4 via JNI.
// ECMAScript expressions are transformed to Lua via EcmaScriptToLuaTransformer.
// Each session gets an isolated lua_State for full variable isolation.
//
// C++ parity: sce/src/scripting/LuaEngine.cpp

package com.sce.scripting.lua

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.SceXmlDom
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.w3c.dom.Node

/**
 * Lua 5.4, running SCXML expressions that were rewritten from ECMAScript.
 *
 * **This is not an ECMAScript engine, and a `datamodel="ecmascript"` document
 * does not run correctly on it.** Measured 2026-08-14 against the shared
 * table in `tests/ecmascript/ecma262_semantics.json`: 27 of its 58 cases are
 * answered differently from what ECMA-262 says, and the disagreements are not
 * exotic — `0 && x` comes back true, `1 == '1'` comes back false, `-7 % 3`
 * comes back 2, a computed array index is off by one. The C++ build measured
 * the same class at 26 of 58 and flipped its default engine away from Lua for
 * that reason. `EcmaScriptSemanticsTest` holds this paragraph to the
 * measurement, so it cannot go back to promising more than the engine does.
 *
 * This header used to read "For AOSP/AAOS production, this replaces Rhino
 * with a faster native engine". It is faster, and that sentence sent a reader
 * building an AAOS product to an engine that answers their guards wrong.
 * For that product the engines that answer ECMA-262 are Rhino on the JVM and
 * QuickJS natively — both measured at 58 of 58 by the same test.
 *
 * What it remains good for is a document whose expressions stay inside what
 * the rewriter covers, on a device with no room for a JS engine. That is a
 * real position; it is just not the same as running ECMAScript.
 *
 * Each session gets its own lua_State (variable isolation). ECMAScript
 * expressions from SCXML are transformed to Lua via
 * [EcmaScriptToLuaTransformer] before evaluation — a rewriter, not an
 * interpreter of the language, which is where the 27 come from.
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
        val activeRefs: MutableSet<Int> = mutableSetOf()
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
    }

    override fun evaluateCondition(sessionId: String, expr: String): Boolean {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaExpr = transformer.transform(expr, EcmaScriptToLuaTransformer.ExpressionContext.Guard)
        return evaluateLuaBoolean(session, luaExpr, expr)
    }

    override fun evaluateExpr(sessionId: String, expr: String): Any? {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaExpr = transformer.transform(expr)

        // Check undeclared simple variable (W3C SCXML compliance: JS throws ReferenceError)
        if (isUndeclaredSimpleVariable(luaExpr, session)) {
            throw ScriptEngineException("ReferenceError: $expr is not defined")
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
            throw ScriptEngineException("Expression evaluation failed: '$expr' (Lua: $err)")
        }

        throw ScriptEngineException("Expression evaluation failed: '$expr'")
    }

    override fun executeScript(sessionId: String, script: String) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val luaScript = transformer.transformScript(script)
        val err = LuaNative.doString(session.handle, luaScript)
        if (err != null) {
            throw ScriptEngineException("Script execution failed: $err")
        }
    }

    override fun setVariable(sessionId: String, name: String, value: Any?) {
        val session = sessions[sessionId] ?: return
        val L = session.handle
        pushKotlinValue(L, value)
        LuaNative.setGlobal(L, name)
        session.declaredVars.add(name)
        unrefIfNeeded(session, value)
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

    override fun assign(sessionId: String, location: String, expr: String) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        // W3C SCXML B.2: System variables are read-only
        if (location.startsWith("_")) {
            throw ScriptEngineException("Cannot assign to system variable: $location")
        }

        val luaExpr = transformer.transform(expr)
        val L = session.handle

        // C++ AssignmentExecutionHelper 3-path strategy
        if (isSystemVariableReference(luaExpr)) {
            // Path 1: System variable reference — use script execution
            val err = LuaNative.doString(L, "$location = $luaExpr")
            if (err != null) throw ScriptEngineException("Assignment failed: $location = $expr ($err)")
        } else if (isSimpleVariableName(location)) {
            // Path 2: Simple variable — evaluate + setGlobal
            val status = LuaNative.loadAndCall(L, "return $luaExpr", 1)
            if (status != 0) {
                val err = LuaNative.getError(L)
                LuaNative.pop(L, 1)
                throw ScriptEngineException("Assignment failed: $location = $expr ($err)")
            }
            LuaNative.setGlobal(L, location)
        } else {
            // Path 3: Complex path — use script execution
            val err = LuaNative.doString(L, "$location = ($luaExpr)")
            if (err != null) throw ScriptEngineException("Assignment failed: $location = $expr ($err)")
        }
        session.declaredVars.add(location.split('.')[0])
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs) {
        val session = sessions[sessionId] ?: return
        val L = session.handle

        LuaNative.createTable(L, 0, 8)
        pushField(L, "name", args.name)
        pushField(L, "type", args.type.ifEmpty { "external" })
        pushField(L, "sendid", args.sendId)
        pushField(L, "origin", args.origin)
        pushField(L, "origintype", args.originType)
        pushField(L, "invokeid", args.invokeId)

        if (args.data.isNotEmpty()) {
            val parsed = parseDataValueInternal(L, args.data)
            if (parsed) {
                LuaNative.setField(L, -2, "data")
            } else {
                LuaNative.pushNil(L)
                LuaNative.setField(L, -2, "data")
            }
        } else {
            LuaNative.pushNil(L)
            LuaNative.setField(L, -2, "data")
        }

        LuaNative.setGlobal(L, "_event")
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
    ) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        if (!isLegalVariableName(item))
            throw ScriptEngineException("Illegal foreach item variable name: '$item'")
        if (index.isNotEmpty() && !isLegalVariableName(index))
            throw ScriptEngineException("Illegal foreach index variable name: '$index'")

        val L = session.handle
        val luaArray = transformer.transform(array)

        // Evaluate array expression
        val status = LuaNative.loadAndCall(L, "return $luaArray", 1)
        if (status != 0) {
            val err = LuaNative.getError(L)
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Foreach array evaluation failed: $array ($err)")
        }

        if (!LuaNative.isTable(L, -1)) {
            LuaNative.pop(L, 1)
            throw ScriptEngineException("Foreach expression is not an array: $array")
        }

        // W3C SCXML 4.6: foreach auto-declares item/index variables
        session.declaredVars.add(item)
        if (index.isNotEmpty()) session.declaredVars.add(index)

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
        val pushed = parseDataValueInternal(L, data)
        return if (pushed) {
            wrapLuaResult(L, session)
        } else data
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

    private fun isUndeclaredSimpleVariable(luaExpr: String, session: Session): Boolean {
        val trimmed = luaExpr.trim()
        if (trimmed.isEmpty()) return false
        if (!isSimpleVariableName(trimmed)) return false
        if (trimmed.startsWith("_") || trimmed in BUILTINS) return false
        return trimmed !in session.declaredVars
    }

    /**
     * W3C SCXML B.2: Parse raw data value as XML DOM, Lua expression, JSON, or space-normalized string.
     * C++ parity: sce/src/scripting/LuaEngine.cpp:1231-1261
     * Returns true if a value was pushed onto the Lua stack, false otherwise.
     */
    private fun parseDataValueInternal(L: Long, data: String): Boolean {
        // Step 1: XML detection
        val firstNonWs = data.indexOfFirst { !it.isWhitespace() }
        if (firstNonWs >= 0 && data[firstNonWs] == '<') {
            if (pushDOMObject(L, data)) return true
        }

        // Step 2: Try as Lua expression (for structured data like Lua tables)
        // C++ pattern: "return " + eventData via luaL_dostring
        val luaExprStatus = LuaNative.loadAndCall(L, "return $data", 1)
        if (luaExprStatus == 0) {
            return true
        }
        LuaNative.pop(L, 1)  // pop error

        // Step 3: JSON.parse via atomic chunk — cleanup guaranteed inside chunk
        LuaNative.pushString(L, data)
        LuaNative.setGlobal(L, "__sce_tmp")
        val jsonStatus = LuaNative.loadAndCall(L,
            "local d = __sce_tmp; __sce_tmp = nil; if d then return JSON.parse(d) end", 1)
        if (jsonStatus != 0) {
            LuaNative.pop(L, 1)
            LuaNative.pushNil(L); LuaNative.setGlobal(L, "__sce_tmp")
        } else if (!LuaNative.isNil(L, -1)) {
            return true
        } else {
            LuaNative.pop(L, 1)
        }

        // Step 4: Space-normalized string (W3C SCXML B.2, test 562)
        LuaNative.pushString(L, normalizeWhitespace(data))
        return true
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

        private val BUILTINS = setOf(
            "true", "false", "nil", "math", "string", "table", "type",
            "tostring", "tonumber", "pairs", "ipairs", "print",
            "In", "_scxml_truthy", "_typeof", "_isArray", "_indexOf", "_concat",
            "parseInt", "parseFloat", "JSON", "_NULL", "_UNDEFINED", "Object", "debug"
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
