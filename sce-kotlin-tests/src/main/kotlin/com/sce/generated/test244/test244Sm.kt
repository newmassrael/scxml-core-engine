// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e8782a5c8351481fc8f6e7fcdb09caae80cbe9e47c6019dcf15afff703e3c3b3
// generated-at: 1780407549

// GENERATED CODE — DO NOT EDIT
// Source: resources/244/test244.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test244.scxml:8

package com.sce.generated.test244

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test244State : State {
    data object Fail : Test244State
    data object Pass : Test244State
    data object S0 : Test244State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test244Event : Event {
    sealed interface Cancel : Test244Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test244Event {
        data object Invoke : Done
    }
    sealed interface Error : Test244Event {
        data object Execution : Error
    }
    data object Failure : Test244Event
    data object Success : Test244Event
    data object Timeout : Test244Event
}
// --- State Machine (W3C SCXML) ---

class Test244StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test244State, Test244Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test244State = Test244State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test244State? = when (stateId) {
        "fail" -> Test244State.Fail
        "pass" -> Test244State.Pass
        "s0" -> Test244State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test244State): String = when (state) {
        is Test244State.Fail -> "fail"
        is Test244State.Pass -> "pass"
        is Test244State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test244State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test244State): Int = when (state) {
        is Test244State.Fail -> 2
        is Test244State.Pass -> 1
        is Test244State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test244Event? = when (name) {
        "cancel.invoke" -> Test244Event.Cancel.Invoke
        "done.invoke" -> Test244Event.Done.Invoke
        "error.execution" -> Test244Event.Error.Execution
        "failure" -> Test244Event.Failure
        "success" -> Test244Event.Success
        "timeout" -> Test244Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test244Event): String? = when (event) {
        is Test244Event.Cancel.Invoke -> "cancel.invoke"
        is Test244Event.Done.Invoke -> "done.invoke"
        is Test244Event.Error.Execution -> "error.execution"
        is Test244Event.Failure -> "failure"
        is Test244Event.Success -> "success"
        is Test244Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test244")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test244Event.Error.Execution)
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
            raiseInternal(Test244Event.Error.Execution)
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
            raiseInternal(Test244Event.Error.Execution)
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
            raiseInternal(Test244Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test244Event) {
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
        state: Test244State,
        event: Test244Event
    ): TransitionResult<Test244State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test244State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test244Event
    ): TransitionResult<Test244State> = when {
        event is Test244Event.Success -> TransitionResult.External(Test244State.Pass, Test244State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test244State.Fail, Test244State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test244.scxml:8
    override fun onEntry(state: Test244State) {
        when (state) {
            is Test244State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test244State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test244State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test244Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "Var1")) {
                        raiseInternal(Test244Event.Error.Execution)
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test244SceSynthInvokeInvoke0StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test244Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test244.scxml:8
    override fun onExit(state: Test244State) {
        when (state) {
            is Test244State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test244State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test244State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test244.scxml:8
    override fun executeTransitionActions(
        source: Test244State,
        event: Test244Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
