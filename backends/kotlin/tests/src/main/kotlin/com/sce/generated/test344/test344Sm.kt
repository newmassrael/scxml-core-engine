// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/344/test344.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test344.scxml:7

package com.sce.generated.test344

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test344State : State {
    data object Fail : Test344State
    data object Pass : Test344State
    data object S0 : Test344State
    data object S1 : Test344State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test344Event : Event {
    sealed interface Error : Test344Event {
        data object Execution : Error
    }
    data object Foo : Test344Event
}
// --- State Machine (W3C SCXML) ---

class Test344StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test344State, Test344Event>(scriptEngine) {

    override val initialState: Test344State = Test344State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test344State? = when (stateId) {
        "fail" -> Test344State.Fail
        "pass" -> Test344State.Pass
        "s0" -> Test344State.S0
        "s1" -> Test344State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test344State): String = when (state) {
        is Test344State.Fail -> "fail"
        is Test344State.Pass -> "pass"
        is Test344State.S0 -> "s0"
        is Test344State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test344State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test344State): Int = when (state) {
        is Test344State.Fail -> 3
        is Test344State.Pass -> 2
        is Test344State.S0 -> 0
        is Test344State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test344Event? = when (name) {
        "error.execution" -> Test344Event.Error.Execution
        "foo" -> Test344Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test344Event): String? = when (event) {
        is Test344Event.Error.Execution -> "error.execution"
        is Test344Event.Foo -> "foo"
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
            "test344",
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
            raiseInternal(Test344Event.Error.Execution)
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
            raiseInternal(Test344Event.Error.Execution)
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
            raiseInternal(Test344Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test344Event) {
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
        state: Test344State,
        event: Test344Event
    ): TransitionResult<Test344State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test344State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test344State
    ): TransitionResult<Test344State> = when (state) {
        is Test344State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test344State> = when {
        safeEvaluateGuard("return") -> TransitionResult.External(Test344State.Fail, Test344State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test344State.S1, Test344State.S0)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test344Event
    ): TransitionResult<Test344State> = when {
        event is Test344Event.Error.Execution -> TransitionResult.External(Test344State.Pass, Test344State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test344State.Fail, Test344State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test344.scxml:7
    override fun onEntry(state: Test344State) {
        when (state) {
            is Test344State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test344State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test344State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test344State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test344Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test344.scxml:7
    override fun onExit(state: Test344State) {
        when (state) {
            is Test344State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test344State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test344State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test344State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test344.scxml:7
    override fun executeTransitionActions(
        source: Test344State,
        event: Test344Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
