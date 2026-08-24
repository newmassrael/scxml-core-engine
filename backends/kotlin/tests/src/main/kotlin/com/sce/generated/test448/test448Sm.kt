// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 672592645a46a971e7d9a638044244b01d838f9ebaf5e6860dc88538368c4548
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/448/test448.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test448.scxml:5 :: _machine

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

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `var1` was assigned a value of another type, or the engine refused.
     */
    fun var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "var1")

    /**
     * §scxml-5.3: what the `var2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `var2` was assigned a value of another type, or the engine refused.
     */
    fun var2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "var2")

    override val initialState: Test448State = Test448State.S01

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

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "test448",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )


        // W3C SCXML 5.3: Early binding — initialize state-level datamodel variables at startup
        // State 's01' variable 'var1'
        try {
            val initResult_var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raisePlatformError(Test448Event.Error.Execution, "<data id='var1'> expr failed to evaluate")
        }
        // State 's01p2' variable 'var2'
        try {
            val initResult_var2 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "var2", initResult_var2)
        } catch (e: Exception) {
            raisePlatformError(Test448Event.Error.Execution, "<data id='var2'> expr failed to evaluate")
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
            raisePlatformError(Test448Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test448Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test448Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test448Event.Error.Execution, "<script> failed to execute")
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
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun onEntry(state: Test448State, pathChild: Test448State?) {
        when (state) {
            is Test448State.Fail -> {
                // SCE-MAP: test448.scxml:34 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.Pass -> {
                // SCE-MAP: test448.scxml:33 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test448State.S0 -> {
                // SCE-MAP: test448.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test448State.S01)
                }
            }
            is Test448State.S01 -> {
                // SCE-MAP: test448.scxml:12 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
            }
            is Test448State.S01p -> {
                // SCE-MAP: test448.scxml:19 :: s01p :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test448State.S01p1) {
                    onEntry(Test448State.S01p1)
                }
                if (pathChild != Test448State.S01p2) {
                    onEntry(Test448State.S01p2)
                }
            }
            is Test448State.S01p1 -> {
                // SCE-MAP: test448.scxml:20 :: s01p1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p1")) return
            }
            is Test448State.S01p2 -> {
                // SCE-MAP: test448.scxml:25 :: s01p2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01p2")) return
            }
            is Test448State.S1 -> {
                // SCE-MAP: test448.scxml:18 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test448State.S01p)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun onExit(state: Test448State) {
        when (state) {
            is Test448State.Fail -> {
                // SCE-MAP: test448.scxml:34 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test448State.Pass -> {
                // SCE-MAP: test448.scxml:33 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test448State.S0 -> {
                // SCE-MAP: test448.scxml:8 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test448State.S01 -> {
                // SCE-MAP: test448.scxml:12 :: s01 :: _state_body
                activeStateIds.remove("s01")
            }
            is Test448State.S01p -> {
                // SCE-MAP: test448.scxml:19 :: s01p :: _state_body
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
                // SCE-MAP: test448.scxml:20 :: s01p1 :: _state_body
                activeStateIds.remove("s01p1")
            }
            is Test448State.S01p2 -> {
                // SCE-MAP: test448.scxml:25 :: s01p2 :: _state_body
                activeStateIds.remove("s01p2")
            }
            is Test448State.S1 -> {
                // SCE-MAP: test448.scxml:18 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test448State,
        event: Test448Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
