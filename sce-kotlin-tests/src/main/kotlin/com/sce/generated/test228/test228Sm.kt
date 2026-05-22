// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d588114b3294b4cb4d7e02d63e6d31a3c0326d3afa0a691deb12b545b5ff5045
// generated-at: 1779460271

// GENERATED CODE — DO NOT EDIT
// Source: resources/228/test228.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test228.scxml:5

package com.sce.generated.test228

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test228State : State {
    data object Fail : Test228State
    data object Pass : Test228State
    data object S0 : Test228State
    data object S1 : Test228State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test228Event : Event {
    sealed interface Cancel : Test228Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test228Event {
        data object Invoke : Done
    }
    sealed interface Error : Test228Event {
        data object Execution : Error
    }
    data object Timeout : Test228Event
}
// --- State Machine (W3C SCXML) ---

class Test228StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test228State, Test228Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test228State = Test228State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test228State? = when (stateId) {
        "fail" -> Test228State.Fail
        "pass" -> Test228State.Pass
        "s0" -> Test228State.S0
        "s1" -> Test228State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test228State): String = when (state) {
        is Test228State.Fail -> "fail"
        is Test228State.Pass -> "pass"
        is Test228State.S0 -> "s0"
        is Test228State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test228State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test228State): Int = when (state) {
        is Test228State.Fail -> 3
        is Test228State.Pass -> 2
        is Test228State.S0 -> 0
        is Test228State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test228Event? = when (name) {
        "cancel.invoke" -> Test228Event.Cancel.Invoke
        "done.invoke" -> Test228Event.Done.Invoke
        "error.execution" -> Test228Event.Error.Execution
        "timeout" -> Test228Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test228Event): String? = when (event) {
        is Test228Event.Cancel.Invoke -> "cancel.invoke"
        is Test228Event.Done.Invoke -> "done.invoke"
        is Test228Event.Error.Execution -> "error.execution"
        is Test228Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test228")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
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
            raiseInternal(Test228Event.Error.Execution)
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
            raiseInternal(Test228Event.Error.Execution)
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
            raiseInternal(Test228Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test228Event) {
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
        state: Test228State,
        event: Test228Event
    ): TransitionResult<Test228State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test228State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test228State
    ): TransitionResult<Test228State> = when (state) {
        is Test228State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test228State> = when {
        safeEvaluateGuard("Var1 == 'foo'") -> TransitionResult.External(Test228State.Pass, Test228State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test228State.Fail, Test228State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test228Event
    ): TransitionResult<Test228State> = when {
        event is Test228Event.Done.Invoke -> TransitionResult.External(Test228State.S1, Test228State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test228State.Fail, Test228State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test228.scxml:5
    override fun onEntry(state: Test228State) {
        when (state) {
            is Test228State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test228State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test228State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test228Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}.foo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test228SceSynthInvokeFooStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("foo", childSM, false, Test228Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test228State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test228.scxml:5
    override fun onExit(state: Test228State) {
        when (state) {
            is Test228State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test228State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test228State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("foo")
                activeStateIds.remove("s0")
            }
            is Test228State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test228.scxml:5
    override fun executeTransitionActions(
        source: Test228State,
        event: Test228Event?
    ) {
        when (source) {
        is Test228State.S0 -> when {
            event is Test228Event.Done.Invoke -> {


            executeAssign("Var1", "_event.invokeid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
