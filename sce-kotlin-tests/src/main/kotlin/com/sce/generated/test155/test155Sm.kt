// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e798da33d5279236b681cdea18a53a3971a9b769ae5a0bc652a7f8fc89ca7b27
// generated-at: 1779450894

// GENERATED CODE — DO NOT EDIT
// Source: resources/155/test155.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test155.scxml:6

package com.sce.generated.test155

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test155State : State {
    data object Fail : Test155State
    data object Pass : Test155State
    data object S0 : Test155State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test155Event : Event {
    sealed interface Error : Test155Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test155StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test155State, Test155Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test155State = Test155State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test155State? = when (stateId) {
        "fail" -> Test155State.Fail
        "pass" -> Test155State.Pass
        "s0" -> Test155State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test155State): String = when (state) {
        is Test155State.Fail -> "fail"
        is Test155State.Pass -> "pass"
        is Test155State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test155State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test155State): Int = when (state) {
        is Test155State.Fail -> 2
        is Test155State.Pass -> 1
        is Test155State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test155Event? = when (name) {
        "error.execution" -> Test155Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test155Event): String? = when (event) {
        is Test155Event.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test155")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test155Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}
        // W3C SCXML B.2: Initialize variable 'Var3' with inline content (C++ parseEventData pattern)
        try {
            val initResult_Var3 = engine.parseDataValue(sid, "[1,2,3]")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raiseInternal(Test155Event.Error.Execution)
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
            raiseInternal(Test155Event.Error.Execution)
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
            raiseInternal(Test155Event.Error.Execution)
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
            raiseInternal(Test155Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test155Event) {
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
        state: Test155State,
        event: Test155Event
    ): TransitionResult<Test155State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test155State
    ): TransitionResult<Test155State> = when (state) {
        is Test155State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test155State> = when {
        safeEvaluateGuard("Var1 == 6") -> TransitionResult.External(Test155State.Pass, Test155State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test155State.Fail, Test155State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test155.scxml:6
    override fun onEntry(state: Test155State) {
        when (state) {
            is Test155State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test155State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test155State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, "Var3", "Var2", "") {


            engine.assign(sid, "Var1", "Var1 + Var2")
                    }
                } catch (e: Exception) {
                    raiseInternal(Test155Event.Error.Execution)
                }
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test155.scxml:6
    override fun onExit(state: Test155State) {
        when (state) {
            is Test155State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test155State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test155State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test155.scxml:6
    override fun executeTransitionActions(
        source: Test155State,
        event: Test155Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
