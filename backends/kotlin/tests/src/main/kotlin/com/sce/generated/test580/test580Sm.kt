// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a432b690c7990abdc6b5ce0526e592fee5b7d55e84a37b350376bb446a9dc3cf
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/580/test580.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test580.scxml:5 :: _machine

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
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test580State, Test580Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test580State = Test580State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
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

    // W3C SCXML: Resolve state ID string to State object
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
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test580State): Boolean = when (state) {
        is Test580State.P1 -> false
        is Test580State.S1 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test580State): Boolean = when (state) {
        is Test580State.P1 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test580State): List<Test580State> = when (state) {
        is Test580State.P1 -> listOf(Test580State.S0, Test580State.S1)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test580State): Int = when (state) {
        is Test580State.Fail -> 6
        is Test580State.P1 -> 0
        is Test580State.Pass -> 5
        is Test580State.S0 -> 1
        is Test580State.S1 -> 2
        is Test580State.S11 -> 3
        is Test580State.S12 -> 4
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
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

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
            "test580",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test580Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test580Event.Error.Execution, "a <transition> cond failed to evaluate")
            false
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    private fun evaluateSendContent(source: String): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateExpr(sid, "JSON.stringify((" + source + "))")?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(Test580Event.Error.Execution, "an expression could not be serialised to JSON")
            ""
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
            raisePlatformError(Test580Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test580Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test580Event) {
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
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
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
        notePayloadReading(event, payloadReading)
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
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail, Test580State.S0)
        else -> TransitionResult.Ignored
    }

    private fun processNullS1(
    ): TransitionResult<Test580State> = when {
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail, Test580State.S1)
        safeEvaluateGuard("Var1 == 0") -> TransitionResult.External((historyStore["sh1"]?.takeIf { it.isNotEmpty() }?.let { resolveState(it[0]) } ?: Test580State.S11), Test580State.S1)
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test580State.Pass, Test580State.S1)
        else -> TransitionResult.Ignored
    }

    private fun processNullS11(
    ): TransitionResult<Test580State> = when {
        isStateActive("sh1") -> TransitionResult.External(Test580State.Fail, Test580State.S11)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test580State.S12, Test580State.S11)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test580Event
    ): TransitionResult<Test580State> = when {
        event is Test580Event.Timeout -> TransitionResult.External(Test580State.Fail, Test580State.S0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun onEntry(state: Test580State, pathChild: Test580State?) {
        when (state) {
            is Test580State.Fail -> {
                // SCE-MAP: test580.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.P1 -> {
                // SCE-MAP: test580.scxml:10 :: p1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p1")) return


            scheduleSend("__send_0", 2000L, Test580Event.Timeout)
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test580State.S0) {
                    onEntry(Test580State.S0)
                }
                if (pathChild != Test580State.S1) {
                    onEntry(Test580State.S1)
                }
            }
            is Test580State.Pass -> {
                // SCE-MAP: test580.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test580State.S0 -> {
                // SCE-MAP: test580.scxml:16 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test580State.S1 -> {
                // SCE-MAP: test580.scxml:22 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
                if (pathChild == null) {
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
            }
            is Test580State.S11 -> {
                // SCE-MAP: test580.scxml:32 :: s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
            }
            is Test580State.S12 -> {
                // SCE-MAP: test580.scxml:37 :: s12 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s12")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun onExit(state: Test580State) {
        when (state) {
            is Test580State.Fail -> {
                // SCE-MAP: test580.scxml:50 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test580State.P1 -> {
                // SCE-MAP: test580.scxml:10 :: p1 :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test580State, Int>>()
                if (activeStateIds.contains("s0")) {
                    toExit.add(Test580State.S0 to 1)
                }
                if (activeStateIds.contains("s1")) {
                    toExit.add(Test580State.S1 to 2)
                }
                if (activeStateIds.contains("s11")) {
                    toExit.add(Test580State.S11 to 3)
                }
                if (activeStateIds.contains("s12")) {
                    toExit.add(Test580State.S12 to 4)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p1")
            }
            is Test580State.Pass -> {
                // SCE-MAP: test580.scxml:49 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test580State.S0 -> {
                // SCE-MAP: test580.scxml:16 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test580State.S1 -> {
                // SCE-MAP: test580.scxml:22 :: s1 :: _state_body
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
                // SCE-MAP: test580.scxml:32 :: s11 :: _state_body
                activeStateIds.remove("s11")
            }
            is Test580State.S12 -> {
                // SCE-MAP: test580.scxml:37 :: s12 :: _state_body
                activeStateIds.remove("s12")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test580.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test580State,
        event: Test580Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
