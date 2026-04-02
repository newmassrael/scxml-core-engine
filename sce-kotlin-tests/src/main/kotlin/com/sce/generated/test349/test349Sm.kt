// GENERATED CODE — DO NOT EDIT
// Source: resources/349/test349.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test349

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test349State : State {
    data object Fail : Test349State
    data object Pass : Test349State
    data object S0 : Test349State
    data object S2 : Test349State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test349Event : Event {
    sealed interface Error : Test349Event {
        data object Execution : Error
    }
    data object S0Event : Test349Event
    data object S0Event2 : Test349Event
}
// --- State Machine (W3C SCXML) ---

class Test349StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test349State, Test349Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test349State = Test349State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test349Event? = when (name) {
        "error.execution" -> Test349Event.Error.Execution
        "s0Event" -> Test349Event.S0Event
        "s0Event2" -> Test349Event.S0Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test349Event): String? = when (event) {
        is Test349Event.Error.Execution -> "error.execution"
        is Test349Event.S0Event -> "s0Event"
        is Test349Event.S0Event2 -> "s0Event2"
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
        engine.setupSystemVariables(sid, "test349")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
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
            raiseInternal(Test349Event.Error.Execution)
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
            raiseInternal(Test349Event.Error.Execution)
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
            raiseInternal(Test349Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test349Event) {
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
        state: Test349State,
        event: Test349Event
    ): TransitionResult<Test349State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test349State.S0 -> processS0(event)
        is Test349State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test349Event
    ): TransitionResult<Test349State> = when {
        event is Test349Event.S0Event -> TransitionResult.External(Test349State.S2)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test349State.Fail)
    }

    private fun processS2(
        event: Test349Event
    ): TransitionResult<Test349State> = when {
        event is Test349Event.S0Event2 -> TransitionResult.External(Test349State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test349State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test349State) {
        when (state) {
            is Test349State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test349State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test349State.S0 -> {
            send(Test349Event.S0Event, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is Test349State.S2 -> {
            send(Test349Event.S0Event2, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test349State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test349State,
        event: Test349Event?
    ) {
        when (source) {
        is Test349State.S0 -> when {
            event is Test349Event.S0Event -> {
            executeAssign("Var1", "_event.origin")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
