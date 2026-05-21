// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin QuickJS — QuickJS implementation of ScxmlScriptEngine
//
// Native ECMAScript engine for W3C SCXML datamodel evaluation.
// Unlike the Lua engine, no expression transformation is needed — QuickJS
// evaluates ECMAScript natively. This makes the implementation significantly
// simpler: expressions from SCXML pass directly to JS_Eval.
//
// C++ parity: targets same W3C SCXML compliance as Rhino/Lua engines

package com.sce.scripting.quickjs

import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import javax.xml.parsers.DocumentBuilderFactory
import org.w3c.dom.Element
import org.xml.sax.InputSource
import java.io.StringReader

/**
 * QuickJS ECMAScript engine for W3C SCXML datamodel evaluation.
 *
 * Each session gets its own JSRuntime + JSContext (full isolation).
 * Native ECMAScript support means no expression transformation pipeline.
 *
 * For AOSP/AAOS production: lightweight, embeddable, no JVM dependency.
 */
class QuickJSScriptEngine : ScxmlScriptEngine {

    /**
     * Opaque handle for JS values (objects, functions, DOM elements) that cannot
     * survive Kotlin round-trip. Stored in JS-side __sce_refs registry.
     *
     * `originHandle` records the QuickJS context the ref was created in so that
     * cross-session reuse (`sessions["A"]` → `sessions["B"].setVariable(...)`)
     * can be rejected; the `__sce_refs` registry is per-context, so reading
     * `__sce_refs[refId]` from a different context would silently return the
     * wrong value (or undefined).
     */
    private data class QuickJSRef(val refId: Int, val originHandle: Long)

    private data class Session(
        val handle: Long,  // QJSSession pointer
        var stateQueryCallback: ((String) -> Boolean)? = null,
        val declaredVars: MutableSet<String> = mutableSetOf()
    )

    private val sessions = mutableMapOf<String, Session>()

    override fun createSession(sessionId: String) {
        val handle = QuickJSNative.createContext()
        if (handle == 0L) throw ScriptEngineException("Failed to create QuickJS context")
        sessions[sessionId] = Session(handle)
        registerBuiltins(handle)
    }

    override fun destroySession(sessionId: String) {
        val session = sessions.remove(sessionId) ?: return
        QuickJSNative.destroyContext(session.handle)
    }

    override fun setupSystemVariables(sessionId: String, machineName: String) {
        val session = sessions[sessionId] ?: return
        val handle = session.handle

        // W3C SCXML 5.10: _sessionid
        QuickJSNative.setGlobalString(handle, "_sessionid", sessionId)

        // W3C SCXML 5.10: _name
        QuickJSNative.setGlobalString(handle, "_name", machineName)

        // W3C SCXML 5.10: _ioprocessors
        QuickJSNative.eval(handle,
            "_ioprocessors = { scxml: { location: ${jsStringLiteral(sessionId)} } }")

        session.declaredVars.addAll(listOf("_sessionid", "_name", "_ioprocessors", "_event"))
    }

    override fun evaluateCondition(sessionId: String, expr: String): Boolean {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val result = QuickJSNative.evalToBoolean(session.handle, cleanExpr(expr))
        if (result < 0) {
            val err = QuickJSNative.getLastError(session.handle) ?: "unknown error"
            throw ScriptEngineException("Guard evaluation failed: '$expr' ($err)")
        }
        return result == 1
    }

    override fun evaluateExpr(sessionId: String, expr: String): Any? {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        // W3C SCXML compliance: undeclared variable access throws ReferenceError
        val trimmed = expr.trim()
        if (isUndeclaredSimpleVariable(trimmed, session)) {
            throw ScriptEngineException("ReferenceError: $expr is not defined")
        }

        val cleaned = cleanExpr(expr)

        // Try wrapped in parens — forces expression context for function expressions,
        // object literals, etc. that would otherwise be parsed as statements.
        val result = QuickJSNative.evalExpression(session.handle, "(\n$cleaned\n)")
        if (result != null) return decodeTypedResult(result, session)

        // Try without wrapping (for edge cases like multi-statement expressions)
        val result2 = QuickJSNative.evalExpression(session.handle, cleaned)
        if (result2 != null) return decodeTypedResult(result2, session)

        // Capture error immediately — before assignment fallback overwrites it
        val err = QuickJSNative.getLastError(session.handle)

        // Fallback: try as assignment statement (e.g., "x = 5")
        if ('=' in cleaned && "==" !in cleaned && "!=" !in cleaned &&
            "<=" !in cleaned && ">=" !in cleaned && "===" !in cleaned && "!==" !in cleaned) {
            val assignErr = QuickJSNative.eval(session.handle, cleaned)
            if (assignErr == null) return null
        }

        throw ScriptEngineException("Expression evaluation failed: '$expr' ($err)")
    }

    override fun executeScript(sessionId: String, script: String) {
        val session = sessions[sessionId]
            ?: throw ScriptEngineException("Session not found: $sessionId")

        val err = QuickJSNative.eval(session.handle, script)
        if (err != null) {
            throw ScriptEngineException("Script execution failed: $err")
        }
    }

    override fun setVariable(sessionId: String, name: String, value: Any?) {
        val session = sessions[sessionId] ?: return
        val handle = session.handle

        // Validate variable name before interpolation into JS code (injection guard)
        fun requireSafeName() {
            if (!isSimpleVariableName(name))
                throw ScriptEngineException("Illegal variable name: '$name'")
        }

        when (value) {
            null -> QuickJSNative.setGlobalUndefined(handle, name)
            is Boolean -> QuickJSNative.setGlobalBoolean(handle, name, value)
            is Int -> QuickJSNative.setGlobalInt(handle, name, value.toLong())
            is Long -> QuickJSNative.setGlobalInt(handle, name, value)
            is Double -> QuickJSNative.setGlobalDouble(handle, name, value)
            is Float -> QuickJSNative.setGlobalDouble(handle, name, value.toDouble())
            is String -> QuickJSNative.setGlobalString(handle, name, value)
            is QuickJSRef -> {
                requireSafeName()
                if (value.originHandle != handle) {
                    throw ScriptEngineException(
                        "Cross-session QuickJSRef rejected: refId=${value.refId} " +
                            "origin=0x${value.originHandle.toString(16)} " +
                            "target=0x${handle.toString(16)}")
                }
                // Restore from JS-side registry and release the ref
                QuickJSNative.eval(handle,
                    "$name = __sce_refs[${value.refId}]; delete __sce_refs[${value.refId}]")
            }
            is List<*> -> {
                requireSafeName()
                val json = toJSON(value)
                QuickJSNative.eval(handle, "$name = JSON.parse(${jsStringLiteral(json)})")
            }
            is Map<*, *> -> {
                requireSafeName()
                val json = toJSON(value)
                QuickJSNative.eval(handle, "$name = JSON.parse(${jsStringLiteral(json)})")
            }
            else -> QuickJSNative.setGlobalString(handle, name, value.toString())
        }
        session.declaredVars.add(name)
    }

    override fun getVariable(sessionId: String, name: String): Any? {
        val session = sessions[sessionId] ?: return null
        if (!isSimpleVariableName(name)) return null
        val result = QuickJSNative.evalExpression(session.handle, name)
        return if (result != null) decodeTypedResult(result, session) else null
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

        // Validate location is a legal variable path (injection guard)
        val rootVar = location.split('.')[0]
        if (!isSimpleVariableName(rootVar)) {
            throw ScriptEngineException("Illegal assignment location: '$location'")
        }

        // Native JS assignment — handles simple and complex paths (a.b.c) uniformly
        val cleaned = cleanExpr(expr)
        val err = QuickJSNative.eval(session.handle, "$location = ($cleaned)")
        if (err != null) {
            throw ScriptEngineException("Assignment failed: $location = $expr ($err)")
        }
        session.declaredVars.add(rootVar)
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs) {
        val session = sessions[sessionId] ?: return
        val handle = session.handle
        val typeStr = args.type.ifEmpty { "external" }

        // Build _event object with basic properties
        QuickJSNative.eval(handle, """
            _event = {
                name: ${jsStringLiteral(args.name)},
                type: ${jsStringLiteral(typeStr)},
                sendid: ${jsStringLiteral(args.sendId)},
                origin: ${jsStringLiteral(args.origin)},
                origintype: ${jsStringLiteral(args.originType)},
                invokeid: ${jsStringLiteral(args.invokeId)},
                data: undefined
            }
        """.trimIndent())

        // Parse and set data if present
        if (args.data.isNotEmpty()) {
            setEventData(handle, args.data)
        }
    }

    override fun clearCurrentEvent(sessionId: String) {
        val session = sessions[sessionId] ?: return
        QuickJSNative.setGlobalUndefined(session.handle, "_event")
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

        val handle = session.handle

        // Evaluate array once, store in temp variable
        val cleanedArray = cleanExpr(array)
        val arrErr = QuickJSNative.eval(handle, "__sce_foreach = ($cleanedArray)")
        if (arrErr != null) {
            throw ScriptEngineException("Foreach array evaluation failed: $array ($arrErr)")
        }

        // Verify array-like (has numeric length property)
        val check = QuickJSNative.evalToBoolean(handle,
            "typeof __sce_foreach === 'object' && __sce_foreach !== null && " +
                "typeof __sce_foreach.length === 'number'")
        if (check != 1) {
            QuickJSNative.eval(handle, "delete __sce_foreach")
            throw ScriptEngineException("Foreach expression is not an array: $array")
        }

        // Get array length
        val lengthResult = QuickJSNative.evalExpression(handle, "__sce_foreach.length")
        val length = decodeIntResult(lengthResult)
            ?: run {
                QuickJSNative.eval(handle, "delete __sce_foreach")
                throw ScriptEngineException("Foreach array has no valid length: $array")
            }

        // W3C SCXML 4.6: foreach auto-declares item/index variables
        session.declaredVars.add(item)
        if (index.isNotEmpty()) session.declaredVars.add(index)

        for (i in 0 until length) {
            QuickJSNative.eval(handle, "$item = __sce_foreach[$i]")
            if (index.isNotEmpty()) {
                QuickJSNative.eval(handle, "$index = $i")
            }
            body()
        }

        QuickJSNative.eval(handle, "delete __sce_foreach")
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
        val handle = session.handle

        // W3C SCXML B.2: Detection order — XML, JS expression, JSON, string

        // Step 1: XML DOM
        val firstNonWs = data.indexOfFirst { !it.isWhitespace() }
        if (firstNonWs >= 0 && data[firstNonWs] == '<') {
            val domExpr = buildDOMExpression(data)
            if (domExpr != null) {
                val err = QuickJSNative.eval(handle, "__sce_parse_tmp = $domExpr")
                if (err == null) {
                    val result = QuickJSNative.evalExpression(handle, "__sce_parse_tmp")
                    QuickJSNative.eval(handle, "delete __sce_parse_tmp")
                    if (result != null) return decodeTypedResult(result, session)
                }
            }
        }

        // Step 2: Try as JS expression (handles literals, variables, object expressions)
        val result1 = QuickJSNative.evalExpression(handle, data)
        if (result1 != null) return decodeTypedResult(result1, session)

        // Step 3: Try JSON.parse
        val result2 = QuickJSNative.evalExpression(handle,
            "JSON.parse(${jsStringLiteral(data)})")
        if (result2 != null) return decodeTypedResult(result2, session)

        // Step 4: Space-normalized string (W3C SCXML B.2, test 562)
        return normalizeWhitespace(data)
    }

    /**
     * Update the In() predicate state table for a session.
     * Called by StateMachineEngine before processing events.
     */
    fun updateActiveStates(
        sessionId: String,
        activeStateIds: Set<String>
    ) {
        val session = sessions[sessionId] ?: return
        val entries = activeStateIds.joinToString(",") { "${jsStringLiteral(it)}:true" }
        QuickJSNative.eval(session.handle, "__sce_active_states = {$entries}")
    }

    // === Private Helpers ===

    private fun registerBuiltins(handle: Long) {
        // Reference registry for complex values (functions, DOM objects)
        var err = QuickJSNative.eval(handle, "var __sce_refs = {}")
        if (err != null) throw ScriptEngineException("Failed to initialize ref registry: $err")

        // W3C SCXML 5.9.2: In() predicate via lookup table
        err = QuickJSNative.eval(handle, """
            var __sce_active_states = {};
            function In(stateId) { return __sce_active_states[stateId] === true; }
        """.trimIndent())
        if (err != null) throw ScriptEngineException("Failed to initialize In() predicate: $err")

        // W3C DOM: Prototype and factory for XML element objects
        err = QuickJSNative.eval(handle, DOM_PROTO_SETUP)
        if (err != null) throw ScriptEngineException("Failed to initialize DOM support: $err")
    }

    /**
     * Parse and set _event.data from raw string.
     * Detection order: XML DOM, JS expression, JSON, normalized string.
     */
    private fun setEventData(handle: Long, data: String) {
        // Step 1: XML DOM
        val firstNonWs = data.indexOfFirst { !it.isWhitespace() }
        if (firstNonWs >= 0 && data[firstNonWs] == '<') {
            val domExpr = buildDOMExpression(data)
            if (domExpr != null) {
                val err = QuickJSNative.eval(handle, "_event.data = $domExpr")
                if (err == null) return
            }
        }

        // Step 2: Try as JS expression
        val err1 = QuickJSNative.eval(handle, "_event.data = ($data)")
        if (err1 == null) return

        // Step 3: Try JSON
        val err2 = QuickJSNative.eval(handle,
            "_event.data = JSON.parse(${jsStringLiteral(data)})")
        if (err2 == null) return

        // Step 4: Normalized string
        QuickJSNative.eval(handle,
            "_event.data = ${jsStringLiteral(normalizeWhitespace(data))}")
    }

    /**
     * Decode typed result string from JNI evalExpression.
     * Protocol: U=undefined, N=null, T=true, F=false, I<int>, D<double>, S<str>, R<ref>
     */
    private fun decodeTypedResult(encoded: String, session: Session): Any? {
        if (encoded.isEmpty()) return null
        return when (encoded[0]) {
            'U' -> null
            'N' -> null
            'T' -> true
            'F' -> false
            'I' -> encoded.substring(1).toLongOrNull()
            'D' -> encoded.substring(1).toDoubleOrNull()
            'S' -> encoded.substring(1)
            'R' -> {
                val refId = encoded.substring(1).toIntOrNull() ?: return null
                QuickJSRef(refId, session.handle)
            }
            else -> null
        }
    }

    private fun decodeIntResult(encoded: String?): Long? {
        if (encoded == null || encoded.isEmpty()) return null
        return when (encoded[0]) {
            'I' -> encoded.substring(1).toLongOrNull()
            'D' -> encoded.substring(1).toDoubleOrNull()?.toLong()
            else -> null
        }
    }

    // === DOM Support ===

    /**
     * Parse XML content and build a JS expression that creates a DOM-like object tree.
     * Returns null if XML parsing fails.
     */
    private fun buildDOMExpression(xmlContent: String): String? {
        return try {
            val factory = DocumentBuilderFactory.newInstance()
            factory.isNamespaceAware = true
            val builder = factory.newDocumentBuilder()
            val doc = builder.parse(InputSource(StringReader(xmlContent)))
            buildElementExpression(doc.documentElement)
        } catch (_: Exception) {
            null
        }
    }

    /**
     * Recursively build a JS expression for a DOM element.
     * Output: __sce_dom_create({__tagName:"...",__attrs:{...},__children:{...}})
     */
    private fun buildElementExpression(element: Element): String {
        val sb = StringBuilder()
        sb.append("__sce_dom_create({")

        // __tagName
        sb.append("__tagName:").append(jsStringLiteral(element.tagName)).append(",")

        // __attrs
        sb.append("__attrs:{")
        val attrs = element.attributes
        for (i in 0 until attrs.length) {
            if (i > 0) sb.append(",")
            val attr = attrs.item(i)
            sb.append(jsStringLiteral(attr.nodeName))
                .append(":")
                .append(jsStringLiteral(attr.nodeValue ?: ""))
        }
        sb.append("},")

        // __children grouped by tag name
        val childNodes = element.childNodes
        val tagGroups = mutableMapOf<String, MutableList<Element>>()
        for (i in 0 until childNodes.length) {
            val child = childNodes.item(i)
            if (child is Element) {
                tagGroups.getOrPut(child.tagName) { mutableListOf() }.add(child)
            }
        }

        sb.append("__children:{")
        var first = true
        for ((tag, elements) in tagGroups) {
            if (!first) sb.append(",")
            first = false
            sb.append(jsStringLiteral(tag)).append(":[")
            for (j in elements.indices) {
                if (j > 0) sb.append(",")
                sb.append(buildElementExpression(elements[j]))
            }
            sb.append("]")
        }
        sb.append("}")

        sb.append("})")
        return sb.toString()
    }

    // === Utility Functions ===

    /**
     * Strip trailing semicolons and whitespace from SCXML expressions.
     * SCXML authors sometimes include trailing semicolons (e.g., "new Foo();")
     * which are invalid inside parens in JS expression context.
     */
    private fun cleanExpr(expr: String): String {
        return expr.trimEnd().trimEnd(';').trimEnd()
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

    private fun isUndeclaredSimpleVariable(expr: String, session: Session): Boolean {
        if (expr.isEmpty()) return false
        if (!isSimpleVariableName(expr)) return false
        if (expr.startsWith("_") || expr in BUILTINS) return false
        return expr !in session.declaredVars
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

    /**
     * Serialize Kotlin value to JSON string for pushing into JS via JSON.parse.
     */
    private fun toJSON(value: Any?): String {
        return when (value) {
            null -> "null"
            is Boolean -> value.toString()
            is Number -> value.toString()
            is String -> jsStringLiteral(value)
            is List<*> -> "[${value.joinToString(",") { toJSON(it) }}]"
            is Map<*, *> -> "{${value.entries.joinToString(",") {
                "${jsStringLiteral(it.key.toString())}:${toJSON(it.value)}"
            }}}"
            else -> jsStringLiteral(value.toString())
        }
    }

    companion object {
        /**
         * Escape a Kotlin string as a JavaScript string literal (double-quoted).
         * Handles all special characters including control characters.
         */
        internal fun jsStringLiteral(s: String): String {
            val sb = StringBuilder(s.length + 10)
            sb.append('"')
            for (c in s) {
                when (c) {
                    '\\' -> sb.append("\\\\")
                    '"' -> sb.append("\\\"")
                    '\n' -> sb.append("\\n")
                    '\r' -> sb.append("\\r")
                    '\t' -> sb.append("\\t")
                    '\b' -> sb.append("\\b")
                    '\u000c' -> sb.append("\\f")
                    else -> {
                        if (c.code < 0x20) {
                            sb.append("\\u${c.code.toString(16).padStart(4, '0')}")
                        } else {
                            sb.append(c)
                        }
                    }
                }
            }
            sb.append('"')
            return sb.toString()
        }

        private val BUILTINS = setOf(
            "true", "false", "null", "undefined", "NaN", "Infinity",
            "Math", "String", "Number", "Boolean", "Array", "Object", "JSON",
            "parseInt", "parseFloat", "isNaN", "isFinite",
            "In", "console", "globalThis", "Error", "TypeError", "RangeError",
            "Date", "RegExp", "Map", "Set", "Promise", "Symbol"
        )

        // W3C DOM: Prototype and factory for XML element objects
        // C++ parity: getElementsByTagName searches entire subtree
        private val DOM_PROTO_SETUP = """
            var __sce_dom_proto = {
                getElementsByTagName: function(tagName) {
                    var result = [];
                    function collect(elem) {
                        var children = elem.__children;
                        if (!children) return;
                        for (var tag in children) {
                            var elems = children[tag];
                            for (var i = 0; i < elems.length; i++) {
                                if (tag === tagName) result.push(elems[i]);
                                collect(elems[i]);
                            }
                        }
                    }
                    collect(this);
                    return result;
                },
                getAttribute: function(attrName) {
                    return (this.__attrs && this.__attrs[attrName]) || "";
                },
                getTagName: function() {
                    return this.__tagName || "";
                }
            };
            function __sce_dom_create(obj) {
                Object.setPrototypeOf(obj, __sce_dom_proto);
                if (obj.__children) {
                    for (var tag in obj.__children) {
                        var arr = obj.__children[tag];
                        for (var i = 0; i < arr.length; i++) {
                            __sce_dom_create(arr[i]);
                        }
                    }
                }
                return obj;
            }
        """.trimIndent()
    }
}
