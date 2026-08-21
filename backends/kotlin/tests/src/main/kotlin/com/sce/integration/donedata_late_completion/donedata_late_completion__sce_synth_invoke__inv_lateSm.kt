// SCE-GENERATED — DO NOT EDIT
// source-hash: a31c47a0247af69ee06a626967ff0d05ffe8ed68e66f9b9928d0b71cb7eccebd
// template-hash: 2cf4917c7dff79eaf746b52e649909e9c7318e80b65f49555ba6a2bcd0d8eaca
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/donedata_late_completion/donedata_late_completion__sce_synth_invoke__inv_late.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:3 :: _machine

package com.sce.integration.donedata_late_completion

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLateCompletionSceSynthInvokeInvLateState : State {
    data object Settled : DonedataLateCompletionSceSynthInvokeInvLateState
    data object Waiting : DonedataLateCompletionSceSynthInvokeInvLateState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLateCompletionSceSynthInvokeInvLateEvent : Event {
    sealed interface Error : DonedataLateCompletionSceSynthInvokeInvLateEvent {
        data object Execution : Error
    }
    data object Finish : DonedataLateCompletionSceSynthInvokeInvLateEvent
    data object Ready : DonedataLateCompletionSceSynthInvokeInvLateEvent
}
// --- State Machine (W3C SCXML) ---

class DonedataLateCompletionSceSynthInvokeInvLateStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DonedataLateCompletionSceSynthInvokeInvLateState, DonedataLateCompletionSceSynthInvokeInvLateEvent>(scriptEngine) {

    override val initialState: DonedataLateCompletionSceSynthInvokeInvLateState = DonedataLateCompletionSceSynthInvokeInvLateState.Waiting

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
    override fun resolveState(stateId: String): DonedataLateCompletionSceSynthInvokeInvLateState? = when (stateId) {
        "settled" -> DonedataLateCompletionSceSynthInvokeInvLateState.Settled
        "waiting" -> DonedataLateCompletionSceSynthInvokeInvLateState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLateCompletionSceSynthInvokeInvLateState): String = when (state) {
        is DonedataLateCompletionSceSynthInvokeInvLateState.Settled -> "settled"
        is DonedataLateCompletionSceSynthInvokeInvLateState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLateCompletionSceSynthInvokeInvLateState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLateCompletionSceSynthInvokeInvLateState): Int = when (state) {
        is DonedataLateCompletionSceSynthInvokeInvLateState.Settled -> 1
        is DonedataLateCompletionSceSynthInvokeInvLateState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLateCompletionSceSynthInvokeInvLateEvent? = when (name) {
        "error.execution" -> DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution
        "finish" -> DonedataLateCompletionSceSynthInvokeInvLateEvent.Finish
        "ready" -> DonedataLateCompletionSceSynthInvokeInvLateEvent.Ready
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLateCompletionSceSynthInvokeInvLateEvent): String? = when (event) {
        is DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution -> "error.execution"
        is DonedataLateCompletionSceSynthInvokeInvLateEvent.Finish -> "finish"
        is DonedataLateCompletionSceSynthInvokeInvLateEvent.Ready -> "ready"
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
            "donedata_late_completion__sce_synth_invoke__inv_late",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )





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
            raisePlatformError(DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLateCompletionSceSynthInvokeInvLateEvent) {
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
        state: DonedataLateCompletionSceSynthInvokeInvLateState,
        event: DonedataLateCompletionSceSynthInvokeInvLateEvent
    ): TransitionResult<DonedataLateCompletionSceSynthInvokeInvLateState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is DonedataLateCompletionSceSynthInvokeInvLateState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processWaiting(
        event: DonedataLateCompletionSceSynthInvokeInvLateEvent
    ): TransitionResult<DonedataLateCompletionSceSynthInvokeInvLateState> = when {
        event is DonedataLateCompletionSceSynthInvokeInvLateEvent.Finish -> TransitionResult.External(DonedataLateCompletionSceSynthInvokeInvLateState.Settled, DonedataLateCompletionSceSynthInvokeInvLateState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:3 :: _machine
    override fun onEntry(state: DonedataLateCompletionSceSynthInvokeInvLateState, pathChild: DonedataLateCompletionSceSynthInvokeInvLateState?) {
        when (state) {
            is DonedataLateCompletionSceSynthInvokeInvLateState.Settled -> {
                // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:11 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    var doneParamStructuralError = false
                    try {
                        doneParams["result"] = engineDD.evaluateExpr(sidDD, "42")
                    } catch (_: Exception) {
                        // W3C SCXML 5.7: Runtime param error — raise error.execution but continue
                        raisePlatformError(DonedataLateCompletionSceSynthInvokeInvLateEvent.Error.Execution, "<donedata> <param name='result'> failed to evaluate")
                    }
                    // C++ DoneDataHelper pattern: if (!success) break — skip done.state on structural error only
                    if (doneParamStructuralError) return@run
                    if (doneParams.isNotEmpty()) {
                        doneEventData = buildJsonFromParams(doneParams)
                    }
                    // W3C SCXML 5.5 + 6.3.1: stash onto the engine so the invoking parent's
                    // startInvoke completion callback can lift the payload onto
                    // done.invoke.<id>._event.data. Mirrors C++ AOT stashDonedataAtFinal.
                    stashDonedataAtFinal(doneEventData)
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLateCompletionSceSynthInvokeInvLateState.Waiting -> {
                // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:5 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:3 :: _machine
    override fun onExit(state: DonedataLateCompletionSceSynthInvokeInvLateState) {
        when (state) {
            is DonedataLateCompletionSceSynthInvokeInvLateState.Settled -> {
                // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:11 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is DonedataLateCompletionSceSynthInvokeInvLateState.Waiting -> {
                // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:5 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: donedata_late_completion__sce_synth_invoke__inv_late.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: DonedataLateCompletionSceSynthInvokeInvLateState,
        event: DonedataLateCompletionSceSynthInvokeInvLateEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
