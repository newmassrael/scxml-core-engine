// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425

// GENERATED CODE — DO NOT EDIT
// Source: resources/234/test234.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test234.scxml:8

package com.sce.generated.test234

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test234State : State {
    data object Fail : Test234State
    data object P0 : Test234State
    data object P01 : Test234State
    data object P02 : Test234State
    data object Pass : Test234State
    data object S1 : Test234State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test234Event : Event {
    sealed interface Cancel : Test234Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test234Event
    sealed interface Done : Test234Event {
        data object Invoke : Done
    }
    sealed interface Error : Test234Event {
        data object Execution : Error
    }
    data object Timeout : Test234Event
}
// --- State Machine (W3C SCXML) ---

class Test234StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test234State, Test234Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test234State = Test234State.P01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test234State): Test234State? = when (state) {
        is Test234State.P01 -> Test234State.P0
        is Test234State.P02 -> Test234State.P0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test234State): Test234State = when (state) {
        is Test234State.P0 -> Test234State.P01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test234State? = when (stateId) {
        "fail" -> Test234State.Fail
        "p0" -> Test234State.P0
        "p01" -> Test234State.P01
        "p02" -> Test234State.P02
        "pass" -> Test234State.Pass
        "s1" -> Test234State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test234State): String = when (state) {
        is Test234State.Fail -> "fail"
        is Test234State.P0 -> "p0"
        is Test234State.P01 -> "p01"
        is Test234State.P02 -> "p02"
        is Test234State.Pass -> "pass"
        is Test234State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test234State): Boolean = when (state) {
        is Test234State.P0 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test234State): Boolean = when (state) {
        is Test234State.P0 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test234State): List<Test234State> = when (state) {
        is Test234State.P0 -> listOf(Test234State.P01, Test234State.P02)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test234State): Int = when (state) {
        is Test234State.Fail -> 2
        is Test234State.P0 -> 3
        is Test234State.P01 -> 4
        is Test234State.P02 -> 5
        is Test234State.Pass -> 1
        is Test234State.S1 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test234Event? = when (name) {
        "cancel.invoke" -> Test234Event.Cancel.Invoke
        "childToParent" -> Test234Event.ChildToParent
        "done.invoke" -> Test234Event.Done.Invoke
        "error.execution" -> Test234Event.Error.Execution
        "timeout" -> Test234Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test234Event): String? = when (event) {
        is Test234Event.Cancel.Invoke -> "cancel.invoke"
        is Test234Event.ChildToParent -> "childToParent"
        is Test234Event.Done.Invoke -> "done.invoke"
        is Test234Event.Error.Execution -> "error.execution"
        is Test234Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test234")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test234Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test234Event.Error.Execution)
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
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test234Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test234Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test234Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test234Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
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
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test234State,
        event: Test234Event
    ): TransitionResult<Test234State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test234State.P01 -> {
            val result = processP01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processP0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (p02 has no own event transitions)
        is Test234State.P02 -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test234State
    ): TransitionResult<Test234State> = when (state) {
        is Test234State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test234State> = when {
        safeEvaluateGuard("Var2 == 1") -> TransitionResult.External(Test234State.Pass, Test234State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test234State.Fail, Test234State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processP0(
        event: Test234Event
    ): TransitionResult<Test234State> = when {
        event is Test234Event.Timeout -> TransitionResult.External(Test234State.Fail, Test234State.P0)

        else -> TransitionResult.Ignored
    }

    private fun processP01(
        event: Test234Event
    ): TransitionResult<Test234State> = when {
        event is Test234Event.ChildToParent && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test234State.S1, Test234State.P01)

        event is Test234Event.ChildToParent -> TransitionResult.External(Test234State.Fail, Test234State.P01)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test234.scxml:8
    override fun onEntry(state: Test234State) {
        when (state) {
            is Test234State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test234State.P0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0")) return


            scheduleSend("__send_0", 3000L, Test234Event.Timeout)
                // W3C SCXML 3.4: Parallel states ALWAYS enter all child regions
                // (not affected by suppressChildEntry — C++ buildEntryChain includes parallel children)
                onEntry(Test234State.P01)
                onEntry(Test234State.P02)
            }
            is Test234State.P01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p01")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "p01.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test234SceSynthInvokeInvoke0StateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test234Event.Done.Invoke, "Var1 = _event.data.aParam;", generatedInvokeId)
                    }
                }
            }
            is Test234State.P02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p02")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "p02.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test234SceSynthInvokeInvoke1StateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test234Event.Done.Invoke, "Var2 = _event.data.aParam;", generatedInvokeId)
                    }
                }
            }
            is Test234State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test234State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test234.scxml:8
    override fun onExit(state: Test234State) {
        when (state) {
            is Test234State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test234State.P0 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test234State, Int>>()
                if (activeStateIds.contains("p01")) {
                    toExit.add(Test234State.P01 to 4)
                }
                if (activeStateIds.contains("p02")) {
                    toExit.add(Test234State.P02 to 5)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p0")
            }
            is Test234State.P01 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("p01")
            }
            is Test234State.P02 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("p02")
            }
            is Test234State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test234State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test234.scxml:8
    override fun executeTransitionActions(
        source: Test234State,
        event: Test234Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
