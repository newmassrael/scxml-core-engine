// SCE-GENERATED — DO NOT EDIT
// source-hash: 5eba0d45073fc837c42ae33f20795f0ee658705613f4c4e42a59b557f47ce142
// template-hash: 42298195b20865d87e273e6a89fd9b7e20af26d02f54273007f21322d047b5d4
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/internal_chain_is_bounded/internal_chain_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine

package com.sce.integration.internal_chain_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InternalChainIsBoundedState : State {
    data object Alt : InternalChainIsBoundedState
    data object Bounded : InternalChainIsBoundedState
    data object Idle : InternalChainIsBoundedState
    data object Ignoring : InternalChainIsBoundedState
    data object Resuming : InternalChainIsBoundedState
    data object Spin : InternalChainIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InternalChainIsBoundedEvent : Event {
    data object Alternate : InternalChainIsBoundedEvent
    data object Beat : InternalChainIsBoundedEvent
    data object Beatless : InternalChainIsBoundedEvent
    data object Bounded : InternalChainIsBoundedEvent
    sealed interface Error : InternalChainIsBoundedEvent {
        data object Execution : Error
    }
    data object Link : InternalChainIsBoundedEvent
    data object Poke : InternalChainIsBoundedEvent
    data object Resume : InternalChainIsBoundedEvent
    data object Spin : InternalChainIsBoundedEvent
    data object Tick : InternalChainIsBoundedEvent
    data object Unanswered : InternalChainIsBoundedEvent
    data object Unheard : InternalChainIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class InternalChainIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InternalChainIsBoundedState, InternalChainIsBoundedEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `links` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `links` was assigned a value of another type, or the engine refused.
     */
    fun links(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "links")

    /**
     * §scxml-5.3: what the `beats` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `beats` was assigned a value of another type, or the engine refused.
     */
    fun beats(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "beats")

    /**
     * §scxml-5.3: what the `alts` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `alts` was assigned a value of another type, or the engine refused.
     */
    fun alts(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "alts")

    /**
     * §scxml-5.3: what the `ignores` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `ignores` was assigned a value of another type, or the engine refused.
     */
    fun ignores(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "ignores")

    /**
     * §scxml-5.3: what the `pending` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pending` was assigned a value of another type, or the engine refused.
     */
    fun pending(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "pending")

    override val initialState: InternalChainIsBoundedState = InternalChainIsBoundedState.Idle

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
    override fun resolveState(stateId: String): InternalChainIsBoundedState? = when (stateId) {
        "alt" -> InternalChainIsBoundedState.Alt
        "bounded" -> InternalChainIsBoundedState.Bounded
        "idle" -> InternalChainIsBoundedState.Idle
        "ignoring" -> InternalChainIsBoundedState.Ignoring
        "resuming" -> InternalChainIsBoundedState.Resuming
        "spin" -> InternalChainIsBoundedState.Spin
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InternalChainIsBoundedState): String = when (state) {
        is InternalChainIsBoundedState.Alt -> "alt"
        is InternalChainIsBoundedState.Bounded -> "bounded"
        is InternalChainIsBoundedState.Idle -> "idle"
        is InternalChainIsBoundedState.Ignoring -> "ignoring"
        is InternalChainIsBoundedState.Resuming -> "resuming"
        is InternalChainIsBoundedState.Spin -> "spin"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InternalChainIsBoundedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InternalChainIsBoundedState): Int = when (state) {
        is InternalChainIsBoundedState.Alt -> 4
        is InternalChainIsBoundedState.Bounded -> 1
        is InternalChainIsBoundedState.Idle -> 0
        is InternalChainIsBoundedState.Ignoring -> 5
        is InternalChainIsBoundedState.Resuming -> 3
        is InternalChainIsBoundedState.Spin -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InternalChainIsBoundedEvent? = when (name) {
        "alternate" -> InternalChainIsBoundedEvent.Alternate
        "beat" -> InternalChainIsBoundedEvent.Beat
        "beatless" -> InternalChainIsBoundedEvent.Beatless
        "bounded" -> InternalChainIsBoundedEvent.Bounded
        "error.execution" -> InternalChainIsBoundedEvent.Error.Execution
        "link" -> InternalChainIsBoundedEvent.Link
        "poke" -> InternalChainIsBoundedEvent.Poke
        "resume" -> InternalChainIsBoundedEvent.Resume
        "spin" -> InternalChainIsBoundedEvent.Spin
        "tick" -> InternalChainIsBoundedEvent.Tick
        "unanswered" -> InternalChainIsBoundedEvent.Unanswered
        "unheard" -> InternalChainIsBoundedEvent.Unheard
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InternalChainIsBoundedEvent): String? = when (event) {
        is InternalChainIsBoundedEvent.Alternate -> "alternate"
        is InternalChainIsBoundedEvent.Beat -> "beat"
        is InternalChainIsBoundedEvent.Beatless -> "beatless"
        is InternalChainIsBoundedEvent.Bounded -> "bounded"
        is InternalChainIsBoundedEvent.Error.Execution -> "error.execution"
        is InternalChainIsBoundedEvent.Link -> "link"
        is InternalChainIsBoundedEvent.Poke -> "poke"
        is InternalChainIsBoundedEvent.Resume -> "resume"
        is InternalChainIsBoundedEvent.Spin -> "spin"
        is InternalChainIsBoundedEvent.Tick -> "tick"
        is InternalChainIsBoundedEvent.Unanswered -> "unanswered"
        is InternalChainIsBoundedEvent.Unheard -> "unheard"
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
            "internal_chain_is_bounded",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'laps' with expr
        try {
            val initResult_laps = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "laps", initResult_laps)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='laps'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'links' with expr
        try {
            val initResult_links = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "links", initResult_links)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='links'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'beats' with expr
        try {
            val initResult_beats = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "beats", initResult_beats)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='beats'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'alts' with expr
        try {
            val initResult_alts = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "alts", initResult_alts)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='alts'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'ignores' with expr
        try {
            val initResult_ignores = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "ignores", initResult_ignores)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='ignores'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'pending' with expr
        try {
            val initResult_pending = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pending", initResult_pending)
        } catch (e: Exception) {
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<data id='pending'> expr failed to evaluate")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InternalChainIsBoundedEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InternalChainIsBoundedEvent) {
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
        state: InternalChainIsBoundedState,
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is InternalChainIsBoundedState.Alt -> processAlt(event)
        is InternalChainIsBoundedState.Bounded -> processBounded(event)
        is InternalChainIsBoundedState.Idle -> processIdle(event)
        is InternalChainIsBoundedState.Ignoring -> processIgnoring(event)
        is InternalChainIsBoundedState.Resuming -> processResuming(event)
        is InternalChainIsBoundedState.Spin -> processSpin(event)
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: InternalChainIsBoundedState
    ): TransitionResult<InternalChainIsBoundedState> = when (state) {
        is InternalChainIsBoundedState.Alt -> processNullAlt()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullAlt(
    ): TransitionResult<InternalChainIsBoundedState> = when {
        safeEvaluateGuard("pending == 1") -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    // --- Per-State Event Handlers ---

    private fun processAlt(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Tick -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processBounded(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        event is InternalChainIsBoundedEvent.Link && safeEvaluateGuard("laps < 999") -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Link -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processIdle(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.External(InternalChainIsBoundedState.Idle, InternalChainIsBoundedState.Idle)

        event is InternalChainIsBoundedEvent.Bounded -> TransitionResult.External(InternalChainIsBoundedState.Bounded, InternalChainIsBoundedState.Idle)

        event is InternalChainIsBoundedEvent.Spin -> TransitionResult.External(InternalChainIsBoundedState.Spin, InternalChainIsBoundedState.Idle)

        event is InternalChainIsBoundedEvent.Resume -> TransitionResult.External(InternalChainIsBoundedState.Resuming, InternalChainIsBoundedState.Idle)

        event is InternalChainIsBoundedEvent.Alternate -> TransitionResult.External(InternalChainIsBoundedState.Alt, InternalChainIsBoundedState.Idle)

        event is InternalChainIsBoundedEvent.Unanswered -> TransitionResult.External(InternalChainIsBoundedState.Ignoring, InternalChainIsBoundedState.Idle)

        else -> TransitionResult.Ignored
    }

    private fun processIgnoring(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        event is InternalChainIsBoundedEvent.Beatless && safeEvaluateGuard("ignores < 999") -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Beatless -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processResuming(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        event is InternalChainIsBoundedEvent.Beat && safeEvaluateGuard("beats < 1499") -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Beat -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processSpin(
        event: InternalChainIsBoundedEvent
    ): TransitionResult<InternalChainIsBoundedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Link -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is InternalChainIsBoundedEvent.Poke -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun onEntry(state: InternalChainIsBoundedState, pathChild: InternalChainIsBoundedState?) {
        when (state) {
            is InternalChainIsBoundedState.Alt -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:211 :: alt :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("alt")) return
            }
            is InternalChainIsBoundedState.Bounded -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:153 :: bounded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("bounded")) return
            }
            is InternalChainIsBoundedState.Idle -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:122 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
            is InternalChainIsBoundedState.Ignoring -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:244 :: ignoring :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ignoring")) return
            }
            is InternalChainIsBoundedState.Resuming -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:192 :: resuming :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("resuming")) return
            }
            is InternalChainIsBoundedState.Spin -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:174 :: spin :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("spin")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun onExit(state: InternalChainIsBoundedState) {
        when (state) {
            is InternalChainIsBoundedState.Alt -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:211 :: alt :: _state_body
                activeStateIds.remove("alt")
            }
            is InternalChainIsBoundedState.Bounded -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:153 :: bounded :: _state_body
                activeStateIds.remove("bounded")
            }
            is InternalChainIsBoundedState.Idle -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:122 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
            is InternalChainIsBoundedState.Ignoring -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:244 :: ignoring :: _state_body
                activeStateIds.remove("ignoring")
            }
            is InternalChainIsBoundedState.Resuming -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:192 :: resuming :: _state_body
                activeStateIds.remove("resuming")
            }
            is InternalChainIsBoundedState.Spin -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:174 :: spin :: _state_body
                activeStateIds.remove("spin")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: internal_chain_is_bounded.scxml:90 :: _machine
    override fun executeTransitionActions(
        source: InternalChainIsBoundedState,
        event: InternalChainIsBoundedEvent?
    ) {
        when (source) {
        is InternalChainIsBoundedState.Alt -> when {
            event is InternalChainIsBoundedEvent.Tick -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:212 :: alt :: _transition_0


            executeAssign("alts", "alts + 1")


            executeAssign("pending", "1")
            }
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:220 :: alt :: _transition_2


            executeAssign("pokes", "pokes + 1")
            }
            event == null && safeEvaluateGuard("pending == 1") -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:216 :: alt :: _transition_1


            executeAssign("pending", "0")

            raiseInternal(InternalChainIsBoundedEvent.Tick)
            }
            else -> {}
        }
        is InternalChainIsBoundedState.Bounded -> when {
            event is InternalChainIsBoundedEvent.Link && safeEvaluateGuard("laps < 999") -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:154 :: bounded :: _transition_0


            executeAssign("laps", "laps + 1")

            raiseInternal(InternalChainIsBoundedEvent.Link)
            }
            event is InternalChainIsBoundedEvent.Link -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:158 :: bounded :: _transition_1


            executeAssign("laps", "laps + 1")
            }
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:163 :: bounded :: _transition_2


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        is InternalChainIsBoundedState.Idle -> when {
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:123 :: idle :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            event is InternalChainIsBoundedEvent.Bounded -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:126 :: idle :: _transition_1

            raiseInternal(InternalChainIsBoundedEvent.Link)
            }
            event is InternalChainIsBoundedEvent.Spin -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:129 :: idle :: _transition_2

            raiseInternal(InternalChainIsBoundedEvent.Link)
            }
            event is InternalChainIsBoundedEvent.Resume -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:132 :: idle :: _transition_3

            raiseInternal(InternalChainIsBoundedEvent.Beat)
            }
            event is InternalChainIsBoundedEvent.Alternate -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:135 :: idle :: _transition_4

            raiseInternal(InternalChainIsBoundedEvent.Tick)
            }
            event is InternalChainIsBoundedEvent.Unanswered -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:138 :: idle :: _transition_5

            raiseInternal(InternalChainIsBoundedEvent.Beatless)
            }
            else -> {}
        }
        is InternalChainIsBoundedState.Ignoring -> when {
            event is InternalChainIsBoundedEvent.Beatless && safeEvaluateGuard("ignores < 999") -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:245 :: ignoring :: _transition_0


            executeAssign("ignores", "ignores + 1")

            raiseInternal(InternalChainIsBoundedEvent.Unheard)

            raiseInternal(InternalChainIsBoundedEvent.Beatless)
            }
            event is InternalChainIsBoundedEvent.Beatless -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:250 :: ignoring :: _transition_1


            executeAssign("ignores", "ignores + 1")
            }
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:253 :: ignoring :: _transition_2


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        is InternalChainIsBoundedState.Resuming -> when {
            event is InternalChainIsBoundedEvent.Beat && safeEvaluateGuard("beats < 1499") -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:193 :: resuming :: _transition_0


            executeAssign("beats", "beats + 1")

            raiseInternal(InternalChainIsBoundedEvent.Beat)
            }
            event is InternalChainIsBoundedEvent.Beat -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:197 :: resuming :: _transition_1


            executeAssign("beats", "beats + 1")
            }
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:200 :: resuming :: _transition_2


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        is InternalChainIsBoundedState.Spin -> when {
            event is InternalChainIsBoundedEvent.Link -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:175 :: spin :: _transition_0


            executeAssign("links", "links + 1")

            raiseInternal(InternalChainIsBoundedEvent.Link)
            }
            event is InternalChainIsBoundedEvent.Poke -> {
                // SCE-MAP: internal_chain_is_bounded.scxml:179 :: spin :: _transition_1


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        }
    }
}
