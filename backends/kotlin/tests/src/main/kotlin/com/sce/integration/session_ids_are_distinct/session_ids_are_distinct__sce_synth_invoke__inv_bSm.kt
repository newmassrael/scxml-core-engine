// SCE-GENERATED — DO NOT EDIT
// source-hash: 72e5f6add40450019fedf97192aa7f8b2b99f0983d778103d9af035fcb5f7cfa
// template-hash: b282d63ae523573aa0c92c912a0dda6cb9508b9193d3508ff15b98a4ec52a48a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/session_ids_are_distinct/session_ids_are_distinct__sce_synth_invoke__inv_b.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:3 :: _machine

package com.sce.integration.session_ids_are_distinct

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SessionIdsAreDistinctSceSynthInvokeInvBState : State {
    data object Emit : SessionIdsAreDistinctSceSynthInvokeInvBState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SessionIdsAreDistinctSceSynthInvokeInvBEvent : Event {
    sealed interface Error : SessionIdsAreDistinctSceSynthInvokeInvBEvent {
        data object Execution : Error
    }
    data object FromChild : SessionIdsAreDistinctSceSynthInvokeInvBEvent
}
// --- State Machine (W3C SCXML) ---

class SessionIdsAreDistinctSceSynthInvokeInvBStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<SessionIdsAreDistinctSceSynthInvokeInvBState, SessionIdsAreDistinctSceSynthInvokeInvBEvent>(scriptEngine) {

    override val initialState: SessionIdsAreDistinctSceSynthInvokeInvBState = SessionIdsAreDistinctSceSynthInvokeInvBState.Emit

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SessionIdsAreDistinctSceSynthInvokeInvBState? = when (stateId) {
        "emit" -> SessionIdsAreDistinctSceSynthInvokeInvBState.Emit
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SessionIdsAreDistinctSceSynthInvokeInvBState): String = when (state) {
        is SessionIdsAreDistinctSceSynthInvokeInvBState.Emit -> "emit"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SessionIdsAreDistinctSceSynthInvokeInvBState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: SessionIdsAreDistinctSceSynthInvokeInvBState): Int = when (state) {
        is SessionIdsAreDistinctSceSynthInvokeInvBState.Emit -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SessionIdsAreDistinctSceSynthInvokeInvBEvent? = when (name) {
        "error.execution" -> SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution
        "fromChild" -> SessionIdsAreDistinctSceSynthInvokeInvBEvent.FromChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SessionIdsAreDistinctSceSynthInvokeInvBEvent): String? = when (event) {
        is SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution -> "error.execution"
        is SessionIdsAreDistinctSceSynthInvokeInvBEvent.FromChild -> "fromChild"
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
            "session_ids_are_distinct__sce_synth_invoke__inv_b",
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
            raiseInternal(SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution)
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
            raiseInternal(SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution)
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
            raiseInternal(SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution)
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
            raiseInternal(SessionIdsAreDistinctSceSynthInvokeInvBEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: SessionIdsAreDistinctSceSynthInvokeInvBEvent) {
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
        state: SessionIdsAreDistinctSceSynthInvokeInvBState,
        event: SessionIdsAreDistinctSceSynthInvokeInvBEvent
    ): TransitionResult<SessionIdsAreDistinctSceSynthInvokeInvBState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:3 :: _machine
    override fun onEntry(state: SessionIdsAreDistinctSceSynthInvokeInvBState, pathChild: SessionIdsAreDistinctSceSynthInvokeInvBState?) {
        when (state) {
            is SessionIdsAreDistinctSceSynthInvokeInvBState.Emit -> {
                // SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:5 :: emit :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("emit")) return


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try { putParam(paramsP, "sid", engineP.evaluateExpr(sidP, "_sessionid")) } catch (_: Exception) { putParam(paramsP, "sid", "") }
                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("fromChild", eventDataP)
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:3 :: _machine
    override fun onExit(state: SessionIdsAreDistinctSceSynthInvokeInvBState) {
        when (state) {
            is SessionIdsAreDistinctSceSynthInvokeInvBState.Emit -> {
                // SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:5 :: emit :: _state_body
                activeStateIds.remove("emit")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: session_ids_are_distinct__sce_synth_invoke__inv_b.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: SessionIdsAreDistinctSceSynthInvokeInvBState,
        event: SessionIdsAreDistinctSceSynthInvokeInvBEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
