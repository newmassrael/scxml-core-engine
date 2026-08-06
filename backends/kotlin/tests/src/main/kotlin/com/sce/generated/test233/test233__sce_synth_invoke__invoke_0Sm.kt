// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2eaf0bfe80897ae515b0d732a8bb3914baa7c870ee8dd206a0a3dbc4956501d1
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test233__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test233

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test233SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test233SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test233SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test233SceSynthInvokeInvoke0Event
    sealed interface Error : Test233SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test233SceSynthInvokeInvoke0StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test233SceSynthInvokeInvoke0State, Test233SceSynthInvokeInvoke0Event>(scriptEngine) {

    override val initialState: Test233SceSynthInvokeInvoke0State = Test233SceSynthInvokeInvoke0State.SubFinal

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test233SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test233SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test233SceSynthInvokeInvoke0State): String = when (state) {
        is Test233SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test233SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test233SceSynthInvokeInvoke0State): Int = when (state) {
        is Test233SceSynthInvokeInvoke0State.SubFinal -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test233SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test233SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test233SceSynthInvokeInvoke0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test233SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test233SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test233SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
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
            "test233__sce_synth_invoke__invoke_0",
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
            raiseInternal(Test233SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test233SceSynthInvokeInvoke0Event.Error.Execution)
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
            raiseInternal(Test233SceSynthInvokeInvoke0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test233SceSynthInvokeInvoke0Event) {
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
        state: Test233SceSynthInvokeInvoke0State,
        event: Test233SceSynthInvokeInvoke0Event
    ): TransitionResult<Test233SceSynthInvokeInvoke0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test233__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test233SceSynthInvokeInvoke0State) {
        when (state) {
            is Test233SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return


            // W3C SCXML 5.10: Evaluate params for parent send (test233)
            run {
                ensureScriptEngine()
                val engineP = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidP = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsP = mutableMapOf<String, Any?>()
                try { paramsP["aParam"] = engineP.evaluateExpr(sidP, "2") } catch (_: Exception) { paramsP["aParam"] = "" }
                val eventDataP = buildJsonFromParams(paramsP)
                onSendToParent?.invoke("childToParent", eventDataP)
            }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test233__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test233SceSynthInvokeInvoke0State) {
        when (state) {
            is Test233SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test233__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test233SceSynthInvokeInvoke0State,
        event: Test233SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
