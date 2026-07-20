// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 35c0d03dd34b8d03e7b3891d6751af3cdd0b2bf0e96c5f94ca9790ac72375270
// generated-at: 1784525850

// GENERATED CODE — DO NOT EDIT
// Source: resources/354/test354.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test354.scxml:6

package com.sce.generated.test354

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test354State : State {
    data object Fail : Test354State
    data object Pass : Test354State
    data object S0 : Test354State
    data object S1 : Test354State
    data object S2 : Test354State
    data object S3 : Test354State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test354Event : Event {
    sealed interface Error : Test354Event {
        data object Execution : Error
    }
    data object Event1 : Test354Event
    data object Event2 : Test354Event
    data object Timeout : Test354Event
}
// --- State Machine (W3C SCXML) ---

class Test354StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test354State, Test354Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test354State = Test354State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test354State? = when (stateId) {
        "fail" -> Test354State.Fail
        "pass" -> Test354State.Pass
        "s0" -> Test354State.S0
        "s1" -> Test354State.S1
        "s2" -> Test354State.S2
        "s3" -> Test354State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test354State): String = when (state) {
        is Test354State.Fail -> "fail"
        is Test354State.Pass -> "pass"
        is Test354State.S0 -> "s0"
        is Test354State.S1 -> "s1"
        is Test354State.S2 -> "s2"
        is Test354State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test354State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test354State): Int = when (state) {
        is Test354State.Fail -> 5
        is Test354State.Pass -> 4
        is Test354State.S0 -> 0
        is Test354State.S1 -> 1
        is Test354State.S2 -> 2
        is Test354State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test354Event? = when (name) {
        "error.execution" -> Test354Event.Error.Execution
        "event1" -> Test354Event.Event1
        "event2" -> Test354Event.Event2
        "timeout" -> Test354Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test354Event): String? = when (event) {
        is Test354Event.Error.Execution -> "error.execution"
        is Test354Event.Event1 -> "event1"
        is Test354Event.Event2 -> "event2"
        is Test354Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test354")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test354Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var3' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var3", null)
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
            raiseInternal(Test354Event.Error.Execution)
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
            raiseInternal(Test354Event.Error.Execution)
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
            raiseInternal(Test354Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test354Event) {
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
        state: Test354State,
        event: Test354Event
    ): TransitionResult<Test354State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test354State.S0 -> processS0(event)
        is Test354State.S3 -> processS3(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test354State
    ): TransitionResult<Test354State> = when (state) {
        is Test354State.S1 -> processNullS1()
        is Test354State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test354State> = when {
        safeEvaluateGuard("Var2 == 1") -> TransitionResult.External(Test354State.S2, Test354State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test354State.Fail, Test354State.S1)
    }

    private fun processNullS2(
    ): TransitionResult<Test354State> = when {
        safeEvaluateGuard("Var3 == 2") -> TransitionResult.External(Test354State.S3, Test354State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test354State.Fail, Test354State.S2)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test354Event
    ): TransitionResult<Test354State> = when {
        event is Test354Event.Event1 -> TransitionResult.External(Test354State.S1, Test354State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test354State.Fail, Test354State.S0)
    }

    private fun processS3(
        event: Test354Event
    ): TransitionResult<Test354State> = when {
        event is Test354Event.Event2 -> TransitionResult.External(Test354State.Pass, Test354State.S3)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test354State.Fail, Test354State.S3)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test354.scxml:6
    override fun onEntry(state: Test354State) {
        when (state) {
            is Test354State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test354State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test354State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test354Event.Timeout)


            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                try { paramsE["param1"] = engineE.evaluateExpr(sidE, "2") } catch (_: Exception) { paramsE["param1"] = "" }
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern, test553)
                if (!engineE.hasVariable(sidE, "Var1")) {
                    raiseInternal(Test354Event.Error.Execution)
                    return@run  // W3C SCXML 6.2: Abort send if namelist variable not found
                }
                try { paramsE["Var1"] = engineE.getVariable(sidE, "Var1") } catch (_: Exception) {
                    raiseInternal(Test354Event.Error.Execution)
                    return@run
                }
                val eventDataE = buildJsonFromParams(paramsE)
                send(Test354Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = eventDataE))
            }
            }
            is Test354State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test354State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test354State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return


            scheduleSend("__send_2", 5000L, Test354Event.Timeout)


            send(Test354Event.Event2, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test354.scxml:6
    override fun onExit(state: Test354State) {
        when (state) {
            is Test354State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test354State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test354State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test354State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test354State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test354State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test354.scxml:6
    override fun executeTransitionActions(
        source: Test354State,
        event: Test354Event?
    ) {
        when (source) {
        is Test354State.S0 -> when {
            event is Test354Event.Event1 -> {


            executeAssign("Var2", "_event.data.Var1")


            executeAssign("Var3", "_event.data.param1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
