// SCE-GENERATED — DO NOT EDIT
// source-hash: 9cf4fd5f626a0b8e891563a233492fcdd47cb02fca615778881ec79fcd0199e5
// template-hash: 85660c1341dd8abf7326f61f4efe828117f6cbaf56814ccb03d3fd81b42e6ed0
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/parallel_regions_take_own_transitions/parallel_regions_take_own_transitions.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: parallel_regions_take_own_transitions.scxml:24 :: _machine

package com.sce.integration.parallel_regions_take_own_transitions

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParallelRegionsTakeOwnTransitionsState : State {
    data object Budget : ParallelRegionsTakeOwnTransitionsState
    data object Drive : ParallelRegionsTakeOwnTransitionsState
    data object Judging : ParallelRegionsTakeOwnTransitionsState
    data object Run : ParallelRegionsTakeOwnTransitionsState
    data object Running : ParallelRegionsTakeOwnTransitionsState
    data object Settled : ParallelRegionsTakeOwnTransitionsState
    data object Within : ParallelRegionsTakeOwnTransitionsState
    data object Working : ParallelRegionsTakeOwnTransitionsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParallelRegionsTakeOwnTransitionsEvent : Event {
    data object Check : ParallelRegionsTakeOwnTransitionsEvent
    data object E : ParallelRegionsTakeOwnTransitionsEvent
    sealed interface Error : ParallelRegionsTakeOwnTransitionsEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class ParallelRegionsTakeOwnTransitionsStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ParallelRegionsTakeOwnTransitionsState, ParallelRegionsTakeOwnTransitionsEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `n` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `n` was assigned a value of another type, or the engine refused.
     */
    fun n(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "n")

    /**
     * §scxml-5.3: what the `m` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `m` was assigned a value of another type, or the engine refused.
     */
    fun m(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "m")

    override val initialState: ParallelRegionsTakeOwnTransitionsState = ParallelRegionsTakeOwnTransitionsState.Working

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
    override fun parentOf(state: ParallelRegionsTakeOwnTransitionsState): ParallelRegionsTakeOwnTransitionsState? = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Budget -> ParallelRegionsTakeOwnTransitionsState.Run
        is ParallelRegionsTakeOwnTransitionsState.Drive -> ParallelRegionsTakeOwnTransitionsState.Run
        is ParallelRegionsTakeOwnTransitionsState.Judging -> ParallelRegionsTakeOwnTransitionsState.Running
        is ParallelRegionsTakeOwnTransitionsState.Running -> ParallelRegionsTakeOwnTransitionsState.Drive
        is ParallelRegionsTakeOwnTransitionsState.Within -> ParallelRegionsTakeOwnTransitionsState.Budget
        is ParallelRegionsTakeOwnTransitionsState.Working -> ParallelRegionsTakeOwnTransitionsState.Running
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: ParallelRegionsTakeOwnTransitionsState): ParallelRegionsTakeOwnTransitionsState = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Budget -> ParallelRegionsTakeOwnTransitionsState.Within
        is ParallelRegionsTakeOwnTransitionsState.Drive -> ParallelRegionsTakeOwnTransitionsState.Working
        is ParallelRegionsTakeOwnTransitionsState.Run -> ParallelRegionsTakeOwnTransitionsState.Working
        is ParallelRegionsTakeOwnTransitionsState.Running -> ParallelRegionsTakeOwnTransitionsState.Working
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParallelRegionsTakeOwnTransitionsState? = when (stateId) {
        "budget" -> ParallelRegionsTakeOwnTransitionsState.Budget
        "drive" -> ParallelRegionsTakeOwnTransitionsState.Drive
        "judging" -> ParallelRegionsTakeOwnTransitionsState.Judging
        "run" -> ParallelRegionsTakeOwnTransitionsState.Run
        "running" -> ParallelRegionsTakeOwnTransitionsState.Running
        "settled" -> ParallelRegionsTakeOwnTransitionsState.Settled
        "within" -> ParallelRegionsTakeOwnTransitionsState.Within
        "working" -> ParallelRegionsTakeOwnTransitionsState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParallelRegionsTakeOwnTransitionsState): String = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Budget -> "budget"
        is ParallelRegionsTakeOwnTransitionsState.Drive -> "drive"
        is ParallelRegionsTakeOwnTransitionsState.Judging -> "judging"
        is ParallelRegionsTakeOwnTransitionsState.Run -> "run"
        is ParallelRegionsTakeOwnTransitionsState.Running -> "running"
        is ParallelRegionsTakeOwnTransitionsState.Settled -> "settled"
        is ParallelRegionsTakeOwnTransitionsState.Within -> "within"
        is ParallelRegionsTakeOwnTransitionsState.Working -> "working"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ParallelRegionsTakeOwnTransitionsState): Boolean = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Budget -> false
        is ParallelRegionsTakeOwnTransitionsState.Drive -> false
        is ParallelRegionsTakeOwnTransitionsState.Run -> false
        is ParallelRegionsTakeOwnTransitionsState.Running -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelRegionsTakeOwnTransitionsState): Boolean = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: ParallelRegionsTakeOwnTransitionsState): List<ParallelRegionsTakeOwnTransitionsState> = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Run -> listOf(ParallelRegionsTakeOwnTransitionsState.Budget, ParallelRegionsTakeOwnTransitionsState.Drive)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: ParallelRegionsTakeOwnTransitionsState): Int = when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Budget -> 5
        is ParallelRegionsTakeOwnTransitionsState.Drive -> 1
        is ParallelRegionsTakeOwnTransitionsState.Judging -> 4
        is ParallelRegionsTakeOwnTransitionsState.Run -> 0
        is ParallelRegionsTakeOwnTransitionsState.Running -> 2
        is ParallelRegionsTakeOwnTransitionsState.Settled -> 7
        is ParallelRegionsTakeOwnTransitionsState.Within -> 6
        is ParallelRegionsTakeOwnTransitionsState.Working -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ParallelRegionsTakeOwnTransitionsEvent? = when (name) {
        "check" -> ParallelRegionsTakeOwnTransitionsEvent.Check
        "e" -> ParallelRegionsTakeOwnTransitionsEvent.E
        "error.execution" -> ParallelRegionsTakeOwnTransitionsEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ParallelRegionsTakeOwnTransitionsEvent): String? = when (event) {
        is ParallelRegionsTakeOwnTransitionsEvent.Check -> "check"
        is ParallelRegionsTakeOwnTransitionsEvent.E -> "e"
        is ParallelRegionsTakeOwnTransitionsEvent.Error.Execution -> "error.execution"
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
            "parallel_regions_take_own_transitions",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'n' with expr
        try {
            val initResult_n = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "n", initResult_n)
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "<data id='n'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'm' with expr
        try {
            val initResult_m = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "m", initResult_m)
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "<data id='m'> expr failed to evaluate")
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
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(ParallelRegionsTakeOwnTransitionsEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ParallelRegionsTakeOwnTransitionsEvent) {
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
        state: ParallelRegionsTakeOwnTransitionsState,
        event: ParallelRegionsTakeOwnTransitionsEvent
    ): TransitionResult<ParallelRegionsTakeOwnTransitionsState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is ParallelRegionsTakeOwnTransitionsState.Judging -> processJudging(event)
        is ParallelRegionsTakeOwnTransitionsState.Within -> processWithin(event)
        is ParallelRegionsTakeOwnTransitionsState.Working -> processWorking(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processJudging(
        event: ParallelRegionsTakeOwnTransitionsEvent
    ): TransitionResult<ParallelRegionsTakeOwnTransitionsState> = when {
        event is ParallelRegionsTakeOwnTransitionsEvent.Check && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_eq(n, 1) and _scxml_eq(m, 1))", "n == 1 && m == 1")) -> TransitionResult.External(ParallelRegionsTakeOwnTransitionsState.Settled, ParallelRegionsTakeOwnTransitionsState.Judging, 0)

        else -> TransitionResult.Ignored
    }

    private fun processWithin(
        event: ParallelRegionsTakeOwnTransitionsEvent
    ): TransitionResult<ParallelRegionsTakeOwnTransitionsState> = when {
        event is ParallelRegionsTakeOwnTransitionsEvent.E -> TransitionResult.External(ParallelRegionsTakeOwnTransitionsState.Within, ParallelRegionsTakeOwnTransitionsState.Within, 1)

        else -> TransitionResult.Ignored
    }

    private fun processWorking(
        event: ParallelRegionsTakeOwnTransitionsEvent
    ): TransitionResult<ParallelRegionsTakeOwnTransitionsState> = when {
        event is ParallelRegionsTakeOwnTransitionsEvent.E -> TransitionResult.External(ParallelRegionsTakeOwnTransitionsState.Judging, ParallelRegionsTakeOwnTransitionsState.Working, 2)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_regions_take_own_transitions.scxml:24 :: _machine
    override fun onEntry(state: ParallelRegionsTakeOwnTransitionsState, pathChild: ParallelRegionsTakeOwnTransitionsState?) {
        when (state) {
            is ParallelRegionsTakeOwnTransitionsState.Budget -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:60 :: budget :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("budget")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelRegionsTakeOwnTransitionsState.Within)
                }
            }
            is ParallelRegionsTakeOwnTransitionsState.Drive -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:35 :: drive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("drive")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelRegionsTakeOwnTransitionsState.Running)
                }
            }
            is ParallelRegionsTakeOwnTransitionsState.Judging -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:42 :: judging :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("judging")) return
            }
            is ParallelRegionsTakeOwnTransitionsState.Run -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:32 :: run :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != ParallelRegionsTakeOwnTransitionsState.Budget) {
                    onEntry(ParallelRegionsTakeOwnTransitionsState.Budget)
                }
                if (pathChild != ParallelRegionsTakeOwnTransitionsState.Drive) {
                    onEntry(ParallelRegionsTakeOwnTransitionsState.Drive)
                }
            }
            is ParallelRegionsTakeOwnTransitionsState.Running -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:36 :: running :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("running")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelRegionsTakeOwnTransitionsState.Working)
                }
            }
            is ParallelRegionsTakeOwnTransitionsState.Settled -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:70 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ParallelRegionsTakeOwnTransitionsState.Within -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:61 :: within :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("within")) return
            }
            is ParallelRegionsTakeOwnTransitionsState.Working -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:37 :: working :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("working")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_regions_take_own_transitions.scxml:24 :: _machine
    override fun onExit(state: ParallelRegionsTakeOwnTransitionsState) {
        when (state) {
            is ParallelRegionsTakeOwnTransitionsState.Budget -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:60 :: budget :: _state_body
                activeStateIds.remove("budget")
            }
            is ParallelRegionsTakeOwnTransitionsState.Drive -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:35 :: drive :: _state_body
                activeStateIds.remove("drive")
            }
            is ParallelRegionsTakeOwnTransitionsState.Judging -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:42 :: judging :: _state_body
                activeStateIds.remove("judging")
            }
            is ParallelRegionsTakeOwnTransitionsState.Run -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:32 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<ParallelRegionsTakeOwnTransitionsState, Int>>()
                if (activeStateIds.contains("budget")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Budget to 5)
                }
                if (activeStateIds.contains("within")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Within to 6)
                }
                if (activeStateIds.contains("drive")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Drive to 1)
                }
                if (activeStateIds.contains("running")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Running to 2)
                }
                if (activeStateIds.contains("judging")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Judging to 4)
                }
                if (activeStateIds.contains("working")) {
                    toExit.add(ParallelRegionsTakeOwnTransitionsState.Working to 3)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is ParallelRegionsTakeOwnTransitionsState.Running -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:36 :: running :: _state_body
                activeStateIds.remove("running")
            }
            is ParallelRegionsTakeOwnTransitionsState.Settled -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:70 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is ParallelRegionsTakeOwnTransitionsState.Within -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:61 :: within :: _state_body
                activeStateIds.remove("within")
            }
            is ParallelRegionsTakeOwnTransitionsState.Working -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:37 :: working :: _state_body
                activeStateIds.remove("working")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: parallel_regions_take_own_transitions.scxml:24 :: _machine
    override fun executeTransitionActions(
        source: ParallelRegionsTakeOwnTransitionsState,
        event: ParallelRegionsTakeOwnTransitionsEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is ParallelRegionsTakeOwnTransitionsState.Within -> when (transitionIndex) {
            1 -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:62 :: within :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("m", "m"), com.sce.runtime.ScriptSource.lua("_scxml_add(m, 1)", "m + 1"))
            }
            else -> {}
        }
        is ParallelRegionsTakeOwnTransitionsState.Working -> when (transitionIndex) {
            2 -> {
                // SCE-MAP: parallel_regions_take_own_transitions.scxml:38 :: working :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("n", "n"), com.sce.runtime.ScriptSource.lua("_scxml_add(n, 1)", "n + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
