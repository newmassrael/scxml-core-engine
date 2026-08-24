// SCE-GENERATED — DO NOT EDIT
// source-hash: 448efc1945f51a00d346a070a50c9e40a8fdb0d3297033414fa43984fe293f6e
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/eventless_macrostep_is_bounded/eventless_macrostep_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine

package com.sce.integration.eventless_macrostep_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventlessMacrostepIsBoundedState : State {
    data object BoundedA : EventlessMacrostepIsBoundedState
    data object BoundedB : EventlessMacrostepIsBoundedState
    data object Idle : EventlessMacrostepIsBoundedState
    data object SpinA : EventlessMacrostepIsBoundedState
    data object SpinB : EventlessMacrostepIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventlessMacrostepIsBoundedEvent : Event {
    data object Bounded : EventlessMacrostepIsBoundedEvent
    sealed interface Error : EventlessMacrostepIsBoundedEvent {
        data object Execution : Error
    }
    data object Poke : EventlessMacrostepIsBoundedEvent
    data object Reset : EventlessMacrostepIsBoundedEvent
    data object Spin : EventlessMacrostepIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class EventlessMacrostepIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventlessMacrostepIsBoundedState, EventlessMacrostepIsBoundedEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pokes` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pokes` was assigned a value of another type, or the engine refused.
     */
    fun pokes(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pokes")

    /**
     * §scxml-5.3: what the `laps` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `laps` was assigned a value of another type, or the engine refused.
     */
    fun laps(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "laps")

    /**
     * §scxml-5.3: what the `spins` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `spins` was assigned a value of another type, or the engine refused.
     */
    fun spins(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "spins")

    override val initialState: EventlessMacrostepIsBoundedState = EventlessMacrostepIsBoundedState.Idle

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
    override fun resolveState(stateId: String): EventlessMacrostepIsBoundedState? = when (stateId) {
        "bounded_a" -> EventlessMacrostepIsBoundedState.BoundedA
        "bounded_b" -> EventlessMacrostepIsBoundedState.BoundedB
        "idle" -> EventlessMacrostepIsBoundedState.Idle
        "spin_a" -> EventlessMacrostepIsBoundedState.SpinA
        "spin_b" -> EventlessMacrostepIsBoundedState.SpinB
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventlessMacrostepIsBoundedState): String = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> "bounded_a"
        is EventlessMacrostepIsBoundedState.BoundedB -> "bounded_b"
        is EventlessMacrostepIsBoundedState.Idle -> "idle"
        is EventlessMacrostepIsBoundedState.SpinA -> "spin_a"
        is EventlessMacrostepIsBoundedState.SpinB -> "spin_b"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventlessMacrostepIsBoundedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventlessMacrostepIsBoundedState): Int = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> 1
        is EventlessMacrostepIsBoundedState.BoundedB -> 2
        is EventlessMacrostepIsBoundedState.Idle -> 0
        is EventlessMacrostepIsBoundedState.SpinA -> 3
        is EventlessMacrostepIsBoundedState.SpinB -> 4
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventlessMacrostepIsBoundedEvent? = when (name) {
        "bounded" -> EventlessMacrostepIsBoundedEvent.Bounded
        "error.execution" -> EventlessMacrostepIsBoundedEvent.Error.Execution
        "poke" -> EventlessMacrostepIsBoundedEvent.Poke
        "reset" -> EventlessMacrostepIsBoundedEvent.Reset
        "spin" -> EventlessMacrostepIsBoundedEvent.Spin
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventlessMacrostepIsBoundedEvent): String? = when (event) {
        is EventlessMacrostepIsBoundedEvent.Bounded -> "bounded"
        is EventlessMacrostepIsBoundedEvent.Error.Execution -> "error.execution"
        is EventlessMacrostepIsBoundedEvent.Poke -> "poke"
        is EventlessMacrostepIsBoundedEvent.Reset -> "reset"
        is EventlessMacrostepIsBoundedEvent.Spin -> "spin"
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
            "eventless_macrostep_is_bounded",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'laps' with expr
        try {
            val initResult_laps = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "laps", initResult_laps)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='laps'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'spins' with expr
        try {
            val initResult_spins = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "spins", initResult_spins)
        } catch (e: Exception) {
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<data id='spins'> expr failed to evaluate")
        }




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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventlessMacrostepIsBoundedEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventlessMacrostepIsBoundedEvent) {
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
        state: EventlessMacrostepIsBoundedState,
        event: EventlessMacrostepIsBoundedEvent
    ): TransitionResult<EventlessMacrostepIsBoundedState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> processBoundedA(event)
        is EventlessMacrostepIsBoundedState.Idle -> processIdle(event)
        is EventlessMacrostepIsBoundedState.SpinA -> processSpinA(event)
        is EventlessMacrostepIsBoundedState.SpinB -> processSpinB(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: EventlessMacrostepIsBoundedState
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when (state) {
        is EventlessMacrostepIsBoundedState.BoundedA -> processNullBoundedA()
        is EventlessMacrostepIsBoundedState.BoundedB -> processNullBoundedB()
        is EventlessMacrostepIsBoundedState.SpinA -> processNullSpinA()
        is EventlessMacrostepIsBoundedState.SpinB -> processNullSpinB()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullBoundedA(
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        safeEvaluateGuard("laps < 500") -> TransitionResult.External(EventlessMacrostepIsBoundedState.BoundedB, EventlessMacrostepIsBoundedState.BoundedA)
        else -> TransitionResult.Ignored
    }

    private fun processNullBoundedB(
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(EventlessMacrostepIsBoundedState.BoundedA, EventlessMacrostepIsBoundedState.BoundedB)
    }

    private fun processNullSpinA(
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(EventlessMacrostepIsBoundedState.SpinB, EventlessMacrostepIsBoundedState.SpinA)
    }

    private fun processNullSpinB(
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(EventlessMacrostepIsBoundedState.SpinA, EventlessMacrostepIsBoundedState.SpinB)
    }

    // --- Per-State Event Handlers ---

    private fun processBoundedA(
        event: EventlessMacrostepIsBoundedEvent
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processIdle(
        event: EventlessMacrostepIsBoundedEvent
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionResult.External(EventlessMacrostepIsBoundedState.Idle, EventlessMacrostepIsBoundedState.Idle)

        event is EventlessMacrostepIsBoundedEvent.Bounded -> TransitionResult.External(EventlessMacrostepIsBoundedState.BoundedA, EventlessMacrostepIsBoundedState.Idle)

        event is EventlessMacrostepIsBoundedEvent.Spin -> TransitionResult.External(EventlessMacrostepIsBoundedState.SpinA, EventlessMacrostepIsBoundedState.Idle)

        else -> TransitionResult.Ignored
    }

    private fun processSpinA(
        event: EventlessMacrostepIsBoundedEvent
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is EventlessMacrostepIsBoundedEvent.Poke -> TransitionResult.Internal
        event is EventlessMacrostepIsBoundedEvent.Reset -> TransitionResult.External(EventlessMacrostepIsBoundedState.Idle, EventlessMacrostepIsBoundedState.SpinA)

        else -> TransitionResult.Ignored
    }

    private fun processSpinB(
        event: EventlessMacrostepIsBoundedEvent
    ): TransitionResult<EventlessMacrostepIsBoundedState> = when {
        event is EventlessMacrostepIsBoundedEvent.Reset -> TransitionResult.External(EventlessMacrostepIsBoundedState.Idle, EventlessMacrostepIsBoundedState.SpinB)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun onEntry(state: EventlessMacrostepIsBoundedState, pathChild: EventlessMacrostepIsBoundedState?) {
        when (state) {
            is EventlessMacrostepIsBoundedState.BoundedA -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:84 :: bounded_a :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("bounded_a")) return
            }
            is EventlessMacrostepIsBoundedState.BoundedB -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:95 :: bounded_b :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("bounded_b")) return
            }
            is EventlessMacrostepIsBoundedState.Idle -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:71 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
            is EventlessMacrostepIsBoundedState.SpinA -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:104 :: spin_a :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("spin_a")) return
            }
            is EventlessMacrostepIsBoundedState.SpinB -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:125 :: spin_b :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("spin_b")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun onExit(state: EventlessMacrostepIsBoundedState) {
        when (state) {
            is EventlessMacrostepIsBoundedState.BoundedA -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:84 :: bounded_a :: _state_body
                activeStateIds.remove("bounded_a")
            }
            is EventlessMacrostepIsBoundedState.BoundedB -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:95 :: bounded_b :: _state_body
                activeStateIds.remove("bounded_b")
            }
            is EventlessMacrostepIsBoundedState.Idle -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:71 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
            is EventlessMacrostepIsBoundedState.SpinA -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:104 :: spin_a :: _state_body
                activeStateIds.remove("spin_a")
            }
            is EventlessMacrostepIsBoundedState.SpinB -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:125 :: spin_b :: _state_body
                activeStateIds.remove("spin_b")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: eventless_macrostep_is_bounded.scxml:53 :: _machine
    override fun executeTransitionActions(
        source: EventlessMacrostepIsBoundedState,
        event: EventlessMacrostepIsBoundedEvent?
    ) {
        when (source) {
        is EventlessMacrostepIsBoundedState.BoundedA -> when {
            event is EventlessMacrostepIsBoundedEvent.Poke -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:90 :: bounded_a :: _transition_1


            executeAssign("pokes", "pokes + 1")
            }
            event == null && safeEvaluateGuard("laps < 500") -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:85 :: bounded_a :: _transition_0


            executeAssign("laps", "laps + 1")
            }
            else -> {}
        }
        is EventlessMacrostepIsBoundedState.Idle -> when {
            event is EventlessMacrostepIsBoundedEvent.Poke -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:72 :: idle :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        is EventlessMacrostepIsBoundedState.SpinA -> when {
            event is EventlessMacrostepIsBoundedEvent.Poke -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:108 :: spin_a :: _transition_1


            executeAssign("pokes", "pokes + 1")
            }
            event == null -> {
                // SCE-MAP: eventless_macrostep_is_bounded.scxml:105 :: spin_a :: _transition_0


            executeAssign("spins", "spins + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
