// GENERATED CODE — DO NOT EDIT
// Source: resources/402/test402.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test402

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test402State : State {
    data object Fail : Test402State
    data object Pass : Test402State
    data object S0 : Test402State
    data object S01 : Test402State
    data object S02 : Test402State
    data object S03 : Test402State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test402Event : Event {
    sealed interface Error : Test402Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Event1 : Test402Event
    data object Event2 : Test402Event
    data object Timeout : Test402Event
}
// --- State Machine (W3C SCXML) ---

class Test402StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test402State, Test402Event>(scriptEngine) {

    override val initialState: Test402State = Test402State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test402State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test402State): Test402State? = when (state) {
        is Test402State.S01 -> Test402State.S0
        is Test402State.S02 -> Test402State.S0
        is Test402State.S03 -> Test402State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test402State): Test402State = when (state) {
        is Test402State.S0 -> Test402State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test402Event? = when (name) {
        "error" -> Test402Event.Error.Self
        "error.execution" -> Test402Event.Error.Execution
        "event1" -> Test402Event.Event1
        "event2" -> Test402Event.Event2
        "timeout" -> Test402Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test402Event): String? = when (event) {
        is Test402Event.Error.Self -> "error"
        is Test402Event.Error.Execution -> "error.execution"
        is Test402Event.Event1 -> "event1"
        is Test402Event.Event2 -> "event2"
        is Test402Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test402")




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
            raiseInternal(Test402Event.Error.Execution)
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
            raiseInternal(Test402Event.Error.Execution)
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
            raiseInternal(Test402Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test402Event) {
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
        state: Test402State,
        event: Test402Event
    ): TransitionResult<Test402State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test402State.S0 -> processS0(event)
        is Test402State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test402State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test402State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test402Event
    ): TransitionResult<Test402State> = when {
        event is Test402Event.Timeout -> TransitionResult.External(Test402State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test402Event
    ): TransitionResult<Test402State> = when {
        event is Test402Event.Event1 -> TransitionResult.External(Test402State.S02)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test402State.Fail)
    }

    private fun processS02(
        event: Test402Event
    ): TransitionResult<Test402State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test402Event.Error || event is Test402Event.Error.Execution) -> TransitionResult.External(Test402State.S03)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test402State.Fail)
    }

    private fun processS03(
        event: Test402Event
    ): TransitionResult<Test402State> = when {
        event is Test402Event.Event2 -> TransitionResult.External(Test402State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test402State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test402State) {
        when (state) {
            is Test402State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test402State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test402State.S0 -> {
            scheduleSend("__send_0", 1000L, Test402Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test402State.S01)
            }
            is Test402State.S01 -> {
            raiseInternal(Test402Event.Event1)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test402State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test402State,
        event: Test402Event?
    ) {
        when (source) {
        is Test402State.S01 -> when {
            event is Test402Event.Event1 -> {
            raiseInternal(Test402Event.Event2)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
