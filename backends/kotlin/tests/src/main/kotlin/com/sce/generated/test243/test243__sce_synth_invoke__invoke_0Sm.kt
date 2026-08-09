// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 392bbcde4466dbc0cb9cb0e8b35901796c2cabcfe17ca0552a2f1bf1fe87d8de
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test243

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test243SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test243SceSynthInvokeInvoke0State
    data object SubFinal : Test243SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test243SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test243SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Failure : Test243SceSynthInvokeInvoke0Event
    data object Success : Test243SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test243SceSynthInvokeInvoke0StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test243SceSynthInvokeInvoke0State, Test243SceSynthInvokeInvoke0Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test243SceSynthInvokeInvoke0State = Test243SceSynthInvokeInvoke0State.Sub0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test243SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test243SceSynthInvokeInvoke0State.Sub0
        "subFinal" -> Test243SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test243SceSynthInvokeInvoke0State): String = when (state) {
        is Test243SceSynthInvokeInvoke0State.Sub0 -> "sub0"
        is Test243SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test243SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test243SceSynthInvokeInvoke0State): Int = when (state) {
        is Test243SceSynthInvokeInvoke0State.Sub0 -> 0
        is Test243SceSynthInvokeInvoke0State.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test243SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test243SceSynthInvokeInvoke0Event.Error.Execution
        "failure" -> Test243SceSynthInvokeInvoke0Event.Failure
        "success" -> Test243SceSynthInvokeInvoke0Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test243SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test243SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test243SceSynthInvokeInvoke0Event.Failure -> "failure"
        is Test243SceSynthInvokeInvoke0Event.Success -> "success"
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
            "test243__sce_synth_invoke__invoke_0",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test243SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test243SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test243SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test243SceSynthInvokeInvoke0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test243SceSynthInvokeInvoke0Event) {
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
        state: Test243SceSynthInvokeInvoke0State,
        event: Test243SceSynthInvokeInvoke0Event
    ): TransitionResult<Test243SceSynthInvokeInvoke0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test243SceSynthInvokeInvoke0State
    ): TransitionResult<Test243SceSynthInvokeInvoke0State> = when (state) {
        is Test243SceSynthInvokeInvoke0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test243SceSynthInvokeInvoke0State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test243SceSynthInvokeInvoke0State.SubFinal, Test243SceSynthInvokeInvoke0State.Sub0)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test243SceSynthInvokeInvoke0State.SubFinal, Test243SceSynthInvokeInvoke0State.Sub0)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test243SceSynthInvokeInvoke0State) {
        when (state) {
            is Test243SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:8
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return
            }
            is Test243SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:16
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test243SceSynthInvokeInvoke0State) {
        when (state) {
            is Test243SceSynthInvokeInvoke0State.Sub0 -> {
                // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:8
                activeStateIds.remove("sub0")
            }
            is Test243SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:16
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test243__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test243SceSynthInvokeInvoke0State,
        event: Test243SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        is Test243SceSynthInvokeInvoke0State.Sub0 -> when {
            event == null && safeEvaluateGuard("Var1 == 1") -> {


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
