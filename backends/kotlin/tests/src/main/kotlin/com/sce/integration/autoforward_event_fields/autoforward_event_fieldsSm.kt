// SCE-GENERATED — DO NOT EDIT
// source-hash: 0dee5053a674bb8384e14f6d6265a3a1553a5a10e868880b16cae9929da099b7
// template-hash: 1cfb591080ee0f7028d74f99302d8ee6d7a5b2416447e2ddc2e71e093c1a3c98
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_event_fields/autoforward_event_fields.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_event_fields.scxml:30

package com.sce.integration.autoforward_event_fields

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardEventFieldsState : State {
    data object Fail : AutoforwardEventFieldsState
    data object Pass : AutoforwardEventFieldsState
    data object Phase : AutoforwardEventFieldsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardEventFieldsEvent : Event {
    sealed interface Cancel : AutoforwardEventFieldsEvent {
        data object Invoke : Cancel
    }
    data object ChildToParent : AutoforwardEventFieldsEvent
    sealed interface Done : AutoforwardEventFieldsEvent {
        data object Invoke : Done
    }
    sealed interface Error : AutoforwardEventFieldsEvent {
        data object Execution : Error
    }
    data object FieldsPreserved : AutoforwardEventFieldsEvent
    data object FieldsStripped : AutoforwardEventFieldsEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardEventFieldsStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AutoforwardEventFieldsState, AutoforwardEventFieldsEvent>(scriptEngine) {

    override val initialState: AutoforwardEventFieldsState = AutoforwardEventFieldsState.Phase

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardEventFieldsState? = when (stateId) {
        "fail" -> AutoforwardEventFieldsState.Fail
        "pass" -> AutoforwardEventFieldsState.Pass
        "phase" -> AutoforwardEventFieldsState.Phase
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardEventFieldsState): String = when (state) {
        is AutoforwardEventFieldsState.Fail -> "fail"
        is AutoforwardEventFieldsState.Pass -> "pass"
        is AutoforwardEventFieldsState.Phase -> "phase"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardEventFieldsState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardEventFieldsState): Int = when (state) {
        is AutoforwardEventFieldsState.Fail -> 2
        is AutoforwardEventFieldsState.Pass -> 1
        is AutoforwardEventFieldsState.Phase -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardEventFieldsEvent? = when (name) {
        "cancel.invoke" -> AutoforwardEventFieldsEvent.Cancel.Invoke
        "childToParent" -> AutoforwardEventFieldsEvent.ChildToParent
        "done.invoke" -> AutoforwardEventFieldsEvent.Done.Invoke
        "error.execution" -> AutoforwardEventFieldsEvent.Error.Execution
        "fieldsPreserved" -> AutoforwardEventFieldsEvent.FieldsPreserved
        "fieldsStripped" -> AutoforwardEventFieldsEvent.FieldsStripped
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardEventFieldsEvent): String? = when (event) {
        is AutoforwardEventFieldsEvent.Cancel.Invoke -> "cancel.invoke"
        is AutoforwardEventFieldsEvent.ChildToParent -> "childToParent"
        is AutoforwardEventFieldsEvent.Done.Invoke -> "done.invoke"
        is AutoforwardEventFieldsEvent.Error.Execution -> "error.execution"
        is AutoforwardEventFieldsEvent.FieldsPreserved -> "fieldsPreserved"
        is AutoforwardEventFieldsEvent.FieldsStripped -> "fieldsStripped"
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
            "autoforward_event_fields",
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
            raiseInternal(AutoforwardEventFieldsEvent.Error.Execution)
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
            raiseInternal(AutoforwardEventFieldsEvent.Error.Execution)
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
            raiseInternal(AutoforwardEventFieldsEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AutoforwardEventFieldsEvent) {
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
        state: AutoforwardEventFieldsState,
        event: AutoforwardEventFieldsEvent
    ): TransitionResult<AutoforwardEventFieldsState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is AutoforwardEventFieldsState.Phase -> processPhase(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: AutoforwardEventFieldsEvent
    ): TransitionResult<AutoforwardEventFieldsState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is AutoforwardEventFieldsEvent.ChildToParent -> TransitionResult.Internal
        event is AutoforwardEventFieldsEvent.FieldsPreserved -> TransitionResult.External(AutoforwardEventFieldsState.Pass, AutoforwardEventFieldsState.Phase)

        event is AutoforwardEventFieldsEvent.FieldsStripped -> TransitionResult.External(AutoforwardEventFieldsState.Fail, AutoforwardEventFieldsState.Phase)

        event is AutoforwardEventFieldsEvent.Error.Execution -> TransitionResult.External(AutoforwardEventFieldsState.Fail, AutoforwardEventFieldsState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_event_fields.scxml:30
    override fun onEntry(state: AutoforwardEventFieldsState) {
        when (state) {
            is AutoforwardEventFieldsState.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardEventFieldsState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardEventFieldsState.Phase -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_echo"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = AutoforwardEventFieldsSceSynthInvokeInvEchoStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_echo", childSM, true, AutoforwardEventFieldsEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_event_fields.scxml:30
    override fun onExit(state: AutoforwardEventFieldsState) {
        when (state) {
            is AutoforwardEventFieldsState.Fail -> {
                activeStateIds.remove("fail")
            }
            is AutoforwardEventFieldsState.Pass -> {
                activeStateIds.remove("pass")
            }
            is AutoforwardEventFieldsState.Phase -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_echo")
                activeStateIds.remove("phase")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_event_fields.scxml:30
    override fun executeTransitionActions(
        source: AutoforwardEventFieldsState,
        event: AutoforwardEventFieldsEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
