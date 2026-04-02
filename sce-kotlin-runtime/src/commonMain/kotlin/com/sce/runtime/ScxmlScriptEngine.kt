// SPDX-License-Identifier: MIT
// SPDX-FileCopyrightText: 2025 newmassrael
//
// SCE Kotlin Runtime — Script engine interface for ECMAScript evaluation

package com.sce.runtime

/**
 * Platform-agnostic interface for ECMAScript evaluation in SCXML state machines.
 *
 * W3C SCXML B.1: ECMAScript datamodel support. Provides session-scoped
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
     * W3C SCXML 5.10: Initialize system variables (_sessionid, _name, _ioprocessors).
     *
     * Must be called after [createSession] and before any expression evaluation.
     *
     * @param sessionId Active session
     * @param machineName SCXML document name attribute
     */
    fun setupSystemVariables(sessionId: String, machineName: String)

    /**
     * W3C SCXML 5.9: Evaluate a guard condition expression.
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
     * W3C SCXML 5.3: Evaluate an expression and return the result.
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
     * W3C SCXML 3.8.6: Execute a script block.
     *
     * Used for <script> elements and global scripts at document load time.
     *
     * @param sessionId Active session
     * @param script ECMAScript code to execute
     * @throws ScriptEngineException on execution failure
     */
    fun executeScript(sessionId: String, script: String)

    /**
     * W3C SCXML 5.3: Set a variable in the datamodel.
     *
     * @param sessionId Active session
     * @param name Variable name
     * @param value Variable value (null for "undefined")
     */
    fun setVariable(sessionId: String, name: String, value: Any?)

    /**
     * W3C SCXML 5.3: Get a variable from the datamodel.
     *
     * @param sessionId Active session
     * @param name Variable name
     * @return Variable value (null if undefined)
     */
    fun getVariable(sessionId: String, name: String): Any?

    /**
     * W3C SCXML 5.3: Assign a value to a location using an expression.
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
     * W3C SCXML 5.10: Set the _event system variable for the current event.
     *
     * Must be called before processing an event so that guard conditions
     * and actions can access _event.name, _event.data, etc.
     *
     * @param sessionId Active session
     * @param name Event name
     * @param data Event data (empty string if none)
     * @param type Event type ("external", "internal", "platform")
     * @param sendId Send identifier (empty string if none)
     * @param origin Origin URI (empty string if none)
     * @param originType Origin type (empty string if none)
     * @param invokeId Invoke identifier (empty string if none)
     */
    fun setCurrentEvent(
        sessionId: String,
        name: String,
        data: String = "",
        type: String = "",
        sendId: String = "",
        origin: String = "",
        originType: String = "",
        invokeId: String = ""
    )

    /**
     * Clear the _event system variable.
     *
     * Called for eventless (null) transitions where no event is active.
     *
     * @param sessionId Active session
     */
    fun clearCurrentEvent(sessionId: String)

    /**
     * W3C SCXML 5.9.2: Register In() predicate callback.
     *
     * The callback checks whether a state ID is in the active configuration.
     * Used by ECMAScript In() function to query the state machine.
     *
     * @param sessionId Active session
     * @param callback Function that returns true if the state ID is active, or null to unregister
     */
    fun setStateQueryCallback(sessionId: String, callback: ((String) -> Boolean)?)

    /**
     * W3C SCXML 4.6: Execute foreach iteration over an array variable.
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
}

/**
 * Exception thrown by [ScxmlScriptEngine] on evaluation or execution failure.
 *
 * W3C SCXML 5.9: Callers should catch this and raise error.execution.
 */
class ScriptEngineException(message: String, cause: Throwable? = null) :
    RuntimeException(message, cause)
