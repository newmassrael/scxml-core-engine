// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 0039966e0f3716b85eeb59960e8ad41f86b7aa3caf1343b6b830b8699ccc194e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test240__sce_synth_invoke__invoke_1.scxml:3

package com.sce.generated.test240

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test240SceSynthInvokeInvoke1State : State {
    data object Sub02 : Test240SceSynthInvokeInvoke1State
    data object SubFinal2 : Test240SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test240SceSynthInvokeInvoke1Event : Event {
    sealed interface Error : Test240SceSynthInvokeInvoke1Event {
        data object Execution : Error
    }
    data object Failure : Test240SceSynthInvokeInvoke1Event
    data object Success : Test240SceSynthInvokeInvoke1Event
}
// --- State Machine (W3C SCXML) ---

class Test240SceSynthInvokeInvoke1StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test240SceSynthInvokeInvoke1State, Test240SceSynthInvokeInvoke1Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test240SceSynthInvokeInvoke1State = Test240SceSynthInvokeInvoke1State.Sub02

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test240SceSynthInvokeInvoke1State? = when (stateId) {
        "sub02" -> Test240SceSynthInvokeInvoke1State.Sub02
        "subFinal2" -> Test240SceSynthInvokeInvoke1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test240SceSynthInvokeInvoke1State): String = when (state) {
        is Test240SceSynthInvokeInvoke1State.Sub02 -> "sub02"
        is Test240SceSynthInvokeInvoke1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test240SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test240SceSynthInvokeInvoke1State): Int = when (state) {
        is Test240SceSynthInvokeInvoke1State.Sub02 -> 0
        is Test240SceSynthInvokeInvoke1State.SubFinal2 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test240SceSynthInvokeInvoke1Event? = when (name) {
        "error.execution" -> Test240SceSynthInvokeInvoke1Event.Error.Execution
        "failure" -> Test240SceSynthInvokeInvoke1Event.Failure
        "success" -> Test240SceSynthInvokeInvoke1Event.Success
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test240SceSynthInvokeInvoke1Event): String? = when (event) {
        is Test240SceSynthInvokeInvoke1Event.Error.Execution -> "error.execution"
        is Test240SceSynthInvokeInvoke1Event.Failure -> "failure"
        is Test240SceSynthInvokeInvoke1Event.Success -> "success"
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
            "test240__sce_synth_invoke__invoke_1",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test240SceSynthInvokeInvoke1Event.Error.Execution)
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
            raiseInternal(Test240SceSynthInvokeInvoke1Event.Error.Execution)
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
            raiseInternal(Test240SceSynthInvokeInvoke1Event.Error.Execution)
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
            raiseInternal(Test240SceSynthInvokeInvoke1Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test240SceSynthInvokeInvoke1Event) {
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
        state: Test240SceSynthInvokeInvoke1State,
        event: Test240SceSynthInvokeInvoke1Event
    ): TransitionResult<Test240SceSynthInvokeInvoke1State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test240SceSynthInvokeInvoke1State
    ): TransitionResult<Test240SceSynthInvokeInvoke1State> = when (state) {
        is Test240SceSynthInvokeInvoke1State.Sub02 -> processNullSub02()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub02(
    ): TransitionResult<Test240SceSynthInvokeInvoke1State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test240SceSynthInvokeInvoke1State.SubFinal2, Test240SceSynthInvokeInvoke1State.Sub02)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test240SceSynthInvokeInvoke1State.SubFinal2, Test240SceSynthInvokeInvoke1State.Sub02)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test240__sce_synth_invoke__invoke_1.scxml:3
    override fun onEntry(state: Test240SceSynthInvokeInvoke1State) {
        when (state) {
            is Test240SceSynthInvokeInvoke1State.Sub02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub02")) return
            }
            is Test240SceSynthInvokeInvoke1State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test240__sce_synth_invoke__invoke_1.scxml:3
    override fun onExit(state: Test240SceSynthInvokeInvoke1State) {
        when (state) {
            is Test240SceSynthInvokeInvoke1State.Sub02 -> {
                activeStateIds.remove("sub02")
            }
            is Test240SceSynthInvokeInvoke1State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test240__sce_synth_invoke__invoke_1.scxml:3
    override fun executeTransitionActions(
        source: Test240SceSynthInvokeInvoke1State,
        event: Test240SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        is Test240SceSynthInvokeInvoke1State.Sub02 -> when {
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
