// GENERATED CODE — DO NOT EDIT
// Source: resources/198/test198.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test198

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test198State : State {
    data object Fail : Test198State
    data object Pass : Test198State
    data object S0 : Test198State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test198Event : Event {
    sealed interface Error : Test198Event {
        data object Execution : Error
    }
    data object Event1 : Test198Event
    data object Timeout : Test198Event
}
// --- State Machine (W3C SCXML) ---

class Test198StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test198State, Test198Event>(scriptEngine) {

    override val initialState: Test198State = Test198State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test198Event? = when (name) {
        "error.execution" -> Test198Event.Error.Execution
        "event1" -> Test198Event.Event1
        "timeout" -> Test198Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test198Event): String? = when (event) {
        is Test198Event.Error.Execution -> "error.execution"
        is Test198Event.Event1 -> "event1"
        is Test198Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test198")




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
            raiseInternal(Test198Event.Error.Execution)
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
            raiseInternal(Test198Event.Error.Execution)
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
            raiseInternal(Test198Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test198Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = meta.type,
            sendId = meta.sendId,
            origin = meta.origin.ifEmpty { scriptSessionId ?: "" },
            originType = meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" },
            invokeId = meta.invokeId
        )
    }

    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test198State,
        event: Test198Event
    ): TransitionResult<Test198State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test198State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test198Event
    ): TransitionResult<Test198State> = when {
        event is Test198Event.Event1 && safeEvaluateGuard("_event.origintype == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> TransitionResult.External(Test198State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test198State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test198State) {
        when (state) {
            is Test198State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test198State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test198State.S0 -> {
            send(Test198Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            send(Test198Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test198State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test198State,
        event: Test198Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
