// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: f835a323a3abc9cebc80341e1840b22b95739a2efa1726ad2c440477eff36482
// generated-at: 1781089257

// GENERATED CODE — DO NOT EDIT
// Source: resources/401/test401.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test401.scxml:7

package com.sce.generated.test401

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test401State : State {
    data object Fail : Test401State
    data object Pass : Test401State
    data object S0 : Test401State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test401Event : Event {
    sealed interface Error : Test401Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test401Event
}
// --- State Machine (W3C SCXML) ---

class Test401StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test401State, Test401Event>(scriptEngine) {

    override val initialState: Test401State = Test401State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test401State? = when (stateId) {
        "fail" -> Test401State.Fail
        "pass" -> Test401State.Pass
        "s0" -> Test401State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test401State): String = when (state) {
        is Test401State.Fail -> "fail"
        is Test401State.Pass -> "pass"
        is Test401State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test401State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test401State): Int = when (state) {
        is Test401State.Fail -> 2
        is Test401State.Pass -> 1
        is Test401State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test401Event? = when (name) {
        "error" -> Test401Event.Error.Self
        "error.execution" -> Test401Event.Error.Execution
        "foo" -> Test401Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test401Event): String? = when (event) {
        is Test401Event.Error.Self -> "error"
        is Test401Event.Error.Execution -> "error.execution"
        is Test401Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test401")





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
            raiseInternal(Test401Event.Error.Execution)
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
            raiseInternal(Test401Event.Error.Execution)
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
            raiseInternal(Test401Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test401Event) {
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
        state: Test401State,
        event: Test401Event
    ): TransitionResult<Test401State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test401State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test401Event
    ): TransitionResult<Test401State> = when {
        event is Test401Event.Foo -> TransitionResult.External(Test401State.Fail, Test401State.S0)

        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test401Event.Error || event is Test401Event.Error.Execution) -> TransitionResult.External(Test401State.Pass, Test401State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test401.scxml:7
    override fun onEntry(state: Test401State) {
        when (state) {
            is Test401State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test401State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test401State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            send(Test401Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raiseInternal(Test401Event.Error.Execution, EventMetadata.platform())
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test401.scxml:7
    override fun onExit(state: Test401State) {
        when (state) {
            is Test401State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test401State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test401State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test401.scxml:7
    override fun executeTransitionActions(
        source: Test401State,
        event: Test401Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
