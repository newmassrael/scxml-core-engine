// SCE-GENERATED — DO NOT EDIT
// source-hash: 8e0f0b7b552dfbb89b9083db177a216e77a3534d3f6112690f84145daf0386d4

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: EventDataArrivesAsSentState): Boolean = when (state) {
        is EventDataArrivesAsSentState.Evaluated, is EventDataArrivesAsSentState.Flattened, is EventDataArrivesAsSentState.Garbled, is EventDataArrivesAsSentState.Mangled, is EventDataArrivesAsSentState.Settled, is EventDataArrivesAsSentState.Swallowed -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EventDataArrivesAsSentState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<EventDataArrivesAsSentState, HistoryId>> =
            listOf(StateTarget(EventDataArrivesAsSentState.Waiting))

        // W3C SCXML 3.13: documented's transition 0, as the microstep reads it.
        val transitionDocumentedAt0 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Documented,
            listOf(StateTarget(EventDataArrivesAsSentState.Opening)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: documented's transition 1, as the microstep reads it.
        val transitionDocumentedAt1 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Documented,
            listOf(StateTarget(EventDataArrivesAsSentState.Flattened)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: heard's transition 0, as the microstep reads it.
        val transitionHeardAt0 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Heard,
            listOf(StateTarget(EventDataArrivesAsSentState.Quoted)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: heard's transition 1, as the microstep reads it.
        val transitionHeardAt1 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Heard,
            listOf(StateTarget(EventDataArrivesAsSentState.Garbled)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: opening's transition 0, as the microstep reads it.
        val transitionOpeningAt0 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Opening,
            listOf(StateTarget(EventDataArrivesAsSentState.Settled)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: opening's transition 1, as the microstep reads it.
        val transitionOpeningAt1 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Opening,
            listOf(StateTarget(EventDataArrivesAsSentState.Swallowed)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: quoted's transition 0, as the microstep reads it.
        val transitionQuotedAt0 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Quoted,
            listOf(StateTarget(EventDataArrivesAsSentState.Documented)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: quoted's transition 1, as the microstep reads it.
        val transitionQuotedAt1 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Quoted,
            listOf(StateTarget(EventDataArrivesAsSentState.Evaluated)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Waiting,
            listOf(StateTarget(EventDataArrivesAsSentState.Heard)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<EventDataArrivesAsSentState, HistoryId>(
            EventDataArrivesAsSentState.Waiting,
            listOf(StateTarget(EventDataArrivesAsSentState.Mangled)),
            1,
            hasActions = false,
            isInternal = false,
        )
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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
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
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(EventDataArrivesAsSentEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
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
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
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



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: EventDataArrivesAsSentEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: EventDataArrivesAsSentState,
        event: EventDataArrivesAsSentEvent?
    ): EnabledTransition<EventDataArrivesAsSentState, HistoryId>? = when (state) {
        is EventDataArrivesAsSentState.Documented -> when {
            event is EventDataArrivesAsSentEvent.Doc && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("((_scxml_truthy(_event.data) and _scxml_truthy(_event.data.documentElement)) and (_event.data.documentElement.nodeName == \"books\"))", "_event.data && _event.data.documentElement && _event.data.documentElement.nodeName === 'books'")) -> transitionDocumentedAt0
            event is EventDataArrivesAsSentEvent.Doc -> transitionDocumentedAt1
            else -> null
        }
        is EventDataArrivesAsSentState.Heard -> when {
            event is EventDataArrivesAsSentEvent.Note && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"hold the line\")", "_event.data === 'hold the line'")) -> transitionHeardAt0
            event is EventDataArrivesAsSentEvent.Note -> transitionHeardAt1
            else -> null
        }
        is EventDataArrivesAsSentState.Opening -> when {
            event is EventDataArrivesAsSentEvent.Broken && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"<assign> to detail failed\")", "_event.data === '<assign> to detail failed'")) -> transitionOpeningAt0
            event is EventDataArrivesAsSentEvent.Broken -> transitionOpeningAt1
            else -> null
        }
        is EventDataArrivesAsSentState.Quoted -> when {
            event is EventDataArrivesAsSentEvent.Arith && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"2 + 3\")", "_event.data === '2 + 3'")) -> transitionQuotedAt0
            event is EventDataArrivesAsSentEvent.Arith -> transitionQuotedAt1
            else -> null
        }
        is EventDataArrivesAsSentState.Waiting -> when {
            event is EventDataArrivesAsSentEvent.Payload && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("((_scxml_truthy(_event.data) and (_event.data.milestone == \"refined\")) and (_event.data.turns == 2))", "_event.data && _event.data.milestone === 'refined' && _event.data.turns === 2")) -> transitionWaitingAt0
            event is EventDataArrivesAsSentEvent.Payload -> transitionWaitingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun onEntry(state: EventDataArrivesAsSentState, isDefaultEntry: Boolean) {
        when (state) {
            is EventDataArrivesAsSentState.Documented -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:100 :: documented :: _state_body
            }
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:122 :: evaluated :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Flattened -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:123 :: flattened :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:121 :: garbled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:83 :: heard :: _state_body
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:120 :: mangled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Opening -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:114 :: opening :: _state_body
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:88 :: quoted :: _state_body
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:119 :: settled :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Swallowed -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:124 :: swallowed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:76 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun onExit(state: EventDataArrivesAsSentState) {
        when (state) {
            is EventDataArrivesAsSentState.Documented -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:100 :: documented :: _state_body
            }
            is EventDataArrivesAsSentState.Evaluated -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:122 :: evaluated :: _state_body
            }
            is EventDataArrivesAsSentState.Flattened -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:123 :: flattened :: _state_body
            }
            is EventDataArrivesAsSentState.Garbled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:121 :: garbled :: _state_body
            }
            is EventDataArrivesAsSentState.Heard -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:83 :: heard :: _state_body
            }
            is EventDataArrivesAsSentState.Mangled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:120 :: mangled :: _state_body
            }
            is EventDataArrivesAsSentState.Opening -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:114 :: opening :: _state_body
            }
            is EventDataArrivesAsSentState.Quoted -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:88 :: quoted :: _state_body
            }
            is EventDataArrivesAsSentState.Settled -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:119 :: settled :: _state_body
            }
            is EventDataArrivesAsSentState.Swallowed -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:124 :: swallowed :: _state_body
            }
            is EventDataArrivesAsSentState.Waiting -> {
                // SCE-MAP: event_data_arrives_as_sent.scxml:76 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: event_data_arrives_as_sent.scxml:73 :: _machine
    override fun executeTransitionContent(source: EventDataArrivesAsSentState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
