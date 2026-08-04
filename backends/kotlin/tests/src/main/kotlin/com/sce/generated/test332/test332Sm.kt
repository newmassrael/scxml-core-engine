// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 50d6eb36f321e50c2a6e5457f0a900b925f832ee57619f9b6a33cf22bd75d4e1
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/332/test332.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test332.scxml:5

package com.sce.generated.test332

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test332State : State {
    data object Fail : Test332State
    data object Pass : Test332State
    data object S0 : Test332State
    data object S1 : Test332State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test332Event : Event {
    sealed interface Error : Test332Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test332Event
}
// --- State Machine (W3C SCXML) ---

class Test332StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test332State, Test332Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test332State = Test332State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test332State? = when (stateId) {
        "fail" -> Test332State.Fail
        "pass" -> Test332State.Pass
        "s0" -> Test332State.S0
        "s1" -> Test332State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test332State): String = when (state) {
        is Test332State.Fail -> "fail"
        is Test332State.Pass -> "pass"
        is Test332State.S0 -> "s0"
        is Test332State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test332State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test332State): Int = when (state) {
        is Test332State.Fail -> 3
        is Test332State.Pass -> 2
        is Test332State.S0 -> 0
        is Test332State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test332Event? = when (name) {
        "error" -> Test332Event.Error.Self
        "error.execution" -> Test332Event.Error.Execution
        "foo" -> Test332Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test332Event): String? = when (event) {
        is Test332Event.Error.Self -> "error"
        is Test332Event.Error.Execution -> "error.execution"
        is Test332Event.Foo -> "foo"
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
            "test332",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}
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
            raiseInternal(Test332Event.Error.Execution)
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
            raiseInternal(Test332Event.Error.Execution)
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
            raiseInternal(Test332Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test332Event) {
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
        state: Test332State,
        event: Test332Event
    ): TransitionResult<Test332State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test332State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test332State
    ): TransitionResult<Test332State> = when (state) {
        is Test332State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test332State> = when {
        safeEvaluateGuard("Var1 === Var2") -> TransitionResult.External(Test332State.Pass, Test332State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test332State.Fail, Test332State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test332Event
    ): TransitionResult<Test332State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test332Event.Error || event is Test332Event.Error.Execution) -> TransitionResult.External(Test332State.S1, Test332State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test332State.Fail, Test332State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test332.scxml:5
    override fun onEntry(state: Test332State) {
        when (state) {
            is Test332State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test332State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test332State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            // W3C SCXML 6.2.4: Store sendid in idlocation (test183, test332)
            run {
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try { eng.setVariable(sid, "Var1", "__send_0") } catch (_: Exception) {}
            }
            // W3C SCXML 6.2 (test194): Invalid target raises error.execution
            raiseInternal(Test332Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
            is Test332State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test332.scxml:5
    override fun onExit(state: Test332State) {
        when (state) {
            is Test332State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test332State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test332State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test332State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test332.scxml:5
    override fun executeTransitionActions(
        source: Test332State,
        event: Test332Event?
    ) {
        when (source) {
        is Test332State.S0 -> when {
            (event is Test332Event.Error || event is Test332Event.Error.Execution) -> {


            executeAssign("Var2", "_event.sendid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
