// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Lua — Lua 5.4 implementation of ScxmlScriptEngine
//
// Drop-in replacement for RhinoScriptEngine using Lua 5.4 via JNI.
// ECMAScript expressions are transformed to Lua via EcmaScriptToLuaTransformer.
// Each session gets an isolated lua_State for full variable isolation.
//
// C++ parity: sce/src/scripting/LuaEngine.cpp

package com.sce.scripting.lua

import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element
import org.xml.sax.InputSource
import java.io.StringReader

/**
 * Lua 5.4 ECMAScript engine for W3C SCXML datamodel evaluation.
 *
 * Each session gets its own lua_State (variable isolation).
 * ECMAScript expressions from SCXML are automatically transformed to Lua
 * via [EcmaScriptToLuaTransformer] before evaluation.
 *
 * For AOSP/AAOS production, this replaces Rhino with a faster native engine.
 */
class LuaScriptEngine : ScxmlScriptEngine {

    /**
     * Opaque handle for Lua values (functions, metatabled tables) that cannot survive
     * Kotlin round-trip. Stored in Lua registry via luaL_ref, retrieved via lua_rawgeti.
     * Must be consumed via [setVariable] or explicitly released via [unrefIfNeeded].
     */
    private data class LuaRef(val registryKey: Int)

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
        registerBuiltins(handle, sessionId)
    }

    override fun destroySession(sessionId: String) {
        val session = sessions.remove(sessionId) ?: return
        val regIdx = LuaNative.registryIndex()
        for (ref in session.activeRefs) {
            LuaNative.unref(session.handle, regIdx, ref)
        }
        LuaNative.closeState(session.handle)
    }

    override fun setupSystemVariables(sessionId: String, machineName: String) {
        val session = sessions[sessionId] ?: return
        val L = session.handle

        // W3C SCXML 5.10: _sessionid
        LuaNative.pushString(L, sessionId)
        LuaNative.setGlobal(L, "_sessionid")

        // W3C SCXML 5.10: _name
        LuaNative.pushString(L, machineName)
        LuaNative.setGlobal(L, "_name")

        // W3C SCXML 5.10: _ioprocessors
        LuaNative.createTable(L, 0, 1)
        LuaNative.createTable(L, 0, 1)
        LuaNative.pushString(L, sessionId)
        LuaNative.setField(L, -2, "location")
        LuaNative.setField(L, -2, "scxml")
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

    override fun setCurrentEvent(
        sessionId: String, name: String, data: String,
        type: String, sendId: String, origin: String,
        originType: String, invokeId: String
    ) {
        val session = sessions[sessionId] ?: return
        val L = session.handle

        LuaNative.createTable(L, 0, 8)
        pushField(L, "name", name)
        pushField(L, "type", type.ifEmpty { "external" })
        pushField(L, "sendid", sendId)
        pushField(L, "origin", origin)
        pushField(L, "origintype", originType)
        pushField(L, "invokeid", invokeId)

        if (data.isNotEmpty()) {
            val parsed = parseDataValueInternal(L, data)
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
            return LuaRef(ref)
        }
        if (LuaNative.isTable(L, -1) && LuaNative.getMetatable(L, -1)) {
            // Sentinel tables (_NULL, _UNDEFINED) have metatables but are safe to convert
            val isSentinel = LuaNative.rawLen(L, -1) == 0L && !hasNonMetaKeys(L, -1)
            if (!isSentinel) {
                // DOM object or similar — preserve via registry
                val ref = LuaNative.ref(L, LuaNative.registryIndex())
                session.activeRefs.add(ref)
                return LuaRef(ref)
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
            session.activeRefs.remove(value.registryKey)
            LuaNative.unref(session.handle, LuaNative.registryIndex(), value.registryKey)
        }
    }

    private fun registerBuiltins(L: Long, @Suppress("UNUSED_PARAMETER") sessionId: String) {
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

        // _scxml_truthy: ECMAScript truthiness — C++ parity: includes sentinel checks
        LuaNative.doString(L, """
            function _scxml_truthy(v)
                if v == nil then return false end
                if v == false then return false end
                if rawequal(v, _NULL) then return false end
                if rawequal(v, _UNDEFINED) then return false end
                if v == 0 then return false end
                if v == "" then return false end
                return true
            end
        """.trimIndent())

        // _typeof: ECMAScript typeof operator — C++ parity: null -> "object"
        LuaNative.doString(L, """
            function _typeof(v)
                if v == nil then return "undefined" end
                if rawequal(v, _NULL) then return "object" end
                if rawequal(v, _UNDEFINED) then return "undefined" end
                local t = type(v)
                if t == "table" then return "object" end
                return t
            end
        """.trimIndent())

        // _isArray: instanceof Array — C++ parity: check for sequential integer keys
        LuaNative.doString(L, """
            function _isArray(v)
                if type(v) ~= "table" then return false end
                local n = #v
                if n == 0 then
                    return next(v) == nil
                end
                return true
            end
        """.trimIndent())

        // _indexOf: Array/String indexOf
        LuaNative.doString(L, """
            function _indexOf(obj, value)
                if type(obj) == "string" then
                    if type(value) ~= "string" then return -1 end
                    local pos = string.find(obj, value, 1, true)
                    if pos then return pos - 1 else return -1 end
                end
                if type(obj) == "table" then
                    for i = 1, #obj do
                        if obj[i] == value then return i - 1 end
                    end
                    return -1
                end
                return -1
            end
        """.trimIndent())

        // _concat: Array.concat
        LuaNative.doString(L, """
            function _concat(arr, ...)
                local result = {}
                if type(arr) == "table" then
                    for i = 1, #arr do result[#result + 1] = arr[i] end
                end
                for _, v in ipairs({...}) do
                    if type(v) == "table" then
                        for i = 1, #v do result[#result + 1] = v[i] end
                    else
                        result[#result + 1] = v
                    end
                end
                return result
            end
        """.trimIndent())

        // parseInt / parseFloat
        LuaNative.doString(L, """
            function parseInt(str, base)
                if str == nil then return 0 end
                str = tostring(str)
                str = str:match("^%s*(.-)%s*$")
                if base then return tonumber(str, base) or 0 end
                return math.floor(tonumber(str) or 0)
            end
            function parseFloat(str)
                if str == nil then return 0.0 end
                return tonumber(tostring(str)) or 0.0
            end
        """.trimIndent())

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

        // JSON support — C++ parity: sce/src/scripting/LuaEngine.cpp:630-727
        LuaNative.doString(L, """
            JSON = JSON or {}
            function JSON.stringify(v)
                if v == nil then return "null" end
                local t = type(v)
                if t == "boolean" then return tostring(v) end
                if t == "number" then return tostring(v) end
                if t == "string" then return '"' .. v:gsub('"', '\\"') .. '"' end
                if t == "table" then
                    local isArr = (#v > 0)
                    if isArr then
                        local parts = {}
                        for i = 1, #v do parts[i] = JSON.stringify(v[i]) end
                        return "[" .. table.concat(parts, ",") .. "]"
                    else
                        local parts = {}
                        for k, val2 in pairs(v) do
                            parts[#parts + 1] = '"' .. tostring(k) .. '":' .. JSON.stringify(val2)
                        end
                        return "{" .. table.concat(parts, ",") .. "}"
                    end
                end
                return "null"
            end
            function JSON.parse(s)
                if s == nil or s == "" then return nil end
                local strs = {}
                local buf = {}
                local i = 1
                while i <= #s do
                    if s:sub(i, i) == '"' then
                        local j = i + 1
                        while j <= #s do
                            if s:sub(j, j) == '\\' then j = j + 2
                            elseif s:sub(j, j) == '"' then break
                            else j = j + 1 end
                        end
                        strs[#strs+1] = s:sub(i+1, j-1)
                        buf[#buf+1] = '\x01' .. #strs .. '\x01'
                        i = j + 1
                    else
                        buf[#buf+1] = s:sub(i, i)
                        i = i + 1
                    end
                end
                local str = table.concat(buf)
                str = str:gsub('%[', '{'):gsub('%]', '}')
                str = str:gsub('\x01(%d+)\x01%s*:', function(idx)
                    return '["' .. strs[tonumber(idx)] .. '"]=';
                end)
                str = str:gsub(':true', '=true'):gsub(':false', '=false'):gsub(':null', '=nil')
                str = str:gsub('([=,{])%s*(%d)', '%1%2')
                str = str:gsub('\x01(%d+)\x01', function(idx)
                    return '"' .. strs[tonumber(idx)] .. '"'
                end)
                local fn = load("return " .. str, "json", "t", {})
                if fn then return fn() end
                return nil
            end
        """.trimIndent())

        // Object.keys (ECMAScript standard) — C++ parity
        LuaNative.doString(L, """
            Object = {}
            function Object.keys(t)
                if type(t) ~= "table" then return {} end
                local keys = {}
                for k, _ in pairs(t) do keys[#keys+1] = k end
                table.sort(keys, function(a,b)
                    if type(a) == type(b) then return tostring(a) < tostring(b) end
                    return type(a) < type(b)
                end)
                return keys
            end
        """.trimIndent())

        // W3C DOM: Register metatable once at session init, not per-element
        LuaNative.doString(L, DOM_METATABLE_SETUP)
    }

    /**
     * Update the In() predicate state table for a session.
     * Called by StateMachineEngine before processing events.
     */
    fun updateActiveStates(@Suppress("UNUSED_PARAMETER") sessionId: String, activeStateIds: Set<String>) {
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
            LuaNative.isInteger(L, index) -> LuaNative.toInteger(L, index)
            LuaNative.isNumber(L, index) -> LuaNative.toNumber(L, index)
            LuaNative.isString(L, index) -> LuaNative.toJString(L, index)
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

    private fun pushDOMObject(L: Long, xmlContent: String): Boolean {
        return try {
            val factory = DocumentBuilderFactory.newInstance()
            factory.isNamespaceAware = true
            val builder = factory.newDocumentBuilder()
            val doc = builder.parse(InputSource(StringReader(xmlContent)))
            pushDOMElement(L, doc.documentElement)
            true
        } catch (_: Exception) {
            false
        }
    }

    private fun pushDOMElement(L: Long, element: Element) {
        LuaNative.createTable(L, 0, 3)

        // Store attributes as a sub-table
        val attrs = element.attributes
        LuaNative.createTable(L, 0, attrs.length)
        for (i in 0 until attrs.length) {
            val attr = attrs.item(i)
            LuaNative.pushString(L, attr.nodeValue ?: "")
            LuaNative.setField(L, -2, attr.nodeName)
        }
        LuaNative.setField(L, -2, "__attrs")

        // Store tag name
        LuaNative.pushString(L, element.tagName)
        LuaNative.setField(L, -2, "__tagName")

        // Store child elements grouped by tag name
        val childNodes = element.childNodes
        val tagGroups = mutableMapOf<String, MutableList<Element>>()
        for (i in 0 until childNodes.length) {
            val child = childNodes.item(i)
            if (child is Element) {
                tagGroups.getOrPut(child.tagName) { mutableListOf() }.add(child)
            }
        }
        LuaNative.createTable(L, 0, tagGroups.size)
        for ((tag, elements) in tagGroups) {
            LuaNative.createTable(L, elements.size, 0)
            for (j in elements.indices) {
                pushDOMElement(L, elements[j])
                LuaNative.rawSetI(L, -2, (j + 1).toLong())
            }
            LuaNative.setField(L, -2, tag)
        }
        LuaNative.setField(L, -2, "__children")

        // Metatable registered at session init; apply to this element
        LuaNative.getGlobal(L, "__sce_dom_mt")
        LuaNative.setMetatable(L, -2)
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
        private val BUILTINS = setOf(
            "true", "false", "nil", "math", "string", "table", "type",
            "tostring", "tonumber", "pairs", "ipairs", "print",
            "In", "_scxml_truthy", "_typeof", "_isArray", "_indexOf", "_concat",
            "parseInt", "parseFloat", "JSON", "_NULL", "_UNDEFINED", "Object", "debug"
        )

        // DOM metatable — C++ parity: getElementsByTagName searches entire subtree
        private val DOM_METATABLE_SETUP = """
            __sce_dom_mt = {}
            local function collectByTagName(elem, tagName, result)
                local children = rawget(elem, "__children")
                if not children then return end
                for tag, elems in pairs(children) do
                    for i = 1, #elems do
                        if not getmetatable(elems[i]) then
                            setmetatable(elems[i], __sce_dom_mt)
                        end
                        if tag == tagName then
                            result[#result+1] = elems[i]
                        end
                        collectByTagName(elems[i], tagName, result)
                    end
                end
            end
            __sce_dom_mt.__index = {
                getElementsByTagName = function(self, tagName)
                    local result = {}
                    collectByTagName(self, tagName, result)
                    return result
                end,
                getAttribute = function(self, attrName)
                    local attrs = rawget(self, "__attrs")
                    if attrs then return attrs[attrName] or "" end
                    return ""
                end,
                getTagName = function(self)
                    return rawget(self, "__tagName") or ""
                end
            }
        """.trimIndent()
    }
}
