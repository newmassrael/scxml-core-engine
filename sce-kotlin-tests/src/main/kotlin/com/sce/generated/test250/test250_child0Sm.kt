// GENERATED CODE — DO NOT EDIT
// Source: resources/250/test250_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test250

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test250Child0State : State {
    data object Sub0 : Test250Child0State
    data object Sub01 : Test250Child0State
    data object SubFinal : Test250Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test250Child0Event : Event {
    sealed interface Error : Test250Child0Event {
        data object Execution : Error
    }
    data object Timeout : Test250Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test250Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test250Child0State, Test250Child0Event>(scriptEngine) {

    override val initialState: Test250Child0State = Test250Child0State.Sub01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test250Child0State.Sub0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test250Child0State): Test250Child0State? = when (state) {
        is Test250Child0State.Sub01 -> Test250Child0State.Sub0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test250Child0State): Test250Child0State = when (state) {
        is Test250Child0State.Sub0 -> Test250Child0State.Sub01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test250Child0Event? = when (name) {
        "error.execution" -> Test250Child0Event.Error.Execution
        "timeout" -> Test250Child0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test250Child0Event): String? = when (event) {
        is Test250Child0Event.Error.Execution -> "error.execution"
        is Test250Child0Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test250_child0")




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
            raiseInternal(Test250Child0Event.Error.Execution)
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
            raiseInternal(Test250Child0Event.Error.Execution)
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
            raiseInternal(Test250Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test250Child0Event) {
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
        state: Test250Child0State,
        event: Test250Child0Event
    ): TransitionResult<Test250Child0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test250Child0State.Sub0 -> processSub0(event)
        // W3C SCXML 3.13: Ancestor-only routing (sub01 has no own event transitions)
        is Test250Child0State.Sub01 -> {
            val anc1 = processSub0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test250Child0Event
    ): TransitionResult<Test250Child0State> = when {
        event is Test250Child0Event.Timeout -> TransitionResult.External(Test250Child0State.SubFinal)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test250Child0State) {
        when (state) {
            is Test250Child0State.Sub0 -> {
            scheduleSend("__send_0", 2000L, Test250Child0Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test250Child0State.Sub01)
            }
            is Test250Child0State.SubFinal -> {
            println((scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'entering final state, invocation was not cancelled'")?.toString() ?: ""))
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test250Child0State) {
        when (state) {
            is Test250Child0State.Sub0 -> {
            println((scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'Exiting sub0'")?.toString() ?: ""))
            }
            is Test250Child0State.Sub01 -> {
            println((scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'Exiting sub01'")?.toString() ?: ""))
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test250Child0State,
        event: Test250Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
