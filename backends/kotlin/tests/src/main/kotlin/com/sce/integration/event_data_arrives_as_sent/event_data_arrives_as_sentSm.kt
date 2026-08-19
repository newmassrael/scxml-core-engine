// SCE-GENERATED — DO NOT EDIT
// source-hash: dc158df534067da964bd1c6f80973e1679ee7c64e201d638b706cd25b18535cd
// template-hash: e1ef1a80ec6f1d98421ed2b76701aed66a2f64164d943082fb9a22d750e546a9
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_data_arrives_as_sent.scxml:51 :: _machine

package com.sce.integration.event_data_arrives_as_sent

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventDataArrivesAsSentState : State {
    data object Evaluated : EventDataArrivesAsSentState
    data object Garbled : EventDataArrivesAsSentState
    data object Heard : EventDataArrivesAsSentState
    data object Mangled : EventDataArrivesAsSentState
    data object Quoted : EventDataArrivesAsSentState
    data object Settled : EventDataArrivesAsSentState
    data object Waiting : EventDataArrivesAsSentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventDataArrivesAsSentEvent : Event {
    data object Arith : EventDataArrivesAsSentEvent
    sealed interface Error : EventDataArrivesAsSentEvent {
        data object Execution : Error
    }
    data object Note : EventDataArrivesAsSentEvent
    data object Payload : EventDataArrivesAsSentEvent
}
// --- State Machine (W3C SCXML) ---

class EventDataArrivesAsSentStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventDataArrivesAsSentState, EventDataArrivesAsSentEvent>(scriptEngine) {

    override val initialState: EventDataArrivesAsSentState = EventDataArrivesAsSentState.Waiting

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
    override fun resolveState(stateId: String): EventDataArrivesAsSentState? = when (stateId) {
        "evaluated" -> EventDataArrivesAsSentState.Evaluated
        "garbled" -> EventDataArrivesAsSentState.Garbled
        "heard" -> EventDataArrivesAsSentState.Heard
        "mangled" -> EventDataArrivesAsSentState.Mangled
        "quoted" -> EventDataArrivesAsSentState.Quoted
        "settled" -> EventDataArrivesAsSentState.Settled
        "waiting" -> EventDataArrivesAsSentState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventDataArrivesAsSentState): String = when (state) {
        is EventDataArrivesAsSentState.Evaluated -> "evaluated"
        is EventDataArrivesAsSentState.Garbled -> "garbled"
        is EventDataArrivesAsSentState.Heard -> "heard"
        is EventDataArrivesAsSentState.Mangled -> "mangled"
        is EventDataArrivesAsSentState.Quoted -> "quoted"
        is EventDataArrivesAsSentState.Settled -> "settled"
        is EventDataArrivesAsSentState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventDataArrivesAsSentState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventDataArrivesAsSentState): Int = when (state) {
        is EventDataArrivesAsSentState.Evaluated -> 6
        is EventDataArrivesAsSentState.Garbled -> 5
        is EventDataArrivesAsSentState.Heard -> 1
        is EventDataArrivesAsSentState.Mangled -> 4
        is EventDataArrivesAsSentState.Quoted -> 2
        is EventDataArrivesAsSentState.Settled -> 3
        is EventDataArrivesAsSentState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventDataArrivesAsSentEvent? = when (name) {
        "arith" -> EventDataArrivesAsSentEvent.Arith
        "error.execution" -> EventDataArrivesAsSentEvent.Error.Execution
        "note" -> EventDataArrivesAsSentEvent.Note
        "payload" -> EventDataArrivesAsSentEvent.Payload
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventDataArrivesAsSentEvent): String? = when (event) {
        is EventDataArrivesAsSentEvent.Arith -> "arith"
        is EventDataArrivesAsSentEvent.Error.Execution -> "error.execution"
        is EventDataArrivesAsSentEvent.Note -> "note"
        is EventDataArrivesAsSentEvent.Payload -> "payload"
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
            "event_data_arrives_as_sent",
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
            raisePlatformError(EventDataArrivesAsSentEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EventDataArrivesAsSentEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventDataArrivesAsSentEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventDataArrivesAsSentEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventDataArrivesAsSentEvent) {
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
        state: EventDataArrivesAsSentState,
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EventDataArrivesAsSentState.Heard -> processHeard(event)
        is EventDataArrivesAsSentState.Quoted -> processQuoted(event)
        is EventDataArrivesAsSentState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processHeard(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Note && safeEvaluateGuard("_event.data === 'hold the line'") -> TransitionResult.External(EventDataArrivesAsSentState.Quoted, EventDataArrivesAsSentState.Heard)

        event is EventDataArrivesAsSentEvent.Note -> TransitionResult.External(EventDataArrivesAsSentState.Garbled, EventDataArrivesAsSentState.Heard)

        else -> TransitionResult.Ignored
    }

    private fun processQuoted(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Arith && safeEvaluateGuard("_event.data === '2 + 3'") -> TransitionResult.External(EventDataArrivesAsSentState.Settled, EventDataArrivesAsSentState.Quoted)

        event is EventDataArrivesAsSentEvent.Arith -> TransitionResult.External(EventDataArrivesAsSentState.Evaluated, EventDataArrivesAsSentState.Quoted)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Payload && safeEvaluateGuard("_event.data && _event.data.milestone === 'refined' && _event.data.turns === 2") -> TransitionResult.External(EventDataArrivesAsSentState.Heard, EventDataArrivesAsSentState.Waiting)

        event is EventDataArrivesAsSentEvent.Payload -> TransitionResult.External(EventDataArrivesAsSentState.Mangled, EventDataArrivesAsSentState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_data_arrives_as_sent.scxml:51 :: _machine
    override fun onEntry(state: EventDataArrivesAsSentState, pathChild: EventDataArrivesAsSentState?) {
        when (state) {
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:74 :: evaluated :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("evaluated")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: garbled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("garbled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:61 :: heard :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("heard")) return
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:72 :: mangled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("mangled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:66 :: quoted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("quoted")) return
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:71 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:54 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_data_arrives_as_sent.scxml:51 :: _machine
    override fun onExit(state: EventDataArrivesAsSentState) {
        when (state) {
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:74 :: evaluated :: _state_body
                activeStateIds.remove("evaluated")
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: garbled :: _state_body
                activeStateIds.remove("garbled")
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:61 :: heard :: _state_body
                activeStateIds.remove("heard")
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:72 :: mangled :: _state_body
                activeStateIds.remove("mangled")
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:66 :: quoted :: _state_body
                activeStateIds.remove("quoted")
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:71 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:54 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: event_data_arrives_as_sent.scxml:51 :: _machine
    override fun executeTransitionActions(
        source: EventDataArrivesAsSentState,
        event: EventDataArrivesAsSentEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
