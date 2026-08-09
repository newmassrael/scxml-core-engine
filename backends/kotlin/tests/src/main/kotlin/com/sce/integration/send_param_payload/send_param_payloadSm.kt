// SCE-GENERATED — DO NOT EDIT
// source-hash: 80019160c3aa65735e97becd4bf633d4c0625505c4e9a1dfa038840895ba7e34
// template-hash: f5fde488bb26d050ed6ca4285c6964cc031a9d1311486db8d9c07efbb803316f
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/send_param_payload/send_param_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: send_param_payload.scxml:38

package com.sce.integration.send_param_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SendParamPayloadState : State {
    data object AwaitChild : SendParamPayloadState
    data object FailChildPayload : SendParamPayloadState
    data object FailInternalPayload : SendParamPayloadState
    data object InternalPhase : SendParamPayloadState
    data object Pass : SendParamPayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SendParamPayloadEvent : Event {
    sealed interface Cancel : SendParamPayloadEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : SendParamPayloadEvent {
        data object Invoke : Done
    }
    sealed interface Error : SendParamPayloadEvent {
        data object Execution : Error
    }
    data object FromChild : SendParamPayloadEvent
    data object Loopback : SendParamPayloadEvent
}
// --- State Machine (W3C SCXML) ---

class SendParamPayloadStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<SendParamPayloadState, SendParamPayloadEvent>(scriptEngine) {

    override val initialState: SendParamPayloadState = SendParamPayloadState.AwaitChild

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SendParamPayloadState? = when (stateId) {
        "awaitChild" -> SendParamPayloadState.AwaitChild
        "failChildPayload" -> SendParamPayloadState.FailChildPayload
        "failInternalPayload" -> SendParamPayloadState.FailInternalPayload
        "internalPhase" -> SendParamPayloadState.InternalPhase
        "pass" -> SendParamPayloadState.Pass
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SendParamPayloadState): String = when (state) {
        is SendParamPayloadState.AwaitChild -> "awaitChild"
        is SendParamPayloadState.FailChildPayload -> "failChildPayload"
        is SendParamPayloadState.FailInternalPayload -> "failInternalPayload"
        is SendParamPayloadState.InternalPhase -> "internalPhase"
        is SendParamPayloadState.Pass -> "pass"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SendParamPayloadState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: SendParamPayloadState): Int = when (state) {
        is SendParamPayloadState.AwaitChild -> 0
        is SendParamPayloadState.FailChildPayload -> 3
        is SendParamPayloadState.FailInternalPayload -> 4
        is SendParamPayloadState.InternalPhase -> 1
        is SendParamPayloadState.Pass -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SendParamPayloadEvent? = when (name) {
        "cancel.invoke" -> SendParamPayloadEvent.Cancel.Invoke
        "done.invoke" -> SendParamPayloadEvent.Done.Invoke
        "error.execution" -> SendParamPayloadEvent.Error.Execution
        "fromChild" -> SendParamPayloadEvent.FromChild
        "loopback" -> SendParamPayloadEvent.Loopback
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SendParamPayloadEvent): String? = when (event) {
        is SendParamPayloadEvent.Cancel.Invoke -> "cancel.invoke"
        is SendParamPayloadEvent.Done.Invoke -> "done.invoke"
        is SendParamPayloadEvent.Error.Execution -> "error.execution"
        is SendParamPayloadEvent.FromChild -> "fromChild"
        is SendParamPayloadEvent.Loopback -> "loopback"
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
            "send_param_payload",
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
            raiseInternal(SendParamPayloadEvent.Error.Execution)
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
            raiseInternal(SendParamPayloadEvent.Error.Execution)
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
            raiseInternal(SendParamPayloadEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: SendParamPayloadEvent) {
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
        val effectiveOrigin = if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
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
        state: SendParamPayloadState,
        event: SendParamPayloadEvent
    ): TransitionResult<SendParamPayloadState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is SendParamPayloadState.AwaitChild -> processAwaitChild(event)
        is SendParamPayloadState.InternalPhase -> processInternalPhase(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processAwaitChild(
        event: SendParamPayloadEvent
    ): TransitionResult<SendParamPayloadState> = when {
        event is SendParamPayloadEvent.FromChild && safeEvaluateGuard("_event.data && _event.data.value === '42'") -> TransitionResult.External(SendParamPayloadState.InternalPhase, SendParamPayloadState.AwaitChild)

        event is SendParamPayloadEvent.FromChild -> TransitionResult.External(SendParamPayloadState.FailChildPayload, SendParamPayloadState.AwaitChild)

        else -> TransitionResult.Ignored
    }

    private fun processInternalPhase(
        event: SendParamPayloadEvent
    ): TransitionResult<SendParamPayloadState> = when {
        event is SendParamPayloadEvent.Loopback && safeEvaluateGuard("_event.data && _event.data.carried === 'kept'") -> TransitionResult.External(SendParamPayloadState.Pass, SendParamPayloadState.InternalPhase)

        event is SendParamPayloadEvent.Loopback -> TransitionResult.External(SendParamPayloadState.FailInternalPayload, SendParamPayloadState.InternalPhase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_param_payload.scxml:38
    override fun onEntry(state: SendParamPayloadState) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> {
                // SCE-MAP: send_param_payload.scxml:42
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("awaitChild")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "awaitChild.${System.identityHashCode(this)}.inv_emitter"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = SendParamPayloadSceSynthInvokeInvEmitterStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_emitter", childSM, false, SendParamPayloadEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is SendParamPayloadState.FailChildPayload -> {
                // SCE-MAP: send_param_payload.scxml:80
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failChildPayload")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.FailInternalPayload -> {
                // SCE-MAP: send_param_payload.scxml:81
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failInternalPayload")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.InternalPhase -> {
                // SCE-MAP: send_param_payload.scxml:67
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("internalPhase")) return


            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                val paramsI = mutableMapOf<String, Any?>()
                paramsI["carried"] = "kept"
                raiseInternal(SendParamPayloadEvent.Loopback, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            }
            is SendParamPayloadState.Pass -> {
                // SCE-MAP: send_param_payload.scxml:79
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: send_param_payload.scxml:38
    override fun onExit(state: SendParamPayloadState) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> {
                // SCE-MAP: send_param_payload.scxml:42
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_emitter")
                activeStateIds.remove("awaitChild")
            }
            is SendParamPayloadState.FailChildPayload -> {
                // SCE-MAP: send_param_payload.scxml:80
                activeStateIds.remove("failChildPayload")
            }
            is SendParamPayloadState.FailInternalPayload -> {
                // SCE-MAP: send_param_payload.scxml:81
                activeStateIds.remove("failInternalPayload")
            }
            is SendParamPayloadState.InternalPhase -> {
                // SCE-MAP: send_param_payload.scxml:67
                activeStateIds.remove("internalPhase")
            }
            is SendParamPayloadState.Pass -> {
                // SCE-MAP: send_param_payload.scxml:79
                activeStateIds.remove("pass")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: send_param_payload.scxml:38
    override fun executeTransitionActions(
        source: SendParamPayloadState,
        event: SendParamPayloadEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
