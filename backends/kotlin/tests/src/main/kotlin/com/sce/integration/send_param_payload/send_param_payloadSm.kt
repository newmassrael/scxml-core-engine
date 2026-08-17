// SCE-GENERATED — DO NOT EDIT
// source-hash: d307f64e2f440b5e64842da73b67f9e9208ee8c09e59399434add3727578f7cc
// template-hash: 039ad389d30ffb729c7c2441931b41f36924cbf4b6013115d42ef3094467532b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/send_param_payload/send_param_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: send_param_payload.scxml:62 :: _machine

package com.sce.integration.send_param_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SendParamPayloadState : State {
    data object AwaitChild : SendParamPayloadState
    data object FailChildPayload : SendParamPayloadState
    data object FailDuplicateParams : SendParamPayloadState
    data object FailInternalPayload : SendParamPayloadState
    data object FailNumberType : SendParamPayloadState
    data object InternalPhase : SendParamPayloadState
    data object Pass : SendParamPayloadState
    data object TypedPhase : SendParamPayloadState
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
    data object Typed : SendParamPayloadEvent
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
        "failDuplicateParams" -> SendParamPayloadState.FailDuplicateParams
        "failInternalPayload" -> SendParamPayloadState.FailInternalPayload
        "failNumberType" -> SendParamPayloadState.FailNumberType
        "internalPhase" -> SendParamPayloadState.InternalPhase
        "pass" -> SendParamPayloadState.Pass
        "typedPhase" -> SendParamPayloadState.TypedPhase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SendParamPayloadState): String = when (state) {
        is SendParamPayloadState.AwaitChild -> "awaitChild"
        is SendParamPayloadState.FailChildPayload -> "failChildPayload"
        is SendParamPayloadState.FailDuplicateParams -> "failDuplicateParams"
        is SendParamPayloadState.FailInternalPayload -> "failInternalPayload"
        is SendParamPayloadState.FailNumberType -> "failNumberType"
        is SendParamPayloadState.InternalPhase -> "internalPhase"
        is SendParamPayloadState.Pass -> "pass"
        is SendParamPayloadState.TypedPhase -> "typedPhase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SendParamPayloadState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: SendParamPayloadState): Int = when (state) {
        is SendParamPayloadState.AwaitChild -> 0
        is SendParamPayloadState.FailChildPayload -> 4
        is SendParamPayloadState.FailDuplicateParams -> 7
        is SendParamPayloadState.FailInternalPayload -> 5
        is SendParamPayloadState.FailNumberType -> 6
        is SendParamPayloadState.InternalPhase -> 1
        is SendParamPayloadState.Pass -> 3
        is SendParamPayloadState.TypedPhase -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SendParamPayloadEvent? = when (name) {
        "cancel.invoke" -> SendParamPayloadEvent.Cancel.Invoke
        "done.invoke" -> SendParamPayloadEvent.Done.Invoke
        "error.execution" -> SendParamPayloadEvent.Error.Execution
        "fromChild" -> SendParamPayloadEvent.FromChild
        "loopback" -> SendParamPayloadEvent.Loopback
        "typed" -> SendParamPayloadEvent.Typed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SendParamPayloadEvent): String? = when (event) {
        is SendParamPayloadEvent.Cancel.Invoke -> "cancel.invoke"
        is SendParamPayloadEvent.Done.Invoke -> "done.invoke"
        is SendParamPayloadEvent.Error.Execution -> "error.execution"
        is SendParamPayloadEvent.FromChild -> "fromChild"
        is SendParamPayloadEvent.Loopback -> "loopback"
        is SendParamPayloadEvent.Typed -> "typed"
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
            raiseInternal(SendParamPayloadEvent.Error.Execution)
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
        state: SendParamPayloadState,
        event: SendParamPayloadEvent
    ): TransitionResult<SendParamPayloadState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is SendParamPayloadState.AwaitChild -> processAwaitChild(event)
        is SendParamPayloadState.InternalPhase -> processInternalPhase(event)
        is SendParamPayloadState.TypedPhase -> processTypedPhase(event)
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
        event is SendParamPayloadEvent.Loopback && safeEvaluateGuard("_event.data && _event.data.carried === 'kept'") -> TransitionResult.External(SendParamPayloadState.TypedPhase, SendParamPayloadState.InternalPhase)

        event is SendParamPayloadEvent.Loopback -> TransitionResult.External(SendParamPayloadState.FailInternalPayload, SendParamPayloadState.InternalPhase)

        else -> TransitionResult.Ignored
    }

    private fun processTypedPhase(
        event: SendParamPayloadEvent
    ): TransitionResult<SendParamPayloadState> = when {
        event is SendParamPayloadEvent.Typed && safeEvaluateGuard("_event.data.n !== 7") -> TransitionResult.External(SendParamPayloadState.FailNumberType, SendParamPayloadState.TypedPhase)

        event is SendParamPayloadEvent.Typed && safeEvaluateGuard("_event.data.d.length === 2 && _event.data.d[0] === 1 && _event.data.d[1] === 2") -> TransitionResult.External(SendParamPayloadState.Pass, SendParamPayloadState.TypedPhase)

        event is SendParamPayloadEvent.Typed -> TransitionResult.External(SendParamPayloadState.FailDuplicateParams, SendParamPayloadState.TypedPhase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_param_payload.scxml:62 :: _machine
    override fun onEntry(state: SendParamPayloadState, pathChild: SendParamPayloadState?) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> {
                // SCE-MAP: send_param_payload.scxml:66 :: awaitChild :: _state_body
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
                // SCE-MAP: send_param_payload.scxml:123 :: failChildPayload :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failChildPayload")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.FailDuplicateParams -> {
                // SCE-MAP: send_param_payload.scxml:126 :: failDuplicateParams :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failDuplicateParams")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.FailInternalPayload -> {
                // SCE-MAP: send_param_payload.scxml:124 :: failInternalPayload :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failInternalPayload")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.FailNumberType -> {
                // SCE-MAP: send_param_payload.scxml:125 :: failNumberType :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNumberType")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.InternalPhase -> {
                // SCE-MAP: send_param_payload.scxml:91 :: internalPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("internalPhase")) return


            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                val paramsI = mutableMapOf<String, Any?>()
                putParam(paramsI, "carried", "kept")
                raiseInternal(SendParamPayloadEvent.Loopback, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            }
            is SendParamPayloadState.Pass -> {
                // SCE-MAP: send_param_payload.scxml:122 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendParamPayloadState.TypedPhase -> {
                // SCE-MAP: send_param_payload.scxml:107 :: typedPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("typedPhase")) return


            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                ensureScriptEngine()
                val engineI = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidI = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsI = mutableMapOf<String, Any?>()
                try { putParam(paramsI, "n", engineI.evaluateExpr(sidI, "7")) } catch (_: Exception) { putParam(paramsI, "n", "") }
                try { putParam(paramsI, "d", engineI.evaluateExpr(sidI, "1")) } catch (_: Exception) { putParam(paramsI, "d", "") }
                try { putParam(paramsI, "d", engineI.evaluateExpr(sidI, "2")) } catch (_: Exception) { putParam(paramsI, "d", "") }
                raiseInternal(SendParamPayloadEvent.Typed, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: send_param_payload.scxml:62 :: _machine
    override fun onExit(state: SendParamPayloadState) {
        when (state) {
            is SendParamPayloadState.AwaitChild -> {
                // SCE-MAP: send_param_payload.scxml:66 :: awaitChild :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_emitter")
                activeStateIds.remove("awaitChild")
            }
            is SendParamPayloadState.FailChildPayload -> {
                // SCE-MAP: send_param_payload.scxml:123 :: failChildPayload :: _state_body
                activeStateIds.remove("failChildPayload")
            }
            is SendParamPayloadState.FailDuplicateParams -> {
                // SCE-MAP: send_param_payload.scxml:126 :: failDuplicateParams :: _state_body
                activeStateIds.remove("failDuplicateParams")
            }
            is SendParamPayloadState.FailInternalPayload -> {
                // SCE-MAP: send_param_payload.scxml:124 :: failInternalPayload :: _state_body
                activeStateIds.remove("failInternalPayload")
            }
            is SendParamPayloadState.FailNumberType -> {
                // SCE-MAP: send_param_payload.scxml:125 :: failNumberType :: _state_body
                activeStateIds.remove("failNumberType")
            }
            is SendParamPayloadState.InternalPhase -> {
                // SCE-MAP: send_param_payload.scxml:91 :: internalPhase :: _state_body
                activeStateIds.remove("internalPhase")
            }
            is SendParamPayloadState.Pass -> {
                // SCE-MAP: send_param_payload.scxml:122 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is SendParamPayloadState.TypedPhase -> {
                // SCE-MAP: send_param_payload.scxml:107 :: typedPhase :: _state_body
                activeStateIds.remove("typedPhase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: send_param_payload.scxml:62 :: _machine
    override fun executeTransitionActions(
        source: SendParamPayloadState,
        event: SendParamPayloadEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
