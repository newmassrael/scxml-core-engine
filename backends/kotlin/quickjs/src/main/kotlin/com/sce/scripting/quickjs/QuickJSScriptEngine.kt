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

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.SceXmlDom
import com.sce.runtime.PayloadReading
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.w3c.dom.Node

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

    // No list of declared names here on purpose. One lived here, written by
    // four call sites and read by two, and `executeScript` was not among the
    // four — so a variable a `<script>` block declared was absent from it
    // while present in the engine. The engine owns the variables; asking it
    // (`QuickJSNative.hasGlobal`) is the only answer that cannot be
    // incomplete, and it is what both readers now do.
    private data class Session(
        val handle: Long,  // QJSSession pointer
        var stateQueryCallback: ((String) -> Boolean)? = null,
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

    override fun setupSystemVariables(
        sessionId: String,
        machineName: String,
        ioProcessors: List<IoProcessorDescriptor>,
    ) {
        val session = sessions[sessionId] ?: return
        val handle = session.handle

        // W3C SCXML 5.10: _sessionid
        QuickJSNative.setGlobalString(handle, "_sessionid", sessionId)

        // W3C SCXML 5.10: _name
        QuickJSNative.setGlobalString(handle, "_name", machineName)

        // §scxml-C-1-1 / §scxml-C-2-3: one entry per processor the deployment
        // supports, each with a 'location' field. Names and locations are
        // decided by IoProcessors.build, so this engine's view of
        // `_ioprocessors` matches every other backend's. Both the key and the
        // location go through jsStringLiteral — the entry names carry ':' and
        // '#', and the locations are deployment-supplied.
        val entries = ioProcessors.joinToString(", ") { processor ->
            "[${jsStringLiteral(processor.name)}]: { location: ${jsStringLiteral(processor.location)} }"
        }
        QuickJSNative.eval(handle, "_ioprocessors = { $entries }")
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
    }

    override fun getVariable(sessionId: String, name: String): Any? {
        val session = sessions[sessionId] ?: return null
        if (!isSimpleVariableName(name)) return null
        val result = QuickJSNative.evalExpression(session.handle, name)
        return if (result != null) decodeTypedResult(result, session) else null
    }

    override fun hasVariable(sessionId: String, name: String): Boolean {
        val session = sessions[sessionId] ?: return false
        return QuickJSNative.hasGlobal(session.handle, name)
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
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs): PayloadReading {
        val session = sessions[sessionId] ?: return PayloadReading.Absent
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
        return if (args.data.isNotEmpty()) setEventData(handle, args.data) else PayloadReading.Absent
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

        // W3C SCXML 4.6: foreach declares its item and index variables, and it
        // declares them whether or not the array has anything in it. So they
        // are declared HERE rather than by the assignments below, which a
        // zero-length array never reaches.
        //
        // Declared in the engine, not recorded beside it. This used to add the
        // two names to a set the adapter kept, which made `hasVariable` answer
        // yes for a variable the engine did not have — true enough for that
        // one question and false for every other reader, including the
        // document's own `expr=`.
        QuickJSNative.eval(handle, "var $item;")
        if (index.isNotEmpty()) {
            QuickJSNative.eval(handle, "var $index;")
        }

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
    private fun setEventData(handle: Long, data: String): PayloadReading {
        // Step 1: XML DOM
        val firstNonWs = data.indexOfFirst { !it.isWhitespace() }
        if (firstNonWs >= 0 && data[firstNonWs] == '<') {
            val domExpr = buildDOMExpression(data)
            if (domExpr != null) {
                val err = QuickJSNative.eval(handle, "_event.data = $domExpr")
                if (err == null) return PayloadReading.Dom
            }
        }

        // Step 2: JSON
        //
        // §scxml-B-2-8-1 gives `_event.data` three readings and no fourth:
        // XML becomes a DOM, JSON becomes the value, anything else becomes a
        // space-normalized string. There used to be a rung between the two
        // below — `_event.data = ($data)`, evaluating the payload as
        // JavaScript before anything looked at it — and it decided all three
        // of the following, measured 2026-08-17 on the sibling Lua engines
        // that carried the same rung in their own language:
        //
        //   * `2 + 3` from a host arrived as the number 5 here, and as the
        //     string "2 + 3" on the cpp engine and on Rhino, which read the
        //     clause instead. One payload, two answers, from two engines
        //     behind ONE backend.
        //   * a payload that is a function call RAN, in the session's own
        //     globals. `_event.data` is the one field a document takes from
        //     outside itself.
        //   * `2 + 3` on the Lua engines meant Lua's `2 + 3`, so the payload
        //     was read in whatever language the receiver happened to be.
        //
        // Rhino never had the rung, and this engine's own sender ships
        // `JSON.stringify` output (§scxml-B-2-9), so removing it makes the
        // two engines of this backend answer the same question the same way.
        val err2 = QuickJSNative.eval(handle,
            "_event.data = JSON.parse(${jsStringLiteral(data)})")
        if (err2 == null) return PayloadReading.Structured

        // Step 3: Normalized string
        QuickJSNative.eval(handle,
            "_event.data = ${jsStringLiteral(normalizeWhitespace(data))}")
        // Which of the two third-rung readings this is. The clause treats them
        // the same and a host does not: prose arriving as text is the ladder
        // working (W3C test 562), and a payload that opened with a brace and
        // would not parse is a payload whose fields have just stopped
        // existing. Only here is that difference still visible, because only
        // here was the structured read attempted.
        return PayloadReading.ofText(data)
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
        val document = SceXmlDom.parse(xmlContent) ?: return null
        val root = document.documentElement ?: return null
        // The variable holds the document, which answers the Node
        // interface as a document and the Element vocabulary for its
        // document element (§scxml-B-2-1).
        return "__sce_dom_document(${buildElementExpression(root)})"
    }

    /**
     * Recursively build a JS expression for one DOM node.
     *
     * Output: `__sce_dom_node({__type:1,__name:"book",__tagName:"book",
     * __attrs:{…},__kids:[…]})`, and `__sce_dom_node` links each child's
     * `__parent` as it goes — `parentNode` needs that link and a JS object
     * literal cannot refer to itself.
     *
     * Children are in document order, and character data is among them:
     * §scxml-B-2-1's "corresponding DOM structure" is DOM Level 1 Core's
     * tree, so grouping the element children by tag name — which is what
     * this built while `getElementsByTagName` was the only reader — cannot
     * answer `firstChild`, `nextSibling` or `textContent` at all.
     */
    private fun buildElementExpression(node: Node): String {
        val sb = StringBuilder()
        sb.append("__sce_dom_node({__type:").append(SceXmlDom.nodeType(node))
        sb.append(",__name:").append(jsStringLiteral(SceXmlDom.nodeName(node)))

        if (SceXmlDom.hasNodeValue(node)) {
            sb.append(",__value:").append(jsStringLiteral(node.nodeValue ?: ""))
            sb.append("})")
            return sb.toString()
        }

        sb.append(",__tagName:").append(jsStringLiteral(node.nodeName ?: ""))
        sb.append(",__attrs:{")
        val attrs = node.attributes
        if (attrs != null) {
            for (i in 0 until attrs.length) {
                if (i > 0) sb.append(",")
                val attr = attrs.item(i)
                sb.append(jsStringLiteral(attr.nodeName))
                    .append(":")
                    .append(jsStringLiteral(attr.nodeValue ?: ""))
            }
        }
        sb.append("}")

        val kids = SceXmlDom.children(node)
        if (kids.isNotEmpty()) {
            sb.append(",__kids:[")
            for ((index, child) in kids.withIndex()) {
                if (index > 0) sb.append(",")
                sb.append(buildElementExpression(child))
            }
            sb.append("]")
        }

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

    /**
     * §scxml-B-2: reading a name the datamodel never declared is an error,
     * not `undefined`.
     *
     * Asked of the engine, which owns the variables. This used to consult a
     * set of names the adapter maintained, and that set was written by four
     * call sites — none of them `executeScript`. So a variable a `<script>`
     * block declared was absent from it while present in the engine, and this
     * raised a ReferenceError for a name the document had defined itself.
     */
    private fun isUndeclaredSimpleVariable(expr: String, session: Session): Boolean {
        if (expr.isEmpty()) return false
        if (!isSimpleVariableName(expr)) return false
        if (expr.startsWith("_") || expr in BUILTINS) return false
        return !QuickJSNative.hasGlobal(session.handle, expr)
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

        // §scxml-B-2-1's DOM read surface — DOM Level 1 Core, not the two
        // calls the W3C IRP suite happens to read.
        //
        // The Node interface is defined with getters on the prototype so
        // `parentNode` and `childNodes` can point at each other: eager
        // properties would walk the tree until it ran out of stack. A
        // document handle holds `__root` and answers the Node interface as
        // the document it is, while answering the Element vocabulary for
        // its document element — the delegation `getAttribute` and
        // `getTagName` have always performed.
        private val DOM_PROTO_SETUP = """
            var __SCE_DOM_ELEMENT = 1, __SCE_DOM_TEXT = 3, __SCE_DOM_CDATA = 4,
                __SCE_DOM_DOCUMENT = 9;
            function __sce_dom_node_of(handle) {
                return handle.__root !== undefined ? handle.__root : handle;
            }
            function __sce_dom_is_document(handle) {
                return handle.__root !== undefined;
            }
            function __sce_dom_has_value(node) {
                return node.__type === __SCE_DOM_TEXT || node.__type === __SCE_DOM_CDATA;
            }
            function __sce_dom_text_content(node) {
                if (__sce_dom_has_value(node)) return node.__value || "";
                var kids = node.__kids || [], text = "";
                for (var i = 0; i < kids.length; i++) text += __sce_dom_text_content(kids[i]);
                return text;
            }
            function __sce_dom_sibling(node, step) {
                var parent = node.__parent;
                if (!parent || !parent.__kids) return null;
                var kids = parent.__kids;
                for (var i = 0; i < kids.length; i++) {
                    if (kids[i] === node) return kids[i + step] || null;
                }
                return null;
            }
            var __sce_dom_proto = {
                getElementsByTagName: function(tagName) {
                    var node = __sce_dom_node_of(this), result = [];
                    // A document matches its root inclusively, an element
                    // only descends: DOM Level 1 Core 1.2's split.
                    if (__sce_dom_is_document(this) && node.__tagName === tagName) {
                        result.push(node);
                    }
                    function collect(current) {
                        var kids = current.__kids || [];
                        for (var i = 0; i < kids.length; i++) {
                            if (kids[i].__type !== __SCE_DOM_ELEMENT) continue;
                            if (kids[i].__tagName === tagName) result.push(kids[i]);
                            collect(kids[i]);
                        }
                    }
                    collect(node);
                    return result;
                },
                getAttribute: function(attrName) {
                    var node = __sce_dom_node_of(this);
                    return (node.__attrs && node.__attrs[attrName]) || "";
                },
                hasAttribute: function(attrName) {
                    var node = __sce_dom_node_of(this);
                    return !!(node.__attrs && node.__attrs[attrName] !== undefined);
                },
                getTagName: function() {
                    return __sce_dom_node_of(this).__tagName || "";
                },
                hasChildNodes: function() {
                    if (__sce_dom_is_document(this)) return true;
                    var kids = __sce_dom_node_of(this).__kids;
                    return !!(kids && kids.length > 0);
                }
            };
            function __sce_dom_getter(name, read) {
                Object.defineProperty(__sce_dom_proto, name, {
                    get: function() {
                        return read(__sce_dom_node_of(this), __sce_dom_is_document(this));
                    },
                    configurable: true
                });
            }
            __sce_dom_getter("nodeType", function(node, isDoc) {
                return isDoc ? __SCE_DOM_DOCUMENT : node.__type;
            });
            __sce_dom_getter("nodeName", function(node, isDoc) {
                return isDoc ? "#document" : node.__name;
            });
            // DOM Level 1 Core gives an element and a document a null
            // nodeValue; `data` is CharacterData's own name for the value.
            __sce_dom_getter("nodeValue", function(node, isDoc) {
                return (isDoc || !__sce_dom_has_value(node)) ? null : node.__value;
            });
            __sce_dom_getter("data", function(node, isDoc) {
                return (isDoc || !__sce_dom_has_value(node)) ? null : node.__value;
            });
            __sce_dom_getter("tagName", function(node, isDoc) {
                if (!isDoc && __sce_dom_has_value(node)) return null;
                return node.__tagName || "";
            });
            __sce_dom_getter("textContent", function(node) {
                return __sce_dom_text_content(node);
            });
            __sce_dom_getter("childNodes", function(node, isDoc) {
                if (isDoc) return [node];
                return (node.__kids || []).slice(0);
            });
            __sce_dom_getter("firstChild", function(node, isDoc) {
                if (isDoc) return node;
                var kids = node.__kids || [];
                return kids.length > 0 ? kids[0] : null;
            });
            __sce_dom_getter("lastChild", function(node, isDoc) {
                if (isDoc) return node;
                var kids = node.__kids || [];
                return kids.length > 0 ? kids[kids.length - 1] : null;
            });
            __sce_dom_getter("nextSibling", function(node, isDoc) {
                return isDoc ? null : __sce_dom_sibling(node, 1);
            });
            __sce_dom_getter("previousSibling", function(node, isDoc) {
                return isDoc ? null : __sce_dom_sibling(node, -1);
            });
            __sce_dom_getter("parentNode", function(node, isDoc) {
                if (isDoc) return null;
                // The document element's parent is the document — DOM
                // Level 1 Core 1.3 — which is the handle the variable
                // already holds.
                return node.__parent !== undefined ? node.__parent : (node.__doc || null);
            });
            __sce_dom_getter("documentElement", function(node, isDoc) {
                // Only the document handle carries this, which is how a
                // document tells the two kinds apart.
                return isDoc ? node : null;
            });
            function __sce_dom_node(obj) {
                Object.setPrototypeOf(obj, __sce_dom_proto);
                var kids = obj.__kids || [];
                for (var i = 0; i < kids.length; i++) {
                    Object.defineProperty(kids[i], "__parent", {
                        value: obj, enumerable: false, configurable: true
                    });
                }
                return obj;
            }
            function __sce_dom_document(root) {
                var doc = Object.create(__sce_dom_proto);
                Object.defineProperty(doc, "__root", {
                    value: root, enumerable: false, configurable: true
                });
                Object.defineProperty(root, "__doc", {
                    value: doc, enumerable: false, configurable: true
                });
                return doc;
            }
        """.trimIndent()
    }
}
