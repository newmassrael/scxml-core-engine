// GENERATED CODE — DO NOT EDIT
// Source: resources/448/test448.scxml
// Generator: SCE Kotlin Code Generator v1.0

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
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test448State, Test448Event>(scriptEngine) {

    override val initialState: Test448State = Test448State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test448State.S0)
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

    // W3C SCXML: Resolve state ID string to State object (for parallel processing)
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
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test448State): Boolean = when (state) {
        is Test448State.S0 -> false
        is Test448State.S01p -> false
        is Test448State.S1 -> false
        else -> true
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
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test448Event? = when (name) {
        "error.execution" -> Test448Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test448Event): String? = when (event) {
        is Test448Event.Error.Execution -> "error.execution"
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
        engine.setupSystemVariables(sid, "test448")



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
            raiseInternal(Test448Event.Error.Execution)
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
            raiseInternal(Test448Event.Error.Execution)
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
            raiseInternal(Test448Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test448Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = meta.type,
            sendId = meta.sendId,
            origin = meta.origin.ifEmpty { scriptSessionId ?: "" },
            originType = meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" },
            invokeId = meta.invokeId
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
        safeEvaluateGuard("var1==1") -> TransitionResult.External(Test448State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test448State.Fail)
    }

    private fun processNullS01p1(
    ): TransitionResult<Test448State> = when {
        safeEvaluateGuard("var2==1") -> TransitionResult.External(Test448State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test448State.Fail)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test448State) {
        when (state) {
            is Test448State.Fail -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.Pass -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.S0 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s0")) return
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test448State.S01)
            }
            is Test448State.S01 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s01")) return
            }
            is Test448State.S01p -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s01p")) return
                // W3C SCXML 3.4: Enter all child regions of parallel state
                onEntry(Test448State.S01p1)
                onEntry(Test448State.S01p2)
            }
            is Test448State.S01p1 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s01p1")) return
            }
            is Test448State.S01p2 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s01p2")) return
            }
            is Test448State.S1 -> {
                // W3C SCXML 3.8: Skip duplicate entry (parallel re-entry guard)
                if (!activeStateIds.add("s1")) return
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test448State.S01p)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
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
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test448State,
        event: Test448Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
