// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712

// GENERATED CODE — DO NOT EDIT
// Source: resources/533/test533.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test533.scxml:5

package com.sce.generated.test533

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test533State : State {
    data object Fail : Test533State
    data object P : Test533State
    data object Pass : Test533State
    data object Ps1 : Test533State
    data object Ps2 : Test533State
    data object S1 : Test533State
    data object S2 : Test533State
    data object S3 : Test533State
    data object S4 : Test533State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test533Event : Event {
    data object Bar : Test533Event
    sealed interface Error : Test533Event {
        data object Execution : Error
    }
    data object Foo : Test533Event
}
// --- State Machine (W3C SCXML) ---

class Test533StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test533State, Test533Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test533State = Test533State.S1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test533State): Test533State? = when (state) {
        is Test533State.Ps1 -> Test533State.P
        is Test533State.Ps2 -> Test533State.P
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test533State): Test533State = when (state) {
        is Test533State.P -> Test533State.Ps1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test533State? = when (stateId) {
        "fail" -> Test533State.Fail
        "p" -> Test533State.P
        "pass" -> Test533State.Pass
        "ps1" -> Test533State.Ps1
        "ps2" -> Test533State.Ps2
        "s1" -> Test533State.S1
        "s2" -> Test533State.S2
        "s3" -> Test533State.S3
        "s4" -> Test533State.S4
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test533State): String = when (state) {
        is Test533State.Fail -> "fail"
        is Test533State.P -> "p"
        is Test533State.Pass -> "pass"
        is Test533State.Ps1 -> "ps1"
        is Test533State.Ps2 -> "ps2"
        is Test533State.S1 -> "s1"
        is Test533State.S2 -> "s2"
        is Test533State.S3 -> "s3"
        is Test533State.S4 -> "s4"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test533State): Boolean = when (state) {
        is Test533State.P -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test533State): Boolean = when (state) {
        is Test533State.P -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test533State): List<Test533State> = when (state) {
        is Test533State.P -> listOf(Test533State.Ps1, Test533State.Ps2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test533State): Int = when (state) {
        is Test533State.Fail -> 5
        is Test533State.P -> 6
        is Test533State.Pass -> 4
        is Test533State.Ps1 -> 7
        is Test533State.Ps2 -> 8
        is Test533State.S1 -> 0
        is Test533State.S2 -> 1
        is Test533State.S3 -> 2
        is Test533State.S4 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test533Event? = when (name) {
        "bar" -> Test533Event.Bar
        "error.execution" -> Test533Event.Error.Execution
        "foo" -> Test533Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test533Event): String? = when (event) {
        is Test533Event.Bar -> "bar"
        is Test533Event.Error.Execution -> "error.execution"
        is Test533Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test533")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test533Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test533Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var3' with expr
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raiseInternal(Test533Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raiseInternal(Test533Event.Error.Execution)
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
            raiseInternal(Test533Event.Error.Execution)
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
            raiseInternal(Test533Event.Error.Execution)
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
            raiseInternal(Test533Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test533Event) {
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
        state: Test533State,
        event: Test533Event
    ): TransitionResult<Test533State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (ps1 has no own event transitions)
        is Test533State.Ps1 -> {
            val anc1 = processP(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (ps2 has no own event transitions)
        is Test533State.Ps2 -> {
            val anc1 = processP(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test533State
    ): TransitionResult<Test533State> = when (state) {
        is Test533State.S1 -> processNullS1()
        is Test533State.S2 -> processNullS2()
        is Test533State.S3 -> processNullS3()
        is Test533State.S4 -> processNullS4()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test533State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test533State.P, Test533State.S1)
    }

    private fun processNullS2(
    ): TransitionResult<Test533State> = when {
        safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test533State.S3, Test533State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test533State.Fail, Test533State.S2)
    }

    private fun processNullS3(
    ): TransitionResult<Test533State> = when {
        safeEvaluateGuard("Var2 == 2") -> TransitionResult.External(Test533State.S4, Test533State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test533State.Fail, Test533State.S3)
    }

    private fun processNullS4(
    ): TransitionResult<Test533State> = when {
        safeEvaluateGuard("Var3 == 2") -> TransitionResult.External(Test533State.Pass, Test533State.S4)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test533State.Fail, Test533State.S4)
    }

    // --- Per-State Event Handlers ---

    private fun processP(
        event: Test533Event
    ): TransitionResult<Test533State> = when {
        event is Test533Event.Foo -> TransitionResult.External(Test533State.Ps1, Test533State.P)

        event is Test533Event.Bar && safeEvaluateGuard("Var4 == 1") -> TransitionResult.External(Test533State.S2, Test533State.P)

        event is Test533Event.Bar -> TransitionResult.External(Test533State.Fail, Test533State.P)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test533.scxml:5
    override fun onEntry(state: Test533State) {
        when (state) {
            is Test533State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test533State.P -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p")) return
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test533State.Ps1)
                onEntry(Test533State.Ps2)
            }
            is Test533State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test533State.Ps1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps1")) return
            }
            is Test533State.Ps2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ps2")) return
            }
            is Test533State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test533Event.Foo)

            raiseInternal(Test533Event.Bar)
            }
            is Test533State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test533State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
            is Test533State.S4 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s4")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test533.scxml:5
    override fun onExit(state: Test533State) {
        when (state) {
            is Test533State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test533State.P -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test533State, Int>>()
                if (activeStateIds.contains("ps1")) {
                    toExit.add(Test533State.Ps1 to 7)
                }
                if (activeStateIds.contains("ps2")) {
                    toExit.add(Test533State.Ps2 to 8)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p")


            executeAssign("Var1", "Var1 + 1")
            }
            is Test533State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test533State.Ps1 -> {
                activeStateIds.remove("ps1")


            executeAssign("Var2", "Var2 + 1")
            }
            is Test533State.Ps2 -> {
                activeStateIds.remove("ps2")


            executeAssign("Var3", "Var3 + 1")
            }
            is Test533State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test533State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test533State.S3 -> {
                activeStateIds.remove("s3")
            }
            is Test533State.S4 -> {
                activeStateIds.remove("s4")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test533.scxml:5
    override fun executeTransitionActions(
        source: Test533State,
        event: Test533Event?
    ) {
        when (source) {
        is Test533State.P -> when {
            event is Test533Event.Foo -> {


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        is Test533State.Ps1 -> when {
            event is Test533Event.Foo -> {


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        is Test533State.Ps2 -> when {
            event is Test533Event.Foo -> {


            executeAssign("Var4", "Var4 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
