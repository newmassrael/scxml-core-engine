// GENERATED CODE — DO NOT EDIT
// Source: resources/580/test580.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test580

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test580State : State {
    data object Fail : Test580State
    data object P1 : Test580State
    data object Pass : Test580State
    data object S0 : Test580State
    data object S1 : Test580State
    data object S11 : Test580State
    data object S12 : Test580State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test580Event : Event {
    sealed interface Error : Test580Event {
        data object Execution : Error
    }
    data object Timeout : Test580Event
}
// --- State Machine (W3C SCXML) ---

class Test580StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test580State, Test580Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test580State = Test580State.S0

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test580State.P1)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test580State): Test580State? = when (state) {
        is Test580State.S0 -> Test580State.P1
        is Test580State.S1 -> Test580State.P1
        is Test580State.S11 -> Test580State.S1
        is Test580State.S12 -> Test580State.S1
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test580State): Test580State = when (state) {
        is Test580State.P1 -> Test580State.S0
        is Test580State.S1 -> Test580State.S11
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object (for parallel processing)
    override fun resolveState(stateId: String): Test580State? = when (stateId) {
        "fail" -> Test580State.Fail
        "p1" -> Test580State.P1
        "pass" -> Test580State.Pass
        "s0" -> Test580State.S0
        "s1" -> Test580State.S1
        "s11" -> Test580State.S11
        "s12" -> Test580State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test580State): String = when (state) {
        is Test580State.Fail -> "fail"
        is Test580State.P1 -> "p1"
        is Test580State.Pass -> "pass"
        is Test580State.S0 -> "s0"
        is Test580State.S1 -> "s1"
        is Test580State.S11 -> "s11"
        is Test580State.S12 -> "s12"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test580State): Boolean = when (state) {
        is Test580State.P1 -> false
        is Test580State.S1 -> false
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test580State): Int = when (state) {
        is Test580State.Fail -> 1
        is Test580State.P1 -> 2
        is Test580State.Pass -> 0
        is Test580State.S0 -> 3
        is Test580State.S1 -> 4
        is Test580State.S11 -> 5
        is Test580State.S12 -> 6
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test580Event? = when (name) {
        "error.execution" -> Test580Event.Error.Execution
        "timeout" -> Test580Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test580Event): String? = when (event) {
        is Test580Event.Error.Execution -> "error.execution"
        is Test580Event.Timeout -> "timeout"
        else -> null
    }


    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test580")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test580Event.Error.Execution)
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
            raiseInternal(Test580Event.Error.Execution)
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
            raiseInternal(Test580Event.Error.Execution)
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
            raiseInternal(Test580Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test580Event) {
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
        state: Test580State,
        event: Test580Event
    ): TransitionResult<Test580State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test580State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test580State
    ): TransitionResult<Test580State> = when (state) {
        is Test580State.S0 -> processNullS0()
        is Test580State.S1 -> processNullS1()
        is Test580State.S11 -> {
            val null1 = processNullS11()
            if (null1 !is TransitionResult.Ignored) null1
            else {
                val null2 = processNullS1()
                if (null2 !is TransitionResult.Ignored) null2
            else TransitionResult.Ignored
            }
        }
        is Test580State.S12 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test580State> = when {
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processNullS1(
    ): TransitionResult<Test580State> = when {
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail)
        safeEvaluateGuard("Var1 == 0") -> TransitionResult.External((historyStore["sh1"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test580State.S11))
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test580State.Pass)
        else -> TransitionResult.Ignored
    }

    private fun processNullS11(
    ): TransitionResult<Test580State> = when {
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test580State.S12)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test580Event
    ): TransitionResult<Test580State> = when {
        event is Test580Event.Timeout -> TransitionResult.External(Test580State.Fail, Test580State.S0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test580State) {
        when (state) {
            is Test580State.Fail -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.P1 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("p1")) return
            scheduleSend("__send_0", 2000L, Test580Event.Timeout)
                // W3C SCXML 3.4: Enter all child regions of parallel state
                onEntry(Test580State.S0)
                onEntry(Test580State.S1)
            }
            is Test580State.Pass -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.S0 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s0")) return
            }
            is Test580State.S1 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s1")) return
                // W3C SCXML 3.11: Enter history-restored state or default target
                run {
                    val stored = historyStore["sh1"]
                    if (stored != null && stored.isNotEmpty()) {
                        val histTarget = resolveState(stored[0])
                        if (histTarget != null) {
                            onEntry(histTarget)
                        } else {
                            onEntry(Test580State.S11)
                        }
                    } else {
                        onEntry(Test580State.S11)
                    }
                }
            }
            is Test580State.S11 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s11")) return
            }
            is Test580State.S12 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s12")) return
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test580State) {
        when (state) {
            is Test580State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test580State.P1 -> {
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test580State, Int>>()
                if (activeStateIds.contains("s0")) {
                    toExit.add(Test580State.S0 to 3)
                }
                if (activeStateIds.contains("s1")) {
                    toExit.add(Test580State.S1 to 4)
                }
                if (activeStateIds.contains("s11")) {
                    toExit.add(Test580State.S11 to 5)
                }
                if (activeStateIds.contains("s12")) {
                    toExit.add(Test580State.S12 to 6)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p1")
            }
            is Test580State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test580State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test580State.S1 -> {
                // W3C SCXML 3.11: Record shallow history for sh1
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["sh1"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "s1"
                }.toList()
                activeStateIds.remove("s1")
            executeAssign("Var1", "Var1 + 1")
            }
            is Test580State.S11 -> {
                activeStateIds.remove("s11")
            }
            is Test580State.S12 -> {
                activeStateIds.remove("s12")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test580State,
        event: Test580Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
