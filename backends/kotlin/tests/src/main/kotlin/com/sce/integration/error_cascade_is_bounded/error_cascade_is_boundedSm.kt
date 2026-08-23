// SCE-GENERATED — DO NOT EDIT
// source-hash: 4731a6ba40787ab928e39e6fce63f290cd233b0d7081f439713483c0324e40fe
// template-hash: c44eb8ea1f7a6700f381c20ea1b37f015805c8beff30d4d12e22d7c96e5e1124
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/error_cascade_is_bounded/error_cascade_is_bounded.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: error_cascade_is_bounded.scxml:45 :: _machine

package com.sce.integration.error_cascade_is_bounded

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ErrorCascadeIsBoundedState : State {
    data object Idle : ErrorCascadeIsBoundedState
    data object Runaway : ErrorCascadeIsBoundedState
    data object Settling : ErrorCascadeIsBoundedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ErrorCascadeIsBoundedEvent : Event {
    data object Boom : ErrorCascadeIsBoundedEvent
    sealed interface Error : ErrorCascadeIsBoundedEvent {
        data object Execution : Error
    }
    data object Poke : ErrorCascadeIsBoundedEvent
    data object Reset : ErrorCascadeIsBoundedEvent
    data object Settle : ErrorCascadeIsBoundedEvent
    data object Spin : ErrorCascadeIsBoundedEvent
    data object Tick : ErrorCascadeIsBoundedEvent
}
// --- State Machine (W3C SCXML) ---

class ErrorCascadeIsBoundedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ErrorCascadeIsBoundedState, ErrorCascadeIsBoundedEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `repairs` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `repairs` was assigned a value of another type, or the engine refused.
     */
    fun repairs(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "repairs")

    /**
     * §scxml-5.3: what the `runs` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `runs` was assigned a value of another type, or the engine refused.
     */
    fun runs(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "runs")

    /**
     * §scxml-5.3: what the `ticks` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `ticks` was assigned a value of another type, or the engine refused.
     */
    fun ticks(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "ticks")

    override val initialState: ErrorCascadeIsBoundedState = ErrorCascadeIsBoundedState.Idle

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
    override fun resolveState(stateId: String): ErrorCascadeIsBoundedState? = when (stateId) {
        "idle" -> ErrorCascadeIsBoundedState.Idle
        "runaway" -> ErrorCascadeIsBoundedState.Runaway
        "settling" -> ErrorCascadeIsBoundedState.Settling
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ErrorCascadeIsBoundedState): String = when (state) {
        is ErrorCascadeIsBoundedState.Idle -> "idle"
        is ErrorCascadeIsBoundedState.Runaway -> "runaway"
        is ErrorCascadeIsBoundedState.Settling -> "settling"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ErrorCascadeIsBoundedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: ErrorCascadeIsBoundedState): Int = when (state) {
        is ErrorCascadeIsBoundedState.Idle -> 0
        is ErrorCascadeIsBoundedState.Runaway -> 2
        is ErrorCascadeIsBoundedState.Settling -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ErrorCascadeIsBoundedEvent? = when (name) {
        "boom" -> ErrorCascadeIsBoundedEvent.Boom
        "error.execution" -> ErrorCascadeIsBoundedEvent.Error.Execution
        "poke" -> ErrorCascadeIsBoundedEvent.Poke
        "reset" -> ErrorCascadeIsBoundedEvent.Reset
        "settle" -> ErrorCascadeIsBoundedEvent.Settle
        "spin" -> ErrorCascadeIsBoundedEvent.Spin
        "tick" -> ErrorCascadeIsBoundedEvent.Tick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ErrorCascadeIsBoundedEvent): String? = when (event) {
        is ErrorCascadeIsBoundedEvent.Boom -> "boom"
        is ErrorCascadeIsBoundedEvent.Error.Execution -> "error.execution"
        is ErrorCascadeIsBoundedEvent.Poke -> "poke"
        is ErrorCascadeIsBoundedEvent.Reset -> "reset"
        is ErrorCascadeIsBoundedEvent.Settle -> "settle"
        is ErrorCascadeIsBoundedEvent.Spin -> "spin"
        is ErrorCascadeIsBoundedEvent.Tick -> "tick"
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
            "error_cascade_is_bounded",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'repairs' with expr
        try {
            val initResult_repairs = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "repairs", initResult_repairs)
        } catch (e: Exception) {
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<data id='repairs'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'runs' with expr
        try {
            val initResult_runs = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "runs", initResult_runs)
        } catch (e: Exception) {
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<data id='runs'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'ticks' with expr
        try {
            val initResult_ticks = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "ticks", initResult_ticks)
        } catch (e: Exception) {
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<data id='ticks'> expr failed to evaluate")
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
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ErrorCascadeIsBoundedEvent) {
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
        state: ErrorCascadeIsBoundedState,
        event: ErrorCascadeIsBoundedEvent
    ): TransitionResult<ErrorCascadeIsBoundedState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is ErrorCascadeIsBoundedState.Idle -> processIdle(event)
        is ErrorCascadeIsBoundedState.Runaway -> processRunaway(event)
        is ErrorCascadeIsBoundedState.Settling -> processSettling(event)
    }
    }


    // --- Per-State Event Handlers ---

    private fun processIdle(
        event: ErrorCascadeIsBoundedEvent
    ): TransitionResult<ErrorCascadeIsBoundedState> = when {
        event is ErrorCascadeIsBoundedEvent.Poke -> TransitionResult.External(ErrorCascadeIsBoundedState.Idle, ErrorCascadeIsBoundedState.Idle)

        event is ErrorCascadeIsBoundedEvent.Boom -> TransitionResult.External(ErrorCascadeIsBoundedState.Idle, ErrorCascadeIsBoundedState.Idle)

        event is ErrorCascadeIsBoundedEvent.Settle -> TransitionResult.External(ErrorCascadeIsBoundedState.Settling, ErrorCascadeIsBoundedState.Idle)

        event is ErrorCascadeIsBoundedEvent.Spin -> TransitionResult.External(ErrorCascadeIsBoundedState.Runaway, ErrorCascadeIsBoundedState.Idle)

        else -> TransitionResult.Ignored
    }

    private fun processRunaway(
        event: ErrorCascadeIsBoundedEvent
    ): TransitionResult<ErrorCascadeIsBoundedState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is ErrorCascadeIsBoundedEvent.Error.Execution -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is ErrorCascadeIsBoundedEvent.Tick -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is ErrorCascadeIsBoundedEvent.Poke -> TransitionResult.Internal
        event is ErrorCascadeIsBoundedEvent.Reset -> TransitionResult.External(ErrorCascadeIsBoundedState.Idle, ErrorCascadeIsBoundedState.Runaway)

        else -> TransitionResult.Ignored
    }

    private fun processSettling(
        event: ErrorCascadeIsBoundedEvent
    ): TransitionResult<ErrorCascadeIsBoundedState> = when {
        event is ErrorCascadeIsBoundedEvent.Error.Execution && safeEvaluateGuard("repairs < 3") -> TransitionResult.Internal
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is ErrorCascadeIsBoundedEvent.Poke -> TransitionResult.Internal
        event is ErrorCascadeIsBoundedEvent.Reset -> TransitionResult.External(ErrorCascadeIsBoundedState.Idle, ErrorCascadeIsBoundedState.Settling)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: error_cascade_is_bounded.scxml:45 :: _machine
    override fun onEntry(state: ErrorCascadeIsBoundedState, pathChild: ErrorCascadeIsBoundedState?) {
        when (state) {
            is ErrorCascadeIsBoundedState.Idle -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:67 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
            is ErrorCascadeIsBoundedState.Runaway -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:103 :: runaway :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("runaway")) return


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            is ErrorCascadeIsBoundedState.Settling -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:85 :: settling :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settling")) return


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: error_cascade_is_bounded.scxml:45 :: _machine
    override fun onExit(state: ErrorCascadeIsBoundedState) {
        when (state) {
            is ErrorCascadeIsBoundedState.Idle -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:67 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
            is ErrorCascadeIsBoundedState.Runaway -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:103 :: runaway :: _state_body
                activeStateIds.remove("runaway")
            }
            is ErrorCascadeIsBoundedState.Settling -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:85 :: settling :: _state_body
                activeStateIds.remove("settling")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: error_cascade_is_bounded.scxml:45 :: _machine
    override fun executeTransitionActions(
        source: ErrorCascadeIsBoundedState,
        event: ErrorCascadeIsBoundedEvent?
    ) {
        when (source) {
        is ErrorCascadeIsBoundedState.Idle -> when {
            event is ErrorCascadeIsBoundedEvent.Poke -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:68 :: idle :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            event is ErrorCascadeIsBoundedEvent.Boom -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:74 :: idle :: _transition_1


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            else -> {}
        }
        is ErrorCascadeIsBoundedState.Runaway -> when {
            event is ErrorCascadeIsBoundedEvent.Error.Execution -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:107 :: runaway :: _transition_0


            executeAssign("runs", "runs + 1")

            raiseInternal(ErrorCascadeIsBoundedEvent.Tick)


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            event is ErrorCascadeIsBoundedEvent.Tick -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:120 :: runaway :: _transition_1


            executeAssign("ticks", "ticks + 1")
            }
            event is ErrorCascadeIsBoundedEvent.Poke -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:123 :: runaway :: _transition_2


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        is ErrorCascadeIsBoundedState.Settling -> when {
            event is ErrorCascadeIsBoundedEvent.Error.Execution && safeEvaluateGuard("repairs < 3") -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:89 :: settling :: _transition_0


            executeAssign("repairs", "repairs + 1")


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(ErrorCascadeIsBoundedEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            event is ErrorCascadeIsBoundedEvent.Poke -> {
                // SCE-MAP: error_cascade_is_bounded.scxml:93 :: settling :: _transition_1


            executeAssign("pokes", "pokes + 1")
            }
            else -> {}
        }
        }
    }
}
