// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c328b7a85ff2f465624a51fc9ec80940f3b78fbf4df26d1c6eaabfe6afd320f8
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/501/test501.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test501.scxml:5 :: _machine

package com.sce.generated.test501

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test501State : State {
    data object Fail : Test501State
    data object Pass : Test501State
    data object S0 : Test501State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test501Event : Event {
    sealed interface Error : Test501Event {
        data object Execution : Error
    }
    data object Foo : Test501Event
    data object Timeout : Test501Event
}
// --- State Machine (W3C SCXML) ---

class Test501StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test501State, Test501Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test501State = Test501State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test501State? = when (stateId) {
        "fail" -> Test501State.Fail
        "pass" -> Test501State.Pass
        "s0" -> Test501State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test501State): String = when (state) {
        is Test501State.Fail -> "fail"
        is Test501State.Pass -> "pass"
        is Test501State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test501State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test501State): Int = when (state) {
        is Test501State.Fail -> 2
        is Test501State.Pass -> 1
        is Test501State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test501Event? = when (name) {
        "error.execution" -> Test501Event.Error.Execution
        "foo" -> Test501Event.Foo
        "timeout" -> Test501Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test501Event): String? = when (event) {
        is Test501Event.Error.Execution -> "error.execution"
        is Test501Event.Foo -> "foo"
        is Test501Event.Timeout -> "timeout"
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
            "test501",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "_ioprocessors['scxml']['location']")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test501Event.Error.Execution)
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
            raiseInternal(Test501Event.Error.Execution)
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
            raiseInternal(Test501Event.Error.Execution)
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
            raiseInternal(Test501Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test501Event) {
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
        state: Test501State,
        event: Test501Event
    ): TransitionResult<Test501State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test501State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test501Event
    ): TransitionResult<Test501State> = when {
        event is Test501Event.Foo -> TransitionResult.External(Test501State.Pass, Test501State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test501State.Fail, Test501State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test501.scxml:5 :: _machine
    override fun onEntry(state: Test501State) {
        when (state) {
            is Test501State.Fail -> {
                // SCE-MAP: test501.scxml:21 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test501State.Pass -> {
                // SCE-MAP: test501.scxml:20 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test501State.S0 -> {
                // SCE-MAP: test501.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test501Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            scheduleSend("__send_1", 2000L, Test501Event.Timeout)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test501.scxml:5 :: _machine
    override fun onExit(state: Test501State) {
        when (state) {
            is Test501State.Fail -> {
                // SCE-MAP: test501.scxml:21 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test501State.Pass -> {
                // SCE-MAP: test501.scxml:20 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test501State.S0 -> {
                // SCE-MAP: test501.scxml:10 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test501.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test501State,
        event: Test501Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
