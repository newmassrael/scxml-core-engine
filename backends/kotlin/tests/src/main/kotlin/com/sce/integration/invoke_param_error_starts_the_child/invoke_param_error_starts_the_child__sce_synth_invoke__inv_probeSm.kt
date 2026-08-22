// SCE-GENERATED — DO NOT EDIT
// source-hash: 387554cf9d8d5415c8347a9554c4bb2db1133a43787a7fb935ba3f3f9103b433
// template-hash: 465642caa5c7ae5f006b7e4c3302ebaf26878f27c380322c3cf9d87ca35b0ee6
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_param_error_starts_the_child/invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine

package com.sce.integration.invoke_param_error_starts_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState : State {
    data object Done : InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState
    data object Report : InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent : Event {
    data object ChildUp : InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent
    sealed interface Error : InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState, InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent>(scriptEngine) {

    override val initialState: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState = InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report

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
    override fun resolveState(stateId: String): InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState? = when (stateId) {
        "done" -> InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done
        "report" -> InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState): String = when (state) {
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done -> "done"
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> "report"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState): Int = when (state) {
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done -> 1
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent? = when (name) {
        "childUp" -> InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.ChildUp
        "error.execution" -> InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent): String? = when (event) {
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.ChildUp -> "childUp"
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution -> "error.execution"
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
            "invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'kept' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "kept", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'broken' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "broken", null)
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
            raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent) {
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
        state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState,
        event: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent
    ): TransitionResult<InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState
    ): TransitionResult<InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState> = when (state) {
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> processNullReport()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullReport(
    ): TransitionResult<InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done, InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onEntry(state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState, pathChild: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState?) {
        when (state) {
            is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done -> {
                // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:33 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> {
                // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:25 :: report :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("report")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onExit(state: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState) {
        when (state) {
            is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Done -> {
                // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:33 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> {
                // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:25 :: report :: _state_body
                activeStateIds.remove("report")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState,
        event: InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent?
    ) {
        when (source) {
        is InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeState.Report -> when {
            event == null -> {
                // SCE-MAP: invoke_param_error_starts_the_child__sce_synth_invoke__inv_probe.scxml:26 :: report :: _transition_0


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsP, "kept", engineP.evaluateExpr(sidP, "kept"))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and value.
                    raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "<send> <param name='kept'> expr failed to evaluate")
                }

                try {
                    putParam(paramsP, "brokenPlaceholder", engineP.evaluateExpr(sidP, "broken === ''"))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and value.
                    raisePlatformError(InvokeParamErrorStartsTheChildSceSynthInvokeInvProbeEvent.Error.Execution, "<send> <param name='brokenPlaceholder'> expr failed to evaluate")
                }

                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("childUp", eventDataP)
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
