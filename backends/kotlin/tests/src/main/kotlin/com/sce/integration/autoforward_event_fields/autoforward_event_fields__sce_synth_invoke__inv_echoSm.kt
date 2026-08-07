// SCE-GENERATED — DO NOT EDIT
// source-hash: 0dee5053a674bb8384e14f6d6265a3a1553a5a10e868880b16cae9929da099b7
// template-hash: c56c70704f5f2bb35214b9afedb17072586ef770e8f707a00c3cc931286b4b25
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_event_fields/autoforward_event_fields__sce_synth_invoke__inv_echo.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_event_fields__sce_synth_invoke__inv_echo.scxml:3

package com.sce.integration.autoforward_event_fields

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardEventFieldsSceSynthInvokeInvEchoState : State {
    data object Emit : AutoforwardEventFieldsSceSynthInvokeInvEchoState
    data object Reported : AutoforwardEventFieldsSceSynthInvokeInvEchoState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardEventFieldsSceSynthInvokeInvEchoEvent : Event {
    data object ChildToParent : AutoforwardEventFieldsSceSynthInvokeInvEchoEvent
    sealed interface Error : AutoforwardEventFieldsSceSynthInvokeInvEchoEvent {
        data object Execution : Error
    }
    data object FieldsPreserved : AutoforwardEventFieldsSceSynthInvokeInvEchoEvent
    data object FieldsStripped : AutoforwardEventFieldsSceSynthInvokeInvEchoEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardEventFieldsSceSynthInvokeInvEchoStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AutoforwardEventFieldsSceSynthInvokeInvEchoState, AutoforwardEventFieldsSceSynthInvokeInvEchoEvent>(scriptEngine) {

    override val initialState: AutoforwardEventFieldsSceSynthInvokeInvEchoState = AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardEventFieldsSceSynthInvokeInvEchoState? = when (stateId) {
        "emit" -> AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit
        "reported" -> AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardEventFieldsSceSynthInvokeInvEchoState): String = when (state) {
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> "emit"
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported -> "reported"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardEventFieldsSceSynthInvokeInvEchoState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardEventFieldsSceSynthInvokeInvEchoState): Int = when (state) {
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> 0
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardEventFieldsSceSynthInvokeInvEchoEvent? = when (name) {
        "childToParent" -> AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent
        "error.execution" -> AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.Error.Execution
        "fieldsPreserved" -> AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.FieldsPreserved
        "fieldsStripped" -> AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.FieldsStripped
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardEventFieldsSceSynthInvokeInvEchoEvent): String? = when (event) {
        is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent -> "childToParent"
        is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.Error.Execution -> "error.execution"
        is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.FieldsPreserved -> "fieldsPreserved"
        is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.FieldsStripped -> "fieldsStripped"
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
            "autoforward_event_fields__sce_synth_invoke__inv_echo",
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
            raiseInternal(AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.Error.Execution)
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
            raiseInternal(AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.Error.Execution)
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
            raiseInternal(AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AutoforwardEventFieldsSceSynthInvokeInvEchoEvent) {
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
        state: AutoforwardEventFieldsSceSynthInvokeInvEchoState,
        event: AutoforwardEventFieldsSceSynthInvokeInvEchoEvent
    ): TransitionResult<AutoforwardEventFieldsSceSynthInvokeInvEchoState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> processEmit(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processEmit(
        event: AutoforwardEventFieldsSceSynthInvokeInvEchoEvent
    ): TransitionResult<AutoforwardEventFieldsSceSynthInvokeInvEchoState> = when {
        event is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent && safeEvaluateGuard("_event.data && _event.data.value === 42                                           && _event.origin !== ''                                           && _event.invokeid !== ''") -> TransitionResult.External(AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported, AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit)

        event is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent -> TransitionResult.External(AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported, AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_event_fields__sce_synth_invoke__inv_echo.scxml:3
    override fun onEntry(state: AutoforwardEventFieldsSceSynthInvokeInvEchoState) {
        when (state) {
            is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("emit")) return


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try { paramsP["value"] = engineP.evaluateExpr(sidP, "42") } catch (_: Exception) { paramsP["value"] = "" }
                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("childToParent", eventDataP)
            }
            }
            is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("reported")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_event_fields__sce_synth_invoke__inv_echo.scxml:3
    override fun onExit(state: AutoforwardEventFieldsSceSynthInvokeInvEchoState) {
        when (state) {
            is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> {
                activeStateIds.remove("emit")
            }
            is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Reported -> {
                activeStateIds.remove("reported")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_event_fields__sce_synth_invoke__inv_echo.scxml:3
    override fun executeTransitionActions(
        source: AutoforwardEventFieldsSceSynthInvokeInvEchoState,
        event: AutoforwardEventFieldsSceSynthInvokeInvEchoEvent?
    ) {
        when (source) {
        is AutoforwardEventFieldsSceSynthInvokeInvEchoState.Emit -> when {
            event is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent && safeEvaluateGuard("_event.data && _event.data.value === 42                                           && _event.origin !== ''                                           && _event.invokeid !== ''") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("fieldsPreserved", "")
            }
            event is AutoforwardEventFieldsSceSynthInvokeInvEchoEvent.ChildToParent -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("fieldsStripped", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
