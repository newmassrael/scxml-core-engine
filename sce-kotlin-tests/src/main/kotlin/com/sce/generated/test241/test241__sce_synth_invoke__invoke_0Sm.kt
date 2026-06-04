// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test241__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test241

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test241SceSynthInvokeInvoke0State : State {
    data object Sub01 : Test241SceSynthInvokeInvoke0State
    data object SubFinal1 : Test241SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test241SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test241SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Failure : Test241SceSynthInvokeInvoke0Event
    data object Success : Test241SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test241SceSynthInvokeInvoke0StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test241SceSynthInvokeInvoke0State, Test241SceSynthInvokeInvoke0Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test241SceSynthInvokeInvoke0State = Test241SceSynthInvokeInvoke0State.Sub01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test241SceSynthInvokeInvoke0State? = when (stateId) {
        "sub01" -> Test241SceSynthInvokeInvoke0State.Sub01
        "subFinal1" -> Test241SceSynthInvokeInvoke0State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test241SceSynthInvokeInvoke0State): String = when (state) {
        is Test241SceSynthInvokeInvoke0State.Sub01 -> "sub01"
        is Test241SceSynthInvokeInvoke0State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test241SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test241SceSynthInvokeInvoke0State): Int = when (state) {
        is Test241SceSynthInvokeInvoke0State.Sub01 -> 0
        is Test241SceSynthInvokeInvoke0State.SubFinal1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test241SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test241SceSynthInvokeInvoke0Event.Error.Execution
        "failure" -> Test241SceSynthInvokeInvoke0Event.Failure
        "success" -> Test241SceSynthInvokeInvoke0Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test241SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test241SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test241SceSynthInvokeInvoke0Event.Failure -> "failure"
        is Test241SceSynthInvokeInvoke0Event.Success -> "success"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test241__sce_synth_invoke__invoke_0")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test241SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test241SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test241SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test241SceSynthInvokeInvoke0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test241SceSynthInvokeInvoke0Event) {
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
        state: Test241SceSynthInvokeInvoke0State,
        event: Test241SceSynthInvokeInvoke0Event
    ): TransitionResult<Test241SceSynthInvokeInvoke0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test241SceSynthInvokeInvoke0State
    ): TransitionResult<Test241SceSynthInvokeInvoke0State> = when (state) {
        is Test241SceSynthInvokeInvoke0State.Sub01 -> processNullSub01()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub01(
    ): TransitionResult<Test241SceSynthInvokeInvoke0State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test241SceSynthInvokeInvoke0State.SubFinal1, Test241SceSynthInvokeInvoke0State.Sub01)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test241SceSynthInvokeInvoke0State.SubFinal1, Test241SceSynthInvokeInvoke0State.Sub01)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test241__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test241SceSynthInvokeInvoke0State) {
        when (state) {
            is Test241SceSynthInvokeInvoke0State.Sub01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub01")) return
            }
            is Test241SceSynthInvokeInvoke0State.SubFinal1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test241__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test241SceSynthInvokeInvoke0State) {
        when (state) {
            is Test241SceSynthInvokeInvoke0State.Sub01 -> {
                activeStateIds.remove("sub01")
            }
            is Test241SceSynthInvokeInvoke0State.SubFinal1 -> {
                activeStateIds.remove("subFinal1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test241__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test241SceSynthInvokeInvoke0State,
        event: Test241SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        is Test241SceSynthInvokeInvoke0State.Sub01 -> when {
            event == null && safeEvaluateGuard("Var1 == 1") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("failure", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
