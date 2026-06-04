// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: cf4da7a0913513e15552dabfcd6b53678453b7b4dee1a56eee427fb0db26349a
// generated-at: 1780568754

// GENERATED CODE — DO NOT EDIT
// Source: resources/352/test352.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test352.scxml:4

package com.sce.generated.test352

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test352State : State {
    data object Fail : Test352State
    data object Pass : Test352State
    data object S0 : Test352State
    data object S1 : Test352State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test352Event : Event {
    sealed interface Error : Test352Event {
        data object Execution : Error
    }
    data object S0Event : Test352Event
    data object Timeout : Test352Event
}
// --- State Machine (W3C SCXML) ---

class Test352StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test352State, Test352Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test352State = Test352State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test352State? = when (stateId) {
        "fail" -> Test352State.Fail
        "pass" -> Test352State.Pass
        "s0" -> Test352State.S0
        "s1" -> Test352State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test352State): String = when (state) {
        is Test352State.Fail -> "fail"
        is Test352State.Pass -> "pass"
        is Test352State.S0 -> "s0"
        is Test352State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test352State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test352State): Int = when (state) {
        is Test352State.Fail -> 3
        is Test352State.Pass -> 2
        is Test352State.S0 -> 0
        is Test352State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test352Event? = when (name) {
        "error.execution" -> Test352Event.Error.Execution
        "s0Event" -> Test352Event.S0Event
        "timeout" -> Test352Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test352Event): String? = when (event) {
        is Test352Event.Error.Execution -> "error.execution"
        is Test352Event.S0Event -> "s0Event"
        is Test352Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test352")

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
            raiseInternal(Test352Event.Error.Execution)
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
            raiseInternal(Test352Event.Error.Execution)
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
            raiseInternal(Test352Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test352Event) {
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
        state: Test352State,
        event: Test352Event
    ): TransitionResult<Test352State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test352State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test352State
    ): TransitionResult<Test352State> = when (state) {
        is Test352State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test352State> = when {
        safeEvaluateGuard("Var1 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> TransitionResult.External(Test352State.Pass, Test352State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test352State.Fail, Test352State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test352Event
    ): TransitionResult<Test352State> = when {
        event is Test352Event.S0Event -> TransitionResult.External(Test352State.S1, Test352State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test352State.Fail, Test352State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test352.scxml:4
    override fun onEntry(state: Test352State) {
        when (state) {
            is Test352State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test352State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test352State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test352Event.Timeout)


            send(Test352Event.S0Event, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            is Test352State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test352.scxml:4
    override fun onExit(state: Test352State) {
        when (state) {
            is Test352State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test352State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test352State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test352State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test352.scxml:4
    override fun executeTransitionActions(
        source: Test352State,
        event: Test352Event?
    ) {
        when (source) {
        is Test352State.S0 -> when {
            event is Test352Event.S0Event -> {


            executeAssign("Var1", "_event.origintype")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
