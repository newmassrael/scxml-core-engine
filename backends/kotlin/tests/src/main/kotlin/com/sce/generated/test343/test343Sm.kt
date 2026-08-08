// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e9541de728219e5b918752124cad2b5ba2950a5da7bb328f3588c49d2bba35c4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/343/test343.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test343.scxml:4

package com.sce.generated.test343

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test343State : State {
    data object Fail : Test343State
    data object Pass : Test343State
    data object S0 : Test343State
    data object S01 : Test343State
    data object S02 : Test343State
    data object S1 : Test343State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test343Event : Event {
    sealed interface Done : Test343Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test343Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test343StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test343State, Test343Event>(scriptEngine) {

    override val initialState: Test343State = Test343State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test343State): Test343State? = when (state) {
        is Test343State.S01 -> Test343State.S0
        is Test343State.S02 -> Test343State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test343State): Test343State = when (state) {
        is Test343State.S0 -> Test343State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test343State? = when (stateId) {
        "fail" -> Test343State.Fail
        "pass" -> Test343State.Pass
        "s0" -> Test343State.S0
        "s01" -> Test343State.S01
        "s02" -> Test343State.S02
        "s1" -> Test343State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test343State): String = when (state) {
        is Test343State.Fail -> "fail"
        is Test343State.Pass -> "pass"
        is Test343State.S0 -> "s0"
        is Test343State.S01 -> "s01"
        is Test343State.S02 -> "s02"
        is Test343State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test343State): Boolean = when (state) {
        is Test343State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test343State): Int = when (state) {
        is Test343State.Fail -> 5
        is Test343State.Pass -> 4
        is Test343State.S0 -> 0
        is Test343State.S01 -> 1
        is Test343State.S02 -> 2
        is Test343State.S1 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test343Event? = when (name) {
        "done.state.s0" -> Test343Event.Done.State.S0
        "error.execution" -> Test343Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test343Event): String? = when (event) {
        is Test343Event.Done.State.S0 -> "done.state.s0"
        is Test343Event.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "test343",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )





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
            raiseInternal(Test343Event.Error.Execution)
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
            raiseInternal(Test343Event.Error.Execution)
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
            raiseInternal(Test343Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test343Event) {
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
        state: Test343State,
        event: Test343Event
    ): TransitionResult<Test343State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test343State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test343State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test343State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test343State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test343State
    ): TransitionResult<Test343State> = when (state) {
        is Test343State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test343State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test343State.S02, Test343State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test343Event
    ): TransitionResult<Test343State> = when {
        event is Test343Event.Error.Execution -> TransitionResult.External(Test343State.S1, Test343State.S0)

        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Fail, Test343State.S0)

        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Fail, Test343State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test343Event
    ): TransitionResult<Test343State> = when {
        event is Test343Event.Done.State.S0 -> TransitionResult.External(Test343State.Pass, Test343State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test343State.Fail, Test343State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test343.scxml:4
    override fun onEntry(state: Test343State) {
        when (state) {
            is Test343State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test343State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test343State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test343State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test343State.S02 -> {
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
                    try {
                        val locVal = engineDD.evaluateExpr(sidDD, "foo")
                        doneParams["someParam"] = locVal
                    } catch (_: Exception) {
                        // W3C SCXML 5.7: Runtime location error — raise error.execution but continue
                        raiseInternal(Test343Event.Error.Execution, EventMetadata.platform())
                    }
                    // C++ DoneDataHelper pattern: if (!success) break — skip done.state on structural error only
                    if (doneParamStructuralError) return@run
                    if (doneParams.isNotEmpty()) {
                        doneEventData = buildJsonFromParams(doneParams)
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test343Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
            is Test343State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test343.scxml:4
    override fun onExit(state: Test343State) {
        when (state) {
            is Test343State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test343State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test343State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test343State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test343State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test343State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test343.scxml:4
    override fun executeTransitionActions(
        source: Test343State,
        event: Test343Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
