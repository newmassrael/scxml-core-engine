// SCE-GENERATED — DO NOT EDIT
// source-hash: c56e8b2e82b26aafed117bfaa06905c41b2c8e5d207725d3f84b7293eb1eb4ee
// template-hash: 425ba724b674422eeb8ae587e59be1ebd91946f100b21ea20ddcaaca3bba7133
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_origin_is_a_location/event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:3 :: _machine

package com.sce.integration.event_origin_is_a_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventOriginIsALocationSceSynthInvokeInvPeerState : State {
    data object Acked : EventOriginIsALocationSceSynthInvokeInvPeerState
    data object Emit : EventOriginIsALocationSceSynthInvokeInvPeerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventOriginIsALocationSceSynthInvokeInvPeerEvent : Event {
    sealed interface Error : EventOriginIsALocationSceSynthInvokeInvPeerEvent {
        data object Execution : Error
    }
    data object FromChild : EventOriginIsALocationSceSynthInvokeInvPeerEvent
    data object Reply : EventOriginIsALocationSceSynthInvokeInvPeerEvent
    data object ReplyArrived : EventOriginIsALocationSceSynthInvokeInvPeerEvent
}
// --- State Machine (W3C SCXML) ---

class EventOriginIsALocationSceSynthInvokeInvPeerStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventOriginIsALocationSceSynthInvokeInvPeerState, EventOriginIsALocationSceSynthInvokeInvPeerEvent>(scriptEngine) {

    override val initialState: EventOriginIsALocationSceSynthInvokeInvPeerState = EventOriginIsALocationSceSynthInvokeInvPeerState.Emit

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
    override fun resolveState(stateId: String): EventOriginIsALocationSceSynthInvokeInvPeerState? = when (stateId) {
        "acked" -> EventOriginIsALocationSceSynthInvokeInvPeerState.Acked
        "emit" -> EventOriginIsALocationSceSynthInvokeInvPeerState.Emit
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventOriginIsALocationSceSynthInvokeInvPeerState): String = when (state) {
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Acked -> "acked"
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> "emit"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventOriginIsALocationSceSynthInvokeInvPeerState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventOriginIsALocationSceSynthInvokeInvPeerState): Int = when (state) {
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Acked -> 1
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventOriginIsALocationSceSynthInvokeInvPeerEvent? = when (name) {
        "error.execution" -> EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution
        "fromChild" -> EventOriginIsALocationSceSynthInvokeInvPeerEvent.FromChild
        "reply" -> EventOriginIsALocationSceSynthInvokeInvPeerEvent.Reply
        "replyArrived" -> EventOriginIsALocationSceSynthInvokeInvPeerEvent.ReplyArrived
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventOriginIsALocationSceSynthInvokeInvPeerEvent): String? = when (event) {
        is EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution -> "error.execution"
        is EventOriginIsALocationSceSynthInvokeInvPeerEvent.FromChild -> "fromChild"
        is EventOriginIsALocationSceSynthInvokeInvPeerEvent.Reply -> "reply"
        is EventOriginIsALocationSceSynthInvokeInvPeerEvent.ReplyArrived -> "replyArrived"
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
            "event_origin_is_a_location__sce_synth_invoke__inv_peer",
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
            raisePlatformError(EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventOriginIsALocationSceSynthInvokeInvPeerEvent) {
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
        state: EventOriginIsALocationSceSynthInvokeInvPeerState,
        event: EventOriginIsALocationSceSynthInvokeInvPeerEvent
    ): TransitionResult<EventOriginIsALocationSceSynthInvokeInvPeerState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> processEmit(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processEmit(
        event: EventOriginIsALocationSceSynthInvokeInvPeerEvent
    ): TransitionResult<EventOriginIsALocationSceSynthInvokeInvPeerState> = when {
        event is EventOriginIsALocationSceSynthInvokeInvPeerEvent.Reply -> TransitionResult.External(EventOriginIsALocationSceSynthInvokeInvPeerState.Acked, EventOriginIsALocationSceSynthInvokeInvPeerState.Emit)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:3 :: _machine
    override fun onEntry(state: EventOriginIsALocationSceSynthInvokeInvPeerState, pathChild: EventOriginIsALocationSceSynthInvokeInvPeerState?) {
        when (state) {
            is EventOriginIsALocationSceSynthInvokeInvPeerState.Acked -> {
                // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:16 :: acked :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("acked")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> {
                // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:5 :: emit :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("emit")) return


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsP, "myLocation", engineP.evaluateExpr(sidP, "_ioprocessors['scxml'].location"))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and value.
                    raisePlatformError(EventOriginIsALocationSceSynthInvokeInvPeerEvent.Error.Execution, "<send> <param name='myLocation'> expr failed to evaluate")
                }

                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("fromChild", eventDataP)
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:3 :: _machine
    override fun onExit(state: EventOriginIsALocationSceSynthInvokeInvPeerState) {
        when (state) {
            is EventOriginIsALocationSceSynthInvokeInvPeerState.Acked -> {
                // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:16 :: acked :: _state_body
                activeStateIds.remove("acked")
            }
            is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> {
                // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:5 :: emit :: _state_body
                activeStateIds.remove("emit")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: EventOriginIsALocationSceSynthInvokeInvPeerState,
        event: EventOriginIsALocationSceSynthInvokeInvPeerEvent?
    ) {
        when (source) {
        is EventOriginIsALocationSceSynthInvokeInvPeerState.Emit -> when {
            event is EventOriginIsALocationSceSynthInvokeInvPeerEvent.Reply -> {
                // SCE-MAP: event_origin_is_a_location__sce_synth_invoke__inv_peer.scxml:12 :: emit :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("replyArrived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
