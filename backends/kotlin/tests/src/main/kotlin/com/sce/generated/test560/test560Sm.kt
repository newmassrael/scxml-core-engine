// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2eaf0bfe80897ae515b0d732a8bb3914baa7c870ee8dd206a0a3dbc4956501d1
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/560/test560.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test560.scxml:5

package com.sce.generated.test560

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test560State : State {
    data object Fail : Test560State
    data object Pass : Test560State
    data object S0 : Test560State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test560Event : Event {
    sealed interface Error : Test560Event {
        data object Execution : Error
    }
    data object Foo : Test560Event
}
// --- State Machine (W3C SCXML) ---

class Test560StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test560State, Test560Event>(scriptEngine) {

    override val initialState: Test560State = Test560State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test560State? = when (stateId) {
        "fail" -> Test560State.Fail
        "pass" -> Test560State.Pass
        "s0" -> Test560State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test560State): String = when (state) {
        is Test560State.Fail -> "fail"
        is Test560State.Pass -> "pass"
        is Test560State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test560State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test560State): Int = when (state) {
        is Test560State.Fail -> 2
        is Test560State.Pass -> 1
        is Test560State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test560Event? = when (name) {
        "error.execution" -> Test560Event.Error.Execution
        "foo" -> Test560Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test560Event): String? = when (event) {
        is Test560Event.Error.Execution -> "error.execution"
        is Test560Event.Foo -> "foo"
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
            "test560",
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
            raiseInternal(Test560Event.Error.Execution)
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
            raiseInternal(Test560Event.Error.Execution)
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
            raiseInternal(Test560Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test560Event) {
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
        state: Test560State,
        event: Test560Event
    ): TransitionResult<Test560State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test560State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test560Event
    ): TransitionResult<Test560State> = when {
        event is Test560Event.Foo && safeEvaluateGuard("_event.data.aParam == 1") -> TransitionResult.External(Test560State.Pass, Test560State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test560State.Fail, Test560State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test560.scxml:5
    override fun onEntry(state: Test560State) {
        when (state) {
            is Test560State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test560State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test560State.S0 -> {
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
                send(Test560Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = eventDataE))
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test560.scxml:5
    override fun onExit(state: Test560State) {
        when (state) {
            is Test560State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test560State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test560State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test560.scxml:5
    override fun executeTransitionActions(
        source: Test560State,
        event: Test560Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
