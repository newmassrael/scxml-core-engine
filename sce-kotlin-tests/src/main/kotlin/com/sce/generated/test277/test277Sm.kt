// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955

// GENERATED CODE — DO NOT EDIT
// Source: resources/277/test277.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test277.scxml:6

package com.sce.generated.test277

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test277State : State {
    data object Fail : Test277State
    data object Pass : Test277State
    data object S0 : Test277State
    data object S1 : Test277State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test277Event : Event {
    sealed interface Error : Test277Event {
        data object Execution : Error
    }
    data object Foo : Test277Event
}
// --- State Machine (W3C SCXML) ---

class Test277StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test277State, Test277Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test277State = Test277State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test277State? = when (stateId) {
        "fail" -> Test277State.Fail
        "pass" -> Test277State.Pass
        "s0" -> Test277State.S0
        "s1" -> Test277State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test277State): String = when (state) {
        is Test277State.Fail -> "fail"
        is Test277State.Pass -> "pass"
        is Test277State.S0 -> "s0"
        is Test277State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test277State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test277State): Int = when (state) {
        is Test277State.Fail -> 3
        is Test277State.Pass -> 2
        is Test277State.S0 -> 0
        is Test277State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test277Event? = when (name) {
        "error.execution" -> Test277Event.Error.Execution
        "foo" -> Test277Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test277Event): String? = when (event) {
        is Test277Event.Error.Execution -> "error.execution"
        is Test277Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test277")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "undefined.invalidProperty")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test277Event.Error.Execution)
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
            raiseInternal(Test277Event.Error.Execution)
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
            raiseInternal(Test277Event.Error.Execution)
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
            raiseInternal(Test277Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test277Event) {
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
        state: Test277State,
        event: Test277Event
    ): TransitionResult<Test277State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test277State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test277State
    ): TransitionResult<Test277State> = when (state) {
        is Test277State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test277State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test277State.Pass, Test277State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test277State.Fail, Test277State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test277Event
    ): TransitionResult<Test277State> = when {
        event is Test277Event.Error.Execution -> TransitionResult.External(Test277State.S1, Test277State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test277State.Fail, Test277State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test277.scxml:6
    override fun onEntry(state: Test277State) {
        when (state) {
            is Test277State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test277State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test277State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test277Event.Foo)
            }
            is Test277State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            executeAssign("Var1", "1")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test277.scxml:6
    override fun onExit(state: Test277State) {
        when (state) {
            is Test277State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test277State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test277State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test277State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test277.scxml:6
    override fun executeTransitionActions(
        source: Test277State,
        event: Test277Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
