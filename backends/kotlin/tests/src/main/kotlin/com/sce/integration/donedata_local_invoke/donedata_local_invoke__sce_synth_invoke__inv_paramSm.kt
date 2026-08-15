// SCE-GENERATED — DO NOT EDIT
// source-hash: 7072491d11c203791302209b1bf9b82270fe7555d8209b82381d2a9f2ebc3c9f
// template-hash: b5bef7d045160440c6e2790d4f2e0be757d7c1cc42dee75b2002b23fd477161e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/donedata_local_invoke/donedata_local_invoke__sce_synth_invoke__inv_param.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:3 :: _machine

package com.sce.integration.donedata_local_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLocalInvokeSceSynthInvokeInvParamState : State {
    data object Done : DonedataLocalInvokeSceSynthInvokeInvParamState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLocalInvokeSceSynthInvokeInvParamEvent : Event {
    sealed interface Error : DonedataLocalInvokeSceSynthInvokeInvParamEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class DonedataLocalInvokeSceSynthInvokeInvParamStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DonedataLocalInvokeSceSynthInvokeInvParamState, DonedataLocalInvokeSceSynthInvokeInvParamEvent>(scriptEngine) {

    override val initialState: DonedataLocalInvokeSceSynthInvokeInvParamState = DonedataLocalInvokeSceSynthInvokeInvParamState.Done

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): DonedataLocalInvokeSceSynthInvokeInvParamState? = when (stateId) {
        "done" -> DonedataLocalInvokeSceSynthInvokeInvParamState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLocalInvokeSceSynthInvokeInvParamState): String = when (state) {
        is DonedataLocalInvokeSceSynthInvokeInvParamState.Done -> "done"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLocalInvokeSceSynthInvokeInvParamState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLocalInvokeSceSynthInvokeInvParamState): Int = when (state) {
        is DonedataLocalInvokeSceSynthInvokeInvParamState.Done -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLocalInvokeSceSynthInvokeInvParamEvent? = when (name) {
        "error.execution" -> DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLocalInvokeSceSynthInvokeInvParamEvent): String? = when (event) {
        is DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution -> "error.execution"
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
            "donedata_local_invoke__sce_synth_invoke__inv_param",
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution)
            false
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution)
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLocalInvokeSceSynthInvokeInvParamEvent) {
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
        state: DonedataLocalInvokeSceSynthInvokeInvParamState,
        event: DonedataLocalInvokeSceSynthInvokeInvParamEvent
    ): TransitionResult<DonedataLocalInvokeSceSynthInvokeInvParamState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:3 :: _machine
    override fun onEntry(state: DonedataLocalInvokeSceSynthInvokeInvParamState, pathChild: DonedataLocalInvokeSceSynthInvokeInvParamState?) {
        when (state) {
            is DonedataLocalInvokeSceSynthInvokeInvParamState.Done -> {
                // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:5 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
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
                        raiseInternal(DonedataLocalInvokeSceSynthInvokeInvParamEvent.Error.Execution, EventMetadata.platform())
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
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:3 :: _machine
    override fun onExit(state: DonedataLocalInvokeSceSynthInvokeInvParamState) {
        when (state) {
            is DonedataLocalInvokeSceSynthInvokeInvParamState.Done -> {
                // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:5 :: done :: _state_body
                activeStateIds.remove("done")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_param.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: DonedataLocalInvokeSceSynthInvokeInvParamState,
        event: DonedataLocalInvokeSceSynthInvokeInvParamEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
