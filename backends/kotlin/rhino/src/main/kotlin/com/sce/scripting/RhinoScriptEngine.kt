// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin — Rhino-based ECMAScript engine for W3C SCXML datamodel evaluation

package com.sce.scripting

import com.sce.runtime.IoProcessorDescriptor
import com.sce.runtime.SceXmlDom
import com.sce.runtime.PayloadReading
import com.sce.runtime.ScriptEngineException
import com.sce.runtime.ScxmlScriptEngine
import com.sce.runtime.SetCurrentEventArgs
import org.mozilla.javascript.Context
import org.mozilla.javascript.ScriptableObject
import org.mozilla.javascript.Scriptable
import org.mozilla.javascript.Undefined
import org.w3c.dom.Node

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

        // W3C SCXML B.2: System variables are read-only
        if (location.startsWith("_")) {
            throw ScriptEngineException("Cannot assign to system variable: $location")
        }

        val cx = Context.enter()
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1

            // C++ AssignmentExecutionHelper 3-path strategy:
            // 1. System variable reference in expr → executeScript (preserves JS object references)
            if (isSystemVariableReference(expr)) {
                cx.evaluateString(session.scope, "$location = $expr;", "assign", 1, null)
            }
            // 2. Simple variable name → evaluate + setVariable
            else if (isSimpleVariableName(location)) {
                val result = cx.evaluateString(session.scope, expr, "assign", 1, null)
                ScriptableObject.putProperty(session.scope, location, result)
            }
            // 3. Complex path (e.g., "foo.bar") → executeScript
            else {
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

    /**
     * C++ AssignmentExecutionHelper::isSystemVariableReference pattern.
     * W3C SCXML 5.10: System variables that require direct script execution
     * to preserve JavaScript object reference semantics (test 329: Var2 = _event).
     */
    private fun isSystemVariableReference(expr: String): Boolean {
        return expr == "_sessionid" || expr == "_event" || expr == "_name" ||
               expr == "_ioprocessors" || expr == "_x"
    }

    /**
     * C++ AssignmentExecutionHelper regex: ^[a-zA-Z_][a-zA-Z0-9_]*$
     * Simple variable names use setVariable, complex paths use executeScript.
     */
    private fun isSimpleVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        val first = name[0]
        if (!first.isLetter() && first != '_') return false
        return name.all { it.isLetterOrDigit() || it == '_' }
    }

    override fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs): PayloadReading {
        val session = sessions[sessionId] ?: return PayloadReading.Absent
        var reading = PayloadReading.Absent
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

            // W3C SCXML B.2: Parse event data using C++ parseEventData pattern
            if (args.data.isNotEmpty()) {
                val parsed = parseDataValueInternal(cx, scope, args.data)
                ScriptableObject.putProperty(eventObj, "data", parsed.value ?: Undefined.instance)
                reading = parsed.reading
            } else {
                ScriptableObject.putProperty(eventObj, "data", Undefined.instance)
            }

            ScriptableObject.putProperty(scope, "_event", eventObj)
        } finally {
            Context.exit()
        }
        return reading
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

        // W3C SCXML 4.6: Validate item variable name (C++ ForeachHelper::isLegalVariableName)
        if (!isLegalVariableName(item)) {
            throw ScriptEngineException("Illegal foreach item variable name: '$item'")
        }
        // W3C SCXML 4.6: Validate index variable name if specified
        if (index.isNotEmpty() && !isLegalVariableName(index)) {
            throw ScriptEngineException("Illegal foreach index variable name: '$index'")
        }

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

    /**
     * W3C SCXML 5.2.2: Load external data source content at runtime.
     * C++ DataModelInitHelper::initializeVariableFromSrc pattern.
     */
    override fun loadDataFromSrc(src: String, basePath: String): String? {
        val filename = if (src.startsWith("file:")) src.substring(5) else src
        val file = java.io.File(basePath, filename)
        return try {
            file.readText(Charsets.UTF_8).trim()
        } catch (_: Exception) {
            null
        }
    }

    /**
     * W3C SCXML B.2: Parse raw data value as XML DOM, JSON, or space-normalized string.
     * C++ parseEventData() pattern.
     */
    override fun parseDataValue(sessionId: String, data: String): Any? {
        val session = sessions[sessionId] ?: return data
        val cx = Context.enter()
        try {
            // This caller wants the value only — it is not delivering an event,
            // so there is no reading for anyone to act on.
            return parseDataValueInternal(cx, session.scope, data).value
        } finally {
            Context.exit()
        }
    }

    /**
     * W3C SCXML B.2: Validate ECMAScript variable name.
     * Mirrors C++ ForeachHelper::isLegalVariableName().
     */
    private fun isLegalVariableName(name: String): Boolean {
        if (name.isEmpty()) return false
        // Must not be quoted (e.g., 'continue' or "continue")
        if (name.first() == '\'' || name.first() == '"') return false
        // Must start with letter, underscore, or dollar sign
        if (!name.first().isLetter() && name.first() != '_' && name.first() != '$') return false
        // Must contain only alphanumeric, underscore, or dollar sign
        return name.all { it.isLetterOrDigit() || it == '_' || it == '$' }
    }

    // --- Private Helpers ---

    /**
     * W3C SCXML B.2: Internal parseDataValue with pre-entered Context.
     * C++ parseEventData() pattern: XML → DOM, JSON → JS object, else → space-normalized string.
     */
    private fun parseDataValueInternal(cx: Context, scope: ScriptableObject, data: String): ParsedPayload {
        // Step 1: XML detection (C++ pattern: skip leading whitespace, check for '<')
        val firstNonWhitespace = data.indexOfFirst { it != ' ' && it != '\t' && it != '\r' && it != '\n' }
        if (firstNonWhitespace >= 0 && data[firstNonWhitespace] == '<') {
            val domObj = createRhinoDOMObject(cx, scope, data)
            if (domObj != null) return ParsedPayload(domObj, PayloadReading.Dom)
        }

        // Step 2: Try strict JSON parsing (C++ JS_ParseJSON pattern)
        // Use JSON.parse() via script — Rhino 1.7.15 NativeJSON.parse() NPEs on null reviver
        try {
            cx.languageVersion = Context.VERSION_ES6
            cx.optimizationLevel = -1
            // Store data in temp variable to avoid escaping issues, then JSON.parse it
            ScriptableObject.putProperty(scope, "__sce_tmp_data__", data)
            val parsed = cx.evaluateString(scope, "JSON.parse(__sce_tmp_data__)", "json-parse", 1, null)
            ScriptableObject.deleteProperty(scope, "__sce_tmp_data__")
            if (parsed != null && parsed !is Undefined) return ParsedPayload(parsed, PayloadReading.Structured)
        } catch (_: Exception) {
            ScriptableObject.deleteProperty(scope, "__sce_tmp_data__")
            // Not valid JSON — fall through
        }

        // Step 3: Space normalization (C++ parseEventData fallback)
        //
        // Which of the two third-rung readings this is. The clause treats them
        // the same and a host does not: prose arriving as text is the ladder
        // working (W3C test 562), and a payload that opened with a brace and
        // would not parse is a payload whose fields have just stopped
        // existing. Only here is that difference still visible, because only
        // here was the structured read attempted.
        return ParsedPayload(normalizeWhitespace(data), PayloadReading.ofText(data))
    }

    /**
     * What the ladder produced, and which of its rungs produced it.
     *
     * The rung is returned rather than re-derived by the caller: a caller can
     * tell a string from an object afterwards, but not a string the ladder
     * chose because the content was prose from one it chose because a
     * structured read failed — and that difference is the whole point of
     * [PayloadReading].
     */
    private data class ParsedPayload(val value: Any?, val reading: PayloadReading)

    /**
     * W3C SCXML B.2: Create Rhino-compatible DOM object from XML content.
     * C++ DOMBinding::createDOMObject() pattern using Java javax.xml.
     */
    private fun createRhinoDOMObject(cx: Context, scope: ScriptableObject, xmlContent: String): Scriptable? {
        val document = SceXmlDom.parse(xmlContent) ?: return null
        val root = document.documentElement ?: return null
        // The variable holds the document: it answers the Node interface as
        // a document and the Element vocabulary for its document element
        // (§scxml-B-2-1).
        return RhinoDomHandle(scope, root, isDocument = true)
    }

    /**
     * §scxml-B-2-1's DOM read surface, as a Rhino object.
     *
     * The Recommendation obliges the Processor to create *"the
     * corresponding DOM structure"*, so what a handle answers is DOM Level
     * 1 Core and not the two calls the W3C IRP suite happens to read.
     * Measured 2026-08-18, two was all this engine carried — it had no
     * `getTagName` either, which the frontend lowers and the other
     * backends implement — so `d.tagName`, `d.firstChild` and
     * `d.childNodes.length` were undefined here with every W3C fixture
     * green.
     *
     * Members are resolved in [get] rather than installed as properties,
     * and they have to be: `parentNode` and `childNodes` point at each
     * other, so materialising either eagerly walks the tree until it runs
     * out of stack. The [node] a document handle carries is its document
     * element, which is what makes the Element vocabulary answer for the
     * root the way `getAttribute` and `getTagName` always did.
     */
    private class RhinoDomHandle(
        private val topScope: Scriptable,
        private val node: Node,
        private val isDocument: Boolean
    ) : ScriptableObject() {

        init {
            parentScope = topScope
            prototype = getObjectPrototype(topScope)
        }

        override fun getClassName(): String = "DOMNode"

        override fun has(name: String, start: Scriptable): Boolean = get(name, start) != NOT_FOUND

        override fun get(name: String, start: Scriptable): Any? =
            when (name) {
                // ── methods ──
                "getElementsByTagName" -> method(name) { args ->
                    val tag = if (args.isEmpty()) "" else Context.toString(args[0])
                    // A document matches its root inclusively, an element
                    // only descends: DOM Level 1 Core 1.2's split.
                    val found = mutableListOf<Node>()
                    if (isDocument && node.nodeName == tag) {
                        found.add(node)
                    }
                    collect(node, tag, found)
                    nodeList(found)
                }
                "getAttribute" -> method(name) { args ->
                    if (args.isEmpty()) {
                        ""
                    } else {
                        attribute(Context.toString(args[0])) ?: ""
                    }
                }
                "hasAttribute" -> method(name) { args ->
                    args.isNotEmpty() && attribute(Context.toString(args[0])) != null
                }
                "getTagName" -> method(name) { node.nodeName ?: "" }
                "hasChildNodes" -> method(name) {
                    isDocument || SceXmlDom.children(node).isNotEmpty()
                }

                // ── the Node interface, as properties ──
                "nodeType" -> if (isDocument) SceXmlDom.TYPE_DOCUMENT else SceXmlDom.nodeType(node)
                "nodeName" -> if (isDocument) "#document" else SceXmlDom.nodeName(node)
                // DOM Level 1 Core gives an element and a document a null
                // nodeValue; `data` is CharacterData's own name for it.
                "nodeValue", "data" ->
                    if (isDocument || !SceXmlDom.hasNodeValue(node)) null else node.nodeValue
                "tagName" ->
                    if (!isDocument && SceXmlDom.hasNodeValue(node)) null else (node.nodeName ?: "")
                "textContent" -> SceXmlDom.textContent(node)
                "childNodes" ->
                    if (isDocument) nodeList(listOf(node)) else nodeList(SceXmlDom.children(node))
                "firstChild" ->
                    if (isDocument) wrap(node) else wrap(SceXmlDom.children(node).firstOrNull())
                "lastChild" ->
                    if (isDocument) wrap(node) else wrap(SceXmlDom.children(node).lastOrNull())
                "nextSibling" -> if (isDocument) null else wrap(sibling(1))
                "previousSibling" -> if (isDocument) null else wrap(sibling(-1))
                "parentNode" -> when {
                    isDocument -> null
                    // The document element's parent is the document — DOM
                    // Level 1 Core 1.3 — which is the handle the variable
                    // already holds.
                    node.parentNode == null ||
                        SceXmlDom.nodeType(node.parentNode) == SceXmlDom.TYPE_DOCUMENT ->
                        RhinoDomHandle(topScope, node, isDocument = true)
                    else -> wrap(node.parentNode)
                }
                // Only the document handle carries this, which is how a
                // document tells the two kinds apart.
                "documentElement" -> if (isDocument) wrap(node) else null

                else -> super.get(name, start)
            }

        /**
         * The attribute's value, or null when the element does not carry
         * it — the distinction `getAttribute`'s "" cannot make and
         * `hasAttribute` exists for. Collapsing the two with `?: ""` made
         * `hasAttribute('missing')` answer true.
         */
        private fun attribute(name: String): String? {
            val attributes = node.attributes ?: return null
            val item = attributes.getNamedItem(name) ?: return null
            return item.nodeValue ?: ""
        }

        private fun sibling(step: Int): Node? {
            val parent = node.parentNode ?: return null
            val siblings = SceXmlDom.children(parent)
            val index = siblings.indexOfFirst { it === node }
            if (index < 0) {
                return null
            }
            return siblings.getOrNull(index + step)
        }

        private fun collect(current: Node, tag: String, found: MutableList<Node>) {
            for (child in SceXmlDom.children(current)) {
                if (SceXmlDom.nodeType(child) != SceXmlDom.TYPE_ELEMENT) {
                    continue
                }
                if (child.nodeName == tag) {
                    found.add(child)
                }
                collect(child, tag, found)
            }
        }

        private fun wrap(target: Node?): Any? =
            if (target == null) null else RhinoDomHandle(topScope, target, isDocument = false)

        /**
         * A NodeList: Rhino's own array, because the frontend rewrites
         * `[0]` to `[1]` upstream only for Lua and reads `length` here —
         * which is why `item(i)` is refused by the frontend rather than
         * implemented, in every backend.
         */
        private fun nodeList(nodes: List<Node>): Scriptable {
            val cx = Context.getCurrentContext()
            // Rhino requires Object[] component type, not Scriptable[]
            val arr = arrayOfNulls<Any>(nodes.size)
            for ((index, member) in nodes.withIndex()) {
                arr[index] = RhinoDomHandle(topScope, member, isDocument = false)
            }
            return cx.newArray(topScope, arr)
        }

        private fun method(name: String, body: (Array<out Any?>) -> Any?): Scriptable =
            object : org.mozilla.javascript.BaseFunction() {
                override fun call(
                    cx: Context,
                    scope: Scriptable,
                    thisObj: Scriptable,
                    args: Array<out Any?>
                ): Any? = body(args)

                override fun getArity(): Int = 1

                override fun getFunctionName(): String = name
            }
    }

    /**
     * W3C SCXML B.2: Space normalization for plain text data.
     * C++ parseEventData fallback: collapse whitespace, strip leading/trailing.
     */
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

        // Remove trailing whitespace
        if (normalized.isNotEmpty() && normalized.last() == ' ') {
            normalized.deleteCharAt(normalized.length - 1)
        }

        return normalized.toString()
    }

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
