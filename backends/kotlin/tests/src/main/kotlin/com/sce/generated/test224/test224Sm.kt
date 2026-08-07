// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 3f00b6ad29c2eff5bb5558a6167abdac4572045d11f8d695901879b002032c6b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/224/test224.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test224.scxml:5

package com.sce.generated.test224

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test224State : State {
    data object Fail : Test224State
    data object Pass : Test224State
    data object S0 : Test224State
    data object S1 : Test224State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test224Event : Event {
    sealed interface Cancel : Test224Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test224Event {
        data object Invoke : Done
    }
    sealed interface Error : Test224Event {
        data object Execution : Error
    }
    data object Timeout : Test224Event
}
// --- State Machine (W3C SCXML) ---

class Test224StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test224State, Test224Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test224State = Test224State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test224State? = when (stateId) {
        "fail" -> Test224State.Fail
        "pass" -> Test224State.Pass
        "s0" -> Test224State.S0
        "s1" -> Test224State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test224State): String = when (state) {
        is Test224State.Fail -> "fail"
        is Test224State.Pass -> "pass"
        is Test224State.S0 -> "s0"
        is Test224State.S1 -> "s1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test224State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test224State): Int = when (state) {
        is Test224State.Fail -> 3
        is Test224State.Pass -> 2
        is Test224State.S0 -> 0
        is Test224State.S1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test224Event? = when (name) {
        "cancel.invoke" -> Test224Event.Cancel.Invoke
        "done.invoke" -> Test224Event.Done.Invoke
        "error.execution" -> Test224Event.Error.Execution
        "timeout" -> Test224Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test224Event): String? = when (event) {
        is Test224Event.Cancel.Invoke -> "cancel.invoke"
        is Test224Event.Done.Invoke -> "done.invoke"
        is Test224Event.Error.Execution -> "error.execution"
        is Test224Event.Timeout -> "timeout"
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
            "test224",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "'s0.'")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test224Event.Error.Execution)
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
            raiseInternal(Test224Event.Error.Execution)
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
            raiseInternal(Test224Event.Error.Execution)
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
            raiseInternal(Test224Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test224Event) {
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
        state: Test224State,
        event: Test224Event
    ): TransitionResult<Test224State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test224State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test224State
    ): TransitionResult<Test224State> = when (state) {
        is Test224State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test224State> = when {
        safeEvaluateGuard("Var1.indexOf(Var2) === 0") -> TransitionResult.External(Test224State.Pass, Test224State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test224State.Fail, Test224State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test224Event
    ): TransitionResult<Test224State> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test224State.S1, Test224State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test224.scxml:5
    override fun onEntry(state: Test224State) {
        when (state) {
            is Test224State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test224State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test224State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 1000L, Test224Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4.1: Store generated invokeId in parent datamodel via idlocation
                    ensureScriptEngine()
                    scriptEngine?.let { eng ->
                        scriptSessionId?.let { sid ->
                            eng.setVariable(sid, "Var1", generatedInvokeId)
                        }
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test224SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test224Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test224State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test224.scxml:5
    override fun onExit(state: Test224State) {
        when (state) {
            is Test224State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test224State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test224State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
            is Test224State.S1 -> {
                activeStateIds.remove("s1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test224.scxml:5
    override fun executeTransitionActions(
        source: Test224State,
        event: Test224Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
