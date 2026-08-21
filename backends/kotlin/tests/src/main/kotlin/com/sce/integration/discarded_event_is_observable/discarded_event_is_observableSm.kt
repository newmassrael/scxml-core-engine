// SCE-GENERATED — DO NOT EDIT
// source-hash: df2ef2c591564c7e52022e112ae9c5e384db80574b200165584f410ac8201d24
// template-hash: 84a841eae761d6fbf94d15cd646ae14f47646822f90559441b47e8f14bddfb19
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/discarded_event_is_observable/discarded_event_is_observable.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: discarded_event_is_observable.scxml:30 :: _machine

package com.sce.integration.discarded_event_is_observable

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DiscardedEventIsObservableState : State {
    data object Busy : DiscardedEventIsObservableState
    data object Done : DiscardedEventIsObservableState
    data object Idle : DiscardedEventIsObservableState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DiscardedEventIsObservableEvent : Event {
    sealed interface Error : DiscardedEventIsObservableEvent {
        data object Execution : Error
    }
    data object Go : DiscardedEventIsObservableEvent
    data object Nudge : DiscardedEventIsObservableEvent
    data object Poke : DiscardedEventIsObservableEvent
    data object Settle : DiscardedEventIsObservableEvent
}
// --- State Machine (W3C SCXML) ---

class DiscardedEventIsObservableStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DiscardedEventIsObservableState, DiscardedEventIsObservableEvent>(scriptEngine) {

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

    /**
     * §scxml-5.3: what the `nudges` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `nudges` was assigned a value of another type, or the engine refused.
     */
    fun nudges(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "nudges")

    override val initialState: DiscardedEventIsObservableState = DiscardedEventIsObservableState.Idle

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
    override fun resolveState(stateId: String): DiscardedEventIsObservableState? = when (stateId) {
        "busy" -> DiscardedEventIsObservableState.Busy
        "done" -> DiscardedEventIsObservableState.Done
        "idle" -> DiscardedEventIsObservableState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DiscardedEventIsObservableState): String = when (state) {
        is DiscardedEventIsObservableState.Busy -> "busy"
        is DiscardedEventIsObservableState.Done -> "done"
        is DiscardedEventIsObservableState.Idle -> "idle"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DiscardedEventIsObservableState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DiscardedEventIsObservableState): Int = when (state) {
        is DiscardedEventIsObservableState.Busy -> 1
        is DiscardedEventIsObservableState.Done -> 2
        is DiscardedEventIsObservableState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DiscardedEventIsObservableEvent? = when (name) {
        "error.execution" -> DiscardedEventIsObservableEvent.Error.Execution
        "go" -> DiscardedEventIsObservableEvent.Go
        "nudge" -> DiscardedEventIsObservableEvent.Nudge
        "poke" -> DiscardedEventIsObservableEvent.Poke
        "settle" -> DiscardedEventIsObservableEvent.Settle
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DiscardedEventIsObservableEvent): String? = when (event) {
        is DiscardedEventIsObservableEvent.Error.Execution -> "error.execution"
        is DiscardedEventIsObservableEvent.Go -> "go"
        is DiscardedEventIsObservableEvent.Nudge -> "nudge"
        is DiscardedEventIsObservableEvent.Poke -> "poke"
        is DiscardedEventIsObservableEvent.Settle -> "settle"
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
            "discarded_event_is_observable",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'nudges' with expr
        try {
            val initResult_nudges = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "nudges", initResult_nudges)
        } catch (e: Exception) {
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "<data id='nudges'> expr failed to evaluate")
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
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(DiscardedEventIsObservableEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DiscardedEventIsObservableEvent) {
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
        state: DiscardedEventIsObservableState,
        event: DiscardedEventIsObservableEvent
    ): TransitionResult<DiscardedEventIsObservableState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is DiscardedEventIsObservableState.Busy -> processBusy(event)
        is DiscardedEventIsObservableState.Idle -> processIdle(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processBusy(
        event: DiscardedEventIsObservableEvent
    ): TransitionResult<DiscardedEventIsObservableState> = when {
        event is DiscardedEventIsObservableEvent.Settle -> TransitionResult.External(DiscardedEventIsObservableState.Done, DiscardedEventIsObservableState.Busy)

        else -> TransitionResult.Ignored
    }

    private fun processIdle(
        event: DiscardedEventIsObservableEvent
    ): TransitionResult<DiscardedEventIsObservableState> = when {
        event is DiscardedEventIsObservableEvent.Poke -> TransitionResult.External(DiscardedEventIsObservableState.Idle, DiscardedEventIsObservableState.Idle)

        // W3C SCXML 3.13: Targetless transition (actions only)
        event is DiscardedEventIsObservableEvent.Nudge -> TransitionResult.Internal
        event is DiscardedEventIsObservableEvent.Go -> TransitionResult.External(DiscardedEventIsObservableState.Busy, DiscardedEventIsObservableState.Idle)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: discarded_event_is_observable.scxml:30 :: _machine
    override fun onEntry(state: DiscardedEventIsObservableState, pathChild: DiscardedEventIsObservableState?) {
        when (state) {
            is DiscardedEventIsObservableState.Busy -> {
                // SCE-MAP: discarded_event_is_observable.scxml:46 :: busy :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("busy")) return
            }
            is DiscardedEventIsObservableState.Done -> {
                // SCE-MAP: discarded_event_is_observable.scxml:49 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DiscardedEventIsObservableState.Idle -> {
                // SCE-MAP: discarded_event_is_observable.scxml:37 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: discarded_event_is_observable.scxml:30 :: _machine
    override fun onExit(state: DiscardedEventIsObservableState) {
        when (state) {
            is DiscardedEventIsObservableState.Busy -> {
                // SCE-MAP: discarded_event_is_observable.scxml:46 :: busy :: _state_body
                activeStateIds.remove("busy")
            }
            is DiscardedEventIsObservableState.Done -> {
                // SCE-MAP: discarded_event_is_observable.scxml:49 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is DiscardedEventIsObservableState.Idle -> {
                // SCE-MAP: discarded_event_is_observable.scxml:37 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: discarded_event_is_observable.scxml:30 :: _machine
    override fun executeTransitionActions(
        source: DiscardedEventIsObservableState,
        event: DiscardedEventIsObservableEvent?
    ) {
        when (source) {
        is DiscardedEventIsObservableState.Idle -> when {
            event is DiscardedEventIsObservableEvent.Poke -> {
                // SCE-MAP: discarded_event_is_observable.scxml:38 :: idle :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            event is DiscardedEventIsObservableEvent.Nudge -> {
                // SCE-MAP: discarded_event_is_observable.scxml:41 :: idle :: _transition_1


            executeAssign("nudges", "nudges + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
