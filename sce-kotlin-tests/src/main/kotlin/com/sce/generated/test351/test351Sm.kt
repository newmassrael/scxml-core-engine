// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: cf4da7a0913513e15552dabfcd6b53678453b7b4dee1a56eee427fb0db26349a
// generated-at: 1780568754

// GENERATED CODE — DO NOT EDIT
// Source: resources/351/test351.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test351.scxml:5

package com.sce.generated.test351

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test351State : State {
    data object Fail : Test351State
    data object Pass : Test351State
    data object S0 : Test351State
    data object S1 : Test351State
    data object S2 : Test351State
    data object S3 : Test351State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test351Event : Event {
    sealed interface Error : Test351Event {
        data object Execution : Error
    }
    data object S0Event : Test351Event
    data object S0Event2 : Test351Event
    data object Timeout : Test351Event
}
// --- State Machine (W3C SCXML) ---

class Test351StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test351State, Test351Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test351State = Test351State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test351State? = when (stateId) {
        "fail" -> Test351State.Fail
        "pass" -> Test351State.Pass
        "s0" -> Test351State.S0
        "s1" -> Test351State.S1
        "s2" -> Test351State.S2
        "s3" -> Test351State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test351State): String = when (state) {
        is Test351State.Fail -> "fail"
        is Test351State.Pass -> "pass"
        is Test351State.S0 -> "s0"
        is Test351State.S1 -> "s1"
        is Test351State.S2 -> "s2"
        is Test351State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test351State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test351State): Int = when (state) {
        is Test351State.Fail -> 5
        is Test351State.Pass -> 4
        is Test351State.S0 -> 0
        is Test351State.S1 -> 1
        is Test351State.S2 -> 2
        is Test351State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test351Event? = when (name) {
        "error.execution" -> Test351Event.Error.Execution
        "s0Event" -> Test351Event.S0Event
        "s0Event2" -> Test351Event.S0Event2
        "timeout" -> Test351Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test351Event): String? = when (event) {
        is Test351Event.Error.Execution -> "error.execution"
        is Test351Event.S0Event -> "s0Event"
        is Test351Event.S0Event2 -> "s0Event2"
        is Test351Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test351")

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
            raiseInternal(Test351Event.Error.Execution)
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
            raiseInternal(Test351Event.Error.Execution)
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
            raiseInternal(Test351Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test351Event) {
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
        state: Test351State,
        event: Test351Event
    ): TransitionResult<Test351State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test351State.S0 -> processS0(event)
        is Test351State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test351State
    ): TransitionResult<Test351State> = when (state) {
        is Test351State.S1 -> processNullS1()
        is Test351State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test351State> = when {
        safeEvaluateGuard("Var1 == 'send1'") -> TransitionResult.External(Test351State.S2, Test351State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test351State.Fail, Test351State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test351State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test351State.Pass, Test351State.S3)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test351Event
    ): TransitionResult<Test351State> = when {
        event is Test351Event.S0Event -> TransitionResult.External(Test351State.S1, Test351State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test351State.Fail, Test351State.S0)
    }

    private fun processS2(
        event: Test351Event
    ): TransitionResult<Test351State> = when {
        event is Test351Event.S0Event2 -> TransitionResult.External(Test351State.S3, Test351State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test351State.Fail, Test351State.S2)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test351.scxml:5
    override fun onEntry(state: Test351State) {
        when (state) {
            is Test351State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test351State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test351State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test351Event.Timeout)


            send(Test351Event.S0Event, EventMetadata.external(sendId = "send1", origin = scriptSessionId ?: ""))
            }
            is Test351State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test351State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            scheduleSend("__send_1", 5000L, Test351Event.Timeout)


            send(Test351Event.S0Event2, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: ""))
            }
            is Test351State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test351.scxml:5
    override fun onExit(state: Test351State) {
        when (state) {
            is Test351State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test351State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test351State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test351State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test351State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test351State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test351.scxml:5
    override fun executeTransitionActions(
        source: Test351State,
        event: Test351Event?
    ) {
        when (source) {
        is Test351State.S0 -> when {
            event is Test351Event.S0Event -> {


            executeAssign("Var1", "_event.sendid")
            }
            else -> {}
        }
        is Test351State.S2 -> when {
            event is Test351Event.S0Event2 -> {


            executeAssign("Var2", "_event.sendid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
