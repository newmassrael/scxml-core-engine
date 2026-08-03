// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 1648c68c7039bcd2d9f4b6a29e08b82f1fcf3cd79ecb3462ff4016858820460c
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test253__sce_synth_invoke__foo.scxml:3

package com.sce.generated.test253

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test253SceSynthInvokeFooState : State {
    data object Sub0 : Test253SceSynthInvokeFooState
    data object Sub1 : Test253SceSynthInvokeFooState
    data object SubFinal : Test253SceSynthInvokeFooState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test253SceSynthInvokeFooEvent : Event {
    data object ChildRunning : Test253SceSynthInvokeFooEvent
    sealed interface Error : Test253SceSynthInvokeFooEvent {
        data object Execution : Error
    }
    data object Failure : Test253SceSynthInvokeFooEvent
    data object ParentToChild : Test253SceSynthInvokeFooEvent
    data object Success : Test253SceSynthInvokeFooEvent
}
// --- State Machine (W3C SCXML) ---

class Test253SceSynthInvokeFooStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test253SceSynthInvokeFooState, Test253SceSynthInvokeFooEvent>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test253SceSynthInvokeFooState = Test253SceSynthInvokeFooState.Sub0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test253SceSynthInvokeFooState? = when (stateId) {
        "sub0" -> Test253SceSynthInvokeFooState.Sub0
        "sub1" -> Test253SceSynthInvokeFooState.Sub1
        "subFinal" -> Test253SceSynthInvokeFooState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test253SceSynthInvokeFooState): String = when (state) {
        is Test253SceSynthInvokeFooState.Sub0 -> "sub0"
        is Test253SceSynthInvokeFooState.Sub1 -> "sub1"
        is Test253SceSynthInvokeFooState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test253SceSynthInvokeFooState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test253SceSynthInvokeFooState): Int = when (state) {
        is Test253SceSynthInvokeFooState.Sub0 -> 0
        is Test253SceSynthInvokeFooState.Sub1 -> 1
        is Test253SceSynthInvokeFooState.SubFinal -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test253SceSynthInvokeFooEvent? = when (name) {
        "childRunning" -> Test253SceSynthInvokeFooEvent.ChildRunning
        "error.execution" -> Test253SceSynthInvokeFooEvent.Error.Execution
        "failure" -> Test253SceSynthInvokeFooEvent.Failure
        "parentToChild" -> Test253SceSynthInvokeFooEvent.ParentToChild
        "success" -> Test253SceSynthInvokeFooEvent.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test253SceSynthInvokeFooEvent): String? = when (event) {
        is Test253SceSynthInvokeFooEvent.ChildRunning -> "childRunning"
        is Test253SceSynthInvokeFooEvent.Error.Execution -> "error.execution"
        is Test253SceSynthInvokeFooEvent.Failure -> "failure"
        is Test253SceSynthInvokeFooEvent.ParentToChild -> "parentToChild"
        is Test253SceSynthInvokeFooEvent.Success -> "success"
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
            "test253__sce_synth_invoke__foo",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}




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
            raiseInternal(Test253SceSynthInvokeFooEvent.Error.Execution)
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
            raiseInternal(Test253SceSynthInvokeFooEvent.Error.Execution)
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
            raiseInternal(Test253SceSynthInvokeFooEvent.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test253SceSynthInvokeFooEvent) {
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
        state: Test253SceSynthInvokeFooState,
        event: Test253SceSynthInvokeFooEvent
    ): TransitionResult<Test253SceSynthInvokeFooState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test253SceSynthInvokeFooState.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test253SceSynthInvokeFooState
    ): TransitionResult<Test253SceSynthInvokeFooState> = when (state) {
        is Test253SceSynthInvokeFooState.Sub1 -> processNullSub1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub1(
    ): TransitionResult<Test253SceSynthInvokeFooState> = when {
        safeEvaluateGuard("Var2 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> TransitionResult.External(Test253SceSynthInvokeFooState.SubFinal, Test253SceSynthInvokeFooState.Sub1)
        safeEvaluateGuard("Var2 == 'scxml'") -> TransitionResult.External(Test253SceSynthInvokeFooState.SubFinal, Test253SceSynthInvokeFooState.Sub1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test253SceSynthInvokeFooState.SubFinal, Test253SceSynthInvokeFooState.Sub1)
    }

    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test253SceSynthInvokeFooEvent
    ): TransitionResult<Test253SceSynthInvokeFooState> = when {
        event is Test253SceSynthInvokeFooEvent.ParentToChild -> TransitionResult.External(Test253SceSynthInvokeFooState.Sub1, Test253SceSynthInvokeFooState.Sub0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test253__sce_synth_invoke__foo.scxml:3
    override fun onEntry(state: Test253SceSynthInvokeFooState) {
        when (state) {
            is Test253SceSynthInvokeFooState.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childRunning", "")
            }
            is Test253SceSynthInvokeFooState.Sub1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub1")) return
            }
            is Test253SceSynthInvokeFooState.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test253__sce_synth_invoke__foo.scxml:3
    override fun onExit(state: Test253SceSynthInvokeFooState) {
        when (state) {
            is Test253SceSynthInvokeFooState.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test253SceSynthInvokeFooState.Sub1 -> {
                activeStateIds.remove("sub1")
            }
            is Test253SceSynthInvokeFooState.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test253__sce_synth_invoke__foo.scxml:3
    override fun executeTransitionActions(
        source: Test253SceSynthInvokeFooState,
        event: Test253SceSynthInvokeFooEvent?
    ) {
        when (source) {
        is Test253SceSynthInvokeFooState.Sub0 -> when {
            event is Test253SceSynthInvokeFooEvent.ParentToChild -> {


            executeAssign("Var2", "_event.origintype")
            }
            else -> {}
        }
        is Test253SceSynthInvokeFooState.Sub1 -> when {
            event == null && safeEvaluateGuard("Var2 == 'http://www.w3.org/TR/scxml/#SCXMLEventProcessor'") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null && safeEvaluateGuard("Var2 == 'scxml'") -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("success", "")
            }
            event == null -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("failure", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
