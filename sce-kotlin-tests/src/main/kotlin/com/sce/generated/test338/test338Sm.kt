// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d2f0bcf4d5c727ad2446a904193402929b9b2d65dfec5e5c07ad3bc881483b09
// generated-at: 1780358475

// GENERATED CODE — DO NOT EDIT
// Source: resources/338/test338.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test338.scxml:6

package com.sce.generated.test338

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test338State : State {
    data object Fail : Test338State
    data object Pass : Test338State
    data object S0 : Test338State
    data object S1 : Test338State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test338Event : Event {
    sealed interface Cancel : Test338Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test338Event {
        data object Invoke : Done
    }
    sealed interface Error : Test338Event {
        data object Execution : Error
    }
    data object Event0 : Test338Event
    data object Event1 : Test338Event
    data object Timeout : Test338Event
}
// --- State Machine (W3C SCXML) ---

class Test338StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test338State, Test338Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test338State = Test338State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test338State? = when (stateId) {
        "fail" -> Test338State.Fail
        "pass" -> Test338State.Pass
        "s0" -> Test338State.S0
        "s1" -> Test338State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test338State): String = when (state) {
        is Test338State.Fail -> "fail"
        is Test338State.Pass -> "pass"
        is Test338State.S0 -> "s0"
        is Test338State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test338State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test338State): Int = when (state) {
        is Test338State.Fail -> 3
        is Test338State.Pass -> 2
        is Test338State.S0 -> 0
        is Test338State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test338Event? = when (name) {
        "cancel.invoke" -> Test338Event.Cancel.Invoke
        "done.invoke" -> Test338Event.Done.Invoke
        "error.execution" -> Test338Event.Error.Execution
        "event0" -> Test338Event.Event0
        "event1" -> Test338Event.Event1
        "timeout" -> Test338Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test338Event): String? = when (event) {
        is Test338Event.Cancel.Invoke -> "cancel.invoke"
        is Test338Event.Done.Invoke -> "done.invoke"
        is Test338Event.Error.Execution -> "error.execution"
        is Test338Event.Event0 -> "event0"
        is Test338Event.Event1 -> "event1"
        is Test338Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test338")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}




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
            raiseInternal(Test338Event.Error.Execution)
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
            raiseInternal(Test338Event.Error.Execution)
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
            raiseInternal(Test338Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test338Event) {
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
        state: Test338State,
        event: Test338Event
    ): TransitionResult<Test338State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test338State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test338State
    ): TransitionResult<Test338State> = when (state) {
        is Test338State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test338State> = when {
        safeEvaluateGuard("Var1 === Var2") -> TransitionResult.External(Test338State.Pass, Test338State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test338State.Fail, Test338State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test338Event
    ): TransitionResult<Test338State> = when {
        event is Test338Event.Event1 -> TransitionResult.External(Test338State.S1, Test338State.S0)

        event is Test338Event.Event0 -> TransitionResult.External(Test338State.Fail, Test338State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test338.scxml:6
    override fun onEntry(state: Test338State) {
        when (state) {
            is Test338State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test338State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test338State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test338Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4.1: Store generated invokeId in parent datamodel via idlocation
                    ensureScriptEngine()
                    scriptEngine?.let { eng ->
                        scriptSessionId?.let { sid ->
                            eng.setVariable(sid, "Var1", generatedInvokeId)
                        }
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test338SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test338Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test338State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test338.scxml:6
    override fun onExit(state: Test338State) {
        when (state) {
            is Test338State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test338State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test338State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test338State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test338.scxml:6
    override fun executeTransitionActions(
        source: Test338State,
        event: Test338Event?
    ) {
        when (source) {
        is Test338State.S0 -> when {
            event is Test338Event.Event1 -> {


            executeAssign("Var2", "_event.invokeid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
