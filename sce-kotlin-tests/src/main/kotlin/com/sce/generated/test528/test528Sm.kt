// GENERATED CODE — DO NOT EDIT
// Source: resources/528/test528.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test528

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test528State : State {
    data object Fail : Test528State
    data object Pass : Test528State
    data object S0 : Test528State
    data object S01 : Test528State
    data object S02 : Test528State
    data object S1 : Test528State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test528Event : Event {
    sealed interface Done : Test528Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test528Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test528StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test528State, Test528Event>(scriptEngine) {

    override val initialState: Test528State = Test528State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test528State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test528State): Test528State? = when (state) {
        is Test528State.S01 -> Test528State.S0
        is Test528State.S02 -> Test528State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test528State): Test528State = when (state) {
        is Test528State.S0 -> Test528State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test528Event? = when (name) {
        "done.state.s0" -> Test528Event.Done.State.S0
        "error.execution" -> Test528Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test528Event): String? = when (event) {
        is Test528Event.Done.State.S0 -> "done.state.s0"
        is Test528Event.Error.Execution -> "error.execution"
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
        engine.setupSystemVariables(sid, "test528")




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
            raiseInternal(Test528Event.Error.Execution)
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
            raiseInternal(Test528Event.Error.Execution)
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
            raiseInternal(Test528Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test528Event) {
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
        state: Test528State,
        event: Test528Event
    ): TransitionResult<Test528State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test528State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test528State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test528State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test528State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test528State
    ): TransitionResult<Test528State> = when (state) {
        is Test528State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test528State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test528State.S02)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test528Event
    ): TransitionResult<Test528State> = when {
        event is Test528Event.Error.Execution -> TransitionResult.External(Test528State.S1)
        event is Test528Event.Done.State.S0 -> TransitionResult.External(Test528State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test528Event
    ): TransitionResult<Test528State> = when {
        event is Test528Event.Done.State.S0 -> TransitionResult.External(Test528State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test528State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test528State) {
        when (state) {
            is Test528State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test528State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test528State.S0 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test528State.S01)
            }
            is Test528State.S02 -> {
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: return@run
                    val sidDD = scriptSessionId ?: return@run
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <content expr="..."/>
                    try {
                        val contentResult = engineDD.evaluateExpr(sidDD, "undefined.invalidProperty")
                        // C++ DoneDataHelper::evaluateContent: convertScriptValueToJson
                        doneEventData = if (contentResult != null) valueToJson(contentResult) else ""
                    } catch (_: Exception) {
                        raiseInternal(Test528Event.Error.Execution, EventMetadata.platform())
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test528Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test528State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test528State,
        event: Test528Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
