// SCE-GENERATED — DO NOT EDIT
// source-hash: 8e0f0b7b552dfbb89b9083db177a216e77a3534d3f6112690f84145daf0386d4
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_data_arrives_as_sent/event_data_arrives_as_sent.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine

package com.sce.integration.event_data_arrives_as_sent

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventDataArrivesAsSentState : State {
    data object Documented : EventDataArrivesAsSentState
    data object Evaluated : EventDataArrivesAsSentState
    data object Flattened : EventDataArrivesAsSentState
    data object Garbled : EventDataArrivesAsSentState
    data object Heard : EventDataArrivesAsSentState
    data object Mangled : EventDataArrivesAsSentState
    data object Opening : EventDataArrivesAsSentState
    data object Quoted : EventDataArrivesAsSentState
    data object Settled : EventDataArrivesAsSentState
    data object Swallowed : EventDataArrivesAsSentState
    data object Waiting : EventDataArrivesAsSentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventDataArrivesAsSentEvent : Event {
    data object Arith : EventDataArrivesAsSentEvent
    data object Broken : EventDataArrivesAsSentEvent
    data object Doc : EventDataArrivesAsSentEvent
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
        "documented" -> EventDataArrivesAsSentState.Documented
        "evaluated" -> EventDataArrivesAsSentState.Evaluated
        "flattened" -> EventDataArrivesAsSentState.Flattened
        "garbled" -> EventDataArrivesAsSentState.Garbled
        "heard" -> EventDataArrivesAsSentState.Heard
        "mangled" -> EventDataArrivesAsSentState.Mangled
        "opening" -> EventDataArrivesAsSentState.Opening
        "quoted" -> EventDataArrivesAsSentState.Quoted
        "settled" -> EventDataArrivesAsSentState.Settled
        "swallowed" -> EventDataArrivesAsSentState.Swallowed
        "waiting" -> EventDataArrivesAsSentState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventDataArrivesAsSentState): String = when (state) {
        is EventDataArrivesAsSentState.Documented -> "documented"
        is EventDataArrivesAsSentState.Evaluated -> "evaluated"
        is EventDataArrivesAsSentState.Flattened -> "flattened"
        is EventDataArrivesAsSentState.Garbled -> "garbled"
        is EventDataArrivesAsSentState.Heard -> "heard"
        is EventDataArrivesAsSentState.Mangled -> "mangled"
        is EventDataArrivesAsSentState.Opening -> "opening"
        is EventDataArrivesAsSentState.Quoted -> "quoted"
        is EventDataArrivesAsSentState.Settled -> "settled"
        is EventDataArrivesAsSentState.Swallowed -> "swallowed"
        is EventDataArrivesAsSentState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventDataArrivesAsSentState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventDataArrivesAsSentState): Int = when (state) {
        is EventDataArrivesAsSentState.Documented -> 3
        is EventDataArrivesAsSentState.Evaluated -> 8
        is EventDataArrivesAsSentState.Flattened -> 9
        is EventDataArrivesAsSentState.Garbled -> 7
        is EventDataArrivesAsSentState.Heard -> 1
        is EventDataArrivesAsSentState.Mangled -> 6
        is EventDataArrivesAsSentState.Opening -> 4
        is EventDataArrivesAsSentState.Quoted -> 2
        is EventDataArrivesAsSentState.Settled -> 5
        is EventDataArrivesAsSentState.Swallowed -> 10
        is EventDataArrivesAsSentState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventDataArrivesAsSentEvent? = when (name) {
        "arith" -> EventDataArrivesAsSentEvent.Arith
        "broken" -> EventDataArrivesAsSentEvent.Broken
        "doc" -> EventDataArrivesAsSentEvent.Doc
        "error.execution" -> EventDataArrivesAsSentEvent.Error.Execution
        "note" -> EventDataArrivesAsSentEvent.Note
        "payload" -> EventDataArrivesAsSentEvent.Payload
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventDataArrivesAsSentEvent): String? = when (event) {
        is EventDataArrivesAsSentEvent.Arith -> "arith"
        is EventDataArrivesAsSentEvent.Broken -> "broken"
        is EventDataArrivesAsSentEvent.Doc -> "doc"
        is EventDataArrivesAsSentEvent.Error.Execution -> "error.execution"
        is EventDataArrivesAsSentEvent.Note -> "note"
        is EventDataArrivesAsSentEvent.Payload -> "payload"
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
        state: EventDataArrivesAsSentState,
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EventDataArrivesAsSentState.Documented -> processDocumented(event)
        is EventDataArrivesAsSentState.Heard -> processHeard(event)
        is EventDataArrivesAsSentState.Opening -> processOpening(event)
        is EventDataArrivesAsSentState.Quoted -> processQuoted(event)
        is EventDataArrivesAsSentState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processDocumented(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Doc && safeEvaluateGuard("_event.data && _event.data.documentElement && _event.data.documentElement.nodeName === 'books'") -> TransitionResult.External(EventDataArrivesAsSentState.Opening, EventDataArrivesAsSentState.Documented)

        event is EventDataArrivesAsSentEvent.Doc -> TransitionResult.External(EventDataArrivesAsSentState.Flattened, EventDataArrivesAsSentState.Documented)

        else -> TransitionResult.Ignored
    }

    private fun processHeard(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Note && safeEvaluateGuard("_event.data === 'hold the line'") -> TransitionResult.External(EventDataArrivesAsSentState.Quoted, EventDataArrivesAsSentState.Heard)

        event is EventDataArrivesAsSentEvent.Note -> TransitionResult.External(EventDataArrivesAsSentState.Garbled, EventDataArrivesAsSentState.Heard)

        else -> TransitionResult.Ignored
    }

    private fun processOpening(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Broken && safeEvaluateGuard("_event.data === '<assign> to detail failed'") -> TransitionResult.External(EventDataArrivesAsSentState.Settled, EventDataArrivesAsSentState.Opening)

        event is EventDataArrivesAsSentEvent.Broken -> TransitionResult.External(EventDataArrivesAsSentState.Swallowed, EventDataArrivesAsSentState.Opening)

        else -> TransitionResult.Ignored
    }

    private fun processQuoted(
        event: EventDataArrivesAsSentEvent
    ): TransitionResult<EventDataArrivesAsSentState> = when {
        event is EventDataArrivesAsSentEvent.Arith && safeEvaluateGuard("_event.data === '2 + 3'") -> TransitionResult.External(EventDataArrivesAsSentState.Documented, EventDataArrivesAsSentState.Quoted)

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
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun onEntry(state: EventDataArrivesAsSentState, pathChild: EventDataArrivesAsSentState?) {
        when (state) {
            is EventDataArrivesAsSentState.Documented -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:100 :: documented :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("documented")) return
            }
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:122 :: evaluated :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("evaluated")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Flattened -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:123 :: flattened :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("flattened")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:121 :: garbled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("garbled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:83 :: heard :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("heard")) return
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:120 :: mangled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("mangled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Opening -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:114 :: opening :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("opening")) return
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:88 :: quoted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("quoted")) return
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:119 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Swallowed -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:124 :: swallowed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("swallowed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:76 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun onExit(state: EventDataArrivesAsSentState) {
        when (state) {
            is EventDataArrivesAsSentState.Documented -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:100 :: documented :: _state_body
                activeStateIds.remove("documented")
            }
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:122 :: evaluated :: _state_body
                activeStateIds.remove("evaluated")
            }
            is EventDataArrivesAsSentState.Flattened -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:123 :: flattened :: _state_body
                activeStateIds.remove("flattened")
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:121 :: garbled :: _state_body
                activeStateIds.remove("garbled")
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:83 :: heard :: _state_body
                activeStateIds.remove("heard")
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:120 :: mangled :: _state_body
                activeStateIds.remove("mangled")
            }
            is EventDataArrivesAsSentState.Opening -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:114 :: opening :: _state_body
                activeStateIds.remove("opening")
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:88 :: quoted :: _state_body
                activeStateIds.remove("quoted")
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:119 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is EventDataArrivesAsSentState.Swallowed -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:124 :: swallowed :: _state_body
                activeStateIds.remove("swallowed")
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:76 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun executeTransitionActions(
        source: EventDataArrivesAsSentState,
        event: EventDataArrivesAsSentEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
