// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 401d2ad22cf222caeef0633edc48b3c3fd2090ab46bb9a2c354f5be833096227
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/403/test403b.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test403b.scxml:6 :: _machine

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

    override val initialState: Test403bState = Test403bState.P0s1

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

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
            "test403b",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test403bEvent.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test403bEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test403bEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test403bEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test403bEvent.Error.Execution, "<script> failed to execute")
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
    // SCE-MAP: test403b.scxml:6 :: _machine
    override fun onEntry(state: Test403bState, pathChild: Test403bState?) {
        when (state) {
            is Test403bState.Fail -> {
                // SCE-MAP: test403b.scxml:43 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403bState.P0 -> {
                // SCE-MAP: test403b.scxml:20 :: p0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0")) return

            raiseInternal(Test403bEvent.Event1)

            raiseInternal(Test403bEvent.Event2)
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test403bState.P0s1) {
                    onEntry(Test403bState.P0s1)
                }
                if (pathChild != Test403bState.P0s2) {
                    onEntry(Test403bState.P0s2)
                }
            }
            is Test403bState.P0s1 -> {
                // SCE-MAP: test403b.scxml:32 :: p0s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s1")) return
            }
            is Test403bState.P0s2 -> {
                // SCE-MAP: test403b.scxml:37 :: p0s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s2")) return
            }
            is Test403bState.Pass -> {
                // SCE-MAP: test403b.scxml:42 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test403bState.S0 -> {
                // SCE-MAP: test403b.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test403bState.P0)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test403b.scxml:6 :: _machine
    override fun onExit(state: Test403bState) {
        when (state) {
            is Test403bState.Fail -> {
                // SCE-MAP: test403b.scxml:43 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test403bState.P0 -> {
                // SCE-MAP: test403b.scxml:20 :: p0 :: _state_body
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
                // SCE-MAP: test403b.scxml:32 :: p0s1 :: _state_body
                activeStateIds.remove("p0s1")
            }
            is Test403bState.P0s2 -> {
                // SCE-MAP: test403b.scxml:37 :: p0s2 :: _state_body
                activeStateIds.remove("p0s2")
            }
            is Test403bState.Pass -> {
                // SCE-MAP: test403b.scxml:42 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test403bState.S0 -> {
                // SCE-MAP: test403b.scxml:11 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test403b.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test403bState,
        event: Test403bEvent?
    ) {
        when (source) {
        is Test403bState.P0 -> when {
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:28 :: p0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:14 :: s0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.P0s1 -> when {
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:28 :: p0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:14 :: s0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.P0s2 -> when {
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:28 :: p0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:14 :: s0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test403bState.S0 -> when {
            event is Test403bEvent.Event1 -> {
                // SCE-MAP: test403b.scxml:14 :: s0 :: _transition_0


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
