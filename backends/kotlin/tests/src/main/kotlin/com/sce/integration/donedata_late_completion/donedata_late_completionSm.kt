// SCE-GENERATED — DO NOT EDIT
// source-hash: a31c47a0247af69ee06a626967ff0d05ffe8ed68e66f9b9928d0b71cb7eccebd
// template-hash: e1ef1a80ec6f1d98421ed2b76701aed66a2f64164d943082fb9a22d750e546a9
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/donedata_late_completion/donedata_late_completion.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: donedata_late_completion.scxml:45 :: _machine

package com.sce.integration.donedata_late_completion

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLateCompletionState : State {
    data object Fail : DonedataLateCompletionState
    data object Pass : DonedataLateCompletionState
    data object Phase : DonedataLateCompletionState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLateCompletionEvent : Event {
    sealed interface Cancel : DonedataLateCompletionEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : DonedataLateCompletionEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object InvLate : Invoke
        }
    }
    sealed interface Error : DonedataLateCompletionEvent {
        data object Execution : Error
    }
    data object Finish : DonedataLateCompletionEvent
    data object Ready : DonedataLateCompletionEvent
}
// --- State Machine (W3C SCXML) ---

class DonedataLateCompletionStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DonedataLateCompletionState, DonedataLateCompletionEvent>(scriptEngine) {

    override val initialState: DonedataLateCompletionState = DonedataLateCompletionState.Phase

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
    override fun resolveState(stateId: String): DonedataLateCompletionState? = when (stateId) {
        "fail" -> DonedataLateCompletionState.Fail
        "pass" -> DonedataLateCompletionState.Pass
        "phase" -> DonedataLateCompletionState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLateCompletionState): String = when (state) {
        is DonedataLateCompletionState.Fail -> "fail"
        is DonedataLateCompletionState.Pass -> "pass"
        is DonedataLateCompletionState.Phase -> "phase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLateCompletionState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLateCompletionState): Int = when (state) {
        is DonedataLateCompletionState.Fail -> 2
        is DonedataLateCompletionState.Pass -> 1
        is DonedataLateCompletionState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLateCompletionEvent? = when (name) {
        "cancel.invoke" -> DonedataLateCompletionEvent.Cancel.Invoke
        "done.invoke" -> DonedataLateCompletionEvent.Done.Invoke.Self
        "done.invoke.inv_late" -> DonedataLateCompletionEvent.Done.Invoke.InvLate
        "error.execution" -> DonedataLateCompletionEvent.Error.Execution
        "finish" -> DonedataLateCompletionEvent.Finish
        "ready" -> DonedataLateCompletionEvent.Ready
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLateCompletionEvent): String? = when (event) {
        is DonedataLateCompletionEvent.Cancel.Invoke -> "cancel.invoke"
        is DonedataLateCompletionEvent.Done.Invoke.Self -> "done.invoke"
        is DonedataLateCompletionEvent.Done.Invoke.InvLate -> "done.invoke.inv_late"
        is DonedataLateCompletionEvent.Error.Execution -> "error.execution"
        is DonedataLateCompletionEvent.Finish -> "finish"
        is DonedataLateCompletionEvent.Ready -> "ready"
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
            "donedata_late_completion",
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
            raisePlatformError(DonedataLateCompletionEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(DonedataLateCompletionEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(DonedataLateCompletionEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(DonedataLateCompletionEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLateCompletionEvent) {
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
        state: DonedataLateCompletionState,
        event: DonedataLateCompletionEvent
    ): TransitionResult<DonedataLateCompletionState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is DonedataLateCompletionState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: DonedataLateCompletionEvent
    ): TransitionResult<DonedataLateCompletionState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is DonedataLateCompletionEvent.Ready -> TransitionResult.Internal
        event is DonedataLateCompletionEvent.Done.Invoke.InvLate && safeEvaluateGuard("_event.data && _event.data.result === 42") -> TransitionResult.External(DonedataLateCompletionState.Pass, DonedataLateCompletionState.Phase)

        event is DonedataLateCompletionEvent.Done.Invoke.InvLate -> TransitionResult.External(DonedataLateCompletionState.Fail, DonedataLateCompletionState.Phase)

        event is DonedataLateCompletionEvent.Error.Execution -> TransitionResult.External(DonedataLateCompletionState.Fail, DonedataLateCompletionState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: donedata_late_completion.scxml:45 :: _machine
    override fun onEntry(state: DonedataLateCompletionState, pathChild: DonedataLateCompletionState?) {
        when (state) {
            is DonedataLateCompletionState.Fail -> {
                // SCE-MAP: donedata_late_completion.scxml:77 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLateCompletionState.Pass -> {
                // SCE-MAP: donedata_late_completion.scxml:76 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLateCompletionState.Phase -> {
                // SCE-MAP: donedata_late_completion.scxml:48 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_late"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = DonedataLateCompletionSceSynthInvokeInvLateStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_late", childSM, false, DonedataLateCompletionEvent.Done.Invoke.InvLate, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: donedata_late_completion.scxml:45 :: _machine
    override fun onExit(state: DonedataLateCompletionState) {
        when (state) {
            is DonedataLateCompletionState.Fail -> {
                // SCE-MAP: donedata_late_completion.scxml:77 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is DonedataLateCompletionState.Pass -> {
                // SCE-MAP: donedata_late_completion.scxml:76 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is DonedataLateCompletionState.Phase -> {
                // SCE-MAP: donedata_late_completion.scxml:48 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_late")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: donedata_late_completion.scxml:45 :: _machine
    override fun executeTransitionActions(
        source: DonedataLateCompletionState,
        event: DonedataLateCompletionEvent?
    ) {
        when (source) {
        is DonedataLateCompletionState.Phase -> when {
            event is DonedataLateCompletionEvent.Ready -> {
                // SCE-MAP: donedata_late_completion.scxml:67 :: phase :: _transition_0


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_late", "finish")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
