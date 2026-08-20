// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 73faacbaab5efcd5e8cb1580c2fd7ba6894f2db518b69f1a3e5a7cac6fd97efb
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/570/test570.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test570.scxml:5 :: _machine

package com.sce.generated.test570

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test570State : State {
    data object Fail : Test570State
    data object P0 : Test570State
    data object P0s1 : Test570State
    data object P0s11 : Test570State
    data object P0s1final : Test570State
    data object P0s2 : Test570State
    data object P0s21 : Test570State
    data object P0s2final : Test570State
    data object Pass : Test570State
    data object S1 : Test570State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test570Event : Event {
    sealed interface Done : Test570Event {
        sealed interface State : Done {
            data object P0 : State
            data object P0s1 : State
            data object P0s2 : State
        }
    }
    data object E1 : Test570Event
    data object E2 : Test570Event
    sealed interface Error : Test570Event {
        data object Execution : Error
    }
    data object Timeout : Test570Event
}
// --- State Machine (W3C SCXML) ---

class Test570StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test570State, Test570Event>(scriptEngine) {

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

    override val initialState: Test570State = Test570State.P0s11

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
    override fun parentOf(state: Test570State): Test570State? = when (state) {
        is Test570State.P0s1 -> Test570State.P0
        is Test570State.P0s11 -> Test570State.P0s1
        is Test570State.P0s1final -> Test570State.P0s1
        is Test570State.P0s2 -> Test570State.P0
        is Test570State.P0s21 -> Test570State.P0s2
        is Test570State.P0s2final -> Test570State.P0s2
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test570State): Test570State = when (state) {
        is Test570State.P0 -> Test570State.P0s11
        is Test570State.P0s1 -> Test570State.P0s11
        is Test570State.P0s2 -> Test570State.P0s21
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test570State? = when (stateId) {
        "fail" -> Test570State.Fail
        "p0" -> Test570State.P0
        "p0s1" -> Test570State.P0s1
        "p0s11" -> Test570State.P0s11
        "p0s1final" -> Test570State.P0s1final
        "p0s2" -> Test570State.P0s2
        "p0s21" -> Test570State.P0s21
        "p0s2final" -> Test570State.P0s2final
        "pass" -> Test570State.Pass
        "s1" -> Test570State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test570State): String = when (state) {
        is Test570State.Fail -> "fail"
        is Test570State.P0 -> "p0"
        is Test570State.P0s1 -> "p0s1"
        is Test570State.P0s11 -> "p0s11"
        is Test570State.P0s1final -> "p0s1final"
        is Test570State.P0s2 -> "p0s2"
        is Test570State.P0s21 -> "p0s21"
        is Test570State.P0s2final -> "p0s2final"
        is Test570State.Pass -> "pass"
        is Test570State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test570State): Boolean = when (state) {
        is Test570State.P0 -> false
        is Test570State.P0s1 -> false
        is Test570State.P0s2 -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test570State): Boolean = when (state) {
        is Test570State.P0 -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: Test570State): List<Test570State> = when (state) {
        is Test570State.P0 -> listOf(Test570State.P0s1, Test570State.P0s2)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test570State): Int = when (state) {
        is Test570State.Fail -> 9
        is Test570State.P0 -> 0
        is Test570State.P0s1 -> 1
        is Test570State.P0s11 -> 2
        is Test570State.P0s1final -> 3
        is Test570State.P0s2 -> 4
        is Test570State.P0s21 -> 5
        is Test570State.P0s2final -> 6
        is Test570State.Pass -> 8
        is Test570State.S1 -> 7
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test570Event? = when (name) {
        "done.state.p0" -> Test570Event.Done.State.P0
        "done.state.p0s1" -> Test570Event.Done.State.P0s1
        "done.state.p0s2" -> Test570Event.Done.State.P0s2
        "e1" -> Test570Event.E1
        "e2" -> Test570Event.E2
        "error.execution" -> Test570Event.Error.Execution
        "timeout" -> Test570Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test570Event): String? = when (event) {
        is Test570Event.Done.State.P0 -> "done.state.p0"
        is Test570Event.Done.State.P0s1 -> "done.state.p0s1"
        is Test570Event.Done.State.P0s2 -> "done.state.p0s2"
        is Test570Event.E1 -> "e1"
        is Test570Event.E2 -> "e2"
        is Test570Event.Error.Execution -> "error.execution"
        is Test570Event.Timeout -> "timeout"
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
            "test570",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test570Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test570Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test570Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test570Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test570Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test570Event) {
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
        state: Test570State,
        event: Test570Event
    ): TransitionResult<Test570State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (p0s1 has no own event transitions)
        is Test570State.P0s1 -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test570State.P0s11 -> {
            val result = processP0s11(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processP0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (p0s1final has no own event transitions)
        is Test570State.P0s1final -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (p0s2 has no own event transitions)
        is Test570State.P0s2 -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test570State.P0s21 -> {
            val result = processP0s21(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processP0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (p0s2final has no own event transitions)
        is Test570State.P0s2final -> {
            val anc1 = processP0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test570State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processP0(
        event: Test570Event
    ): TransitionResult<Test570State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test570Event.Done.State.P0s1 -> TransitionResult.Internal
        event is Test570Event.Done.State.P0s2 -> TransitionResult.External(Test570State.S1, Test570State.P0)

        event is Test570Event.Timeout -> TransitionResult.External(Test570State.Fail, Test570State.P0)

        else -> TransitionResult.Ignored
    }

    private fun processP0s11(
        event: Test570Event
    ): TransitionResult<Test570State> = when {
        event is Test570Event.E1 -> TransitionResult.External(Test570State.P0s1final, Test570State.P0s11)

        else -> TransitionResult.Ignored
    }

    private fun processP0s21(
        event: Test570Event
    ): TransitionResult<Test570State> = when {
        event is Test570Event.E2 -> TransitionResult.External(Test570State.P0s2final, Test570State.P0s21)

        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test570Event
    ): TransitionResult<Test570State> = when {
        event is Test570Event.Done.State.P0 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test570State.Pass, Test570State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test570State.Fail, Test570State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test570.scxml:5 :: _machine
    override fun onEntry(state: Test570State, pathChild: Test570State?) {
        when (state) {
            is Test570State.Fail -> {
                // SCE-MAP: test570.scxml:47 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test570State.P0 -> {
                // SCE-MAP: test570.scxml:9 :: p0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0")) return


            scheduleSend("__send_0", 2000L, Test570Event.Timeout)

            raiseInternal(Test570Event.E1)

            raiseInternal(Test570Event.E2)
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != Test570State.P0s1) {
                    onEntry(Test570State.P0s1)
                }
                if (pathChild != Test570State.P0s2) {
                    onEntry(Test570State.P0s2)
                }
            }
            is Test570State.P0s1 -> {
                // SCE-MAP: test570.scxml:24 :: p0s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s1")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test570State.P0s11)
                }
            }
            is Test570State.P0s11 -> {
                // SCE-MAP: test570.scxml:25 :: p0s11 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s11")) return
            }
            is Test570State.P0s1final -> {
                // SCE-MAP: test570.scxml:28 :: p0s1final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s1final")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test570Event.Done.State.P0s1, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("p0s1final")) && (activeStateIds.contains("p0s2final"))) {
                    raiseInternal(Test570Event.Done.State.P0)
                }
            }
            is Test570State.P0s2 -> {
                // SCE-MAP: test570.scxml:31 :: p0s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s2")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(Test570State.P0s21)
                }
            }
            is Test570State.P0s21 -> {
                // SCE-MAP: test570.scxml:32 :: p0s21 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s21")) return
            }
            is Test570State.P0s2final -> {
                // SCE-MAP: test570.scxml:35 :: p0s2final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("p0s2final")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test570Event.Done.State.P0s2, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("p0s1final")) && (activeStateIds.contains("p0s2final"))) {
                    raiseInternal(Test570Event.Done.State.P0)
                }
            }
            is Test570State.Pass -> {
                // SCE-MAP: test570.scxml:46 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test570State.S1 -> {
                // SCE-MAP: test570.scxml:40 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test570.scxml:5 :: _machine
    override fun onExit(state: Test570State) {
        when (state) {
            is Test570State.Fail -> {
                // SCE-MAP: test570.scxml:47 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test570State.P0 -> {
                // SCE-MAP: test570.scxml:9 :: p0 :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<Test570State, Int>>()
                if (activeStateIds.contains("p0s1")) {
                    toExit.add(Test570State.P0s1 to 1)
                }
                if (activeStateIds.contains("p0s11")) {
                    toExit.add(Test570State.P0s11 to 2)
                }
                if (activeStateIds.contains("p0s1final")) {
                    toExit.add(Test570State.P0s1final to 3)
                }
                if (activeStateIds.contains("p0s2")) {
                    toExit.add(Test570State.P0s2 to 4)
                }
                if (activeStateIds.contains("p0s21")) {
                    toExit.add(Test570State.P0s21 to 5)
                }
                if (activeStateIds.contains("p0s2final")) {
                    toExit.add(Test570State.P0s2final to 6)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("p0")
            }
            is Test570State.P0s1 -> {
                // SCE-MAP: test570.scxml:24 :: p0s1 :: _state_body
                activeStateIds.remove("p0s1")
            }
            is Test570State.P0s11 -> {
                // SCE-MAP: test570.scxml:25 :: p0s11 :: _state_body
                activeStateIds.remove("p0s11")
            }
            is Test570State.P0s1final -> {
                // SCE-MAP: test570.scxml:28 :: p0s1final :: _state_body
                activeStateIds.remove("p0s1final")
            }
            is Test570State.P0s2 -> {
                // SCE-MAP: test570.scxml:31 :: p0s2 :: _state_body
                activeStateIds.remove("p0s2")
            }
            is Test570State.P0s21 -> {
                // SCE-MAP: test570.scxml:32 :: p0s21 :: _state_body
                activeStateIds.remove("p0s21")
            }
            is Test570State.P0s2final -> {
                // SCE-MAP: test570.scxml:35 :: p0s2final :: _state_body
                activeStateIds.remove("p0s2final")
            }
            is Test570State.Pass -> {
                // SCE-MAP: test570.scxml:46 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test570State.S1 -> {
                // SCE-MAP: test570.scxml:40 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test570.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test570State,
        event: Test570Event?
    ) {
        when (source) {
        is Test570State.P0 -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s1 -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s11 -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s1final -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s2 -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s21 -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        is Test570State.P0s2final -> when {
            event is Test570Event.Done.State.P0s1 -> {
                // SCE-MAP: test570.scxml:16 :: p0 :: _transition_0


            executeAssign("Var1", "1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
