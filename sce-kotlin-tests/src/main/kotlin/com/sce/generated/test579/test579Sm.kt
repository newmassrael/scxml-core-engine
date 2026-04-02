// GENERATED CODE — DO NOT EDIT
// Source: resources/579/test579.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test579

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test579State : State {
    data object Fail : Test579State
    data object Pass : Test579State
    data object S0 : Test579State
    data object S01 : Test579State
    data object S02 : Test579State
    data object S03 : Test579State
    data object S2 : Test579State
    data object S3 : Test579State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test579Event : Event {
    sealed interface Error : Test579Event {
        data object Execution : Error
    }
    data object Event1 : Test579Event
    data object Event2 : Test579Event
    data object Event3 : Test579Event
    data object Timeout : Test579Event
}
// --- State Machine (W3C SCXML) ---

class Test579StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test579State, Test579Event>(scriptEngine) {

    override val initialState: Test579State = Test579State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test579State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test579State): Test579State? = when (state) {
        is Test579State.S01 -> Test579State.S0
        is Test579State.S02 -> Test579State.S0
        is Test579State.S03 -> Test579State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test579State): Test579State = when (state) {
        is Test579State.S0 -> Test579State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test579Event? = when (name) {
        "error.execution" -> Test579Event.Error.Execution
        "event1" -> Test579Event.Event1
        "event2" -> Test579Event.Event2
        "event3" -> Test579Event.Event3
        "timeout" -> Test579Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test579Event): String? = when (event) {
        is Test579Event.Error.Execution -> "error.execution"
        is Test579Event.Event1 -> "event1"
        is Test579Event.Event2 -> "event2"
        is Test579Event.Event3 -> "event3"
        is Test579Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test579")




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
            raiseInternal(Test579Event.Error.Execution)
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
            raiseInternal(Test579Event.Error.Execution)
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
            raiseInternal(Test579Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test579Event) {
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
        state: Test579State,
        event: Test579Event
    ): TransitionResult<Test579State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test579State.S01 -> processS01(event)
        is Test579State.S02 -> processS02(event)
        is Test579State.S03 -> processS03(event)
        is Test579State.S2 -> processS2(event)
        is Test579State.S3 -> processS3(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS01(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event1 -> TransitionResult.External(Test579State.S02)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail)
    }

    private fun processS02(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event2 -> TransitionResult.External(Test579State.S03)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail)
    }

    private fun processS03(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event3 && safeEvaluateGuard("Var1 == 0") -> TransitionResult.External(Test579State.S0)
        event is Test579Event.Event1 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test579State.S2)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail)
    }

    private fun processS2(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event2 -> TransitionResult.External(Test579State.S3)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail)
    }

    private fun processS3(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event3 -> TransitionResult.External(Test579State.Fail)
        event is Test579Event.Timeout -> TransitionResult.External(Test579State.Pass)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test579State) {
        when (state) {
            is Test579State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test579State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test579State.S0 -> {
            scheduleSend("__send_0", 1000L, Test579Event.Timeout)
            raiseInternal(Test579Event.Event1)
                // W3C SCXML 3.3.2: Execute initial transition content
            raiseInternal(Test579Event.Event2)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test579State.S01)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test579State) {
        when (state) {
            is Test579State.S0 -> {
            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test579State,
        event: Test579Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
