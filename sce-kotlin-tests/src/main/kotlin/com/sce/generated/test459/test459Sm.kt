// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 32bb8515e09395468fbe442f393d8fa280b19e8eee3f4849a191223ea6d4c265
// generated-at: 1780369943

// GENERATED CODE — DO NOT EDIT
// Source: resources/459/test459.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test459.scxml:5

package com.sce.generated.test459

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test459State : State {
    data object Fail : Test459State
    data object Pass : Test459State
    data object S0 : Test459State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test459Event : Event {
    sealed interface Error : Test459Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test459StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test459State, Test459Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test459State = Test459State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test459State? = when (stateId) {
        "fail" -> Test459State.Fail
        "pass" -> Test459State.Pass
        "s0" -> Test459State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test459State): String = when (state) {
        is Test459State.Fail -> "fail"
        is Test459State.Pass -> "pass"
        is Test459State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test459State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test459State): Int = when (state) {
        is Test459State.Fail -> 2
        is Test459State.Pass -> 1
        is Test459State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test459Event? = when (name) {
        "error.execution" -> Test459Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test459Event): String? = when (event) {
        is Test459Event.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test459")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test459Event.Error.Execution)
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
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, "[1,2,3]")
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raiseInternal(Test459Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var5' with expr
        try {
            val initResult_Var5 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var5", initResult_Var5)
        } catch (e: Exception) {
            raiseInternal(Test459Event.Error.Execution)
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
            raiseInternal(Test459Event.Error.Execution)
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
            raiseInternal(Test459Event.Error.Execution)
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
            raiseInternal(Test459Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test459Event) {
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
        state: Test459State,
        event: Test459Event
    ): TransitionResult<Test459State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test459State
    ): TransitionResult<Test459State> = when (state) {
        is Test459State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test459State> = when {
        safeEvaluateGuard("Var4==0 | Var3 != 2") -> TransitionResult.External(Test459State.Fail, Test459State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test459State.Pass, Test459State.S0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test459.scxml:5
    override fun onEntry(state: Test459State) {
        when (state) {
            is Test459State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return

            // W3C SCXML 3.8.8: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'fail'")?.toString() ?: ""))
            } catch (_: Exception) {}
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test459State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return

            // W3C SCXML 3.8.8: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'pass'")?.toString() ?: ""))
            } catch (_: Exception) {}
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test459State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, "Var4", "Var2", "Var3") {


            if (safeEvaluateGuard("Var1<Var2")) {


            engine.assign(sid, "Var1", "Var2")
            } else {


            engine.assign(sid, "Var5", "0")
            }
                    }
                } catch (e: Exception) {
                    raiseInternal(Test459Event.Error.Execution)
                }
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test459.scxml:5
    override fun onExit(state: Test459State) {
        when (state) {
            is Test459State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test459State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test459State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test459.scxml:5
    override fun executeTransitionActions(
        source: Test459State,
        event: Test459Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
