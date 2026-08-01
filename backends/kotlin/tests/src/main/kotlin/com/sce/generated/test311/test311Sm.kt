// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: e273e083fd84459760e6b7e00629aa0bbc396fdd49f2f0b96778152f02d02625
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/311/test311.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test311.scxml:5

package com.sce.generated.test311

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test311State : State {
    data object Fail : Test311State
    data object Pass : Test311State
    data object S0 : Test311State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test311Event : Event {
    sealed interface Error : Test311Event {
        data object Execution : Error
    }
    data object Timeout : Test311Event
}
// --- State Machine (W3C SCXML) ---

class Test311StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test311State, Test311Event>(scriptEngine) {

    override val initialState: Test311State = Test311State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test311State? = when (stateId) {
        "fail" -> Test311State.Fail
        "pass" -> Test311State.Pass
        "s0" -> Test311State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test311State): String = when (state) {
        is Test311State.Fail -> "fail"
        is Test311State.Pass -> "pass"
        is Test311State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test311State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test311State): Int = when (state) {
        is Test311State.Fail -> 2
        is Test311State.Pass -> 1
        is Test311State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test311Event? = when (name) {
        "error.execution" -> Test311Event.Error.Execution
        "timeout" -> Test311Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test311Event): String? = when (event) {
        is Test311Event.Error.Execution -> "error.execution"
        is Test311Event.Timeout -> "timeout"
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
            "test311",
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
            raiseInternal(Test311Event.Error.Execution)
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
            raiseInternal(Test311Event.Error.Execution)
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
            raiseInternal(Test311Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test311Event) {
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
        state: Test311State,
        event: Test311Event
    ): TransitionResult<Test311State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test311State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test311Event
    ): TransitionResult<Test311State> = when {
        event is Test311Event.Error.Execution -> TransitionResult.External(Test311State.Pass, Test311State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test311State.Fail, Test311State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test311.scxml:5
    override fun onEntry(state: Test311State) {
        when (state) {
            is Test311State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test311State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test311State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test311Event.Timeout)


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raiseInternal(Test311Event.Error.Execution, EventMetadata.platform())
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test311.scxml:5
    override fun onExit(state: Test311State) {
        when (state) {
            is Test311State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test311State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test311State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test311.scxml:5
    override fun executeTransitionActions(
        source: Test311State,
        event: Test311Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
