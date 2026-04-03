// GENERATED CODE — DO NOT EDIT
// Source: resources/253/test253_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test253

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test253Child0State : State {
    data object Sub0 : Test253Child0State
    data object Sub1 : Test253Child0State
    data object SubFinal : Test253Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test253Child0Event : Event {
    data object ChildRunning : Test253Child0Event
    sealed interface Error : Test253Child0Event {
        data object Execution : Error
    }
    data object Failure : Test253Child0Event
    data object ParentToChild : Test253Child0Event
    data object Success : Test253Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test253Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test253Child0State, Test253Child0Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test253Child0State = Test253Child0State.Sub0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test253Child0State? = when (stateId) {
        "sub0" -> Test253Child0State.Sub0
        "sub1" -> Test253Child0State.Sub1
        "subFinal" -> Test253Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test253Child0State): String = when (state) {
        is Test253Child0State.Sub0 -> "sub0"
        is Test253Child0State.Sub1 -> "sub1"
        is Test253Child0State.SubFinal -> "subFinal"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test253Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test253Child0State): Int = when (state) {
        is Test253Child0State.Sub0 -> 0
        is Test253Child0State.Sub1 -> 1
        is Test253Child0State.SubFinal -> 2
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test253Child0Event? = when (name) {
        "childRunning" -> Test253Child0Event.ChildRunning
        "error.execution" -> Test253Child0Event.Error.Execution
        "failure" -> Test253Child0Event.Failure
        "parentToChild" -> Test253Child0Event.ParentToChild
        "success" -> Test253Child0Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test253Child0Event): String? = when (event) {
        is Test253Child0Event.ChildRunning -> "childRunning"
        is Test253Child0Event.Error.Execution -> "error.execution"
        is Test253Child0Event.Failure -> "failure"
        is Test253Child0Event.ParentToChild -> "parentToChild"
        is Test253Child0Event.Success -> "success"
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
        engine.setupSystemVariables(sid, "test253_child0")

        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}




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
            raiseInternal(Test253Child0Event.Error.Execution)
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
            raiseInternal(Test253Child0Event.Error.Execution)
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
            raiseInternal(Test253Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test253Child0Event) {
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
        state: Test253Child0State,
        event: Test253Child0Event
    ): TransitionResult<Test253Child0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test253Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test253Child0State
    ): TransitionResult<Test253Child0State> = when (state) {
        is Test253Child0State.Sub1 -> processNullSub1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub1(
    ): TransitionResult<Test253Child0State> = when {
        safeEvaluateGuard("Var2 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> TransitionResult.External(Test253Child0State.SubFinal, Test253Child0State.Sub1)
        safeEvaluateGuard("Var2 == 'scxml'") -> TransitionResult.External(Test253Child0State.SubFinal, Test253Child0State.Sub1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test253Child0State.SubFinal, Test253Child0State.Sub1)
    }

    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test253Child0Event
    ): TransitionResult<Test253Child0State> = when {
        event is Test253Child0Event.ParentToChild -> TransitionResult.External(Test253Child0State.Sub1, Test253Child0State.Sub0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test253Child0State) {
        when (state) {
            is Test253Child0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childRunning", "")
            }
            is Test253Child0State.Sub1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub1")) return
            }
            is Test253Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test253Child0State) {
        when (state) {
            is Test253Child0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test253Child0State.Sub1 -> {
                activeStateIds.remove("sub1")
            }
            is Test253Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test253Child0State,
        event: Test253Child0Event?
    ) {
        when (source) {
        is Test253Child0State.Sub0 -> when {
            event is Test253Child0Event.ParentToChild -> {
            executeAssign("Var2", "_event.origintype")
            }
            else -> {}
        }
        is Test253Child0State.Sub1 -> when {
            event == null && safeEvaluateGuard("Var2 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null && safeEvaluateGuard("Var2 == 'scxml'") -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("failure", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
