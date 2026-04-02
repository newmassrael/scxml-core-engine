// GENERATED CODE — DO NOT EDIT
// Source: resources/158/test158.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test158

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test158State : State {
    data object Fail : Test158State
    data object Pass : Test158State
    data object S0 : Test158State
    data object S1 : Test158State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test158Event : Event {
    sealed interface Error : Test158Event {
        data object Execution : Error
    }
    data object Event1 : Test158Event
    data object Event2 : Test158Event
}
// --- State Machine (W3C SCXML) ---

class Test158StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test158State, Test158Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test158State = Test158State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test158Event? = when (name) {
        "error.execution" -> Test158Event.Error.Execution
        "event1" -> Test158Event.Event1
        "event2" -> Test158Event.Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test158Event): String? = when (event) {
        is Test158Event.Error.Execution -> "error.execution"
        is Test158Event.Event1 -> "event1"
        is Test158Event.Event2 -> "event2"
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
        engine.setupSystemVariables(sid, "test158")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test158Event.Error.Execution)
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
            raiseInternal(Test158Event.Error.Execution)
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
            raiseInternal(Test158Event.Error.Execution)
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
            raiseInternal(Test158Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test158Event) {
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
        state: Test158State,
        event: Test158Event
    ): TransitionResult<Test158State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test158State.S0 -> processS0(event)
        is Test158State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test158Event
    ): TransitionResult<Test158State> = when {
        event is Test158Event.Event1 -> TransitionResult.External(Test158State.S1, Test158State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test158State.Fail)
    }

    private fun processS1(
        event: Test158Event
    ): TransitionResult<Test158State> = when {
        event is Test158Event.Event2 -> TransitionResult.External(Test158State.Pass, Test158State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test158State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test158State) {
        when (state) {
            is Test158State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test158State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test158State.S0 -> {
            raiseInternal(Test158Event.Event1)
            raiseInternal(Test158Event.Event2)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test158State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test158State,
        event: Test158Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
