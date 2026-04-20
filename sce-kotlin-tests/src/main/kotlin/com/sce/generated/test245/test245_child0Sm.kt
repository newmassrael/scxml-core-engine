
// GENERATED CODE — DO NOT EDIT
// Source: resources/245/test245_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test245

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test245Child0State : State {
    data object Sub0 : Test245Child0State
    data object SubFinal : Test245Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test245Child0Event : Event {
    sealed interface Error : Test245Child0Event {
        data object Execution : Error
    }
    data object Failure : Test245Child0Event
    data object Success : Test245Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test245Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test245Child0State, Test245Child0Event>(scriptEngine) {

    override val initialState: Test245Child0State = Test245Child0State.Sub0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test245Child0State? = when (stateId) {
        "sub0" -> Test245Child0State.Sub0
        "subFinal" -> Test245Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test245Child0State): String = when (state) {
        is Test245Child0State.Sub0 -> "sub0"
        is Test245Child0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test245Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test245Child0State): Int = when (state) {
        is Test245Child0State.Sub0 -> 0
        is Test245Child0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test245Child0Event? = when (name) {
        "error.execution" -> Test245Child0Event.Error.Execution
        "failure" -> Test245Child0Event.Failure
        "success" -> Test245Child0Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test245Child0Event): String? = when (event) {
        is Test245Child0Event.Error.Execution -> "error.execution"
        is Test245Child0Event.Failure -> "failure"
        is Test245Child0Event.Success -> "success"
        // Kotlin `when` expression exhaustiveness: a child machine that
        // inherits the override (has_parent_communication path) but
        // declares no events of its own produces an empty sealed
        // hierarchy, and `when (event)` without `else` fails to compile.
        // The branch is redundant on non-empty hierarchies but harmless.
        else -> null
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test245_child0")





        // W3C SCXML 6.4: Apply pending invoke params from parent
        // Only set params matching child's declared datamodel variables (C++ DatamodelValidationHelper)
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }

        scriptEngineInitialized = true
    }

    // W3C SCXML 5.9: Guard evaluation with error.execution on failure
    private fun safeEvaluateGuard(guardExpr: String): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test245Child0Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test245Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test245Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test245Child0Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1: C++ pattern — origin/origintype only for external events
        // Internal events (<raise>) have empty origin; external events (<send>) have session ID
        val effectiveOrigin = if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        val effectiveOriginType = if (meta.type == "external") meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" } else meta.originType
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test245Child0State,
        event: Test245Child0Event
    ): TransitionResult<Test245Child0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test245Child0State
    ): TransitionResult<Test245Child0State> = when (state) {
        is Test245Child0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test245Child0State> = when {
        safeEvaluateGuard("typeof Var2 !== 'undefined'") -> TransitionResult.External(Test245Child0State.SubFinal, Test245Child0State.Sub0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test245Child0State.SubFinal, Test245Child0State.Sub0)
    }

    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test245Child0State) {
        when (state) {
            is Test245Child0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return
            }
            is Test245Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test245Child0State) {
        when (state) {
            is Test245Child0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test245Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test245Child0State,
        event: Test245Child0Event?
    ) {
        when (source) {
        is Test245Child0State.Sub0 -> when {
            event == null && safeEvaluateGuard("typeof Var2 !== 'undefined'") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("failure", "")
            }
            event == null -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
