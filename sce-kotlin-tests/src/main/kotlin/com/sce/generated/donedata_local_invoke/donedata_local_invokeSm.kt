
// GENERATED CODE — DO NOT EDIT
// Source: sce-kotlin-tests/src/test/resources/fixtures/donedata_local_invoke.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.donedata_local_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLocalInvokeState : State {
    data object Fail : DonedataLocalInvokeState
    data object Pass : DonedataLocalInvokeState
    data object PhaseContent : DonedataLocalInvokeState
    data object PhaseParam : DonedataLocalInvokeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLocalInvokeEvent : Event {
    sealed interface Cancel : DonedataLocalInvokeEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : DonedataLocalInvokeEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object InvContent : Invoke
            data object InvParam : Invoke
        }
    }
    sealed interface Error : DonedataLocalInvokeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class DonedataLocalInvokeStateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<DonedataLocalInvokeState, DonedataLocalInvokeEvent>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: DonedataLocalInvokeState = DonedataLocalInvokeState.PhaseParam

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): DonedataLocalInvokeState? = when (stateId) {
        "fail" -> DonedataLocalInvokeState.Fail
        "pass" -> DonedataLocalInvokeState.Pass
        "phase_content" -> DonedataLocalInvokeState.PhaseContent
        "phase_param" -> DonedataLocalInvokeState.PhaseParam
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLocalInvokeState): String = when (state) {
        is DonedataLocalInvokeState.Fail -> "fail"
        is DonedataLocalInvokeState.Pass -> "pass"
        is DonedataLocalInvokeState.PhaseContent -> "phase_content"
        is DonedataLocalInvokeState.PhaseParam -> "phase_param"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLocalInvokeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLocalInvokeState): Int = when (state) {
        is DonedataLocalInvokeState.Fail -> 3
        is DonedataLocalInvokeState.Pass -> 2
        is DonedataLocalInvokeState.PhaseContent -> 1
        is DonedataLocalInvokeState.PhaseParam -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLocalInvokeEvent? = when (name) {
        "cancel.invoke" -> DonedataLocalInvokeEvent.Cancel.Invoke
        "done.invoke" -> DonedataLocalInvokeEvent.Done.Invoke.Self
        "done.invoke.inv_content" -> DonedataLocalInvokeEvent.Done.Invoke.InvContent
        "done.invoke.inv_param" -> DonedataLocalInvokeEvent.Done.Invoke.InvParam
        "error.execution" -> DonedataLocalInvokeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLocalInvokeEvent): String? = when (event) {
        is DonedataLocalInvokeEvent.Cancel.Invoke -> "cancel.invoke"
        is DonedataLocalInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is DonedataLocalInvokeEvent.Done.Invoke.InvContent -> "done.invoke.inv_content"
        is DonedataLocalInvokeEvent.Done.Invoke.InvParam -> "done.invoke.inv_param"
        is DonedataLocalInvokeEvent.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "donedata_local_invoke")

        // W3C SCXML 5.3: Initialize variable 'param_ok' with expr
        try {
            val initResult_paramOk = engine.evaluateExpr(sid, "false")
            engine.setVariable(sid, "param_ok", initResult_paramOk)
        } catch (e: Exception) {
            raiseInternal(DonedataLocalInvokeEvent.Error.Execution)
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
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(DonedataLocalInvokeEvent.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(DonedataLocalInvokeEvent.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(DonedataLocalInvokeEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLocalInvokeEvent) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
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
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: DonedataLocalInvokeState,
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is DonedataLocalInvokeState.PhaseContent -> processPhaseContent(event)
        is DonedataLocalInvokeState.PhaseParam -> processPhaseParam(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processPhaseContent(
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> = when {
        event is DonedataLocalInvokeEvent.Done.Invoke.InvContent && safeEvaluateGuard("param_ok && _event.data === 'hello_content'") -> TransitionResult.External(DonedataLocalInvokeState.Pass, DonedataLocalInvokeState.PhaseContent)

        event is DonedataLocalInvokeEvent.Done.Invoke.InvContent -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseContent)

        event is DonedataLocalInvokeEvent.Error.Execution -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseContent)

        else -> TransitionResult.Ignored
    }

    private fun processPhaseParam(
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> = when {
        event is DonedataLocalInvokeEvent.Done.Invoke.InvParam && safeEvaluateGuard("_event.data && _event.data.result === 42") -> TransitionResult.External(DonedataLocalInvokeState.PhaseContent, DonedataLocalInvokeState.PhaseParam)

        event is DonedataLocalInvokeEvent.Done.Invoke.InvParam -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseParam)

        event is DonedataLocalInvokeEvent.Error.Execution -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseParam)

        else -> TransitionResult.Ignored
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: DonedataLocalInvokeState) {
        when (state) {
            is DonedataLocalInvokeState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLocalInvokeState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLocalInvokeState.PhaseContent -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase_content")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase_content.${System.identityHashCode(this)}.inv_content"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = DonedataLocalInvokeSceSynthInvokeInvContentStateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_content", childSM, false, DonedataLocalInvokeEvent.Done.Invoke.InvContent, "", generatedInvokeId)
                    }
                }
            }
            is DonedataLocalInvokeState.PhaseParam -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase_param")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase_param.${System.identityHashCode(this)}.inv_param"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = DonedataLocalInvokeSceSynthInvokeInvParamStateMachine(scriptEngine)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_param", childSM, false, DonedataLocalInvokeEvent.Done.Invoke.InvParam, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: DonedataLocalInvokeState) {
        when (state) {
            is DonedataLocalInvokeState.Fail -> {
                activeStateIds.remove("fail")
            }
            is DonedataLocalInvokeState.Pass -> {
                activeStateIds.remove("pass")
            }
            is DonedataLocalInvokeState.PhaseContent -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_content")
                activeStateIds.remove("phase_content")
            }
            is DonedataLocalInvokeState.PhaseParam -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_param")
                activeStateIds.remove("phase_param")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: DonedataLocalInvokeState,
        event: DonedataLocalInvokeEvent?
    ) {
        when (source) {
        is DonedataLocalInvokeState.PhaseParam -> when {
            event is DonedataLocalInvokeEvent.Done.Invoke.InvParam && safeEvaluateGuard("_event.data && _event.data.result === 42") -> {


            executeAssign("param_ok", "true")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
