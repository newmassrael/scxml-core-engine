// SCE-GENERATED — DO NOT EDIT
// source-hash: a9b3d7b7ea8a5bd6001a98d04817a6efb870e7f83add64eb3bb769017877144d
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:3 :: _machine

package com.sce.integration.invoke_param_seeds_declared_child_data

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState : State {
    data object Done : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState
    data object Report : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent : Event {
    sealed interface Error : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent {
        data object Execution : Error
    }
    sealed interface Seed : InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent {
        data object Missing : Seed
        data object Ok : Seed
        data object Shadowed : Seed
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent>(scriptEngine) {

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

    /**
     * §scxml-5.3: what the `token` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `token` was assigned a value of another type, or the engine refused.
     */
    fun token(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "token")

    override val initialState: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report

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
    override fun resolveState(stateId: String): InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState? = when (stateId) {
        "done" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done
        "report" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState): String = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done -> "done"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> "report"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState): Int = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done -> 1
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent? = when (name) {
        "error.execution" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution
        "seed.missing" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Missing
        "seed.ok" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Ok
        "seed.shadowed" -> InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Shadowed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent): String? = when (event) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution -> "error.execution"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Missing -> "seed.missing"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Ok -> "seed.ok"
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Seed.Shadowed -> "seed.shadowed"
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
            "invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'seen' with expr
        try {
            val initResult_seen = engine.evaluateExpr(sid, "'unset'")
            engine.setVariable(sid, "seen", initResult_seen)
        } catch (e: Exception) {
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "<data id='seen'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'token' with expr
        try {
            val initResult_token = engine.evaluateExpr(sid, "'child'")
            engine.setVariable(sid, "token", initResult_token)
        } catch (e: Exception) {
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "<data id='token'> expr failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent) {
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
        state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState,
        event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState> = when (state) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> processNullReport()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullReport(
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState> = when {
        safeEvaluateGuard("seen === 'parent'") -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report)
        safeEvaluateGuard("seen === 'child'") -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done, InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:3 :: _machine
    override fun onEntry(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState, pathChild: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState?) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:20 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:9 :: report :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("report")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:3 :: _machine
    override fun onExit(state: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Done -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:20 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:9 :: report :: _state_body
                activeStateIds.remove("report")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState,
        event: InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowEvent?
    ) {
        when (source) {
        is InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowState.Report -> when {
            event == null && safeEvaluateGuard("seen === 'parent'") -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:10 :: report :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.ok", "")
            }
            event == null && safeEvaluateGuard("seen === 'child'") -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:13 :: report :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.shadowed", "")
            }
            event == null -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data__sce_synth_invoke__inv_shadow.scxml:16 :: report :: _transition_2


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("seed.missing", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
