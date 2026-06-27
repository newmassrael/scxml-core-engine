// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: a5d5c62df04659924e14ff2b6c6771228646739eefc82472964b6d7b318ffce2
// generated-at: 1782568712

// GENERATED CODE — DO NOT EDIT
// Source: resources/233/test233.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test233.scxml:7

package com.sce.generated.test233

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test233State : State {
    data object Fail : Test233State
    data object Pass : Test233State
    data object S0 : Test233State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test233Event : Event {
    sealed interface Cancel : Test233Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test233Event
    sealed interface Done : Test233Event {
        data object Invoke : Done
    }
    sealed interface Error : Test233Event {
        data object Execution : Error
    }
    data object Timeout : Test233Event
}
// --- State Machine (W3C SCXML) ---

class Test233StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test233State, Test233Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test233State = Test233State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test233State? = when (stateId) {
        "fail" -> Test233State.Fail
        "pass" -> Test233State.Pass
        "s0" -> Test233State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test233State): String = when (state) {
        is Test233State.Fail -> "fail"
        is Test233State.Pass -> "pass"
        is Test233State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test233State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test233State): Int = when (state) {
        is Test233State.Fail -> 2
        is Test233State.Pass -> 1
        is Test233State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test233Event? = when (name) {
        "cancel.invoke" -> Test233Event.Cancel.Invoke
        "childToParent" -> Test233Event.ChildToParent
        "done.invoke" -> Test233Event.Done.Invoke
        "error.execution" -> Test233Event.Error.Execution
        "timeout" -> Test233Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test233Event): String? = when (event) {
        is Test233Event.Cancel.Invoke -> "cancel.invoke"
        is Test233Event.ChildToParent -> "childToParent"
        is Test233Event.Done.Invoke -> "done.invoke"
        is Test233Event.Error.Execution -> "error.execution"
        is Test233Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test233")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test233Event.Error.Execution)
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
            raiseInternal(Test233Event.Error.Execution)
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
            raiseInternal(Test233Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test233Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test233Event) {
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
        state: Test233State,
        event: Test233Event
    ): TransitionResult<Test233State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test233State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test233Event
    ): TransitionResult<Test233State> = when {
        event is Test233Event.ChildToParent && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test233State.Pass, Test233State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test233State.Fail, Test233State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test233.scxml:7
    override fun onEntry(state: Test233State) {
        when (state) {
            is Test233State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test233State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test233State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 3000L, Test233Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test233SceSynthInvokeInvoke0StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test233Event.Done.Invoke, "Var1 = _event.data.aParam;", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test233.scxml:7
    override fun onExit(state: Test233State) {
        when (state) {
            is Test233State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test233State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test233State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test233.scxml:7
    override fun executeTransitionActions(
        source: Test233State,
        event: Test233Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
