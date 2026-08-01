// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/186/test186.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test186.scxml:6

package com.sce.generated.test186

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test186State : State {
    data object Fail : Test186State
    data object Pass : Test186State
    data object S0 : Test186State
    data object S1 : Test186State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test186Event : Event {
    sealed interface Error : Test186Event {
        data object Execution : Error
    }
    data object Event1 : Test186Event
}
// --- State Machine (W3C SCXML) ---

class Test186StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test186State, Test186Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test186State = Test186State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test186State? = when (stateId) {
        "fail" -> Test186State.Fail
        "pass" -> Test186State.Pass
        "s0" -> Test186State.S0
        "s1" -> Test186State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test186State): String = when (state) {
        is Test186State.Fail -> "fail"
        is Test186State.Pass -> "pass"
        is Test186State.S0 -> "s0"
        is Test186State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test186State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test186State): Int = when (state) {
        is Test186State.Fail -> 3
        is Test186State.Pass -> 2
        is Test186State.S0 -> 0
        is Test186State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test186Event? = when (name) {
        "error.execution" -> Test186Event.Error.Execution
        "event1" -> Test186Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test186Event): String? = when (event) {
        is Test186Event.Error.Execution -> "error.execution"
        is Test186Event.Event1 -> "event1"
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
            "test186",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test186Event.Error.Execution)
        }
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
            raiseInternal(Test186Event.Error.Execution)
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
            raiseInternal(Test186Event.Error.Execution)
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
            raiseInternal(Test186Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test186Event) {
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
        state: Test186State,
        event: Test186Event
    ): TransitionResult<Test186State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test186State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test186State
    ): TransitionResult<Test186State> = when (state) {
        is Test186State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test186State> = when {
        safeEvaluateGuard("Var2 == 1") -> TransitionResult.External(Test186State.Pass, Test186State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test186State.Fail, Test186State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test186Event
    ): TransitionResult<Test186State> = when {
        event is Test186Event.Event1 -> TransitionResult.External(Test186State.S1, Test186State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test186State.Fail, Test186State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test186.scxml:6
    override fun onEntry(state: Test186State) {
        when (state) {
            is Test186State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test186State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test186State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                try { paramsE["aParam"] = engineE.evaluateExpr(sidE, "Var1") } catch (_: Exception) { paramsE["aParam"] = "" }
                val eventDataE = buildJsonFromParams(paramsE)
                scheduleSend("__send_0", 1000L, Test186Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = eventDataE))
            }


            executeAssign("Var1", "2")
            }
            is Test186State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test186.scxml:6
    override fun onExit(state: Test186State) {
        when (state) {
            is Test186State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test186State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test186State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test186State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test186.scxml:6
    override fun executeTransitionActions(
        source: Test186State,
        event: Test186Event?
    ) {
        when (source) {
        is Test186State.S0 -> when {
            event is Test186Event.Event1 -> {


            executeAssign("Var2", "_event.data.aParam")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
