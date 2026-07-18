// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c496f893fb4def171deba817f047a2a335356d181c631fa74825a157a7412c3e
// generated-at: 1784370263

// GENERATED CODE — DO NOT EDIT
// Source: resources/372/test372.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test372.scxml:6

package com.sce.generated.test372

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test372State : State {
    data object Fail : Test372State
    data object Pass : Test372State
    data object S0 : Test372State
    data object S0final : Test372State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test372Event : Event {
    sealed interface Done : Test372Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test372Event {
        data object Execution : Error
    }
    data object Timeout : Test372Event
}
// --- State Machine (W3C SCXML) ---

class Test372StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test372State, Test372Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test372State = Test372State.S0final

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test372State): Test372State? = when (state) {
        is Test372State.S0final -> Test372State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test372State): Test372State = when (state) {
        is Test372State.S0 -> Test372State.S0final
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test372State? = when (stateId) {
        "fail" -> Test372State.Fail
        "pass" -> Test372State.Pass
        "s0" -> Test372State.S0
        "s0final" -> Test372State.S0final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test372State): String = when (state) {
        is Test372State.Fail -> "fail"
        is Test372State.Pass -> "pass"
        is Test372State.S0 -> "s0"
        is Test372State.S0final -> "s0final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test372State): Boolean = when (state) {
        is Test372State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test372State): Int = when (state) {
        is Test372State.Fail -> 3
        is Test372State.Pass -> 2
        is Test372State.S0 -> 0
        is Test372State.S0final -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test372Event? = when (name) {
        "done.state.s0" -> Test372Event.Done.State.S0
        "error.execution" -> Test372Event.Error.Execution
        "timeout" -> Test372Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test372Event): String? = when (event) {
        is Test372Event.Done.State.S0 -> "done.state.s0"
        is Test372Event.Error.Execution -> "error.execution"
        is Test372Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test372")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test372Event.Error.Execution)
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
            raiseInternal(Test372Event.Error.Execution)
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
            raiseInternal(Test372Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test372Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test372Event) {
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
        state: Test372State,
        event: Test372Event
    ): TransitionResult<Test372State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test372State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s0final has no own event transitions)
        is Test372State.S0final -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test372Event
    ): TransitionResult<Test372State> = when {
        event is Test372Event.Done.State.S0 && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test372State.Pass, Test372State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test372State.Fail, Test372State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test372.scxml:6
    override fun onEntry(state: Test372State) {
        when (state) {
            is Test372State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test372State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test372State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test372Event.Timeout)
            }
            is Test372State.S0final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0final")) return


            executeAssign("Var1", "2")
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test372Event.Done.State.S0, EventMetadata.platform())
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test372.scxml:6
    override fun onExit(state: Test372State) {
        when (state) {
            is Test372State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test372State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test372State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test372State.S0final -> {
                activeStateIds.remove("s0final")


            executeAssign("Var1", "3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test372.scxml:6
    override fun executeTransitionActions(
        source: Test372State,
        event: Test372Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
