// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b82119528bc210fbc6e453d658ae079f31e3529ce331b1d6045090bb79eaa2ff
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/579/test579.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test579.scxml:8 :: _machine

package com.sce.generated.test579

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test579State : State {
    data object Fail : Test579State
    data object Pass : Test579State
    data object S0 : Test579State
    data object S01 : Test579State
    data object S02 : Test579State
    data object S03 : Test579State
    data object S2 : Test579State
    data object S3 : Test579State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test579Event : Event {
    sealed interface Error : Test579Event {
        data object Execution : Error
    }
    data object Event1 : Test579Event
    data object Event2 : Test579Event
    data object Event3 : Test579Event
    data object Timeout : Test579Event
}
// --- State Machine (W3C SCXML) ---

class Test579StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test579State, Test579Event>(scriptEngine) {

    override val initialState: Test579State = Test579State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test579State): Test579State? = when (state) {
        is Test579State.S01 -> Test579State.S0
        is Test579State.S02 -> Test579State.S0
        is Test579State.S03 -> Test579State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test579State): Test579State = when (state) {
        is Test579State.S0 -> Test579State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test579State? = when (stateId) {
        "fail" -> Test579State.Fail
        "pass" -> Test579State.Pass
        "s0" -> Test579State.S0
        "s01" -> Test579State.S01
        "s02" -> Test579State.S02
        "s03" -> Test579State.S03
        "s2" -> Test579State.S2
        "s3" -> Test579State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test579State): String = when (state) {
        is Test579State.Fail -> "fail"
        is Test579State.Pass -> "pass"
        is Test579State.S0 -> "s0"
        is Test579State.S01 -> "s01"
        is Test579State.S02 -> "s02"
        is Test579State.S03 -> "s03"
        is Test579State.S2 -> "s2"
        is Test579State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test579State): Boolean = when (state) {
        is Test579State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test579State): Int = when (state) {
        is Test579State.Fail -> 7
        is Test579State.Pass -> 6
        is Test579State.S0 -> 0
        is Test579State.S01 -> 1
        is Test579State.S02 -> 2
        is Test579State.S03 -> 3
        is Test579State.S2 -> 4
        is Test579State.S3 -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test579Event? = when (name) {
        "error.execution" -> Test579Event.Error.Execution
        "event1" -> Test579Event.Event1
        "event2" -> Test579Event.Event2
        "event3" -> Test579Event.Event3
        "timeout" -> Test579Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test579Event): String? = when (event) {
        is Test579Event.Error.Execution -> "error.execution"
        is Test579Event.Event1 -> "event1"
        is Test579Event.Event2 -> "event2"
        is Test579Event.Event3 -> "event3"
        is Test579Event.Timeout -> "timeout"
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
            "test579",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )


        // W3C SCXML 5.3: Early binding — initialize state-level datamodel variables at startup
        // State 's0' variable 'Var1'
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test579Event.Error.Execution)
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
            raiseInternal(Test579Event.Error.Execution)
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
            raiseInternal(Test579Event.Error.Execution)
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
            raiseInternal(Test579Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test579Event) {
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
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
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
        state: Test579State,
        event: Test579Event
    ): TransitionResult<Test579State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test579State.S01 -> processS01(event)
        is Test579State.S02 -> processS02(event)
        is Test579State.S03 -> processS03(event)
        is Test579State.S2 -> processS2(event)
        is Test579State.S3 -> processS3(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS01(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event1 -> TransitionResult.External(Test579State.S02, Test579State.S01)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail, Test579State.S01)
    }

    private fun processS02(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event2 -> TransitionResult.External(Test579State.S03, Test579State.S02)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail, Test579State.S02)
    }

    private fun processS03(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event3 && safeEvaluateGuard("Var1 == 0") -> TransitionResult.External(Test579State.S0, Test579State.S03)

        event is Test579Event.Event1 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test579State.S2, Test579State.S03)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail, Test579State.S03)
    }

    private fun processS2(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event2 -> TransitionResult.External(Test579State.S3, Test579State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test579State.Fail, Test579State.S2)
    }

    private fun processS3(
        event: Test579Event
    ): TransitionResult<Test579State> = when {
        event is Test579Event.Event3 -> TransitionResult.External(Test579State.Fail, Test579State.S3)

        event is Test579Event.Timeout -> TransitionResult.External(Test579State.Pass, Test579State.S3)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test579.scxml:8 :: _machine
    override fun onEntry(state: Test579State) {
        when (state) {
            is Test579State.Fail -> {
                // SCE-MAP: test579.scxml:63 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test579State.Pass -> {
                // SCE-MAP: test579.scxml:62 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test579State.S0 -> {
                // SCE-MAP: test579.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test579Event.Timeout)

            raiseInternal(Test579Event.Event1)
                if (!suppressChildEntry) {
                    // W3C SCXML 3.3.2: Execute initial transition content (always, even with stored history)

            raiseInternal(Test579Event.Event2)
                    // W3C SCXML 3.11: Execute history default transition content only if no stored history
                    if (!historyStore.containsKey("sh1") || historyStore["sh1"].isNullOrEmpty()) {

            raiseInternal(Test579Event.Event3)
                    }
                    // W3C SCXML 3.11: Enter history-restored state or default target
                    run {
                        val stored = historyStore["sh1"]
                        if (stored != null && stored.isNotEmpty()) {
                            val histTarget = resolveState(stored[0])
                            if (histTarget != null) {
                                onEntry(histTarget)
                            } else {
                                onEntry(Test579State.S01)
                            }
                        } else {
                            onEntry(Test579State.S01)
                        }
                    }
                }
            }
            is Test579State.S01 -> {
                // SCE-MAP: test579.scxml:33 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test579State.S02 -> {
                // SCE-MAP: test579.scxml:38 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
            }
            is Test579State.S03 -> {
                // SCE-MAP: test579.scxml:42 :: s03 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
            }
            is Test579State.S2 -> {
                // SCE-MAP: test579.scxml:50 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test579State.S3 -> {
                // SCE-MAP: test579.scxml:56 :: s3 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test579.scxml:8 :: _machine
    override fun onExit(state: Test579State) {
        when (state) {
            is Test579State.Fail -> {
                // SCE-MAP: test579.scxml:63 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test579State.Pass -> {
                // SCE-MAP: test579.scxml:62 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test579State.S0 -> {
                // SCE-MAP: test579.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.11: Record shallow history for sh1
                // Uses preTransitionActiveStates (captured before exits, C++ pattern)
                historyStore["sh1"] = preTransitionActiveStates.filter { stateId ->
                    val st = resolveState(stateId) ?: return@filter false
                    parentOf(st)?.let { stateIdOf(it) } == "s0"
                }.toList()
                activeStateIds.remove("s0")


            executeAssign("Var1", "Var1 + 1")
            }
            is Test579State.S01 -> {
                // SCE-MAP: test579.scxml:33 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test579State.S02 -> {
                // SCE-MAP: test579.scxml:38 :: s02 :: _state_body
                activeStateIds.remove("s02")
            }
            is Test579State.S03 -> {
                // SCE-MAP: test579.scxml:42 :: s03 :: _state_body
                activeStateIds.remove("s03")
            }
            is Test579State.S2 -> {
                // SCE-MAP: test579.scxml:50 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
            is Test579State.S3 -> {
                // SCE-MAP: test579.scxml:56 :: s3 :: _state_body
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test579.scxml:8 :: _machine
    override fun executeTransitionActions(
        source: Test579State,
        event: Test579Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
