// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c1736039ea6628ae1068e428522a9d89bbe2ccef2705503db256c49ec169955e
// generated-at: 1778992486

// GENERATED CODE — DO NOT EDIT
// Source: resources/205/test205.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test205.scxml:6

package com.sce.generated.test205

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test205State : State {
    data object Fail : Test205State
    data object Pass : Test205State
    data object S0 : Test205State
    data object S1 : Test205State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test205Event : Event {
    sealed interface Error : Test205Event {
        data object Execution : Error
    }
    data object Event1 : Test205Event
    data object Timeout : Test205Event
}
// --- State Machine (W3C SCXML) ---

class Test205StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test205State, Test205Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test205State = Test205State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test205State? = when (stateId) {
        "fail" -> Test205State.Fail
        "pass" -> Test205State.Pass
        "s0" -> Test205State.S0
        "s1" -> Test205State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test205State): String = when (state) {
        is Test205State.Fail -> "fail"
        is Test205State.Pass -> "pass"
        is Test205State.S0 -> "s0"
        is Test205State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test205State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test205State): Int = when (state) {
        is Test205State.Fail -> 3
        is Test205State.Pass -> 2
        is Test205State.S0 -> 0
        is Test205State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test205Event? = when (name) {
        "error.execution" -> Test205Event.Error.Execution
        "event1" -> Test205Event.Event1
        "timeout" -> Test205Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test205Event): String? = when (event) {
        is Test205Event.Error.Execution -> "error.execution"
        is Test205Event.Event1 -> "event1"
        is Test205Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test205")

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
            raiseInternal(Test205Event.Error.Execution)
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
            raiseInternal(Test205Event.Error.Execution)
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
            raiseInternal(Test205Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test205Event) {
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
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test205State,
        event: Test205Event
    ): TransitionResult<Test205State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test205State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test205State
    ): TransitionResult<Test205State> = when (state) {
        is Test205State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test205State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test205State.Pass, Test205State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test205State.Fail, Test205State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test205Event
    ): TransitionResult<Test205State> = when {
        event is Test205Event.Event1 -> TransitionResult.External(Test205State.S1, Test205State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test205State.Fail, Test205State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test205.scxml:6
    override fun onEntry(state: Test205State) {
        when (state) {
            is Test205State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test205State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test205State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                try { paramsE["aParam"] = engineE.evaluateExpr(sidE, "1") } catch (_: Exception) { paramsE["aParam"] = "" }
                val eventDataE = buildJsonFromParams(paramsE)
                send(Test205Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = eventDataE))
            }


            send(Test205Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            is Test205State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test205.scxml:6
    override fun onExit(state: Test205State) {
        when (state) {
            is Test205State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test205State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test205State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test205State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test205.scxml:6
    override fun executeTransitionActions(
        source: Test205State,
        event: Test205Event?
    ) {
        when (source) {
        is Test205State.S0 -> when {
            event is Test205Event.Event1 -> {


            executeAssign("Var1", "_event.data.aParam")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
