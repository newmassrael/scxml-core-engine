// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa58405544015ba4d1b8207b13e783fe4f4b991c1d05b4cc1602d85ec7348310
// generated-at: 1785367096

// GENERATED CODE — DO NOT EDIT
// Source: resources/298/test298.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test298.scxml:4

package com.sce.generated.test298

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test298State : State {
    data object Fail : Test298State
    data object Pass : Test298State
    data object S0 : Test298State
    data object S01 : Test298State
    data object S02 : Test298State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test298Event : Event {
    sealed interface Done : Test298Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test298Event {
        data object Execution : Error
    }
    data object Timeout : Test298Event
}
// --- State Machine (W3C SCXML) ---

class Test298StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test298State, Test298Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test298State = Test298State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test298State): Test298State? = when (state) {
        is Test298State.S01 -> Test298State.S0
        is Test298State.S02 -> Test298State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test298State): Test298State = when (state) {
        is Test298State.S0 -> Test298State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test298State? = when (stateId) {
        "fail" -> Test298State.Fail
        "pass" -> Test298State.Pass
        "s0" -> Test298State.S0
        "s01" -> Test298State.S01
        "s02" -> Test298State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test298State): String = when (state) {
        is Test298State.Fail -> "fail"
        is Test298State.Pass -> "pass"
        is Test298State.S0 -> "s0"
        is Test298State.S01 -> "s01"
        is Test298State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test298State): Boolean = when (state) {
        is Test298State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test298State): Int = when (state) {
        is Test298State.Fail -> 4
        is Test298State.Pass -> 3
        is Test298State.S0 -> 0
        is Test298State.S01 -> 1
        is Test298State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test298Event? = when (name) {
        "done.state.s0" -> Test298Event.Done.State.S0
        "error.execution" -> Test298Event.Error.Execution
        "timeout" -> Test298Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test298Event): String? = when (event) {
        is Test298Event.Done.State.S0 -> "done.state.s0"
        is Test298Event.Error.Execution -> "error.execution"
        is Test298Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test298")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test298Event.Error.Execution)
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
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test298Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test298Event.Error.Execution)
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test298Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test298Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
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
            sid,
            com.sce.runtime.SetCurrentEventArgs(
                name = eventName,
                data = meta.data,
                type = effectiveType,
                sendId = meta.sendId,
                origin = effectiveOrigin,
                originType = effectiveOriginType,
                invokeId = meta.invokeId
            )
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test298State,
        event: Test298Event
    ): TransitionResult<Test298State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test298State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test298State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test298State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test298State
    ): TransitionResult<Test298State> = when (state) {
        is Test298State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test298State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test298State.S02, Test298State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test298Event
    ): TransitionResult<Test298State> = when {
        event is Test298Event.Error.Execution -> TransitionResult.External(Test298State.Pass, Test298State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test298State.Fail, Test298State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test298.scxml:4
    override fun onEntry(state: Test298State) {
        when (state) {
            is Test298State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test298State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test298State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test298Event.Timeout)
            }
            is Test298State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test298State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    var doneParamStructuralError = false
                    // W3C SCXML 5.7: Empty location — structural error (C++ DoneDataHelper returns false)
                    raiseInternal(Test298Event.Error.Execution, EventMetadata.platform())
                    doneParamStructuralError = true
                    // C++ DoneDataHelper pattern: if (!success) break — skip done.state on structural error only
                    if (doneParamStructuralError) return@run
                    if (doneParams.isNotEmpty()) {
                        doneEventData = buildJsonFromParams(doneParams)
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test298Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test298.scxml:4
    override fun onExit(state: Test298State) {
        when (state) {
            is Test298State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test298State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test298State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test298State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test298State.S02 -> {
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test298.scxml:4
    override fun executeTransitionActions(
        source: Test298State,
        event: Test298Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
