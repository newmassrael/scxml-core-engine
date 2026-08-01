// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ab200b8eb821f02e246ff33a9f9da5a6f5493996f3df460e1a87cc5891e5b49d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/422/test422.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422.scxml:10

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422State : State {
    data object Fail : Test422State
    data object Pass : Test422State
    data object S1 : Test422State
    data object S11 : Test422State
    data object S12 : Test422State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422Event : Event {
    sealed interface Cancel : Test422Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test422Event {
        data object Invoke : Done
    }
    sealed interface Error : Test422Event {
        data object Execution : Error
    }
    data object InvokeS1 : Test422Event
    data object InvokeS11 : Test422Event
    data object InvokeS12 : Test422Event
    data object Timeout : Test422Event
}
// --- State Machine (W3C SCXML) ---

class Test422StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test422State, Test422Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test422State = Test422State.S11

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test422State): Test422State? = when (state) {
        is Test422State.S11 -> Test422State.S1
        is Test422State.S12 -> Test422State.S1
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test422State): Test422State = when (state) {
        is Test422State.S1 -> Test422State.S11
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422State? = when (stateId) {
        "fail" -> Test422State.Fail
        "pass" -> Test422State.Pass
        "s1" -> Test422State.S1
        "s11" -> Test422State.S11
        "s12" -> Test422State.S12
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422State): String = when (state) {
        is Test422State.Fail -> "fail"
        is Test422State.Pass -> "pass"
        is Test422State.S1 -> "s1"
        is Test422State.S11 -> "s11"
        is Test422State.S12 -> "s12"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422State): Boolean = when (state) {
        is Test422State.S1 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422State): Int = when (state) {
        is Test422State.Fail -> 4
        is Test422State.Pass -> 3
        is Test422State.S1 -> 0
        is Test422State.S11 -> 1
        is Test422State.S12 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422Event? = when (name) {
        "cancel.invoke" -> Test422Event.Cancel.Invoke
        "done.invoke" -> Test422Event.Done.Invoke
        "error.execution" -> Test422Event.Error.Execution
        "invokeS1" -> Test422Event.InvokeS1
        "invokeS11" -> Test422Event.InvokeS11
        "invokeS12" -> Test422Event.InvokeS12
        "timeout" -> Test422Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422Event): String? = when (event) {
        is Test422Event.Cancel.Invoke -> "cancel.invoke"
        is Test422Event.Done.Invoke -> "done.invoke"
        is Test422Event.Error.Execution -> "error.execution"
        is Test422Event.InvokeS1 -> "invokeS1"
        is Test422Event.InvokeS11 -> "invokeS11"
        is Test422Event.InvokeS12 -> "invokeS12"
        is Test422Event.Timeout -> "timeout"
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
            "test422",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test422Event.Error.Execution)
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
            raiseInternal(Test422Event.Error.Execution)
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
            raiseInternal(Test422Event.Error.Execution)
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
            raiseInternal(Test422Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test422Event) {
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
        state: Test422State,
        event: Test422Event
    ): TransitionResult<Test422State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test422State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test422State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s12 has no own event transitions)
        is Test422State.S12 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422State
    ): TransitionResult<Test422State> = when (state) {
        is Test422State.S11 -> processNullS11()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS11(
    ): TransitionResult<Test422State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422State.S12, Test422State.S11)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test422Event
    ): TransitionResult<Test422State> = when {
        // W3C SCXML 3.12.1: Multi-descriptor targetless "invokeS1 invokeS12"
        (event is Test422Event.InvokeS1 || event is Test422Event.InvokeS12) -> TransitionResult.Internal
        event is Test422Event.InvokeS11 -> TransitionResult.External(Test422State.Fail, Test422State.S1)

        event is Test422Event.Timeout && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test422State.Pass, Test422State.S1)

        event is Test422Event.Timeout -> TransitionResult.External(Test422State.Fail, Test422State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422.scxml:10
    override fun onEntry(state: Test422State) {
        when (state) {
            is Test422State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test422State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test422State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            scheduleSend("__send_0", 2000L, Test422Event.Timeout)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s1.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test422SceSynthInvokeInvoke0StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test422State.S11 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s11")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s11.${System.identityHashCode(this)}._invoke_1"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test422SceSynthInvokeInvoke1StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test422State.S12 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s12")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s12.${System.identityHashCode(this)}._invoke_2"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test422SceSynthInvokeInvoke2StateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_2", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422.scxml:10
    override fun onExit(state: Test422State) {
        when (state) {
            is Test422State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test422State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test422State.S1 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s1")
            }
            is Test422State.S11 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s11")
            }
            is Test422State.S12 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
                activeStateIds.remove("s12")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test422.scxml:10
    override fun executeTransitionActions(
        source: Test422State,
        event: Test422Event?
    ) {
        when (source) {
        is Test422State.S1 -> when {
            (event is Test422Event.InvokeS1 || event is Test422Event.InvokeS12) -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test422State.S11 -> when {
            (event is Test422Event.InvokeS1 || event is Test422Event.InvokeS12) -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        is Test422State.S12 -> when {
            (event is Test422Event.InvokeS1 || event is Test422Event.InvokeS12) -> {


            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
