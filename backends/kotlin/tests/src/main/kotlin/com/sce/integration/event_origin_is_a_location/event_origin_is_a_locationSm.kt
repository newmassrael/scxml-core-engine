// SCE-GENERATED — DO NOT EDIT
// source-hash: c56e8b2e82b26aafed117bfaa06905c41b2c8e5d207725d3f84b7293eb1eb4ee
// template-hash: 2531476627eb1f2b85917395efe91d1b55da71c6abf9c48b9fabdfd63b215bfa
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine

package com.sce.integration.event_origin_is_a_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventOriginIsALocationState : State {
    data object AwaitReply : EventOriginIsALocationState
    data object Fail : EventOriginIsALocationState
    data object Pass : EventOriginIsALocationState
    data object Phase : EventOriginIsALocationState
    data object Waiting : EventOriginIsALocationState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventOriginIsALocationEvent : Event {
    sealed interface Cancel : EventOriginIsALocationEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : EventOriginIsALocationEvent {
        data object Invoke : Done
    }
    sealed interface Error : EventOriginIsALocationEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object FromChild : EventOriginIsALocationEvent
    data object Reply : EventOriginIsALocationEvent
    data object ReplyArrived : EventOriginIsALocationEvent
}
// --- State Machine (W3C SCXML) ---

class EventOriginIsALocationStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventOriginIsALocationState, EventOriginIsALocationEvent>(scriptEngine) {

    override val initialState: EventOriginIsALocationState = EventOriginIsALocationState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: EventOriginIsALocationState): EventOriginIsALocationState? = when (state) {
        is EventOriginIsALocationState.AwaitReply -> EventOriginIsALocationState.Phase
        is EventOriginIsALocationState.Waiting -> EventOriginIsALocationState.Phase
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: EventOriginIsALocationState): EventOriginIsALocationState = when (state) {
        is EventOriginIsALocationState.Phase -> EventOriginIsALocationState.Waiting
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EventOriginIsALocationState? = when (stateId) {
        "await_reply" -> EventOriginIsALocationState.AwaitReply
        "fail" -> EventOriginIsALocationState.Fail
        "pass" -> EventOriginIsALocationState.Pass
        "phase" -> EventOriginIsALocationState.Phase
        "waiting" -> EventOriginIsALocationState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventOriginIsALocationState): String = when (state) {
        is EventOriginIsALocationState.AwaitReply -> "await_reply"
        is EventOriginIsALocationState.Fail -> "fail"
        is EventOriginIsALocationState.Pass -> "pass"
        is EventOriginIsALocationState.Phase -> "phase"
        is EventOriginIsALocationState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: EventOriginIsALocationState): Boolean = when (state) {
        is EventOriginIsALocationState.Phase -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: EventOriginIsALocationState): Int = when (state) {
        is EventOriginIsALocationState.AwaitReply -> 2
        is EventOriginIsALocationState.Fail -> 4
        is EventOriginIsALocationState.Pass -> 3
        is EventOriginIsALocationState.Phase -> 0
        is EventOriginIsALocationState.Waiting -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventOriginIsALocationEvent? = when (name) {
        "cancel.invoke" -> EventOriginIsALocationEvent.Cancel.Invoke
        "done.invoke" -> EventOriginIsALocationEvent.Done.Invoke
        "error.communication" -> EventOriginIsALocationEvent.Error.Communication
        "error.execution" -> EventOriginIsALocationEvent.Error.Execution
        "fromChild" -> EventOriginIsALocationEvent.FromChild
        "reply" -> EventOriginIsALocationEvent.Reply
        "replyArrived" -> EventOriginIsALocationEvent.ReplyArrived
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventOriginIsALocationEvent): String? = when (event) {
        is EventOriginIsALocationEvent.Cancel.Invoke -> "cancel.invoke"
        is EventOriginIsALocationEvent.Done.Invoke -> "done.invoke"
        is EventOriginIsALocationEvent.Error.Communication -> "error.communication"
        is EventOriginIsALocationEvent.Error.Execution -> "error.execution"
        is EventOriginIsALocationEvent.FromChild -> "fromChild"
        is EventOriginIsALocationEvent.Reply -> "reply"
        is EventOriginIsALocationEvent.ReplyArrived -> "replyArrived"
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
            "event_origin_is_a_location",
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventOriginIsALocationEvent) {
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
        state: EventOriginIsALocationState,
        event: EventOriginIsALocationEvent
    ): TransitionResult<EventOriginIsALocationState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is EventOriginIsALocationState.AwaitReply -> processAwaitReply(event)
        is EventOriginIsALocationState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processAwaitReply(
        event: EventOriginIsALocationEvent
    ): TransitionResult<EventOriginIsALocationState> = when {
        event is EventOriginIsALocationEvent.ReplyArrived -> TransitionResult.External(EventOriginIsALocationState.Pass, EventOriginIsALocationState.AwaitReply)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: EventOriginIsALocationEvent
    ): TransitionResult<EventOriginIsALocationState> = when {
        event is EventOriginIsALocationEvent.FromChild && safeEvaluateGuard("_event.origin == _event.data.myLocation") -> TransitionResult.External(EventOriginIsALocationState.AwaitReply, EventOriginIsALocationState.Waiting)

        event is EventOriginIsALocationEvent.FromChild -> TransitionResult.External(EventOriginIsALocationState.Fail, EventOriginIsALocationState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun onEntry(state: EventOriginIsALocationState, pathChild: EventOriginIsALocationState?) {
        when (state) {
            is EventOriginIsALocationState.AwaitReply -> {
                // SCE-MAP: event_origin_is_a_location.scxml:83 :: await_reply :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("await_reply")) return
            }
            is EventOriginIsALocationState.Fail -> {
                // SCE-MAP: event_origin_is_a_location.scxml:89 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventOriginIsALocationState.Pass -> {
                // SCE-MAP: event_origin_is_a_location.scxml:88 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventOriginIsALocationState.Phase -> {
                // SCE-MAP: event_origin_is_a_location.scxml:49 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_peer"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = EventOriginIsALocationSceSynthInvokeInvPeerStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_peer", childSM, false, EventOriginIsALocationEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is EventOriginIsALocationState.Waiting -> {
                // SCE-MAP: event_origin_is_a_location.scxml:74 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun onExit(state: EventOriginIsALocationState) {
        when (state) {
            is EventOriginIsALocationState.AwaitReply -> {
                // SCE-MAP: event_origin_is_a_location.scxml:83 :: await_reply :: _state_body
                activeStateIds.remove("await_reply")
            }
            is EventOriginIsALocationState.Fail -> {
                // SCE-MAP: event_origin_is_a_location.scxml:89 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is EventOriginIsALocationState.Pass -> {
                // SCE-MAP: event_origin_is_a_location.scxml:88 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is EventOriginIsALocationState.Phase -> {
                // SCE-MAP: event_origin_is_a_location.scxml:49 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_peer")
                activeStateIds.remove("phase")
            }
            is EventOriginIsALocationState.Waiting -> {
                // SCE-MAP: event_origin_is_a_location.scxml:74 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun executeTransitionActions(
        source: EventOriginIsALocationState,
        event: EventOriginIsALocationEvent?
    ) {
        when (source) {
        is EventOriginIsALocationState.Waiting -> when {
            event is EventOriginIsALocationEvent.FromChild && safeEvaluateGuard("_event.origin == _event.data.myLocation") -> {
                // SCE-MAP: event_origin_is_a_location.scxml:75 :: waiting :: _transition_0


            // W3C SCXML 6.2: Resolve dynamic target (targetexpr="_event.origin")
            var _resolvedTarget: String? = null
            run resolveTarget@{
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = eng.evaluateExpr(sid, "_event.origin")
                    val target = v?.toString() ?: ""
                    // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                    if (target.startsWith("!")) {
                        raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raisePlatformError(EventOriginIsALocationEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach")
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send> targetexpr failed to evaluate")
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML 6.2: Dispatch to dynamically resolved target (C++ unified pattern)
            if (_rt == "#_internal") {
                raiseInternal(EventOriginIsALocationEvent.Reply)
            } else if (_rt == "#_parent") {
                onSendToParent?.invoke("reply", "")
            } else if (deliverToChildSession(
                    com.sce.runtime.IoProcessors.sessionIdFromScxmlLocation(_rt),
                    "reply")) {
                // W3C SCXML C.1: see the payload-carrying arm above — a target
                // that decodes to a child's published location is addressed to
                // that child, not to this machine's own external queue.
            } else {
                send(EventOriginIsALocationEvent.Reply, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            } // end of _resolvedTarget?.let
            }
            else -> {}
        }
        else -> {}
        }
    }
}
