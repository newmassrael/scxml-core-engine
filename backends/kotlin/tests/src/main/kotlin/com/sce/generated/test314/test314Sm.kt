// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/314/test314.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test314.scxml:6

package com.sce.generated.test314

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test314State : State {
    data object Fail : Test314State
    data object Pass : Test314State
    data object S0 : Test314State
    data object S01 : Test314State
    data object S02 : Test314State
    data object S03 : Test314State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test314Event : Event {
    sealed interface Error : Test314Event {
        data object Execution : Error
    }
    data object Foo : Test314Event
}
// --- State Machine (W3C SCXML) ---

class Test314StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test314State, Test314Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test314State = Test314State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test314State): Test314State? = when (state) {
        is Test314State.S01 -> Test314State.S0
        is Test314State.S02 -> Test314State.S0
        is Test314State.S03 -> Test314State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test314State): Test314State = when (state) {
        is Test314State.S0 -> Test314State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test314State? = when (stateId) {
        "fail" -> Test314State.Fail
        "pass" -> Test314State.Pass
        "s0" -> Test314State.S0
        "s01" -> Test314State.S01
        "s02" -> Test314State.S02
        "s03" -> Test314State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test314State): String = when (state) {
        is Test314State.Fail -> "fail"
        is Test314State.Pass -> "pass"
        is Test314State.S0 -> "s0"
        is Test314State.S01 -> "s01"
        is Test314State.S02 -> "s02"
        is Test314State.S03 -> "s03"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test314State): Boolean = when (state) {
        is Test314State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test314State): Int = when (state) {
        is Test314State.Fail -> 5
        is Test314State.Pass -> 4
        is Test314State.S0 -> 0
        is Test314State.S01 -> 1
        is Test314State.S02 -> 2
        is Test314State.S03 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test314Event? = when (name) {
        "error.execution" -> Test314Event.Error.Execution
        "foo" -> Test314Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test314Event): String? = when (event) {
        is Test314Event.Error.Execution -> "error.execution"
        is Test314Event.Foo -> "foo"
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
            "test314",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test314Event) {
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
        state: Test314State,
        event: Test314Event
    ): TransitionResult<Test314State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test314State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test314State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test314State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test314State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test314State
    ): TransitionResult<Test314State> = when (state) {
        is Test314State.S01 -> processNullS01()
        is Test314State.S02 -> processNullS02()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test314State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test314State.S02, Test314State.S01)
    }

    private fun processNullS02(
    ): TransitionResult<Test314State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test314State.S03, Test314State.S02)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test314Event
    ): TransitionResult<Test314State> = when {
        event is Test314Event.Error.Execution -> TransitionResult.External(Test314State.Fail, Test314State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test314Event
    ): TransitionResult<Test314State> = when {
        event is Test314Event.Error.Execution -> TransitionResult.External(Test314State.Pass, Test314State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test314State.Fail, Test314State.S03)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test314.scxml:6
    override fun onEntry(state: Test314State) {
        when (state) {
            is Test314State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test314State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test314State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test314State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test314State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test314State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return


            executeAssign("Var1", "undefined.invalidProperty")

            raiseInternal(Test314Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test314.scxml:6
    override fun onExit(state: Test314State) {
        when (state) {
            is Test314State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test314State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test314State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test314State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test314State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test314State.S03 -> {
                activeStateIds.remove("s03")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test314.scxml:6
    override fun executeTransitionActions(
        source: Test314State,
        event: Test314Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
