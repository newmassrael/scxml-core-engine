// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 621809d272871a188158c7111f76c3b2585929cabb7a1e888c3ace81ca2d63d2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/553/test553.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test553.scxml:6

package com.sce.generated.test553

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test553State : State {
    data object Fail : Test553State
    data object Pass : Test553State
    data object S0 : Test553State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test553Event : Event {
    sealed interface Error : Test553Event {
        data object Execution : Error
    }
    data object Event1 : Test553Event
    data object Timeout : Test553Event
}
// --- State Machine (W3C SCXML) ---

class Test553StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test553State, Test553Event>(scriptEngine) {

    override val initialState: Test553State = Test553State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test553State? = when (stateId) {
        "fail" -> Test553State.Fail
        "pass" -> Test553State.Pass
        "s0" -> Test553State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test553State): String = when (state) {
        is Test553State.Fail -> "fail"
        is Test553State.Pass -> "pass"
        is Test553State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test553State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test553State): Int = when (state) {
        is Test553State.Fail -> 2
        is Test553State.Pass -> 1
        is Test553State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test553Event? = when (name) {
        "error.execution" -> Test553Event.Error.Execution
        "event1" -> Test553Event.Event1
        "timeout" -> Test553Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test553Event): String? = when (event) {
        is Test553Event.Error.Execution -> "error.execution"
        is Test553Event.Event1 -> "event1"
        is Test553Event.Timeout -> "timeout"
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
            "test553",
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
            raiseInternal(Test553Event.Error.Execution)
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
            raiseInternal(Test553Event.Error.Execution)
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
            raiseInternal(Test553Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test553Event) {
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
        state: Test553State,
        event: Test553Event
    ): TransitionResult<Test553State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test553State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test553Event
    ): TransitionResult<Test553State> = when {
        event is Test553Event.Timeout -> TransitionResult.External(Test553State.Pass, Test553State.S0)

        event is Test553Event.Event1 -> TransitionResult.External(Test553State.Fail, Test553State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test553.scxml:6
    override fun onEntry(state: Test553State) {
        when (state) {
            is Test553State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test553State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test553State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test553Event.Timeout)


            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern, test553)
                if (!engineE.hasVariable(sidE, "__undefined_variable_for_error__")) {
                    raiseInternal(Test553Event.Error.Execution)
                    return@run  // W3C SCXML 6.2: Abort send if namelist variable not found
                }
                try { paramsE["__undefined_variable_for_error__"] = engineE.getVariable(sidE, "__undefined_variable_for_error__") } catch (_: Exception) {
                    raiseInternal(Test553Event.Error.Execution)
                    return@run
                }
                val eventDataE = buildJsonFromParams(paramsE)
                send(Test553Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = eventDataE))
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test553.scxml:6
    override fun onExit(state: Test553State) {
        when (state) {
            is Test553State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test553State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test553State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test553.scxml:6
    override fun executeTransitionActions(
        source: Test553State,
        event: Test553Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
