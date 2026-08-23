// SCE-GENERATED — DO NOT EDIT
// source-hash: 23347f5c092342ad5655a09f8c78eecec8de3c0705a0affd88f1ecbcd658f869
// template-hash: 1f4fc251a4bb4df71320b116cc055aa1687156c3a3402c346abf1bd3694d0437
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine

package com.sce.integration.targetless_transition_completes_macrostep

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface TargetlessTransitionCompletesMacrostepState : State {
    data object Idle : TargetlessTransitionCompletesMacrostepState
    data object Recycled : TargetlessTransitionCompletesMacrostepState
    data object Settled : TargetlessTransitionCompletesMacrostepState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface TargetlessTransitionCompletesMacrostepEvent : Event {
    data object Arm : TargetlessTransitionCompletesMacrostepEvent
    sealed interface Error : TargetlessTransitionCompletesMacrostepEvent {
        data object Execution : Error
    }
    data object Ping : TargetlessTransitionCompletesMacrostepEvent
    data object Pong : TargetlessTransitionCompletesMacrostepEvent
    data object Quiet : TargetlessTransitionCompletesMacrostepEvent
    data object Recycle : TargetlessTransitionCompletesMacrostepEvent
}
// --- State Machine (W3C SCXML) ---

class TargetlessTransitionCompletesMacrostepStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<TargetlessTransitionCompletesMacrostepState, TargetlessTransitionCompletesMacrostepEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `quiet` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `quiet` was assigned a value of another type, or the engine refused.
     */
    fun quiet(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "quiet")

    /**
     * §scxml-5.3: what the `armed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `armed` was assigned a value of another type, or the engine refused.
     */
    fun armed(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "armed")

    /**
     * §scxml-5.3: what the `chained` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `chained` was assigned a value of another type, or the engine refused.
     */
    fun chained(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "chained")

    /**
     * §scxml-5.3: what the `polished` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `polished` was assigned a value of another type, or the engine refused.
     */
    fun polished(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "polished")

    /**
     * §scxml-5.3: what the `answered` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `answered` was assigned a value of another type, or the engine refused.
     */
    fun answered(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "answered")

    /**
     * §scxml-5.3: what the `entries` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entries` was assigned a value of another type, or the engine refused.
     */
    fun entries(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "entries")

    override val initialState: TargetlessTransitionCompletesMacrostepState = TargetlessTransitionCompletesMacrostepState.Idle

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
    override fun resolveState(stateId: String): TargetlessTransitionCompletesMacrostepState? = when (stateId) {
        "idle" -> TargetlessTransitionCompletesMacrostepState.Idle
        "recycled" -> TargetlessTransitionCompletesMacrostepState.Recycled
        "settled" -> TargetlessTransitionCompletesMacrostepState.Settled
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: TargetlessTransitionCompletesMacrostepState): String = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> "idle"
        is TargetlessTransitionCompletesMacrostepState.Recycled -> "recycled"
        is TargetlessTransitionCompletesMacrostepState.Settled -> "settled"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: TargetlessTransitionCompletesMacrostepState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: TargetlessTransitionCompletesMacrostepState): Int = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> 0
        is TargetlessTransitionCompletesMacrostepState.Recycled -> 2
        is TargetlessTransitionCompletesMacrostepState.Settled -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): TargetlessTransitionCompletesMacrostepEvent? = when (name) {
        "arm" -> TargetlessTransitionCompletesMacrostepEvent.Arm
        "error.execution" -> TargetlessTransitionCompletesMacrostepEvent.Error.Execution
        "ping" -> TargetlessTransitionCompletesMacrostepEvent.Ping
        "pong" -> TargetlessTransitionCompletesMacrostepEvent.Pong
        "quiet" -> TargetlessTransitionCompletesMacrostepEvent.Quiet
        "recycle" -> TargetlessTransitionCompletesMacrostepEvent.Recycle
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TargetlessTransitionCompletesMacrostepEvent): String? = when (event) {
        is TargetlessTransitionCompletesMacrostepEvent.Arm -> "arm"
        is TargetlessTransitionCompletesMacrostepEvent.Error.Execution -> "error.execution"
        is TargetlessTransitionCompletesMacrostepEvent.Ping -> "ping"
        is TargetlessTransitionCompletesMacrostepEvent.Pong -> "pong"
        is TargetlessTransitionCompletesMacrostepEvent.Quiet -> "quiet"
        is TargetlessTransitionCompletesMacrostepEvent.Recycle -> "recycle"
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
            "targetless_transition_completes_macrostep",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'quiet' with expr
        try {
            val initResult_quiet = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "quiet", initResult_quiet)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='quiet'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'armed' with expr
        try {
            val initResult_armed = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "armed", initResult_armed)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='armed'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'chained' with expr
        try {
            val initResult_chained = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "chained", initResult_chained)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='chained'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'polished' with expr
        try {
            val initResult_polished = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "polished", initResult_polished)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='polished'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'answered' with expr
        try {
            val initResult_answered = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "answered", initResult_answered)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='answered'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'entries' with expr
        try {
            val initResult_entries = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "entries", initResult_entries)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='entries'> expr failed to evaluate")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: TargetlessTransitionCompletesMacrostepEvent) {
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
        state: TargetlessTransitionCompletesMacrostepState,
        event: TargetlessTransitionCompletesMacrostepEvent
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> processIdle(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: TargetlessTransitionCompletesMacrostepState
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> processNullIdle()
        is TargetlessTransitionCompletesMacrostepState.Recycled -> processNullRecycled()
        is TargetlessTransitionCompletesMacrostepState.Settled -> processNullSettled()
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullIdle(
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> = when {
        safeEvaluateGuard("armed == 1") -> TransitionResult.External(TargetlessTransitionCompletesMacrostepState.Settled, TargetlessTransitionCompletesMacrostepState.Idle)
        else -> TransitionResult.Ignored
    }

    private fun processNullRecycled(
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> = when {
        safeEvaluateGuard("entries < 2") -> TransitionResult.External(TargetlessTransitionCompletesMacrostepState.Recycled, TargetlessTransitionCompletesMacrostepState.Recycled)
        else -> TransitionResult.Ignored
    }

    private fun processNullSettled(
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> = when {
        safeEvaluateGuard("polished == 0") -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    // --- Per-State Event Handlers ---

    private fun processIdle(
        event: TargetlessTransitionCompletesMacrostepEvent
    ): TransitionResult<TargetlessTransitionCompletesMacrostepState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is TargetlessTransitionCompletesMacrostepEvent.Quiet -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is TargetlessTransitionCompletesMacrostepEvent.Arm -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is TargetlessTransitionCompletesMacrostepEvent.Ping -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is TargetlessTransitionCompletesMacrostepEvent.Pong -> TransitionResult.Internal
        event is TargetlessTransitionCompletesMacrostepEvent.Recycle -> TransitionResult.External(TargetlessTransitionCompletesMacrostepState.Recycled, TargetlessTransitionCompletesMacrostepState.Idle)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun onEntry(state: TargetlessTransitionCompletesMacrostepState, pathChild: TargetlessTransitionCompletesMacrostepState?) {
        when (state) {
            is TargetlessTransitionCompletesMacrostepState.Idle -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:81 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
            is TargetlessTransitionCompletesMacrostepState.Recycled -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:147 :: recycled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("recycled")) return


            executeAssign("entries", "entries + 1")
            }
            is TargetlessTransitionCompletesMacrostepState.Settled -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:125 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun onExit(state: TargetlessTransitionCompletesMacrostepState) {
        when (state) {
            is TargetlessTransitionCompletesMacrostepState.Idle -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:81 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
            is TargetlessTransitionCompletesMacrostepState.Recycled -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:147 :: recycled :: _state_body
                activeStateIds.remove("recycled")
            }
            is TargetlessTransitionCompletesMacrostepState.Settled -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:125 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun executeTransitionActions(
        source: TargetlessTransitionCompletesMacrostepState,
        event: TargetlessTransitionCompletesMacrostepEvent?
    ) {
        when (source) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> when {
            event is TargetlessTransitionCompletesMacrostepEvent.Quiet -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:93 :: idle :: _transition_1


            executeAssign("quiet", "quiet + 1")
            }
            event is TargetlessTransitionCompletesMacrostepEvent.Arm -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:100 :: idle :: _transition_2


            executeAssign("armed", "1")
            }
            event is TargetlessTransitionCompletesMacrostepEvent.Ping -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:107 :: idle :: _transition_3

            raiseInternal(TargetlessTransitionCompletesMacrostepEvent.Pong)
            }
            event is TargetlessTransitionCompletesMacrostepEvent.Pong -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:111 :: idle :: _transition_4


            executeAssign("answered", "answered + 1")
            }
            event == null && safeEvaluateGuard("armed == 1") -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:86 :: idle :: _transition_0


            executeAssign("chained", "chained + 1")
            }
            else -> {}
        }
        is TargetlessTransitionCompletesMacrostepState.Settled -> when {
            event == null && safeEvaluateGuard("polished == 0") -> {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:126 :: settled :: _transition_0


            executeAssign("polished", "polished + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
