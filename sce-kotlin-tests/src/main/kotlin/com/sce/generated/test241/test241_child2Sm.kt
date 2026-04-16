
// GENERATED CODE — DO NOT EDIT
// Source: resources/241/test241_child2.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test241

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test241Child2State : State {
    data object Sub03 : Test241Child2State
    data object SubFinal3 : Test241Child2State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test241Child2Event : Event {
    sealed interface Error : Test241Child2Event {
        data object Execution : Error
    }
    data object Failure : Test241Child2Event
    data object Success : Test241Child2Event
}
// --- State Machine (W3C SCXML) ---

class Test241Child2StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test241Child2State, Test241Child2Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test241Child2State = Test241Child2State.Sub03

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test241Child2State? = when (stateId) {
        "sub03" -> Test241Child2State.Sub03
        "subFinal3" -> Test241Child2State.SubFinal3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test241Child2State): String = when (state) {
        is Test241Child2State.Sub03 -> "sub03"
        is Test241Child2State.SubFinal3 -> "subFinal3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test241Child2State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test241Child2State): Int = when (state) {
        is Test241Child2State.Sub03 -> 0
        is Test241Child2State.SubFinal3 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test241Child2Event? = when (name) {
        "error.execution" -> Test241Child2Event.Error.Execution
        "failure" -> Test241Child2Event.Failure
        "success" -> Test241Child2Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test241Child2Event): String? = when (event) {
        is Test241Child2Event.Error.Execution -> "error.execution"
        is Test241Child2Event.Failure -> "failure"
        is Test241Child2Event.Success -> "success"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test241_child2")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test241Child2Event.Error.Execution)
        }




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
            raiseInternal(Test241Child2Event.Error.Execution)
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
            raiseInternal(Test241Child2Event.Error.Execution)
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
            raiseInternal(Test241Child2Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test241Child2Event) {
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
        state: Test241Child2State,
        event: Test241Child2Event
    ): TransitionResult<Test241Child2State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test241Child2State
    ): TransitionResult<Test241Child2State> = when (state) {
        is Test241Child2State.Sub03 -> processNullSub03()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub03(
    ): TransitionResult<Test241Child2State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test241Child2State.SubFinal3, Test241Child2State.Sub03)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test241Child2State.SubFinal3, Test241Child2State.Sub03)
    }

    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test241Child2State) {
        when (state) {
            is Test241Child2State.Sub03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub03")) return
            }
            is Test241Child2State.SubFinal3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal3")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test241Child2State) {
        when (state) {
            is Test241Child2State.Sub03 -> {
                activeStateIds.remove("sub03")
            }
            is Test241Child2State.SubFinal3 -> {
                activeStateIds.remove("subFinal3")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test241Child2State,
        event: Test241Child2Event?
    ) {
        when (source) {
        is Test241Child2State.Sub03 -> when {
            event == null && safeEvaluateGuard("Var1 == 1") -> {


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
