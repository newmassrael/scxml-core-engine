// SCE-GENERATED — DO NOT EDIT
// source-hash: c97edcb094613d8138825758fc943d853d23ad4854f2fa7dcf6ff6f58539b674
// template-hash: 4cbf0ce468f2db0011b4fa010e6c117357964548e492f95e76a21755c70778e3
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:3 :: _machine

package com.sce.integration.empty_finalize_updates_the_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState : State {
    data object Answer : EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState
    data object Done : EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent : Event {
    sealed interface Error : EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent {
        data object Execution : Error
    }
    data object FromEmptyChild : EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent
}
// --- State Machine (W3C SCXML) ---

class EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState, EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent>(scriptEngine) {

    override val initialState: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState = EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer

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
    override fun resolveState(stateId: String): EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState? = when (stateId) {
        "answer" -> EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer
        "done" -> EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState): String = when (state) {
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> "answer"
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done -> "done"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState): Int = when (state) {
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> 0
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent? = when (name) {
        "error.execution" -> EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution
        "fromEmptyChild" -> EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.FromEmptyChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent): String? = when (event) {
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution -> "error.execution"
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.FromEmptyChild -> "fromEmptyChild"
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
            "empty_finalize_updates_the_location__sce_synth_invoke__inv_empty",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'tally' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "tally", null)
        } catch (_: Exception) {}




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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent) {
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
        state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState,
        event: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState> = when (state) {
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> processNullAnswer()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullAnswer(
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done, EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:3 :: _machine
    override fun onEntry(state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState, pathChild: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState?) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> {
                // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:8 :: answer :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("answer")) return
            }
            is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done -> {
                // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:15 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:3 :: _machine
    override fun onExit(state: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> {
                // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:8 :: answer :: _state_body
                activeStateIds.remove("answer")
            }
            is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Done -> {
                // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:15 :: done :: _state_body
                activeStateIds.remove("done")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState,
        event: EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent?
    ) {
        when (source) {
        is EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyState.Answer -> when {
            event == null -> {
                // SCE-MAP: empty_finalize_updates_the_location__sce_synth_invoke__inv_empty.scxml:9 :: answer :: _transition_0


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsP, "tally", engineP.evaluateExpr(sidP, "7"))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and value.
                    raisePlatformError(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyEvent.Error.Execution, "<send> <param name='tally'> expr failed to evaluate")
                }

                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("fromEmptyChild", eventDataP)
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
