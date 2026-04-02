// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin — Rhino-based ECMAScript engine for W3C SCXML datamodel evaluation

package com.sce.scripting

import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import org.mozilla.javascript.Context
import org.mozilla.javascript.ScriptableObject
import org.mozilla.javascript.Scriptable
import org.mozilla.javascript.Undefined

/**
 * JVM ECMAScript engine using Mozilla Rhino.
 *
 * Each session gets its own Rhino scope (variable isolation).
 * Thread safety: one session is accessed from one coroutine (microstep loop),
 * but Rhino Context is thread-local, so we enter/exit per call.
 *
 * Used for W3C SCXML conformance tests on JVM.
 * For AOSP/AAOS production, replace with QuickJS via JNI/NDK.
 */
class RhinoScriptEngine : ScxmlScriptEngine {

    private data class Session(
        val scope: ScriptableObject,
        var stateQueryCallback: ((String) -> Boolean)? = null
    )

    private val sessions = mutableMapOf<String, Session>()

    override fun createSession(sessionId: String) {
        val cx = Context.enter()
        try {
            // ES5 mode avoids Rhino-specific optimizations that break standard behavior
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1  // Interpreted mode (safe for Android compatibility)
            val scope = cx.initStandardObjects()
            sessions[sessionId] = Session(scope)
        } finally {
            Context.exit()
        }
    }

    override fun destroySession(sessionId: String) {
        sessions.remove(sessionId)
    }

    override fun setupSystemVariables(sessionId: String, machineName: String) {
        val session = sessions[sessionId] ?: return
        val cx = Context.enter()
        try {
            val scope = session.scope

            // W3C SCXML 5.10: _sessionid
            ScriptableObject.putProperty(scope, "_sessionid", sessionId)

            // W3C SCXML 5.10: _name
            ScriptableObject.putProperty(scope, "_name", machineName)

            // W3C SCXML 5.10: _ioprocessors
            val ioProc = cx.newObject(scope)
            val scxmlProc = cx.newObject(scope)
            ScriptableObject.putProperty(scxmlProc, "location", sessionId)
            ScriptableObject.putProperty(ioProc, "scxml", scxmlProc)
            ScriptableObject.putProperty(scope, "_ioprocessors", ioProc)

            // W3C SCXML 5.9.2: Register In() function
            registerInFunction(scope, session)
        } finally {
            Context.exit()
        }
    }

    override fun evaluateCondition(sessionId: String, expr: String): Boolean {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")
        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            val result = cx.evaluateString(session.scope, expr, "cond", 1, null)
            return toBoolean(result)
        } catch (e: Exception) {
            throw ScriptEngineException("Guard evaluation failed: '$expr'", e)
        } finally {
            Context.exit()
        }
    }

    override fun evaluateExpr(sessionId: String, expr: String): Any? {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")
        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            val result = cx.evaluateString(session.scope, expr, "expr", 1, null)
            return unwrap(result)
        } catch (e: Exception) {
            throw ScriptEngineException("Expression evaluation failed: '$expr'", e)
        } finally {
            Context.exit()
        }
    }

    override fun executeScript(sessionId: String, script: String) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")
        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            cx.evaluateString(session.scope, script, "script", 1, null)
        } catch (e: Exception) {
            throw ScriptEngineException("Script execution failed", e)
        } finally {
            Context.exit()
        }
    }

    override fun setVariable(sessionId: String, name: String, value: Any?) {
        val session = sessions[sessionId] ?: return
        Context.enter()
        try {
            val wrapped = when (value) {
                null -> Undefined.instance
                is Scriptable -> value
                else -> Context.javaToJS(value, session.scope)
            }
            ScriptableObject.putProperty(session.scope, name, wrapped)
        } finally {
            Context.exit()
        }
    }

    override fun getVariable(sessionId: String, name: String): Any? {
        val session = sessions[sessionId] ?: return null
        Context.enter()
        try {
            val result = ScriptableObject.getProperty(session.scope, name)
            return unwrap(result)
        } finally {
            Context.exit()
        }
    }

    override fun hasVariable(sessionId: String, name: String): Boolean {
        val session = sessions[sessionId] ?: return false
        Context.enter()
        try {
            return ScriptableObject.hasProperty(session.scope, name)
        } finally {
            Context.exit()
        }
    }

    override fun assign(sessionId: String, location: String, expr: String) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        // W3C SCXML B.2: System variables are read-only
        if (location.startsWith("_")) {
            throw ScriptEngineException("Cannot assign to system variable: $location")
        }

        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            // Evaluate expression and assign to location
            val result = cx.evaluateString(session.scope, expr, "assign", 1, null)
            ScriptableObject.putProperty(session.scope, location, result)
        } catch (e: ScriptEngineException) {
            throw e
        } catch (e: Exception) {
            throw ScriptEngineException("Assignment failed: $location = $expr", e)
        } finally {
            Context.exit()
        }
    }

    override fun setCurrentEvent(
        sessionId: String,
        name: String,
        data: String,
        type: String,
        sendId: String,
        origin: String,
        originType: String,
        invokeId: String
    ) {
        val session = sessions[sessionId] ?: return
        val cx = Context.enter()
        try {
            val scope = session.scope
            val eventObj = cx.newObject(scope)
            ScriptableObject.putProperty(eventObj, "name", name)
            ScriptableObject.putProperty(eventObj, "type", type.ifEmpty { "external" })
            ScriptableObject.putProperty(eventObj, "sendid", sendId)
            ScriptableObject.putProperty(eventObj, "origin", origin)
            ScriptableObject.putProperty(eventObj, "origintype", originType)
            ScriptableObject.putProperty(eventObj, "invokeid", invokeId)

            // W3C SCXML 5.10: _event.data — try to parse as structured data
            if (data.isNotEmpty()) {
                // Try to evaluate as JS expression (handles JSON objects, arrays, primitives)
                try {
                    cx.languageVersion = Context.VERSION_ES6
                    cx.optimizationLevel = -1
                    val parsed = cx.evaluateString(scope, "($data)", "event-data", 1, null)
                    ScriptableObject.putProperty(eventObj, "data", parsed)
                } catch (_: Exception) {
                    // Fall back to raw string
                    ScriptableObject.putProperty(eventObj, "data", data)
                }
            } else {
                ScriptableObject.putProperty(eventObj, "data", Undefined.instance)
            }

            ScriptableObject.putProperty(scope, "_event", eventObj)
        } finally {
            Context.exit()
        }
    }

    override fun clearCurrentEvent(sessionId: String) {
        val session = sessions[sessionId] ?: return
        Context.enter()
        try {
            ScriptableObject.putProperty(session.scope, "_event", Undefined.instance)
        } finally {
            Context.exit()
        }
    }

    override fun setStateQueryCallback(sessionId: String, callback: ((String) -> Boolean)?) {
        sessions[sessionId]?.stateQueryCallback = callback
        // Re-register the In() function with updated callback
        if (callback != null) {
            val session = sessions[sessionId] ?: return
            Context.enter()
            try {
                registerInFunction(session.scope, session)
            } finally {
                Context.exit()
            }
        }
    }

    override fun executeForeach(
        sessionId: String,
        array: String,
        item: String,
        index: String,
        body: () -> Unit
    ) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")
        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1

            // Evaluate array expression
            val arrayResult = cx.evaluateString(session.scope, array, "foreach-array", 1, null)

            // W3C SCXML 4.6: array must be iterable (NativeArray or have length)
            val length: Int
            val arrayObj: Scriptable
            if (arrayResult is Scriptable) {
                arrayObj = arrayResult
                val lenProp = ScriptableObject.getProperty(arrayObj, "length")
                length = when (lenProp) {
                    is Number -> lenProp.toInt()
                    else -> throw ScriptEngineException("Foreach array has no length: $array")
                }
            } else {
                throw ScriptEngineException("Foreach expression is not an array: $array")
            }

            for (i in 0 until length) {
                // Set item variable
                val element = ScriptableObject.getProperty(arrayObj, i)
                ScriptableObject.putProperty(session.scope, item, element)

                // Set index variable (if specified)
                if (index.isNotEmpty()) {
                    ScriptableObject.putProperty(session.scope, index, i)
                }

                // Execute body actions
                body()
            }
        } catch (e: ScriptEngineException) {
            throw e
        } catch (e: Exception) {
            throw ScriptEngineException("Foreach execution failed for array: $array", e)
        } finally {
            Context.exit()
        }
    }

    // --- Private Helpers ---

    /**
     * Register the SCXML In() predicate as a JavaScript function.
     *
     * W3C SCXML 5.9.2: In(stateId) returns true if stateId is in
     * the current active configuration.
     */
    private fun registerInFunction(scope: ScriptableObject, session: Session) {
        val inFunc = object : org.mozilla.javascript.BaseFunction() {
            override fun call(
                cx: Context,
                scope: Scriptable,
                thisObj: Scriptable,
                args: Array<out Any?>
            ): Any {
                if (args.isEmpty()) return false
                val stateId = Context.toString(args[0])
                return session.stateQueryCallback?.invoke(stateId) ?: false
            }

            override fun getArity(): Int = 1
            override fun getFunctionName(): String = "In"
        }
        ScriptableObject.putProperty(scope, "In", inFunc)
    }

    /**
     * Convert Rhino result to Java boolean.
     *
     * W3C SCXML 5.9: ECMAScript truthiness rules.
     */
    private fun toBoolean(value: Any?): Boolean {
        return when (value) {
            null -> false
            is Boolean -> value
            is Undefined -> false
            is Number -> value.toDouble() != 0.0 && !value.toDouble().isNaN()
            is String -> value.isNotEmpty()
            else -> true
        }
    }

    /**
     * Unwrap Rhino internal types to Java equivalents.
     */
    private fun unwrap(value: Any?): Any? {
        return when (value) {
            is Undefined -> null
            Scriptable.NOT_FOUND -> null
            is org.mozilla.javascript.Wrapper -> value.unwrap()
            else -> value
        }
    }
}
