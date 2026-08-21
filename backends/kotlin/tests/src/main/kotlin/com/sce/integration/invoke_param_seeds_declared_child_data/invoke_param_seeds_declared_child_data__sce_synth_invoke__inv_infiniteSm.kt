// SCE-GENERATED — DO NOT EDIT
// source-hash: a9b3d7b7ea8a5bd6001a98d04817a6efb870e7f83add64eb3bb769017877144d
// template-hash: f7291ab6d7896ee95dd448a8f7fc2759f6a0259c69bcc8f54f868651f4b8fe72
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:3 :: _machine

package com.sce.integration.invoke_param_seeds_declared_child_data

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState : State {
    data object Done : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState
    data object Report : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent : Event {
    sealed interface Error : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent {
        data object Execution : Error
    }
    sealed interface Seed : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent {
        data object Collapsed : Seed
        data object Missing : Seed
        data object Ok : Seed
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `seen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `seen` was assigned a value of another type, or the engine refused.
     */
    fun seen(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "seen")

    override val initialState: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report

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
    override fun resolveState(stateId: String): InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState? = when (stateId) {
        "done" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done
        "report" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState): String = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done -> "done"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> "report"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState): Int = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done -> 1
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent? = when (name) {
        "error.execution" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution
        "seed.collapsed" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Collapsed
        "seed.missing" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Missing
        "seed.ok" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Ok
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent): String? = when (event) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution -> "error.execution"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Collapsed -> "seed.collapsed"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Missing -> "seed.missing"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Seed.Ok -> "seed.ok"
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
            "invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'seen' with expr
        try {
            val initResult_seen = engine.evaluateExpr(sid, "'unset'")
            engine.setVariable(sid, "seen", initResult_seen)
        } catch (e: Exception) {
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution, "<data id='seen'> expr failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent) {
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
        state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState,
        event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState> = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> processNullReport()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullReport(
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState> = when {
        safeEvaluateGuard("typeof seen === 'number' && seen > 1e308") -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report)
        safeEvaluateGuard("typeof seen === 'string'") -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:3 :: _machine
    override fun onEntry(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState, pathChild: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState?) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:19 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:8 :: report :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("report")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:3 :: _machine
    override fun onExit(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Done -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:19 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:8 :: report :: _state_body
                activeStateIds.remove("report")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState,
        event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteEvent?
    ) {
        when (source) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteState.Report -> when {
            event == null && safeEvaluateGuard("typeof seen === 'number' && seen > 1e308") -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:9 :: report :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.ok", "")
            }
            event == null && safeEvaluateGuard("typeof seen === 'string'") -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:12 :: report :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.missing", "")
            }
            event == null -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_infinite.scxml:15 :: report :: _transition_2


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.collapsed", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
