// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c86e172c2fe9594af6e85729ce3077686db3dd0850ca7983a9286f577df2546
// template-hash: 40688f16ecbedc989f96890868d75e825c91fd7775d66bfd37b45df9857c9aa5
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/unseen_event_is_reported/unseen_event_is_reported.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: unseen_event_is_reported.scxml:41 :: _machine

package com.sce.integration.unseen_event_is_reported

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface UnseenEventIsReportedState : State {
    data object Done : UnseenEventIsReportedState
    data object Working : UnseenEventIsReportedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface UnseenEventIsReportedEvent : Event {
    sealed interface Error : UnseenEventIsReportedEvent {
        data object Execution : Error
    }
    data object Finish : UnseenEventIsReportedEvent
    data object Poke : UnseenEventIsReportedEvent
}
// --- State Machine (W3C SCXML) ---

class UnseenEventIsReportedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<UnseenEventIsReportedState, UnseenEventIsReportedEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pokes` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pokes` was assigned a value of another type, or the engine refused.
     */
    fun pokes(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pokes")

    override val initialState: UnseenEventIsReportedState = UnseenEventIsReportedState.Working

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): UnseenEventIsReportedState? = when (stateId) {
        "done" -> UnseenEventIsReportedState.Done
        "working" -> UnseenEventIsReportedState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: UnseenEventIsReportedState): String = when (state) {
        is UnseenEventIsReportedState.Done -> "done"
        is UnseenEventIsReportedState.Working -> "working"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: UnseenEventIsReportedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: UnseenEventIsReportedState): Int = when (state) {
        is UnseenEventIsReportedState.Done -> 1
        is UnseenEventIsReportedState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): UnseenEventIsReportedEvent? = when (name) {
        "error.execution" -> UnseenEventIsReportedEvent.Error.Execution
        "finish" -> UnseenEventIsReportedEvent.Finish
        "poke" -> UnseenEventIsReportedEvent.Poke
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: UnseenEventIsReportedEvent): String? = when (event) {
        is UnseenEventIsReportedEvent.Error.Execution -> "error.execution"
        is UnseenEventIsReportedEvent.Finish -> "finish"
        is UnseenEventIsReportedEvent.Poke -> "poke"
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
            "unseen_event_is_reported",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(UnseenEventIsReportedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
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
            raisePlatformError(UnseenEventIsReportedEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(UnseenEventIsReportedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(UnseenEventIsReportedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(UnseenEventIsReportedEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: UnseenEventIsReportedEvent) {
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
        state: UnseenEventIsReportedState,
        event: UnseenEventIsReportedEvent
    ): TransitionResult<UnseenEventIsReportedState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is UnseenEventIsReportedState.Working -> processWorking(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processWorking(
        event: UnseenEventIsReportedEvent
    ): TransitionResult<UnseenEventIsReportedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is UnseenEventIsReportedEvent.Poke -> TransitionResult.Internal
        event is UnseenEventIsReportedEvent.Finish -> TransitionResult.External(UnseenEventIsReportedState.Done, UnseenEventIsReportedState.Working)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: unseen_event_is_reported.scxml:41 :: _machine
    override fun onEntry(state: UnseenEventIsReportedState, pathChild: UnseenEventIsReportedState?) {
        when (state) {
            is UnseenEventIsReportedState.Done -> {
                // SCE-MAP: unseen_event_is_reported.scxml:53 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is UnseenEventIsReportedState.Working -> {
                // SCE-MAP: unseen_event_is_reported.scxml:47 :: working :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("working")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: unseen_event_is_reported.scxml:41 :: _machine
    override fun onExit(state: UnseenEventIsReportedState) {
        when (state) {
            is UnseenEventIsReportedState.Done -> {
                // SCE-MAP: unseen_event_is_reported.scxml:53 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is UnseenEventIsReportedState.Working -> {
                // SCE-MAP: unseen_event_is_reported.scxml:47 :: working :: _state_body
                activeStateIds.remove("working")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: unseen_event_is_reported.scxml:41 :: _machine
    override fun executeTransitionActions(
        source: UnseenEventIsReportedState,
        event: UnseenEventIsReportedEvent?
    ) {
        when (source) {
        is UnseenEventIsReportedState.Working -> when {
            event is UnseenEventIsReportedEvent.Poke -> {
                // SCE-MAP: unseen_event_is_reported.scxml:48 :: working :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
