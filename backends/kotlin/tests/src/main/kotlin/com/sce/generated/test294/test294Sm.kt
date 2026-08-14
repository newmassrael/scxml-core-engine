// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/294/test294.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test294.scxml:5 :: _machine

package com.sce.generated.test294

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test294State : State {
    data object Fail : Test294State
    data object Pass : Test294State
    data object S0 : Test294State
    data object S01 : Test294State
    data object S02 : Test294State
    data object S1 : Test294State
    data object S11 : Test294State
    data object S12 : Test294State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test294Event : Event {
    sealed interface Done : Test294Event {
        sealed interface State : Done {
            data object S0 : State
            data object S1 : State
        }
    }
    sealed interface Error : Test294Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test294StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test294State, Test294Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test294State = Test294State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test294State): Test294State? = when (state) {
        is Test294State.S01 -> Test294State.S0
        is Test294State.S02 -> Test294State.S0
        is Test294State.S11 -> Test294State.S1
        is Test294State.S12 -> Test294State.S1
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test294State): Test294State = when (state) {
        is Test294State.S0 -> Test294State.S01
        is Test294State.S1 -> Test294State.S11
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test294State? = when (stateId) {
        "fail" -> Test294State.Fail
        "pass" -> Test294State.Pass
        "s0" -> Test294State.S0
        "s01" -> Test294State.S01
        "s02" -> Test294State.S02
        "s1" -> Test294State.S1
        "s11" -> Test294State.S11
        "s12" -> Test294State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test294State): String = when (state) {
        is Test294State.Fail -> "fail"
        is Test294State.Pass -> "pass"
        is Test294State.S0 -> "s0"
        is Test294State.S01 -> "s01"
        is Test294State.S02 -> "s02"
        is Test294State.S1 -> "s1"
        is Test294State.S11 -> "s11"
        is Test294State.S12 -> "s12"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test294State): Boolean = when (state) {
        is Test294State.S0 -> false
        is Test294State.S1 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test294State): Int = when (state) {
        is Test294State.Fail -> 7
        is Test294State.Pass -> 6
        is Test294State.S0 -> 0
        is Test294State.S01 -> 1
        is Test294State.S02 -> 2
        is Test294State.S1 -> 3
        is Test294State.S11 -> 4
        is Test294State.S12 -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test294Event? = when (name) {
        "done.state.s0" -> Test294Event.Done.State.S0
        "done.state.s1" -> Test294Event.Done.State.S1
        "error.execution" -> Test294Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test294Event): String? = when (event) {
        is Test294Event.Done.State.S0 -> "done.state.s0"
        is Test294Event.Done.State.S1 -> "done.state.s1"
        is Test294Event.Error.Execution -> "error.execution"
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
            "test294",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test294Event.Error.Execution)
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
            raiseInternal(Test294Event.Error.Execution)
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
            raiseInternal(Test294Event.Error.Execution)
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
            raiseInternal(Test294Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test294Event) {
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
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
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
        state: Test294State,
        event: Test294Event
    ): TransitionResult<Test294State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test294State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test294State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test294State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test294State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test294State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s12 has no own event transitions)
        is Test294State.S12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test294State
    ): TransitionResult<Test294State> = when (state) {
        is Test294State.S01 -> processNullS01()
        is Test294State.S11 -> processNullS11()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test294State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test294State.S02, Test294State.S01)
    }

    private fun processNullS11(
    ): TransitionResult<Test294State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test294State.S12, Test294State.S11)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test294Event
    ): TransitionResult<Test294State> = when {
        event is Test294Event.Done.State.S0 && safeEvaluateGuard("_event.data.Var1 == 1") -> TransitionResult.External(Test294State.S1, Test294State.S0)

        event is Test294Event.Done.State.S0 -> TransitionResult.External(Test294State.Fail, Test294State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test294Event
    ): TransitionResult<Test294State> = when {
        event is Test294Event.Done.State.S1 && safeEvaluateGuard("_event.data == 'foo'") -> TransitionResult.External(Test294State.Pass, Test294State.S1)

        event is Test294Event.Done.State.S1 -> TransitionResult.External(Test294State.Fail, Test294State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test294.scxml:5 :: _machine
    override fun onEntry(state: Test294State) {
        when (state) {
            is Test294State.Fail -> {
                // SCE-MAP: test294.scxml:47 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test294State.Pass -> {
                // SCE-MAP: test294.scxml:46 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test294State.S0 -> {
                // SCE-MAP: test294.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test294State.S01 -> {
                // SCE-MAP: test294.scxml:18 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test294State.S02 -> {
                // SCE-MAP: test294.scxml:21 :: s02 :: _state_body
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
                        doneParams["Var1"] = engineDD.evaluateExpr(sidDD, "1")
                    } catch (_: Exception) {
                        // W3C SCXML 5.7: Runtime param error — raise error.execution but continue
                        raiseInternal(Test294Event.Error.Execution, EventMetadata.platform())
                    }
                    // C++ DoneDataHelper pattern: if (!success) break — skip done.state on structural error only
                    if (doneParamStructuralError) return@run
                    if (doneParams.isNotEmpty()) {
                        doneEventData = buildJsonFromParams(doneParams)
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test294Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
            is Test294State.S1 -> {
                // SCE-MAP: test294.scxml:28 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test294State.S11 -> {
                // SCE-MAP: test294.scxml:36 :: s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
            }
            is Test294State.S12 -> {
                // SCE-MAP: test294.scxml:39 :: s12 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s12")) return
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <content expr="..."/>
                    try {
                        val contentResult = engineDD.evaluateExpr(sidDD, "'foo'")
                        // C++ DoneDataHelper::evaluateContent: EventDataHelper::scriptValueToJsonString
                        doneEventData = if (contentResult != null) valueToJson(contentResult) else ""
                    } catch (_: Exception) {
                        raiseInternal(Test294Event.Error.Execution, EventMetadata.platform())
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test294Event.Done.State.S1, EventMetadata.platform(doneEventData))
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test294.scxml:5 :: _machine
    override fun onExit(state: Test294State) {
        when (state) {
            is Test294State.Fail -> {
                // SCE-MAP: test294.scxml:47 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test294State.Pass -> {
                // SCE-MAP: test294.scxml:46 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test294State.S0 -> {
                // SCE-MAP: test294.scxml:10 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test294State.S01 -> {
                // SCE-MAP: test294.scxml:18 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test294State.S02 -> {
                // SCE-MAP: test294.scxml:21 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
            is Test294State.S1 -> {
                // SCE-MAP: test294.scxml:28 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test294State.S11 -> {
                // SCE-MAP: test294.scxml:36 :: s11 :: _state_body
                activeStateIds.remove("s11")
            }
            is Test294State.S12 -> {
                // SCE-MAP: test294.scxml:39 :: s12 :: _state_body
                activeStateIds.remove("s12")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test294.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test294State,
        event: Test294Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
