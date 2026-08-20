// SCE-GENERATED — DO NOT EDIT
// source-hash: 215c3b8c048d546a929c95bb520cc0c508e71ce4c95c9630e94bb32b22528dc2
// template-hash: 63129ea5a60cce4407210a3c2e3ff224327767ebf6618c3f4ed41b0a49b7454d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/ancestor_entry_is_not_default_entry/ancestor_entry_is_not_default_entry.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine

package com.sce.integration.ancestor_entry_is_not_default_entry

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AncestorEntryIsNotDefaultEntryState : State {
    data object Away : AncestorEntryIsNotDefaultEntryState
    data object ByDefault : AncestorEntryIsNotDefaultEntryState
    data object Chosen : AncestorEntryIsNotDefaultEntryState
    data object Drive : AncestorEntryIsNotDefaultEntryState
    data object FailDefaulted : AncestorEntryIsNotDefaultEntryState
    data object FailIdled : AncestorEntryIsNotDefaultEntryState
    data object FailLobbied : AncestorEntryIsNotDefaultEntryState
    data object FailTargeted : AncestorEntryIsNotDefaultEntryState
    data object Idle : AncestorEntryIsNotDefaultEntryState
    data object Lobby : AncestorEntryIsNotDefaultEntryState
    data object Outer : AncestorEntryIsNotDefaultEntryState
    data object Run : AncestorEntryIsNotDefaultEntryState
    data object Settled : AncestorEntryIsNotDefaultEntryState
    data object Watch : AncestorEntryIsNotDefaultEntryState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AncestorEntryIsNotDefaultEntryEvent : Event {
    data object Again : AncestorEntryIsNotDefaultEntryEvent
    data object Back : AncestorEntryIsNotDefaultEntryEvent
    data object Check : AncestorEntryIsNotDefaultEntryEvent
    data object Cross : AncestorEntryIsNotDefaultEntryEvent
    sealed interface Error : AncestorEntryIsNotDefaultEntryEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class AncestorEntryIsNotDefaultEntryStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AncestorEntryIsNotDefaultEntryState, AncestorEntryIsNotDefaultEntryEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `defaulted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `defaulted` was assigned a value of another type, or the engine refused.
     */
    fun defaulted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "defaulted")

    /**
     * §scxml-5.3: what the `lobbied` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `lobbied` was assigned a value of another type, or the engine refused.
     */
    fun lobbied(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "lobbied")

    /**
     * §scxml-5.3: what the `idled` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `idled` was assigned a value of another type, or the engine refused.
     */
    fun idled(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "idled")

    /**
     * §scxml-5.3: what the `targeted` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `targeted` was assigned a value of another type, or the engine refused.
     */
    fun targeted(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "targeted")

    override val initialState: AncestorEntryIsNotDefaultEntryState = AncestorEntryIsNotDefaultEntryState.Away

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: AncestorEntryIsNotDefaultEntryState): AncestorEntryIsNotDefaultEntryState? = when (state) {
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> AncestorEntryIsNotDefaultEntryState.Outer
        is AncestorEntryIsNotDefaultEntryState.Chosen -> AncestorEntryIsNotDefaultEntryState.Outer
        is AncestorEntryIsNotDefaultEntryState.Drive -> AncestorEntryIsNotDefaultEntryState.Run
        is AncestorEntryIsNotDefaultEntryState.Idle -> AncestorEntryIsNotDefaultEntryState.Watch
        is AncestorEntryIsNotDefaultEntryState.Lobby -> AncestorEntryIsNotDefaultEntryState.Drive
        is AncestorEntryIsNotDefaultEntryState.Outer -> AncestorEntryIsNotDefaultEntryState.Drive
        is AncestorEntryIsNotDefaultEntryState.Watch -> AncestorEntryIsNotDefaultEntryState.Run
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: AncestorEntryIsNotDefaultEntryState): AncestorEntryIsNotDefaultEntryState = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Drive -> AncestorEntryIsNotDefaultEntryState.Lobby
        is AncestorEntryIsNotDefaultEntryState.Outer -> AncestorEntryIsNotDefaultEntryState.ByDefault
        is AncestorEntryIsNotDefaultEntryState.Run -> AncestorEntryIsNotDefaultEntryState.Lobby
        is AncestorEntryIsNotDefaultEntryState.Watch -> AncestorEntryIsNotDefaultEntryState.Idle
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AncestorEntryIsNotDefaultEntryState? = when (stateId) {
        "away" -> AncestorEntryIsNotDefaultEntryState.Away
        "by_default" -> AncestorEntryIsNotDefaultEntryState.ByDefault
        "chosen" -> AncestorEntryIsNotDefaultEntryState.Chosen
        "drive" -> AncestorEntryIsNotDefaultEntryState.Drive
        "failDefaulted" -> AncestorEntryIsNotDefaultEntryState.FailDefaulted
        "failIdled" -> AncestorEntryIsNotDefaultEntryState.FailIdled
        "failLobbied" -> AncestorEntryIsNotDefaultEntryState.FailLobbied
        "failTargeted" -> AncestorEntryIsNotDefaultEntryState.FailTargeted
        "idle" -> AncestorEntryIsNotDefaultEntryState.Idle
        "lobby" -> AncestorEntryIsNotDefaultEntryState.Lobby
        "outer" -> AncestorEntryIsNotDefaultEntryState.Outer
        "run" -> AncestorEntryIsNotDefaultEntryState.Run
        "settled" -> AncestorEntryIsNotDefaultEntryState.Settled
        "watch" -> AncestorEntryIsNotDefaultEntryState.Watch
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AncestorEntryIsNotDefaultEntryState): String = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> "away"
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> "by_default"
        is AncestorEntryIsNotDefaultEntryState.Chosen -> "chosen"
        is AncestorEntryIsNotDefaultEntryState.Drive -> "drive"
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> "failDefaulted"
        is AncestorEntryIsNotDefaultEntryState.FailIdled -> "failIdled"
        is AncestorEntryIsNotDefaultEntryState.FailLobbied -> "failLobbied"
        is AncestorEntryIsNotDefaultEntryState.FailTargeted -> "failTargeted"
        is AncestorEntryIsNotDefaultEntryState.Idle -> "idle"
        is AncestorEntryIsNotDefaultEntryState.Lobby -> "lobby"
        is AncestorEntryIsNotDefaultEntryState.Outer -> "outer"
        is AncestorEntryIsNotDefaultEntryState.Run -> "run"
        is AncestorEntryIsNotDefaultEntryState.Settled -> "settled"
        is AncestorEntryIsNotDefaultEntryState.Watch -> "watch"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Drive -> false
        is AncestorEntryIsNotDefaultEntryState.Outer -> false
        is AncestorEntryIsNotDefaultEntryState.Run -> false
        is AncestorEntryIsNotDefaultEntryState.Watch -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: AncestorEntryIsNotDefaultEntryState): Boolean = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: AncestorEntryIsNotDefaultEntryState): List<AncestorEntryIsNotDefaultEntryState> = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Run -> listOf(AncestorEntryIsNotDefaultEntryState.Drive, AncestorEntryIsNotDefaultEntryState.Watch)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AncestorEntryIsNotDefaultEntryState): Int = when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> 0
        is AncestorEntryIsNotDefaultEntryState.ByDefault -> 5
        is AncestorEntryIsNotDefaultEntryState.Chosen -> 6
        is AncestorEntryIsNotDefaultEntryState.Drive -> 2
        is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> 10
        is AncestorEntryIsNotDefaultEntryState.FailIdled -> 12
        is AncestorEntryIsNotDefaultEntryState.FailLobbied -> 11
        is AncestorEntryIsNotDefaultEntryState.FailTargeted -> 13
        is AncestorEntryIsNotDefaultEntryState.Idle -> 8
        is AncestorEntryIsNotDefaultEntryState.Lobby -> 3
        is AncestorEntryIsNotDefaultEntryState.Outer -> 4
        is AncestorEntryIsNotDefaultEntryState.Run -> 1
        is AncestorEntryIsNotDefaultEntryState.Settled -> 9
        is AncestorEntryIsNotDefaultEntryState.Watch -> 7
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AncestorEntryIsNotDefaultEntryEvent? = when (name) {
        "again" -> AncestorEntryIsNotDefaultEntryEvent.Again
        "back" -> AncestorEntryIsNotDefaultEntryEvent.Back
        "check" -> AncestorEntryIsNotDefaultEntryEvent.Check
        "cross" -> AncestorEntryIsNotDefaultEntryEvent.Cross
        "error.execution" -> AncestorEntryIsNotDefaultEntryEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AncestorEntryIsNotDefaultEntryEvent): String? = when (event) {
        is AncestorEntryIsNotDefaultEntryEvent.Again -> "again"
        is AncestorEntryIsNotDefaultEntryEvent.Back -> "back"
        is AncestorEntryIsNotDefaultEntryEvent.Check -> "check"
        is AncestorEntryIsNotDefaultEntryEvent.Cross -> "cross"
        is AncestorEntryIsNotDefaultEntryEvent.Error.Execution -> "error.execution"
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
            "ancestor_entry_is_not_default_entry",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'defaulted' with expr
        try {
            val initResult_defaulted = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "defaulted", initResult_defaulted)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='defaulted'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'lobbied' with expr
        try {
            val initResult_lobbied = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "lobbied", initResult_lobbied)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='lobbied'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'idled' with expr
        try {
            val initResult_idled = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "idled", initResult_idled)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='idled'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'targeted' with expr
        try {
            val initResult_targeted = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "targeted", initResult_targeted)
        } catch (e: Exception) {
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<data id='targeted'> expr failed to evaluate")
        }



        // W3C SCXML 5.9.2: Register In() predicate callback
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }

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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AncestorEntryIsNotDefaultEntryEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AncestorEntryIsNotDefaultEntryEvent) {
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
        state: AncestorEntryIsNotDefaultEntryState,
        event: AncestorEntryIsNotDefaultEntryEvent
    ): TransitionResult<AncestorEntryIsNotDefaultEntryState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is AncestorEntryIsNotDefaultEntryState.Away -> processAway(event)
        is AncestorEntryIsNotDefaultEntryState.Chosen -> processChosen(event)
        is AncestorEntryIsNotDefaultEntryState.Lobby -> processLobby(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processAway(
        event: AncestorEntryIsNotDefaultEntryEvent
    ): TransitionResult<AncestorEntryIsNotDefaultEntryState> = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Cross -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.Chosen, AncestorEntryIsNotDefaultEntryState.Away)

        else -> TransitionResult.Ignored
    }

    private fun processChosen(
        event: AncestorEntryIsNotDefaultEntryEvent
    ): TransitionResult<AncestorEntryIsNotDefaultEntryState> = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Back -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.Lobby, AncestorEntryIsNotDefaultEntryState.Chosen)

        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard("defaulted != 0") -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.FailDefaulted, AncestorEntryIsNotDefaultEntryState.Chosen)

        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard("lobbied != 1") -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.FailLobbied, AncestorEntryIsNotDefaultEntryState.Chosen)

        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard("idled != 1") -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.FailIdled, AncestorEntryIsNotDefaultEntryState.Chosen)

        event is AncestorEntryIsNotDefaultEntryEvent.Check && safeEvaluateGuard("targeted != 2") -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.FailTargeted, AncestorEntryIsNotDefaultEntryState.Chosen)

        event is AncestorEntryIsNotDefaultEntryEvent.Check -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.Settled, AncestorEntryIsNotDefaultEntryState.Chosen)

        else -> TransitionResult.Ignored
    }

    private fun processLobby(
        event: AncestorEntryIsNotDefaultEntryEvent
    ): TransitionResult<AncestorEntryIsNotDefaultEntryState> = when {
        event is AncestorEntryIsNotDefaultEntryEvent.Again -> TransitionResult.External(AncestorEntryIsNotDefaultEntryState.Chosen, AncestorEntryIsNotDefaultEntryState.Lobby)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onEntry(state: AncestorEntryIsNotDefaultEntryState, pathChild: AncestorEntryIsNotDefaultEntryState?) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("away")) return
            }
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("by_default")) return


            executeAssign("defaulted", "defaulted + 1")
            }
            is AncestorEntryIsNotDefaultEntryState.Chosen -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("chosen")) return


            executeAssign("targeted", "targeted + 1")
            }
            is AncestorEntryIsNotDefaultEntryState.Drive -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("drive")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AncestorEntryIsNotDefaultEntryState.Lobby)
                }
            }
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failDefaulted")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failIdled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failLobbied")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failTargeted")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.Idle -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("idle")) return


            executeAssign("idled", "idled + 1")
            }
            is AncestorEntryIsNotDefaultEntryState.Lobby -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("lobby")) return


            executeAssign("lobbied", "lobbied + 1")
            }
            is AncestorEntryIsNotDefaultEntryState.Outer -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("outer")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AncestorEntryIsNotDefaultEntryState.ByDefault)
                }
            }
            is AncestorEntryIsNotDefaultEntryState.Run -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != AncestorEntryIsNotDefaultEntryState.Drive) {
                    onEntry(AncestorEntryIsNotDefaultEntryState.Drive)
                }
                if (pathChild != AncestorEntryIsNotDefaultEntryState.Watch) {
                    onEntry(AncestorEntryIsNotDefaultEntryState.Watch)
                }
            }
            is AncestorEntryIsNotDefaultEntryState.Settled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AncestorEntryIsNotDefaultEntryState.Watch -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(AncestorEntryIsNotDefaultEntryState.Idle)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun onExit(state: AncestorEntryIsNotDefaultEntryState) {
        when (state) {
            is AncestorEntryIsNotDefaultEntryState.Away -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:89 :: away :: _state_body
                activeStateIds.remove("away")
            }
            is AncestorEntryIsNotDefaultEntryState.ByDefault -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:116 :: by_default :: _state_body
                activeStateIds.remove("by_default")
            }
            is AncestorEntryIsNotDefaultEntryState.Chosen -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:127 :: chosen :: _state_body
                activeStateIds.remove("chosen")
            }
            is AncestorEntryIsNotDefaultEntryState.Drive -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:95 :: drive :: _state_body
                activeStateIds.remove("drive")
            }
            is AncestorEntryIsNotDefaultEntryState.FailDefaulted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:160 :: failDefaulted :: _state_body
                activeStateIds.remove("failDefaulted")
            }
            is AncestorEntryIsNotDefaultEntryState.FailIdled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:162 :: failIdled :: _state_body
                activeStateIds.remove("failIdled")
            }
            is AncestorEntryIsNotDefaultEntryState.FailLobbied -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:161 :: failLobbied :: _state_body
                activeStateIds.remove("failLobbied")
            }
            is AncestorEntryIsNotDefaultEntryState.FailTargeted -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:163 :: failTargeted :: _state_body
                activeStateIds.remove("failTargeted")
            }
            is AncestorEntryIsNotDefaultEntryState.Idle -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:150 :: idle :: _state_body
                activeStateIds.remove("idle")
            }
            is AncestorEntryIsNotDefaultEntryState.Lobby -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:102 :: lobby :: _state_body
                activeStateIds.remove("lobby")
            }
            is AncestorEntryIsNotDefaultEntryState.Outer -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:109 :: outer :: _state_body
                activeStateIds.remove("outer")
            }
            is AncestorEntryIsNotDefaultEntryState.Run -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:93 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<AncestorEntryIsNotDefaultEntryState, Int>>()
                if (activeStateIds.contains("drive")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Drive to 2)
                }
                if (activeStateIds.contains("lobby")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Lobby to 3)
                }
                if (activeStateIds.contains("outer")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Outer to 4)
                }
                if (activeStateIds.contains("by_default")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.ByDefault to 5)
                }
                if (activeStateIds.contains("chosen")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Chosen to 6)
                }
                if (activeStateIds.contains("watch")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Watch to 7)
                }
                if (activeStateIds.contains("idle")) {
                    toExit.add(AncestorEntryIsNotDefaultEntryState.Idle to 8)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is AncestorEntryIsNotDefaultEntryState.Settled -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:159 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
            is AncestorEntryIsNotDefaultEntryState.Watch -> {
                // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:149 :: watch :: _state_body
                activeStateIds.remove("watch")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: ancestor_entry_is_not_default_entry.scxml:69 :: _machine
    override fun executeTransitionActions(
        source: AncestorEntryIsNotDefaultEntryState,
        event: AncestorEntryIsNotDefaultEntryEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
