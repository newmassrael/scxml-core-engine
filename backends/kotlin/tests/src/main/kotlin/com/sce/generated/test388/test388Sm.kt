// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/388/test388.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test388.scxml:9

package com.sce.generated.test388

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test388State : State {
    data object Fail : Test388State
    data object Pass : Test388State
    data object S0 : Test388State
    data object S01 : Test388State
    data object S011 : Test388State
    data object S012 : Test388State
    data object S02 : Test388State
    data object S021 : Test388State
    data object S022 : Test388State
    data object S1 : Test388State
    data object S2 : Test388State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test388Event : Event {
    sealed interface Entering : Test388Event {
        data object Self : Entering
        data object S011 : Entering
        data object S012 : Entering
        data object S021 : Entering
        data object S022 : Entering
    }
    sealed interface Error : Test388Event {
        data object Execution : Error
    }
    data object Timeout : Test388Event
}
// --- State Machine (W3C SCXML) ---

class Test388StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test388State, Test388Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test388State = Test388State.S012

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test388State): Test388State? = when (state) {
        is Test388State.S01 -> Test388State.S0
        is Test388State.S011 -> Test388State.S01
        is Test388State.S012 -> Test388State.S01
        is Test388State.S02 -> Test388State.S0
        is Test388State.S021 -> Test388State.S02
        is Test388State.S022 -> Test388State.S02
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test388State): Test388State = when (state) {
        is Test388State.S0 -> Test388State.S011
        is Test388State.S01 -> Test388State.S011
        is Test388State.S02 -> Test388State.S021
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test388State? = when (stateId) {
        "fail" -> Test388State.Fail
        "pass" -> Test388State.Pass
        "s0" -> Test388State.S0
        "s01" -> Test388State.S01
        "s011" -> Test388State.S011
        "s012" -> Test388State.S012
        "s02" -> Test388State.S02
        "s021" -> Test388State.S021
        "s022" -> Test388State.S022
        "s1" -> Test388State.S1
        "s2" -> Test388State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test388State): String = when (state) {
        is Test388State.Fail -> "fail"
        is Test388State.Pass -> "pass"
        is Test388State.S0 -> "s0"
        is Test388State.S01 -> "s01"
        is Test388State.S011 -> "s011"
        is Test388State.S012 -> "s012"
        is Test388State.S02 -> "s02"
        is Test388State.S021 -> "s021"
        is Test388State.S022 -> "s022"
        is Test388State.S1 -> "s1"
        is Test388State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test388State): Boolean = when (state) {
        is Test388State.S0 -> false
        is Test388State.S01 -> false
        is Test388State.S02 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test388State): Int = when (state) {
        is Test388State.Fail -> 10
        is Test388State.Pass -> 9
        is Test388State.S0 -> 0
        is Test388State.S01 -> 1
        is Test388State.S011 -> 2
        is Test388State.S012 -> 3
        is Test388State.S02 -> 4
        is Test388State.S021 -> 5
        is Test388State.S022 -> 6
        is Test388State.S1 -> 7
        is Test388State.S2 -> 8
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test388Event? = when (name) {
        "entering" -> Test388Event.Entering.Self
        "entering.s011" -> Test388Event.Entering.S011
        "entering.s012" -> Test388Event.Entering.S012
        "entering.s021" -> Test388Event.Entering.S021
        "entering.s022" -> Test388Event.Entering.S022
        "error.execution" -> Test388Event.Error.Execution
        "timeout" -> Test388Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test388Event): String? = when (event) {
        is Test388Event.Entering.Self -> "entering"
        is Test388Event.Entering.S011 -> "entering.s011"
        is Test388Event.Entering.S012 -> "entering.s012"
        is Test388Event.Entering.S021 -> "entering.s021"
        is Test388Event.Entering.S022 -> "entering.s022"
        is Test388Event.Error.Execution -> "error.execution"
        is Test388Event.Timeout -> "timeout"
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
            "test388",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test388Event) {
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
        state: Test388State,
        event: Test388Event
    ): TransitionResult<Test388State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test388State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test388State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test388State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s012 has no own event transitions)
        is Test388State.S012 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test388State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s021 has no own event transitions)
        is Test388State.S021 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s022 has no own event transitions)
        is Test388State.S022 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test388State
    ): TransitionResult<Test388State> = when (state) {
        is Test388State.S1 -> processNullS1()
        is Test388State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test388State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External((historyStore["s0HistDeep"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test388State.S022), Test388State.S1)
    }

    private fun processNullS2(
    ): TransitionResult<Test388State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External((historyStore["s0HistShallow"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test388State.S021), Test388State.S2)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test388Event
    ): TransitionResult<Test388State> = when {
        event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test388State.S1, Test388State.S0)

        event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test388State.S2, Test388State.S0)

        // W3C SCXML 3.12.1: Prefix match for "entering"
        (event is Test388Event.Entering || event is Test388Event.Entering.S011 || event is Test388Event.Entering.S012 || event is Test388Event.Entering.S021 || event is Test388Event.Entering.S022) && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        event is Test388Event.Entering.S011 && safeEvaluateGuard("Var1 == 3") -> TransitionResult.External(Test388State.Pass, Test388State.S0)

        // W3C SCXML 3.12.1: Prefix match for "entering"
        (event is Test388Event.Entering || event is Test388Event.Entering.S011 || event is Test388Event.Entering.S012 || event is Test388Event.Entering.S021 || event is Test388Event.Entering.S022) && safeEvaluateGuard("Var1 == 3") -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        event is Test388Event.Timeout -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test388.scxml:9
    override fun onEntry(state: Test388State) {
        when (state) {
            is Test388State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test388State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test388State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            executeAssign("Var1", "Var1 + 1")
            }
            is Test388State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test388State.S011 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s011")) return

            raiseInternal(Test388Event.Entering.S011)
            }
            is Test388State.S012 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s012")) return

            raiseInternal(Test388Event.Entering.S012)
            }
            is Test388State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test388State.S021 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s021")) return

            raiseInternal(Test388Event.Entering.S021)
            }
            is Test388State.S022 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s022")) return

            raiseInternal(Test388Event.Entering.S022)
            }
            is Test388State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test388State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test388.scxml:9
    override fun onExit(state: Test388State) {
        when (state) {
            is Test388State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test388State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test388State.S0 -> {
                // W3C SCXML 3.11: Record deep history for s0HistDeep
                historyStore["s0HistDeep"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    isDescendantOf(st, Test388State.S0) && isAtomicState(st)
                }.toList()
                // W3C SCXML 3.11: Record shallow history for s0HistShallow
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["s0HistShallow"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "s0"
                }.toList()
                activeStateIds.remove("s0")
            }
            is Test388State.S01 -> {
                activeStateIds.remove("s01")
            }
            is Test388State.S011 -> {
                activeStateIds.remove("s011")
            }
            is Test388State.S012 -> {
                activeStateIds.remove("s012")
            }
            is Test388State.S02 -> {
                activeStateIds.remove("s02")
            }
            is Test388State.S021 -> {
                activeStateIds.remove("s021")
            }
            is Test388State.S022 -> {
                activeStateIds.remove("s022")
            }
            is Test388State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test388State.S2 -> {
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test388.scxml:9
    override fun executeTransitionActions(
        source: Test388State,
        event: Test388Event?
    ) {
        when (source) {
        is Test388State.S0 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S01 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S011 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S012 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S02 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S021 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S022 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {


            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
