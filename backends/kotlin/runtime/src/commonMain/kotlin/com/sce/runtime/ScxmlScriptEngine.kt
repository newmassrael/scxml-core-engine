// SPDX-License-Identifier: LGPL-2.1-or-later WITH LicenseRef-SCE-Linking-Exception OR LicenseRef-SCE-Commercial
// SPDX-FileCopyrightText: Copyright (c) 2025 newmassrael
//
// SCE Kotlin Runtime — Script engine interface for ECMAScript evaluation

package com.sce.runtime

/**
 * Platform-agnostic interface for ECMAScript evaluation in SCXML state machines.
 *
 * §scxml-B-1: ECMAScript datamodel support. Provides session-scoped
 * expression evaluation, variable management, and event metadata access.
 *
 * Implementations:
 *   - JVM tests: Rhino (javax.script JSR-223)
 *   - Android/AAOS production: QuickJS via JNI/NDK
 *
 * Each state machine instance creates its own session via [createSession],
 * sharing the same engine instance for memory efficiency.
 */
interface ScxmlScriptEngine {

    /**
     * Create a new evaluation session for a state machine instance.
     *
     * Each session has its own variable scope (datamodel).
     * Must be called before any evaluation methods.
     *
     * @param sessionId Unique session identifier (typically hashCode-based)
     */
    fun createSession(sessionId: String)

    /**
     * Destroy a session and release its resources.
     *
     * Called from [StateMachineEngine.stop] during cleanup.
     *
     * @param sessionId Session to destroy
     */
    fun destroySession(sessionId: String)

    /**
     * §scxml-5.10: Initialize system variables (_sessionid, _name, _ioprocessors).
     *
     * Must be called after [createSession] and before any expression evaluation.
     *
     * The descriptors arrive fully resolved from [IoProcessors.build]. An
     * implementation files each one under its name with its location and
     * invents neither, so `_ioprocessors` reads identically whichever engine
     * backs the session.
     *
     * @param sessionId Active session
     * @param machineName SCXML document name attribute
     * @param ioProcessors Entries to publish in `_ioprocessors`
     */
    fun setupSystemVariables(
        sessionId: String,
        machineName: String,
        ioProcessors: List<IoProcessorDescriptor> = IoProcessors.build(sessionId),
    )

    /**
     * §scxml-5.9: Evaluate a guard condition expression.
     *
     * Returns the boolean result of the expression. On evaluation failure,
     * throws an exception (caller is responsible for raising error.execution).
     *
     * @param sessionId Active session
     * @param expr ECMAScript boolean expression (e.g., "Var1 == 1")
     * @return Boolean result of the expression
     * @throws ScriptEngineException on evaluation failure
     */
    fun evaluateCondition(sessionId: String, expr: String): Boolean

    /**
     * §scxml-5.3: Evaluate an expression and return the result.
     *
     * Used for variable initialization, param expr evaluation, etc.
     *
     * @param sessionId Active session
     * @param expr ECMAScript expression
     * @return Evaluation result (may be null for "undefined")
     * @throws ScriptEngineException on evaluation failure
     */
    fun evaluateExpr(sessionId: String, expr: String): Any?

    /**
     * §scxml-5.8: Execute a script block.
     *
     * Used for <script> elements and global scripts at document load time.
     *
     * @param sessionId Active session
     * @param script ECMAScript code to execute
     * @throws ScriptEngineException on execution failure
     */
    fun executeScript(sessionId: String, script: String)

    /**
     * §scxml-5.3: Set a variable in the datamodel.
     *
     * @param sessionId Active session
     * @param name Variable name
     * @param value Variable value (null for "undefined")
     */
    fun setVariable(sessionId: String, name: String, value: Any?)

    /**
     * §scxml-5.3: Get a variable from the datamodel.
     *
     * @param sessionId Active session
     * @param name Variable name
     * @return Variable value (null if undefined)
     */
    fun getVariable(sessionId: String, name: String): Any?

    /**
     * §scxml-6.4: Check if a variable is declared in the datamodel.
     *
     * Used by invoke param validation to skip variables not declared in child's datamodel.
     * Returns true if the variable property exists in the session scope.
     *
     * @param sessionId Active session
     * @param name Variable name to check
     * @return true if the variable is declared
     */
    fun hasVariable(sessionId: String, name: String): Boolean

    /**
     * §scxml-5.3: Assign a value to a location using an expression.
     *
     * Evaluates [expr] and assigns the result to [location].
     * Validates that location is not a system variable (_event, _sessionid, etc.).
     *
     * @param sessionId Active session
     * @param location Assignment target variable name
     * @param expr ECMAScript expression to evaluate
     * @throws ScriptEngineException on evaluation failure or invalid location
     */
    fun assign(sessionId: String, location: String, expr: String)

    /**
     * §scxml-5.10: Set the _event system variable for the current event.
     *
     * Must be called before processing an event so that guard conditions
     * and actions can access _event.name, _event.data, etc.
     *
     * @param sessionId Active session
     * @param args [SetCurrentEventArgs] bundling event name + 6 metadata fields
     * @return which rung of §scxml-B-2-8-1 the payload got. The implementation
     *   is walking that ladder either way, and the rung is the one fact about
     *   a delivered event that nothing else can recover afterwards — see
     *   [PayloadReading]. An implementation that binds no payload returns
     *   [PayloadReading.Absent]; one that cannot tell the rungs apart must not
     *   guess, because a wrong [PayloadReading.Undecodable] is a host chasing
     *   a payload that arrived intact.
     */
    fun setCurrentEvent(sessionId: String, args: SetCurrentEventArgs): PayloadReading

    /**
     * Clear the _event system variable.
     *
     * Called for eventless (null) transitions where no event is active.
     *
     * @param sessionId Active session
     */
    fun clearCurrentEvent(sessionId: String)

    /**
     * §scxml-5.9.2: Register In() predicate callback.
     *
     * The callback checks whether a state ID is in the active configuration.
     * Used by ECMAScript In() function to query the state machine.
     *
     * @param sessionId Active session
     * @param callback Function that returns true if the state ID is active, or null to unregister
     */
    fun setStateQueryCallback(sessionId: String, callback: ((String) -> Boolean)?)

    /**
     * §scxml-4.6: Execute foreach iteration over an array variable.
     *
     * Iterates over the array, setting item/index variables for each element,
     * and calls [body] for each iteration.
     *
     * @param sessionId Active session
     * @param array ECMAScript expression that evaluates to an array
     * @param item Variable name to bind the current element
     * @param index Variable name to bind the current index (empty string if not used)
     * @param body Callback executed for each iteration
     * @throws ScriptEngineException if array is not iterable or execution fails
     */
    fun executeForeach(
        sessionId: String,
        array: String,
        item: String,
        index: String,
        body: () -> Unit
    )

    /**
     * §scxml-5.2.2: Load external data source content at runtime.
     *
     * C++ DataModelInitHelper::initializeVariableFromSrc pattern.
     * Resolves file:// URIs relative to [basePath] and returns file content.
     *
     * @param src Source URI (e.g., "file:test552.txt")
     * @param basePath Base directory for relative path resolution
     * @return File content as string, or null if resolution fails
     */
    fun loadDataFromSrc(src: String, basePath: String): String? = null

    /**
     * §scxml-B-2: Parse raw data value as XML DOM, JSON, or space-normalized string.
     *
     * C++ parseEventData() pattern. Used for:
     * - Inline <content> in <data> elements
     * - External <data src="..."> file content
     * - Event data in <send>/<content>
     *
     * Detection order:
     * 1. XML (starts with '<') → DOM object with getElementsByTagName()/getAttribute()
     * 2. JSON → parsed JavaScript value
     * 3. Plain text → space-normalized string (collapse whitespace, strip leading/trailing)
     *
     * @param sessionId Active session (needed for creating JS-compatible objects)
     * @param data Raw data string
     * @return Parsed value suitable for the script engine's datamodel
     */
    fun parseDataValue(sessionId: String, data: String): Any? = data
}

/**
 * Parameter object for the §scxml-5.10 [ScxmlScriptEngine.setCurrentEvent] boundary.
 *
 * Bundles the seven `_event.*` metadata fields (name + 6 metadata) that every
 * script engine impl must surface before guard evaluation / action execution.
 * Cross-language sibling: `SCE::SetCurrentEventArgs` in C++ and
 * `sce_rust_runtime::SetCurrentEventArgs` in Rust.
 */
data class SetCurrentEventArgs(
    val name: String,
    val data: String = "",
    val type: String = "",
    val sendId: String = "",
    val origin: String = "",
    val originType: String = "",
    val invokeId: String = ""
)

/**
 * Which reading of §scxml-B-2-8-1 a payload actually got.
 *
 * The clause gives `_event.data` three readings and no fourth: content the
 * processor can interpret as XML becomes a DOM, content it can interpret as a
 * value becomes that value, and "otherwise, the Processor MUST treat the
 * content as a space-normalized string literal". Every engine here walks that
 * ladder, and until now every engine dropped which rung it landed on.
 *
 * Dropping it is what makes a lost payload silent. Measured 2026-08-22 on
 * three independent Lua implementations (mlua, go-lua and Lua 5.4), a host
 * that hands over `{["milestone"]="refined"}` — Lua's own table syntax — gets
 * the third rung, and a document that then reads `_event.data.milestone`
 * assigns nothing. In the worked supervision loop that emptied `start_prompt`
 * as well, so the restarted session was primed with an empty string and the
 * run converged anyway. Nothing failed; the information stopped existing.
 *
 * [Undecodable] is the one a host acts on, and it is not the engine guessing
 * from a leading brace: the script engine reports it because it ATTEMPTED a
 * structured read and that read failed, which is a fact only the ladder holds.
 *
 * Cross-language siblings: `SCE::PayloadReading` (C++),
 * `sce_rust_runtime::PayloadReading`, `sce.PayloadReading` (Go) and
 * `sce_runtime.payload_reading.PayloadReading` (Python).
 */
enum class PayloadReading {
    /** The event carried no payload, so no rung applies. */
    Absent,

    /** Rung one: read as an XML document, bound as a DOM. */
    Dom,

    /** Rung two: read as a value, bound as that value. */
    Structured,

    /**
     * Rung three, and nothing suggested the content was structured. A
     * `<content>` element holding prose lands here, and that is correct —
     * W3C test 562 pins it.
     */
    Text,

    /**
     * Rung three, taken AFTER a structured read was attempted and failed.
     *
     * The payload announced itself as structure and the datamodel could not
     * read it, so `_event.data` holds the raw characters and every
     * `_event.data.<field>` the document reads is empty.
     */
    Undecodable;

    companion object {
        /**
         * Which third-rung reading a payload that fell through to text
         * deserves.
         *
         * The clause treats prose and a malformed object identically — both
         * are "otherwise" — and a host does not. This is the one place that
         * rule is written, so the ladder's implementations mirror a definition
         * instead of each deciding for itself what "looks structured" means.
         *
         * The test is the opening character, and deliberately only `{` and
         * `[`. A number, a bare word or a quoted string is what an author
         * writes in a `<content>` element, and W3C test 562 requires those to
         * arrive as text without complaint; an object or an array is what a
         * host CONSTRUCTS, and nobody constructs one by accident. Widening
         * this to "anything not obviously prose" would report the ladder
         * working as a defect, which is the failure that gets a diagnostic
         * ignored.
         */
        fun ofText(payload: String): PayloadReading {
            val trimmed = payload.trimStart()
            return if (trimmed.startsWith("{") || trimmed.startsWith("[")) Undecodable else Text
        }
    }
}

/**
 * Exception thrown by [ScxmlScriptEngine] on evaluation or execution failure.
 *
 * §scxml-5.9: Callers should catch this and raise error.execution.
 */
class ScriptEngineException(message: String, cause: Throwable? = null) :
    RuntimeException(message, cause)
