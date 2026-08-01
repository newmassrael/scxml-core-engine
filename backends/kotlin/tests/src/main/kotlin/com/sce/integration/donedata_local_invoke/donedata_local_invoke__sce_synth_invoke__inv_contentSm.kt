// SCE-GENERATED — DO NOT EDIT
// source-hash: 7072491d11c203791302209b1bf9b82270fe7555d8209b82381d2a9f2ebc3c9f
// template-hash: 621809d272871a188158c7111f76c3b2585929cabb7a1e888c3ace81ca2d63d2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/donedata_local_invoke/donedata_local_invoke__sce_synth_invoke__inv_content.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_content.scxml:3

package com.sce.integration.donedata_local_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLocalInvokeSceSynthInvokeInvContentState : State {
    data object Done : DonedataLocalInvokeSceSynthInvokeInvContentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLocalInvokeSceSynthInvokeInvContentEvent : Event {
    sealed interface Error : DonedataLocalInvokeSceSynthInvokeInvContentEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class DonedataLocalInvokeSceSynthInvokeInvContentStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DonedataLocalInvokeSceSynthInvokeInvContentState, DonedataLocalInvokeSceSynthInvokeInvContentEvent>(scriptEngine) {

    override val initialState: DonedataLocalInvokeSceSynthInvokeInvContentState = DonedataLocalInvokeSceSynthInvokeInvContentState.Done

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): DonedataLocalInvokeSceSynthInvokeInvContentState? = when (stateId) {
        "done" -> DonedataLocalInvokeSceSynthInvokeInvContentState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLocalInvokeSceSynthInvokeInvContentState): String = when (state) {
        is DonedataLocalInvokeSceSynthInvokeInvContentState.Done -> "done"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLocalInvokeSceSynthInvokeInvContentState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLocalInvokeSceSynthInvokeInvContentState): Int = when (state) {
        is DonedataLocalInvokeSceSynthInvokeInvContentState.Done -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLocalInvokeSceSynthInvokeInvContentEvent? = when (name) {
        "error.execution" -> DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLocalInvokeSceSynthInvokeInvContentEvent): String? = when (event) {
        is DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution -> "error.execution"
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
            "donedata_local_invoke__sce_synth_invoke__inv_content",
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution)
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution)
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
            raiseInternal(DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLocalInvokeSceSynthInvokeInvContentEvent) {
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
        val effectiveOrigin = if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
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
        state: DonedataLocalInvokeSceSynthInvokeInvContentState,
        event: DonedataLocalInvokeSceSynthInvokeInvContentEvent
    ): TransitionResult<DonedataLocalInvokeSceSynthInvokeInvContentState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_content.scxml:3
    override fun onEntry(state: DonedataLocalInvokeSceSynthInvokeInvContentState) {
        when (state) {
            is DonedataLocalInvokeSceSynthInvokeInvContentState.Done -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    ensureScriptEngine()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <content expr="..."/>
                    try {
                        val contentResult = engineDD.evaluateExpr(sidDD, "'hello_content'")
                        // C++ DoneDataHelper::evaluateContent: EventDataHelper::scriptValueToJsonString
                        doneEventData = if (contentResult != null) valueToJson(contentResult) else ""
                    } catch (_: Exception) {
                        raiseInternal(DonedataLocalInvokeSceSynthInvokeInvContentEvent.Error.Execution, EventMetadata.platform())
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
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_content.scxml:3
    override fun onExit(state: DonedataLocalInvokeSceSynthInvokeInvContentState) {
        when (state) {
            is DonedataLocalInvokeSceSynthInvokeInvContentState.Done -> {
                activeStateIds.remove("done")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: donedata_local_invoke__sce_synth_invoke__inv_content.scxml:3
    override fun executeTransitionActions(
        source: DonedataLocalInvokeSceSynthInvokeInvContentState,
        event: DonedataLocalInvokeSceSynthInvokeInvContentEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
