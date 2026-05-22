// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: bee566d0969cba6048cf66f73f5f775d02dafd3fb011e32cfb151e43f5c41677
// generated-at: 1779444436

// GENERATED CODE — DO NOT EDIT
// Source: resources/448/test448.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test448.scxml:5

package com.sce.generated.test448

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test448State : State {
    data object Fail : Test448State
    data object Pass : Test448State
    data object S0 : Test448State
    data object S01 : Test448State
    data object S01p : Test448State
    data object S01p1 : Test448State
    data object S01p2 : Test448State
    data object S1 : Test448State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test448Event : Event {
    sealed interface Error : Test448Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test448StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test448State, Test448Event>(scriptEngine) {

    override val initialState: Test448State = Test448State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test448State): Test448State? = when (state) {
        is Test448State.S01 -> Test448State.S0
        is Test448State.S01p -> Test448State.S1
        is Test448State.S01p1 -> Test448State.S01p
        is Test448State.S01p2 -> Test448State.S01p
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test448State): Test448State = when (state) {
        is Test448State.S0 -> Test448State.S01
        is Test448State.S01p -> Test448State.S01p1
        is Test448State.S1 -> Test448State.S01p1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test448State? = when (stateId) {
        "fail" -> Test448State.Fail
        "pass" -> Test448State.Pass
        "s0" -> Test448State.S0
        "s01" -> Test448State.S01
        "s01p" -> Test448State.S01p
        "s01p1" -> Test448State.S01p1
        "s01p2" -> Test448State.S01p2
        "s1" -> Test448State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test448State): String = when (state) {
        is Test448State.Fail -> "fail"
        is Test448State.Pass -> "pass"
        is Test448State.S0 -> "s0"
        is Test448State.S01 -> "s01"
        is Test448State.S01p -> "s01p"
        is Test448State.S01p1 -> "s01p1"
        is Test448State.S01p2 -> "s01p2"
        is Test448State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test448State): Boolean = when (state) {
        is Test448State.S0 -> false
        is Test448State.S01p -> false
        is Test448State.S1 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test448State): Boolean = when (state) {
        is Test448State.S01p -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test448State): List<Test448State> = when (state) {
        is Test448State.S01p -> listOf(Test448State.S01p1, Test448State.S01p2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test448State): Int = when (state) {
        is Test448State.Fail -> 7
        is Test448State.Pass -> 6
        is Test448State.S0 -> 0
        is Test448State.S01 -> 1
        is Test448State.S01p -> 3
        is Test448State.S01p1 -> 4
        is Test448State.S01p2 -> 5
        is Test448State.S1 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test448Event? = when (name) {
        "error.execution" -> Test448Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test448Event): String? = when (event) {
        is Test448Event.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test448")


        // W3C SCXML 5.3: Early binding — initialize state-level datamodel variables at startup
        // State 's01' variable 'var1'
        try {
            val initResult_var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raiseInternal(Test448Event.Error.Execution)
        }
        // State 's01p2' variable 'var2'
        try {
            val initResult_var2 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "var2", initResult_var2)
        } catch (e: Exception) {
            raiseInternal(Test448Event.Error.Execution)
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
            raiseInternal(Test448Event.Error.Execution)
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
            raiseInternal(Test448Event.Error.Execution)
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
            raiseInternal(Test448Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test448Event) {
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
        state: Test448State,
        event: Test448Event
    ): TransitionResult<Test448State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test448State
    ): TransitionResult<Test448State> = when (state) {
        is Test448State.S0 -> processNullS0()
        is Test448State.S01 -> processNullS0()
        is Test448State.S01p1 -> processNullS01p1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test448State> = when {
        safeEvaluateGuard("var1==1") -> TransitionResult.External(Test448State.S1, Test448State.S0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test448State.Fail, Test448State.S0)
    }

    private fun processNullS01p1(
    ): TransitionResult<Test448State> = when {
        safeEvaluateGuard("var2==1") -> TransitionResult.External(Test448State.Pass, Test448State.S01p1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test448State.Fail, Test448State.S01p1)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test448.scxml:5
    override fun onEntry(state: Test448State) {
        when (state) {
            is Test448State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test448State.S01)
                }
            }
            is Test448State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test448State.S01p -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test448State.S01p1)
                onEntry(Test448State.S01p2)
            }
            is Test448State.S01p1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p1")) return
            }
            is Test448State.S01p2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p2")) return
            }
            is Test448State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test448State.S01p)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test448.scxml:5
    override fun onExit(state: Test448State) {
        when (state) {
            is Test448State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test448State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test448State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test448State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test448State.S01p -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test448State, Int>>()
                if (activeStateIds.contains("s01p1")) {
                    toExit.add(Test448State.S01p1 to 4)
                }
                if (activeStateIds.contains("s01p2")) {
                    toExit.add(Test448State.S01p2 to 5)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("s01p")
            }
            is Test448State.S01p1 -> {
                activeStateIds.remove("s01p1")
            }
            is Test448State.S01p2 -> {
                activeStateIds.remove("s01p2")
            }
            is Test448State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test448.scxml:5
    override fun executeTransitionActions(
        source: Test448State,
        event: Test448Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
