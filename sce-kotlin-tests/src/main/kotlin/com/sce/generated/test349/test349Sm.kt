// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: d9c7eeffd42250afac7bb84392f7db6b4e0a95d9e7e2e16957a4ecc188fd0aa8
// generated-at: 1779980218

// GENERATED CODE — DO NOT EDIT
// Source: resources/349/test349.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test349.scxml:5

package com.sce.generated.test349

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test349State : State {
    data object Fail : Test349State
    data object Pass : Test349State
    data object S0 : Test349State
    data object S2 : Test349State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test349Event : Event {
    sealed interface Error : Test349Event {
        data object Execution : Error
    }
    data object S0Event : Test349Event
    data object S0Event2 : Test349Event
}
// --- State Machine (W3C SCXML) ---

class Test349StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test349State, Test349Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test349State = Test349State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test349State? = when (stateId) {
        "fail" -> Test349State.Fail
        "pass" -> Test349State.Pass
        "s0" -> Test349State.S0
        "s2" -> Test349State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test349State): String = when (state) {
        is Test349State.Fail -> "fail"
        is Test349State.Pass -> "pass"
        is Test349State.S0 -> "s0"
        is Test349State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test349State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test349State): Int = when (state) {
        is Test349State.Fail -> 3
        is Test349State.Pass -> 2
        is Test349State.S0 -> 0
        is Test349State.S2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test349Event? = when (name) {
        "error.execution" -> Test349Event.Error.Execution
        "s0Event" -> Test349Event.S0Event
        "s0Event2" -> Test349Event.S0Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test349Event): String? = when (event) {
        is Test349Event.Error.Execution -> "error.execution"
        is Test349Event.S0Event -> "s0Event"
        is Test349Event.S0Event2 -> "s0Event2"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test349")

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
            raiseInternal(Test349Event.Error.Execution)
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
            raiseInternal(Test349Event.Error.Execution)
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
            raiseInternal(Test349Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test349Event) {
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
        state: Test349State,
        event: Test349Event
    ): TransitionResult<Test349State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test349State.S0 -> processS0(event)
        is Test349State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test349Event
    ): TransitionResult<Test349State> = when {
        event is Test349Event.S0Event -> TransitionResult.External(Test349State.S2, Test349State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test349State.Fail, Test349State.S0)
    }

    private fun processS2(
        event: Test349Event
    ): TransitionResult<Test349State> = when {
        event is Test349Event.S0Event2 -> TransitionResult.External(Test349State.Pass, Test349State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test349State.Fail, Test349State.S2)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test349.scxml:5
    override fun onEntry(state: Test349State) {
        when (state) {
            is Test349State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test349State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test349State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test349Event.S0Event, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is Test349State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            send(Test349Event.S0Event2, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test349.scxml:5
    override fun onExit(state: Test349State) {
        when (state) {
            is Test349State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test349State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test349State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test349State.S2 -> {
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test349.scxml:5
    override fun executeTransitionActions(
        source: Test349State,
        event: Test349Event?
    ) {
        when (source) {
        is Test349State.S0 -> when {
            event is Test349Event.S0Event -> {


            executeAssign("Var1", "_event.origin")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
