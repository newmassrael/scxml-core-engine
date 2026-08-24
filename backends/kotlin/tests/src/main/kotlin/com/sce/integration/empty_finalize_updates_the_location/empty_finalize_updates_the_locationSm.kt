// SCE-GENERATED — DO NOT EDIT
// source-hash: c97edcb094613d8138825758fc943d853d23ad4854f2fa7dcf6ff6f58539b674
// template-hash: 672592645a46a971e7d9a638044244b01d838f9ebaf5e6860dc88538368c4548
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine

package com.sce.integration.empty_finalize_updates_the_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EmptyFinalizeUpdatesTheLocationState : State {
    data object AbsentPhase : EmptyFinalizeUpdatesTheLocationState
    data object EmptyPhase : EmptyFinalizeUpdatesTheLocationState
    data object FailAbsentChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailEmptyChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailNotUpdated : EmptyFinalizeUpdatesTheLocationState
    data object FailUnmatchedChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailUnmatchedNameWrote : EmptyFinalizeUpdatesTheLocationState
    data object FailUpdatedWithoutFinalize : EmptyFinalizeUpdatesTheLocationState
    data object Pass : EmptyFinalizeUpdatesTheLocationState
    data object UnmatchedPhase : EmptyFinalizeUpdatesTheLocationState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EmptyFinalizeUpdatesTheLocationEvent : Event {
    sealed interface Cancel : EmptyFinalizeUpdatesTheLocationEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : EmptyFinalizeUpdatesTheLocationEvent {
        data object Invoke : Done
    }
    sealed interface Error : EmptyFinalizeUpdatesTheLocationEvent {
        data object Execution : Error
    }
    data object FromAbsentChild : EmptyFinalizeUpdatesTheLocationEvent
    data object FromEmptyChild : EmptyFinalizeUpdatesTheLocationEvent
    data object FromUnmatchedChild : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutAbsent : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutEmpty : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutUnmatched : EmptyFinalizeUpdatesTheLocationEvent
}
// --- State Machine (W3C SCXML) ---

class EmptyFinalizeUpdatesTheLocationStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EmptyFinalizeUpdatesTheLocationState, EmptyFinalizeUpdatesTheLocationEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `tally` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tally` was assigned a value of another type, or the engine refused.
     */
    fun tally(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "tally")

    /**
     * §scxml-5.3: what the `guard` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `guard` was assigned a value of another type, or the engine refused.
     */
    fun guard(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "guard")

    /**
     * §scxml-5.3: what the `keeper` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `keeper` was assigned a value of another type, or the engine refused.
     */
    fun keeper(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "keeper")

    override val initialState: EmptyFinalizeUpdatesTheLocationState = EmptyFinalizeUpdatesTheLocationState.EmptyPhase

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EmptyFinalizeUpdatesTheLocationState? = when (stateId) {
        "absentPhase" -> EmptyFinalizeUpdatesTheLocationState.AbsentPhase
        "emptyPhase" -> EmptyFinalizeUpdatesTheLocationState.EmptyPhase
        "failAbsentChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent
        "failEmptyChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent
        "failNotUpdated" -> EmptyFinalizeUpdatesTheLocationState.FailNotUpdated
        "failUnmatchedChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent
        "failUnmatchedNameWrote" -> EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote
        "failUpdatedWithoutFinalize" -> EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize
        "pass" -> EmptyFinalizeUpdatesTheLocationState.Pass
        "unmatchedPhase" -> EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EmptyFinalizeUpdatesTheLocationState): String = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> "absentPhase"
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> "emptyPhase"
        is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> "failAbsentChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> "failEmptyChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> "failNotUpdated"
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> "failUnmatchedChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> "failUnmatchedNameWrote"
        is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> "failUpdatedWithoutFinalize"
        is EmptyFinalizeUpdatesTheLocationState.Pass -> "pass"
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> "unmatchedPhase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EmptyFinalizeUpdatesTheLocationState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EmptyFinalizeUpdatesTheLocationState): Int = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> 1
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> 0
        is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> 8
        is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> 7
        is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> 4
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> 9
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> 6
        is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> 5
        is EmptyFinalizeUpdatesTheLocationState.Pass -> 3
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EmptyFinalizeUpdatesTheLocationEvent? = when (name) {
        "cancel.invoke" -> EmptyFinalizeUpdatesTheLocationEvent.Cancel.Invoke
        "done.invoke" -> EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke
        "error.execution" -> EmptyFinalizeUpdatesTheLocationEvent.Error.Execution
        "fromAbsentChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild
        "fromEmptyChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild
        "fromUnmatchedChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild
        "timeoutAbsent" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent
        "timeoutEmpty" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty
        "timeoutUnmatched" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EmptyFinalizeUpdatesTheLocationEvent): String? = when (event) {
        is EmptyFinalizeUpdatesTheLocationEvent.Cancel.Invoke -> "cancel.invoke"
        is EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke -> "done.invoke"
        is EmptyFinalizeUpdatesTheLocationEvent.Error.Execution -> "error.execution"
        is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild -> "fromAbsentChild"
        is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild -> "fromEmptyChild"
        is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild -> "fromUnmatchedChild"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent -> "timeoutAbsent"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty -> "timeoutEmpty"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched -> "timeoutUnmatched"
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
            "empty_finalize_updates_the_location",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'tally' with expr
        try {
            val initResult_tally = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "tally", initResult_tally)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='tally'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'guard' with expr
        try {
            val initResult_guard = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "guard", initResult_guard)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='guard'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'keeper' with expr
        try {
            val initResult_keeper = engine.evaluateExpr(sid, "3")
            engine.setVariable(sid, "keeper", initResult_keeper)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='keeper'> expr failed to evaluate")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EmptyFinalizeUpdatesTheLocationEvent) {
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
        state: EmptyFinalizeUpdatesTheLocationState,
        event: EmptyFinalizeUpdatesTheLocationEvent
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> processAbsentPhase(event)
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> processEmptyPhase(event)
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> processUnmatchedPhase(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processAbsentPhase(
        event: EmptyFinalizeUpdatesTheLocationEvent
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationState> = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild && safeEvaluateGuard("guard !== 1") -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize, EmptyFinalizeUpdatesTheLocationState.AbsentPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase, EmptyFinalizeUpdatesTheLocationState.AbsentPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent, EmptyFinalizeUpdatesTheLocationState.AbsentPhase)

        else -> TransitionResult.Ignored
    }

    private fun processEmptyPhase(
        event: EmptyFinalizeUpdatesTheLocationEvent
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationState> = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild && safeEvaluateGuard("tally === 7") -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.AbsentPhase, EmptyFinalizeUpdatesTheLocationState.EmptyPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailNotUpdated, EmptyFinalizeUpdatesTheLocationState.EmptyPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent, EmptyFinalizeUpdatesTheLocationState.EmptyPhase)

        else -> TransitionResult.Ignored
    }

    private fun processUnmatchedPhase(
        event: EmptyFinalizeUpdatesTheLocationEvent
    ): TransitionResult<EmptyFinalizeUpdatesTheLocationState> = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild && safeEvaluateGuard("keeper !== 3") -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote, EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.Pass, EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)

        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched -> TransitionResult.External(EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent, EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun onEntry(state: EmptyFinalizeUpdatesTheLocationState, pathChild: EmptyFinalizeUpdatesTheLocationState?) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:105 :: absentPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("absentPhase")) return


            scheduleSend("__send_1", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "absentPhase.${System.identityHashCode(this)}.inv_absent"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "guard")) {
                        raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names 'guard', which the parent does not declare")
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["guard"] = engineInv.getVariable(sidInv, "guard")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvAbsentStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_absent", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:71 :: emptyPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("emptyPhase")) return


            scheduleSend("__send_0", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "emptyPhase.${System.identityHashCode(this)}.inv_empty"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "tally")) {
                        raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names 'tally', which the parent does not declare")
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["tally"] = engineInv.getVariable(sidInv, "tally")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_empty", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "if (_event.data && _event.data.tally !== undefined) { tally = _event.data.tally; }", generatedInvokeId)
                    }
                }
            }
            is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:183 :: failAbsentChildSilent :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failAbsentChildSilent")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:182 :: failEmptyChildSilent :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failEmptyChildSilent")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:179 :: failNotUpdated :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNotUpdated")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:184 :: failUnmatchedChildSilent :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failUnmatchedChildSilent")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:181 :: failUnmatchedNameWrote :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failUnmatchedNameWrote")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:180 :: failUpdatedWithoutFinalize :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failUpdatedWithoutFinalize")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.Pass -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:178 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:144 :: unmatchedPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("unmatchedPhase")) return


            scheduleSend("__send_2", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "unmatchedPhase.${System.identityHashCode(this)}.inv_unmatched"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "keeper")) {
                        raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names 'keeper', which the parent does not declare")
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["keeper"] = engineInv.getVariable(sidInv, "keeper")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvUnmatchedStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_unmatched", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "if (_event.data && _event.data.keeper !== undefined) { keeper = _event.data.keeper; }", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun onExit(state: EmptyFinalizeUpdatesTheLocationState) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:105 :: absentPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_absent")
                activeStateIds.remove("absentPhase")
            }
            is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:71 :: emptyPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_empty")
                activeStateIds.remove("emptyPhase")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:183 :: failAbsentChildSilent :: _state_body
                activeStateIds.remove("failAbsentChildSilent")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:182 :: failEmptyChildSilent :: _state_body
                activeStateIds.remove("failEmptyChildSilent")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:179 :: failNotUpdated :: _state_body
                activeStateIds.remove("failNotUpdated")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:184 :: failUnmatchedChildSilent :: _state_body
                activeStateIds.remove("failUnmatchedChildSilent")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:181 :: failUnmatchedNameWrote :: _state_body
                activeStateIds.remove("failUnmatchedNameWrote")
            }
            is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:180 :: failUpdatedWithoutFinalize :: _state_body
                activeStateIds.remove("failUpdatedWithoutFinalize")
            }
            is EmptyFinalizeUpdatesTheLocationState.Pass -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:178 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:144 :: unmatchedPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_unmatched")
                activeStateIds.remove("unmatchedPhase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun executeTransitionActions(
        source: EmptyFinalizeUpdatesTheLocationState,
        event: EmptyFinalizeUpdatesTheLocationEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
