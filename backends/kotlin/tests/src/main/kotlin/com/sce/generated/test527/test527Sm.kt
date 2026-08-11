// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 56bec87d0124f368b72ecb45f170dc38a324027a2fa3663195c8aeaa13f5d24d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/527/test527.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test527.scxml:4 :: _machine

package com.sce.generated.test527

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test527State : State {
    data object Fail : Test527State
    data object Pass : Test527State
    data object S0 : Test527State
    data object S01 : Test527State
    data object S02 : Test527State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test527Event : Event {
    sealed interface Done : Test527Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test527Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test527StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test527State, Test527Event>(scriptEngine) {

    override val initialState: Test527State = Test527State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test527State): Test527State? = when (state) {
        is Test527State.S01 -> Test527State.S0
        is Test527State.S02 -> Test527State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test527State): Test527State = when (state) {
        is Test527State.S0 -> Test527State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test527State? = when (stateId) {
        "fail" -> Test527State.Fail
        "pass" -> Test527State.Pass
        "s0" -> Test527State.S0
        "s01" -> Test527State.S01
        "s02" -> Test527State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test527State): String = when (state) {
        is Test527State.Fail -> "fail"
        is Test527State.Pass -> "pass"
        is Test527State.S0 -> "s0"
        is Test527State.S01 -> "s01"
        is Test527State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test527State): Boolean = when (state) {
        is Test527State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test527State): Int = when (state) {
        is Test527State.Fail -> 4
        is Test527State.Pass -> 3
        is Test527State.S0 -> 0
        is Test527State.S01 -> 1
        is Test527State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test527Event? = when (name) {
        "done.state.s0" -> Test527Event.Done.State.S0
        "error.execution" -> Test527Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test527Event): String? = when (event) {
        is Test527Event.Done.State.S0 -> "done.state.s0"
        is Test527Event.Error.Execution -> "error.execution"
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
            "test527",
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
            raiseInternal(Test527Event.Error.Execution)
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
            raiseInternal(Test527Event.Error.Execution)
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
            raiseInternal(Test527Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test527Event) {
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
        state: Test527State,
        event: Test527Event
    ): TransitionResult<Test527State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test527State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test527State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test527State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test527State
    ): TransitionResult<Test527State> = when (state) {
        is Test527State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test527State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test527State.S02, Test527State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test527Event
    ): TransitionResult<Test527State> = when {
        event is Test527Event.Done.State.S0 && safeEvaluateGuard("_event.data == 'foo'") -> TransitionResult.External(Test527State.Pass, Test527State.S0)

        event is Test527Event.Done.State.S0 -> TransitionResult.External(Test527State.Fail, Test527State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test527.scxml:4 :: _machine
    override fun onEntry(state: Test527State) {
        when (state) {
            is Test527State.Fail -> {
                // SCE-MAP: test527.scxml:26 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test527State.Pass -> {
                // SCE-MAP: test527.scxml:25 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test527State.S0 -> {
                // SCE-MAP: test527.scxml:7 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test527State.S01 -> {
                // SCE-MAP: test527.scxml:15 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test527State.S02 -> {
                // SCE-MAP: test527.scxml:18 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
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
                        raiseInternal(Test527Event.Error.Execution, EventMetadata.platform())
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test527Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test527.scxml:4 :: _machine
    override fun onExit(state: Test527State) {
        when (state) {
            is Test527State.Fail -> {
                // SCE-MAP: test527.scxml:26 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test527State.Pass -> {
                // SCE-MAP: test527.scxml:25 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test527State.S0 -> {
                // SCE-MAP: test527.scxml:7 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test527State.S01 -> {
                // SCE-MAP: test527.scxml:15 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test527State.S02 -> {
                // SCE-MAP: test527.scxml:18 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test527.scxml:4 :: _machine
    override fun executeTransitionActions(
        source: Test527State,
        event: Test527Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
