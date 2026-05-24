// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e58c03089e515b4f87df3e09e89234f06d61979361ed8fef1646aeb0069c2169
// generated-at: 1779596481

// GENERATED CODE — DO NOT EDIT
// Source: resources/403/test403b.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test403b.scxml:6

package com.sce.generated.test403b

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test403bState : State {
    data object Fail : Test403bState
    data object P0 : Test403bState
    data object P0s1 : Test403bState
    data object P0s2 : Test403bState
    data object Pass : Test403bState
    data object S0 : Test403bState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test403bEvent : Event {
    sealed interface Error : Test403bEvent {
        data object Execution : Error
    }
    data object Event1 : Test403bEvent
    data object Event2 : Test403bEvent
}
// --- State Machine (W3C SCXML) ---

class Test403bStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test403bState, Test403bEvent>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test403bState = Test403bState.P0s1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test403bState): Test403bState? = when (state) {
        is Test403bState.P0 -> Test403bState.S0
        is Test403bState.P0s1 -> Test403bState.P0
        is Test403bState.P0s2 -> Test403bState.P0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test403bState): Test403bState = when (state) {
        is Test403bState.P0 -> Test403bState.P0s1
        is Test403bState.S0 -> Test403bState.P0s1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test403bState? = when (stateId) {
        "fail" -> Test403bState.Fail
        "p0" -> Test403bState.P0
        "p0s1" -> Test403bState.P0s1
        "p0s2" -> Test403bState.P0s2
        "pass" -> Test403bState.Pass
        "s0" -> Test403bState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test403bState): String = when (state) {
        is Test403bState.Fail -> "fail"
        is Test403bState.P0 -> "p0"
        is Test403bState.P0s1 -> "p0s1"
        is Test403bState.P0s2 -> "p0s2"
        is Test403bState.Pass -> "pass"
        is Test403bState.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test403bState): Boolean = when (state) {
        is Test403bState.P0 -> false
        is Test403bState.S0 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test403bState): Boolean = when (state) {
        is Test403bState.P0 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test403bState): List<Test403bState> = when (state) {
        is Test403bState.P0 -> listOf(Test403bState.P0s1, Test403bState.P0s2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test403bState): Int = when (state) {
        is Test403bState.Fail -> 5
        is Test403bState.P0 -> 1
        is Test403bState.P0s1 -> 2
        is Test403bState.P0s2 -> 3
        is Test403bState.Pass -> 4
        is Test403bState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test403bEvent? = when (name) {
        "error.execution" -> Test403bEvent.Error.Execution
        "event1" -> Test403bEvent.Event1
        "event2" -> Test403bEvent.Event2
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test403bEvent): String? = when (event) {
        is Test403bEvent.Error.Execution -> "error.execution"
        is Test403bEvent.Event1 -> "event1"
        is Test403bEvent.Event2 -> "event2"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test403b")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test403bEvent.Error.Execution)
        }



        // W3C SCXML 5.9.2: Register In() predicate callback
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }

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
            raiseInternal(Test403bEvent.Error.Execution)
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
            raiseInternal(Test403bEvent.Error.Execution)
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
            raiseInternal(Test403bEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test403bEvent) {
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
        state: Test403bState,
        event: Test403bEvent
    ): TransitionResult<Test403bState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test403bState.P0s1 -> {
            val result = processP0s1(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processP0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (p0s2 has no own event transitions)
        is Test403bState.P0s2 -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else {
                val anc2 = processS0(event)
                if (anc2 !is TransitionResult.Ignored) anc2
            else TransitionResult.Ignored
            }
        }
        is Test403bState.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processP0(
        event: Test403bEvent
    ): TransitionResult<Test403bState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test403bEvent.Event1 -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processP0s1(
        event: Test403bEvent
    ): TransitionResult<Test403bState> = when {
        event is Test403bEvent.Event2 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test403bState.Pass, Test403bState.P0s1)

        event is Test403bEvent.Event2 -> TransitionResult.External(Test403bState.Fail, Test403bState.P0s1)

        else -> TransitionResult.Ignored
    }

    private fun processS0(
        event: Test403bEvent
    ): TransitionResult<Test403bState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test403bEvent.Event1 -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test403b.scxml:6
    override fun onEntry(state: Test403bState) {
        when (state) {
            is Test403bState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403bState.P0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0")) return

            raiseInternal(Test403bEvent.Event1)

            raiseInternal(Test403bEvent.Event2)
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test403bState.P0s1)
                onEntry(Test403bState.P0s2)
            }
            is Test403bState.P0s1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s1")) return
            }
            is Test403bState.P0s2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s2")) return
            }
            is Test403bState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403bState.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test403bState.P0)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test403b.scxml:6
    override fun onExit(state: Test403bState) {
        when (state) {
            is Test403bState.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test403bState.P0 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test403bState, Int>>()
                if (activeStateIds.contains("p0s1")) {
                    toExit.add(Test403bState.P0s1 to 2)
                }
                if (activeStateIds.contains("p0s2")) {
                    toExit.add(Test403bState.P0s2 to 3)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p0")
            }
            is Test403bState.P0s1 -> {
                activeStateIds.remove("p0s1")
            }
            is Test403bState.P0s2 -> {
                activeStateIds.remove("p0s2")
            }
            is Test403bState.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test403bState.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test403b.scxml:6
    override fun executeTransitionActions(
        source: Test403bState,
        event: Test403bEvent?
    ) {
        when (source) {
        is Test403bState.P0 -> when {
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.P0s1 -> when {
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.P0s2 -> when {
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.S0 -> when {
            event is Test403bEvent.Event1 -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
