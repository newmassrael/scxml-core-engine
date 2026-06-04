// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 7b1a0066fa6a7fefceddfcf4d1e81b9d1fe50e95dd2b02645dfe86a65f3b96fe
// generated-at: 1780606838

// GENERATED CODE — DO NOT EDIT
// Source: resources/326/test326.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test326.scxml:4

package com.sce.generated.test326

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test326State : State {
    data object Fail : Test326State
    data object Pass : Test326State
    data object S0 : Test326State
    data object S1 : Test326State
    data object S2 : Test326State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test326Event : Event {
    sealed interface Error : Test326Event {
        data object Execution : Error
    }
    data object Foo : Test326Event
}
// --- State Machine (W3C SCXML) ---

class Test326StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test326State, Test326Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test326State = Test326State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test326State? = when (stateId) {
        "fail" -> Test326State.Fail
        "pass" -> Test326State.Pass
        "s0" -> Test326State.S0
        "s1" -> Test326State.S1
        "s2" -> Test326State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test326State): String = when (state) {
        is Test326State.Fail -> "fail"
        is Test326State.Pass -> "pass"
        is Test326State.S0 -> "s0"
        is Test326State.S1 -> "s1"
        is Test326State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test326State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test326State): Int = when (state) {
        is Test326State.Fail -> 4
        is Test326State.Pass -> 3
        is Test326State.S0 -> 0
        is Test326State.S1 -> 1
        is Test326State.S2 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test326Event? = when (name) {
        "error.execution" -> Test326Event.Error.Execution
        "foo" -> Test326Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test326Event): String? = when (event) {
        is Test326Event.Error.Execution -> "error.execution"
        is Test326Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test326")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "_ioprocessors")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test326Event.Error.Execution)
        }
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
            raiseInternal(Test326Event.Error.Execution)
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
            raiseInternal(Test326Event.Error.Execution)
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
            raiseInternal(Test326Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test326Event) {
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
        state: Test326State,
        event: Test326Event
    ): TransitionResult<Test326State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test326State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test326State
    ): TransitionResult<Test326State> = when (state) {
        is Test326State.S0 -> processNullS0()
        is Test326State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test326State> = when {
        safeEvaluateGuard("typeof Var1 !== 'undefined'") -> TransitionResult.External(Test326State.S1, Test326State.S0)
        safeEvaluateGuard("true") -> TransitionResult.External(Test326State.Fail, Test326State.S0)
        else -> TransitionResult.Ignored
    }

    private fun processNullS2(
    ): TransitionResult<Test326State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test326State.Pass, Test326State.S2)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test326Event
    ): TransitionResult<Test326State> = when {
        event is Test326Event.Error.Execution -> TransitionResult.External(Test326State.S2, Test326State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test326State.Fail, Test326State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test326.scxml:4
    override fun onEntry(state: Test326State) {
        when (state) {
            is Test326State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test326State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test326State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test326State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            executeAssign("_ioprocessors", "'otherName'")

            raiseInternal(Test326Event.Foo)
            }
            is Test326State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            executeAssign("Var2", "_ioprocessors")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test326.scxml:4
    override fun onExit(state: Test326State) {
        when (state) {
            is Test326State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test326State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test326State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test326State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test326State.S2 -> {
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test326.scxml:4
    override fun executeTransitionActions(
        source: Test326State,
        event: Test326Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
