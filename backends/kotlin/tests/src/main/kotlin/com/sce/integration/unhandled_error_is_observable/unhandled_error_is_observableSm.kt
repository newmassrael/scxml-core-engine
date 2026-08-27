// SCE-GENERATED — DO NOT EDIT
// source-hash: 88c46d955f89d1b6f7eb00aaedced29c5fbacc4db8ed4464fa38145a023ef16c
// template-hash: b86a6724a480cf92be72e95758ccfbe504b1a188bc95f743f8c94a7991541c4b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/unhandled_error_is_observable/unhandled_error_is_observable.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: unhandled_error_is_observable.scxml:40 :: _machine

package com.sce.integration.unhandled_error_is_observable

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface UnhandledErrorIsObservableState : State {
    data object Guarded : UnhandledErrorIsObservableState
    data object Idle : UnhandledErrorIsObservableState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface UnhandledErrorIsObservableEvent : Event {
    data object Boom : UnhandledErrorIsObservableEvent
    sealed interface Error : UnhandledErrorIsObservableEvent {
        data object Execution : Error
    }
    data object Go : UnhandledErrorIsObservableEvent
    data object Heard : UnhandledErrorIsObservableEvent
    data object Poke : UnhandledErrorIsObservableEvent
    sealed interface Retry : UnhandledErrorIsObservableEvent {
        sealed interface Error : Retry {
            data object Execution : Error
        }
    }
    data object Unheard : UnhandledErrorIsObservableEvent
    data object Whisper : UnhandledErrorIsObservableEvent
}
// --- State Machine (W3C SCXML) ---

class UnhandledErrorIsObservableStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<UnhandledErrorIsObservableState, UnhandledErrorIsObservableEvent>(scriptEngine) {

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
     * §scxml-5.3: what the `booms` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `booms` was assigned a value of another type, or the engine refused.
     */
    fun booms(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "booms")

    /**
     * §scxml-5.3: what the `caught` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `caught` was assigned a value of another type, or the engine refused.
     */
    fun caught(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "caught")

    /**
     * §scxml-5.3: what the `detail` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `detail` was assigned a value of another type, or the engine refused.
     */
    fun detail(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "detail")

    /**
     * §scxml-5.3: what the `heards` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `heards` was assigned a value of another type, or the engine refused.
     */
    fun heards(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "heards")

    override val initialState: UnhandledErrorIsObservableState = UnhandledErrorIsObservableState.Idle

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
    override fun resolveState(stateId: String): UnhandledErrorIsObservableState? = when (stateId) {
        "guarded" -> UnhandledErrorIsObservableState.Guarded
        "idle" -> UnhandledErrorIsObservableState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: UnhandledErrorIsObservableState): String = when (state) {
        is UnhandledErrorIsObservableState.Guarded -> "guarded"
        is UnhandledErrorIsObservableState.Idle -> "idle"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: UnhandledErrorIsObservableState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: UnhandledErrorIsObservableState): Int = when (state) {
        is UnhandledErrorIsObservableState.Guarded -> 1
        is UnhandledErrorIsObservableState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): UnhandledErrorIsObservableEvent? = when (name) {
        "boom" -> UnhandledErrorIsObservableEvent.Boom
        "error.execution" -> UnhandledErrorIsObservableEvent.Error.Execution
        "go" -> UnhandledErrorIsObservableEvent.Go
        "heard" -> UnhandledErrorIsObservableEvent.Heard
        "poke" -> UnhandledErrorIsObservableEvent.Poke
        "retry.error.execution" -> UnhandledErrorIsObservableEvent.Retry.Error.Execution
        "unheard" -> UnhandledErrorIsObservableEvent.Unheard
        "whisper" -> UnhandledErrorIsObservableEvent.Whisper
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: UnhandledErrorIsObservableEvent): String? = when (event) {
        is UnhandledErrorIsObservableEvent.Boom -> "boom"
        is UnhandledErrorIsObservableEvent.Error.Execution -> "error.execution"
        is UnhandledErrorIsObservableEvent.Go -> "go"
        is UnhandledErrorIsObservableEvent.Heard -> "heard"
        is UnhandledErrorIsObservableEvent.Poke -> "poke"
        is UnhandledErrorIsObservableEvent.Retry.Error.Execution -> "retry.error.execution"
        is UnhandledErrorIsObservableEvent.Unheard -> "unheard"
        is UnhandledErrorIsObservableEvent.Whisper -> "whisper"
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
            "unhandled_error_is_observable",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pokes' with expr
        try {
            val initResult_pokes = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "pokes", initResult_pokes)
        } catch (e: Exception) {
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<data id='pokes'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'booms' with expr
        try {
            val initResult_booms = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "booms", initResult_booms)
        } catch (e: Exception) {
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<data id='booms'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'caught' with expr
        try {
            val initResult_caught = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "caught", initResult_caught)
        } catch (e: Exception) {
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<data id='caught'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'detail' with expr
        try {
            val initResult_detail = engine.evaluateExpr(sid, "'none'")
            engine.setVariable(sid, "detail", initResult_detail)
        } catch (e: Exception) {
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<data id='detail'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'heards' with expr
        try {
            val initResult_heards = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "heards", initResult_heards)
        } catch (e: Exception) {
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<data id='heards'> expr failed to evaluate")
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
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: UnhandledErrorIsObservableEvent) {
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
        state: UnhandledErrorIsObservableState,
        event: UnhandledErrorIsObservableEvent
    ): TransitionResult<UnhandledErrorIsObservableState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is UnhandledErrorIsObservableState.Guarded -> processGuarded(event)
        is UnhandledErrorIsObservableState.Idle -> processIdle(event)
    }
    }


    // --- Per-State Event Handlers ---

    private fun processGuarded(
        event: UnhandledErrorIsObservableEvent
    ): TransitionResult<UnhandledErrorIsObservableState> = when {
        event is UnhandledErrorIsObservableEvent.Boom -> TransitionResult.External(UnhandledErrorIsObservableState.Guarded, UnhandledErrorIsObservableState.Guarded)

        event is UnhandledErrorIsObservableEvent.Error.Execution -> TransitionResult.External(UnhandledErrorIsObservableState.Guarded, UnhandledErrorIsObservableState.Guarded)

        else -> TransitionResult.Ignored
    }

    private fun processIdle(
        event: UnhandledErrorIsObservableEvent
    ): TransitionResult<UnhandledErrorIsObservableState> = when {
        event is UnhandledErrorIsObservableEvent.Poke -> TransitionResult.External(UnhandledErrorIsObservableState.Idle, UnhandledErrorIsObservableState.Idle)

        event is UnhandledErrorIsObservableEvent.Whisper -> TransitionResult.External(UnhandledErrorIsObservableState.Idle, UnhandledErrorIsObservableState.Idle)

        // W3C SCXML 3.13: Targetless transition (actions only)
        event is UnhandledErrorIsObservableEvent.Heard -> TransitionResult.Internal
        event is UnhandledErrorIsObservableEvent.Boom -> TransitionResult.External(UnhandledErrorIsObservableState.Idle, UnhandledErrorIsObservableState.Idle)

        event is UnhandledErrorIsObservableEvent.Go -> TransitionResult.External(UnhandledErrorIsObservableState.Guarded, UnhandledErrorIsObservableState.Idle)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: unhandled_error_is_observable.scxml:40 :: _machine
    override fun onEntry(state: UnhandledErrorIsObservableState, pathChild: UnhandledErrorIsObservableState?) {
        when (state) {
            is UnhandledErrorIsObservableState.Guarded -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:89 :: guarded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("guarded")) return
            }
            is UnhandledErrorIsObservableState.Idle -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:54 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: unhandled_error_is_observable.scxml:40 :: _machine
    override fun onExit(state: UnhandledErrorIsObservableState) {
        when (state) {
            is UnhandledErrorIsObservableState.Guarded -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:89 :: guarded :: _state_body
                activeStateIds.remove("guarded")
            }
            is UnhandledErrorIsObservableState.Idle -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:54 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: unhandled_error_is_observable.scxml:40 :: _machine
    override fun executeTransitionActions(
        source: UnhandledErrorIsObservableState,
        event: UnhandledErrorIsObservableEvent?
    ) {
        when (source) {
        is UnhandledErrorIsObservableState.Guarded -> when {
            event is UnhandledErrorIsObservableEvent.Boom -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:90 :: guarded :: _transition_0


            executeAssign("booms", "booms + 1")


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            event is UnhandledErrorIsObservableEvent.Error.Execution -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:94 :: guarded :: _transition_1


            executeAssign("caught", "caught + 1")


            executeAssign("detail", "_event.name")
            }
            else -> {}
        }
        is UnhandledErrorIsObservableState.Idle -> when {
            event is UnhandledErrorIsObservableEvent.Poke -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:55 :: idle :: _transition_0


            executeAssign("pokes", "pokes + 1")
            }
            event is UnhandledErrorIsObservableEvent.Whisper -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:58 :: idle :: _transition_1

            raiseInternal(UnhandledErrorIsObservableEvent.Unheard)

            raiseInternal(UnhandledErrorIsObservableEvent.Retry.Error.Execution)

            raiseInternal(UnhandledErrorIsObservableEvent.Heard)
            }
            event is UnhandledErrorIsObservableEvent.Heard -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:80 :: idle :: _transition_2


            executeAssign("heards", "heards + 1")
            }
            event is UnhandledErrorIsObservableEvent.Boom -> {
                // SCE-MAP: unhandled_error_is_observable.scxml:83 :: idle :: _transition_3


            executeAssign("booms", "booms + 1")


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(UnhandledErrorIsObservableEvent.Error.Execution, "<assign> has an invalid or read-only location")
            }
            else -> {}
        }
        }
    }
}
