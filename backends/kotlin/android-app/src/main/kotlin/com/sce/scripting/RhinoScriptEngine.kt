// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Android — Rhino-based ECMAScript engine for W3C SCXML datamodel evaluation
//
// Copy of backends/kotlin/tests/src/main/kotlin/com/sce/scripting/RhinoScriptEngine.kt
// Reason: sce-kotlin-tests contains 247 generated state machines that would bloat the APK.
// Keep in sync with the original when modifying Rhino engine behavior.

package com.sce.scripting

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.mozilla.javascript.Context
import org.mozilla.javascript.ScriptableObject
import org.mozilla.javascript.Scriptable
import org.mozilla.javascript.Undefined
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element
import org.w3c.dom.Document
import org.xml.sax.InputSource
import java.io.StringReader

/**
 * JVM/Android ECMAScript engine using Mozilla Rhino.
 *
 * Each session gets its own Rhino scope (variable isolation).
 * Thread safety: one session is accessed from one coroutine (microstep loop),
 * but Rhino Context is thread-local, so we enter/exit per call.
 *
 * On Android, runs in interpreted mode (optimizationLevel = -1) to avoid
 * Rhino's bytecode generation which conflicts with Android's DEX format.
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
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1  // Interpreted mode (required for Android)
            val scope = cx.initStandardObjects()
            sessions[sessionId] = Session(scope)
        } finally {
            Context.exit()
        }
    }

    override fun destroySession(sessionId: String) {
        sessions.remove(sessionId)
    }

    override fun setupSystemVariables(
        sessionId: String,
        machineName: String,
        ioProcessors: List<IoProcessorDescriptor>,
    ) {
        val session = sessions[sessionId] ?: return
        val cx = Context.enter()
        try {
            val scope = session.scope

            // W3C SCXML 5.10: _sessionid
            ScriptableObject.putProperty(scope, "_sessionid", sessionId)

            // W3C SCXML 5.10: _name
            ScriptableObject.putProperty(scope, "_name", machineName)

            // §scxml-C-1-1 / §scxml-C-2-3: one entry per processor the
            // deployment supports, each with a 'location' field. Names and
            // locations are decided by IoProcessors.build, so this engine's
            // view of `_ioprocessors` matches every other backend's.
            val ioProc = cx.newObject(scope)
            for (processor in ioProcessors) {
                val entry = cx.newObject(scope)
                ScriptableObject.putProperty(entry, "location", processor.location)
                ScriptableObject.putProperty(ioProc, processor.name, entry)
            }
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

        if (location.startsWith("_")) {
            throw ScriptEngineException("Cannot assign to system variable: $location")
        }

        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1

            if (isSystemVariableReference(expr)) {
                cx.evaluateString(session.scope, "$location = $expr;", "assign", 1, null)
            } else if (isSimpleVariableName(location)) {
                val result = cx.evaluateString(session.scope, expr, "assign", 1, null)
                ScriptableObject.putProperty(session.scope, location, result)
            } else {
                cx.evaluateString(session.scope, "$location = ($expr);", "assign", 1, null)
            }
        } catch (e: ScriptEngineException) {
            throw e
        } catch (e: Exception) {
            throw ScriptEngineException("Assignment failed: $location = $expr", e)
        } finally {
            Context.exit()
        }
    }

    private fun isSystemVariableReference(expr: String): Boolean {
        return expr == "_sessionid" || expr == "_event" || expr == "_name" ||
               expr == "_ioprocessors" || expr == "_x"
    }

    private fun isSimpleVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        val first = name[0]
        if (!first.isLetter() && first != '_') return false
        return name.all { it.isLetterOrDigit() || it == '_' }
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs) {
        val session = sessions[sessionId] ?: return
        val cx = Context.enter()
        try {
            val scope = session.scope
            val eventObj = cx.newObject(scope)
            ScriptableObject.putProperty(eventObj, "name", args.name)
            ScriptableObject.putProperty(eventObj, "type", args.type.ifEmpty { "external" })
            ScriptableObject.putProperty(eventObj, "sendid", args.sendId)
            ScriptableObject.putProperty(eventObj, "origin", args.origin)
            ScriptableObject.putProperty(eventObj, "origintype", args.originType)
            ScriptableObject.putProperty(eventObj, "invokeid", args.invokeId)

            if (args.data.isNotEmpty()) {
                val parsed = parseDataValueInternal(cx, scope, args.data)
                ScriptableObject.putProperty(eventObj, "data", parsed ?: Undefined.instance)
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

        if (!isLegalVariableName(item)) {
            throw ScriptEngineException("Illegal foreach item variable name: '$item'")
        }
        if (index.isNotEmpty() && !isLegalVariableName(index)) {
            throw ScriptEngineException("Illegal foreach index variable name: '$index'")
        }

        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1

            val arrayResult = cx.evaluateString(session.scope, array, "foreach-array", 1, null)

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
                val element = ScriptableObject.getProperty(arrayObj, i)
                ScriptableObject.putProperty(session.scope, item, element)

                if (index.isNotEmpty()) {
                    ScriptableObject.putProperty(session.scope, index, i)
                }

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
        val cx = Context.enter()
        try {
            return parseDataValueInternal(cx, session.scope, data)
        } finally {
            Context.exit()
        }
    }

    private fun isLegalVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        if (name.first() == '\'' || name.first() == '"') return false
        if (!name.first().isLetter() && name.first() != '_' && name.first() != '$') return false
        return name.all { it.isLetterOrDigit() || it == '_' || it == '$' }
    }

    private fun parseDataValueInternal(cx: Context, scope: ScriptableObject, data: String): Any? {
        val firstNonWhitespace = data.indexOfFirst { it != ' ' && it != '\t' && it != '\r' && it != '\n' }
        if (firstNonWhitespace >= 0 && data[firstNonWhitespace] == '<') {
            val domObj = createRhinoDOMObject(cx, scope, data)
            if (domObj != null) return domObj
        }

        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            ScriptableObject.putProperty(scope, "__sce_tmp_data__", data)
            val parsed = cx.evaluateString(scope, "JSON.parse(__sce_tmp_data__)", "json-parse", 1, null)
            ScriptableObject.deleteProperty(scope, "__sce_tmp_data__")
            if (parsed != null && parsed !is Undefined) return parsed
        } catch (_: Exception) {
            ScriptableObject.deleteProperty(scope, "__sce_tmp_data__")
        }

        return normalizeWhitespace(data)
    }

    private fun createRhinoDOMObject(cx: Context, scope: ScriptableObject, xmlContent: String): Scriptable? {
        return try {
            val factory = DocumentBuilderFactory.newInstance()
            factory.isNamespaceAware = true
            val builder = factory.newDocumentBuilder()
            val doc = builder.parse(InputSource(StringReader(xmlContent)))
            val docElement = doc.documentElement
            createRhinoElementWrapper(cx, scope, doc, docElement)
        } catch (_: Exception) {
            null
        }
    }

    private fun createRhinoElementWrapper(
        cx: Context,
        topScope: ScriptableObject,
        doc: Document?,
        element: Element
    ): Scriptable {
        val obj = cx.newObject(topScope)

        val getElementsFunc = object : org.mozilla.javascript.BaseFunction() {
            override fun call(
                cx: Context, scope: Scriptable, thisObj: Scriptable, args: Array<out Any?>
            ): Any {
                if (args.isEmpty()) return cx.newArray(topScope, 0)
                val tagName = Context.toString(args[0])
                val nodeList = if (doc != null) {
                    doc.getElementsByTagName(tagName)
                } else {
                    element.getElementsByTagName(tagName)
                }
                val arr = arrayOfNulls<Any>(nodeList.length)
                for (i in 0 until nodeList.length) {
                    arr[i] = createRhinoElementWrapper(cx, topScope, null, nodeList.item(i) as Element)
                }
                return cx.newArray(topScope, arr)
            }
            override fun getArity(): Int = 1
            override fun getFunctionName(): String = "getElementsByTagName"
        }
        ScriptableObject.putProperty(obj, "getElementsByTagName", getElementsFunc)

        val getAttrFunc = object : org.mozilla.javascript.BaseFunction() {
            override fun call(
                cx: Context, scope: Scriptable, thisObj: Scriptable, args: Array<out Any?>
            ): Any {
                if (args.isEmpty()) return ""
                val attrName = Context.toString(args[0])
                return element.getAttribute(attrName) ?: ""
            }
            override fun getArity(): Int = 1
            override fun getFunctionName(): String = "getAttribute"
        }
        ScriptableObject.putProperty(obj, "getAttribute", getAttrFunc)

        return obj
    }

    private fun normalizeWhitespace(data: String): String {
        val normalized = StringBuilder(data.length)
        var inWhitespace = false
        var hasContent = false

        for (c in data) {
            if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
                if (hasContent && !inWhitespace) {
                    normalized.append(' ')
                    inWhitespace = true
                }
            } else {
                normalized.append(c)
                inWhitespace = false
                hasContent = true
            }
        }

        if (normalized.isNotEmpty() && normalized.last() == ' ') {
            normalized.deleteCharAt(normalized.length - 1)
        }

        return normalized.toString()
    }

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

    private fun unwrap(value: Any?): Any? {
        return when (value) {
            is Undefined -> null
            Scriptable.NOT_FOUND -> null
            is org.mozilla.javascript.Wrapper -> value.unwrap()
            else -> value
        }
    }
}
