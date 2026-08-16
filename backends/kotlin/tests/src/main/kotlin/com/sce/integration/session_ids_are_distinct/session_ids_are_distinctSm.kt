// SCE-GENERATED — DO NOT EDIT
// source-hash: 72e5f6add40450019fedf97192aa7f8b2b99f0983d778103d9af035fcb5f7cfa
// template-hash: 128f5bda1db8a8695e204b38e87b8d2d3815bdde9691186823a5ecdc7374af1d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/session_ids_are_distinct/session_ids_are_distinct.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: session_ids_are_distinct.scxml:38 :: _machine

package com.sce.integration.session_ids_are_distinct

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SessionIdsAreDistinctState : State {
    data object Fail : SessionIdsAreDistinctState
    data object OneSeen : SessionIdsAreDistinctState
    data object Pass : SessionIdsAreDistinctState
    data object Phase : SessionIdsAreDistinctState
    data object Waiting : SessionIdsAreDistinctState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SessionIdsAreDistinctEvent : Event {
    sealed interface Cancel : SessionIdsAreDistinctEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : SessionIdsAreDistinctEvent {
        data object Invoke : Done
    }
    sealed interface Error : SessionIdsAreDistinctEvent {
        data object Execution : Error
    }
    data object FromChild : SessionIdsAreDistinctEvent
}
// --- State Machine (W3C SCXML) ---

class SessionIdsAreDistinctStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<SessionIdsAreDistinctState, SessionIdsAreDistinctEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `firstSid` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `firstSid` was assigned a value of another type, or the engine refused.
     */
    fun firstSid(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "firstSid")

    /**
     * §scxml-5.3: what the `readBackProbe` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `readBackProbe` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun readBackProbe(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "readBackProbe")

    override val initialState: SessionIdsAreDistinctState = SessionIdsAreDistinctState.Waiting

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: SessionIdsAreDistinctState): SessionIdsAreDistinctState? = when (state) {
        is SessionIdsAreDistinctState.OneSeen -> SessionIdsAreDistinctState.Phase
        is SessionIdsAreDistinctState.Waiting -> SessionIdsAreDistinctState.Phase
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: SessionIdsAreDistinctState): SessionIdsAreDistinctState = when (state) {
        is SessionIdsAreDistinctState.Phase -> SessionIdsAreDistinctState.Waiting
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SessionIdsAreDistinctState? = when (stateId) {
        "fail" -> SessionIdsAreDistinctState.Fail
        "one_seen" -> SessionIdsAreDistinctState.OneSeen
        "pass" -> SessionIdsAreDistinctState.Pass
        "phase" -> SessionIdsAreDistinctState.Phase
        "waiting" -> SessionIdsAreDistinctState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SessionIdsAreDistinctState): String = when (state) {
        is SessionIdsAreDistinctState.Fail -> "fail"
        is SessionIdsAreDistinctState.OneSeen -> "one_seen"
        is SessionIdsAreDistinctState.Pass -> "pass"
        is SessionIdsAreDistinctState.Phase -> "phase"
        is SessionIdsAreDistinctState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SessionIdsAreDistinctState): Boolean = when (state) {
        is SessionIdsAreDistinctState.Phase -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: SessionIdsAreDistinctState): Int = when (state) {
        is SessionIdsAreDistinctState.Fail -> 4
        is SessionIdsAreDistinctState.OneSeen -> 2
        is SessionIdsAreDistinctState.Pass -> 3
        is SessionIdsAreDistinctState.Phase -> 0
        is SessionIdsAreDistinctState.Waiting -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SessionIdsAreDistinctEvent? = when (name) {
        "cancel.invoke" -> SessionIdsAreDistinctEvent.Cancel.Invoke
        "done.invoke" -> SessionIdsAreDistinctEvent.Done.Invoke
        "error.execution" -> SessionIdsAreDistinctEvent.Error.Execution
        "fromChild" -> SessionIdsAreDistinctEvent.FromChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SessionIdsAreDistinctEvent): String? = when (event) {
        is SessionIdsAreDistinctEvent.Cancel.Invoke -> "cancel.invoke"
        is SessionIdsAreDistinctEvent.Done.Invoke -> "done.invoke"
        is SessionIdsAreDistinctEvent.Error.Execution -> "error.execution"
        is SessionIdsAreDistinctEvent.FromChild -> "fromChild"
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
            "session_ids_are_distinct",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'firstSid' with expr
        try {
            val initResult_firstSid = engine.evaluateExpr(sid, "''")
            engine.setVariable(sid, "firstSid", initResult_firstSid)
        } catch (e: Exception) {
            raiseInternal(SessionIdsAreDistinctEvent.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'readBackProbe' with expr
        try {
            val initResult_readBackProbe = engine.evaluateExpr(sid, "[{ name: 'first', keys: 'Escape' }, { name: 'second' }]")
            engine.setVariable(sid, "readBackProbe", initResult_readBackProbe)
        } catch (e: Exception) {
            raiseInternal(SessionIdsAreDistinctEvent.Error.Execution)
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
            raiseInternal(SessionIdsAreDistinctEvent.Error.Execution)
            false
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
            raiseInternal(SessionIdsAreDistinctEvent.Error.Execution)
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
            raiseInternal(SessionIdsAreDistinctEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: SessionIdsAreDistinctEvent) {
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
        state: SessionIdsAreDistinctState,
        event: SessionIdsAreDistinctEvent
    ): TransitionResult<SessionIdsAreDistinctState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is SessionIdsAreDistinctState.OneSeen -> processOneSeen(event)
        is SessionIdsAreDistinctState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processOneSeen(
        event: SessionIdsAreDistinctEvent
    ): TransitionResult<SessionIdsAreDistinctState> = when {
        event is SessionIdsAreDistinctEvent.FromChild && safeEvaluateGuard("_event.data.sid != firstSid") -> TransitionResult.External(SessionIdsAreDistinctState.Pass, SessionIdsAreDistinctState.OneSeen)

        event is SessionIdsAreDistinctEvent.FromChild -> TransitionResult.External(SessionIdsAreDistinctState.Fail, SessionIdsAreDistinctState.OneSeen)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: SessionIdsAreDistinctEvent
    ): TransitionResult<SessionIdsAreDistinctState> = when {
        event is SessionIdsAreDistinctEvent.FromChild -> TransitionResult.External(SessionIdsAreDistinctState.OneSeen, SessionIdsAreDistinctState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: session_ids_are_distinct.scxml:38 :: _machine
    override fun onEntry(state: SessionIdsAreDistinctState, pathChild: SessionIdsAreDistinctState?) {
        when (state) {
            is SessionIdsAreDistinctState.Fail -> {
                // SCE-MAP: session_ids_are_distinct.scxml:116 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SessionIdsAreDistinctState.OneSeen -> {
                // SCE-MAP: session_ids_are_distinct.scxml:109 :: one_seen :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("one_seen")) return
            }
            is SessionIdsAreDistinctState.Pass -> {
                // SCE-MAP: session_ids_are_distinct.scxml:115 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SessionIdsAreDistinctState.Phase -> {
                // SCE-MAP: session_ids_are_distinct.scxml:70 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_a"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = SessionIdsAreDistinctSceSynthInvokeInvAStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_a", childSM, false, SessionIdsAreDistinctEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_b"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = SessionIdsAreDistinctSceSynthInvokeInvBStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_b", childSM, false, SessionIdsAreDistinctEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is SessionIdsAreDistinctState.Waiting -> {
                // SCE-MAP: session_ids_are_distinct.scxml:100 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: session_ids_are_distinct.scxml:38 :: _machine
    override fun onExit(state: SessionIdsAreDistinctState) {
        when (state) {
            is SessionIdsAreDistinctState.Fail -> {
                // SCE-MAP: session_ids_are_distinct.scxml:116 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is SessionIdsAreDistinctState.OneSeen -> {
                // SCE-MAP: session_ids_are_distinct.scxml:109 :: one_seen :: _state_body
                activeStateIds.remove("one_seen")
            }
            is SessionIdsAreDistinctState.Pass -> {
                // SCE-MAP: session_ids_are_distinct.scxml:115 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is SessionIdsAreDistinctState.Phase -> {
                // SCE-MAP: session_ids_are_distinct.scxml:70 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_a")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_b")
                activeStateIds.remove("phase")
            }
            is SessionIdsAreDistinctState.Waiting -> {
                // SCE-MAP: session_ids_are_distinct.scxml:100 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: session_ids_are_distinct.scxml:38 :: _machine
    override fun executeTransitionActions(
        source: SessionIdsAreDistinctState,
        event: SessionIdsAreDistinctEvent?
    ) {
        when (source) {
        is SessionIdsAreDistinctState.Waiting -> when {
            event is SessionIdsAreDistinctEvent.FromChild -> {
                // SCE-MAP: session_ids_are_distinct.scxml:101 :: waiting :: _transition_0


            executeAssign("firstSid", "_event.data.sid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
