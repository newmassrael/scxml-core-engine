// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 7afc591fda192b42ad8a433570c001416f9be57edde17b6193960abf579021c2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/529/test529.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test529.scxml:4

package com.sce.generated.test529

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test529State : State {
    data object Fail : Test529State
    data object Pass : Test529State
    data object S0 : Test529State
    data object S01 : Test529State
    data object S02 : Test529State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test529Event : Event {
    sealed interface Done : Test529Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test529Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test529StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test529State, Test529Event>(scriptEngine) {

    override val initialState: Test529State = Test529State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test529State): Test529State? = when (state) {
        is Test529State.S01 -> Test529State.S0
        is Test529State.S02 -> Test529State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test529State): Test529State = when (state) {
        is Test529State.S0 -> Test529State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test529State? = when (stateId) {
        "fail" -> Test529State.Fail
        "pass" -> Test529State.Pass
        "s0" -> Test529State.S0
        "s01" -> Test529State.S01
        "s02" -> Test529State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test529State): String = when (state) {
        is Test529State.Fail -> "fail"
        is Test529State.Pass -> "pass"
        is Test529State.S0 -> "s0"
        is Test529State.S01 -> "s01"
        is Test529State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test529State): Boolean = when (state) {
        is Test529State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test529State): Int = when (state) {
        is Test529State.Fail -> 4
        is Test529State.Pass -> 3
        is Test529State.S0 -> 0
        is Test529State.S01 -> 1
        is Test529State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test529Event? = when (name) {
        "done.state.s0" -> Test529Event.Done.State.S0
        "error.execution" -> Test529Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test529Event): String? = when (event) {
        is Test529Event.Done.State.S0 -> "done.state.s0"
        is Test529Event.Error.Execution -> "error.execution"
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
            "test529",
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
            raiseInternal(Test529Event.Error.Execution)
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
            raiseInternal(Test529Event.Error.Execution)
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
            raiseInternal(Test529Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test529Event) {
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
        state: Test529State,
        event: Test529Event
    ): TransitionResult<Test529State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test529State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test529State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test529State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test529State
    ): TransitionResult<Test529State> = when (state) {
        is Test529State.S01 -> processNullS01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test529State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test529State.S02, Test529State.S01)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test529Event
    ): TransitionResult<Test529State> = when {
        event is Test529Event.Done.State.S0 && safeEvaluateGuard("_event.data == 21") -> TransitionResult.External(Test529State.Pass, Test529State.S0)

        event is Test529Event.Done.State.S0 -> TransitionResult.External(Test529State.Fail, Test529State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test529.scxml:4
    override fun onEntry(state: Test529State) {
        when (state) {
            is Test529State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test529State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test529State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test529State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test529State.S02 -> {
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
                        val contentResult = engineDD.evaluateExpr(sidDD, "21")
                        // C++ DoneDataHelper::evaluateContent: EventDataHelper::scriptValueToJsonString
                        doneEventData = if (contentResult != null) valueToJson(contentResult) else ""
                    } catch (_: Exception) {
                        raiseInternal(Test529Event.Error.Execution, EventMetadata.platform())
                    }
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test529Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test529.scxml:4
    override fun onExit(state: Test529State) {
        when (state) {
            is Test529State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test529State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test529State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test529State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test529State.S02 -> {
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test529.scxml:4
    override fun executeTransitionActions(
        source: Test529State,
        event: Test529Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
