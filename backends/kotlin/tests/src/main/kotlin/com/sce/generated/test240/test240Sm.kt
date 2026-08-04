// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4c41e247b67cf81983b818aeaf91bedaefbcf466fafdc8dd55875b0211b1bb15
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/240/test240.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test240.scxml:8

package com.sce.generated.test240

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test240State : State {
    data object Fail : Test240State
    data object Pass : Test240State
    data object S0 : Test240State
    data object S01 : Test240State
    data object S02 : Test240State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test240Event : Event {
    sealed interface Cancel : Test240Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test240Event {
        data object Invoke : Done
    }
    sealed interface Error : Test240Event {
        data object Execution : Error
    }
    data object Failure : Test240Event
    data object Success : Test240Event
    data object Timeout : Test240Event
}
// --- State Machine (W3C SCXML) ---

class Test240StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test240State, Test240Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test240State = Test240State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test240State): Test240State? = when (state) {
        is Test240State.S01 -> Test240State.S0
        is Test240State.S02 -> Test240State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test240State): Test240State = when (state) {
        is Test240State.S0 -> Test240State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test240State? = when (stateId) {
        "fail" -> Test240State.Fail
        "pass" -> Test240State.Pass
        "s0" -> Test240State.S0
        "s01" -> Test240State.S01
        "s02" -> Test240State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test240State): String = when (state) {
        is Test240State.Fail -> "fail"
        is Test240State.Pass -> "pass"
        is Test240State.S0 -> "s0"
        is Test240State.S01 -> "s01"
        is Test240State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test240State): Boolean = when (state) {
        is Test240State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test240State): Int = when (state) {
        is Test240State.Fail -> 4
        is Test240State.Pass -> 3
        is Test240State.S0 -> 0
        is Test240State.S01 -> 1
        is Test240State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test240Event? = when (name) {
        "cancel.invoke" -> Test240Event.Cancel.Invoke
        "done.invoke" -> Test240Event.Done.Invoke
        "error.execution" -> Test240Event.Error.Execution
        "failure" -> Test240Event.Failure
        "success" -> Test240Event.Success
        "timeout" -> Test240Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test240Event): String? = when (event) {
        is Test240Event.Cancel.Invoke -> "cancel.invoke"
        is Test240Event.Done.Invoke -> "done.invoke"
        is Test240Event.Error.Execution -> "error.execution"
        is Test240Event.Failure -> "failure"
        is Test240Event.Success -> "success"
        is Test240Event.Timeout -> "timeout"
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
            "test240",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test240Event.Error.Execution)
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
            raiseInternal(Test240Event.Error.Execution)
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
            raiseInternal(Test240Event.Error.Execution)
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
            raiseInternal(Test240Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test240Event) {
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
        state: Test240State,
        event: Test240Event
    ): TransitionResult<Test240State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test240State.S0 -> processS0(event)
        is Test240State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test240State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Timeout -> TransitionResult.External(Test240State.Fail, Test240State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Success -> TransitionResult.External(Test240State.S02, Test240State.S01)

        event is Test240Event.Failure -> TransitionResult.External(Test240State.Fail, Test240State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Success -> TransitionResult.External(Test240State.Pass, Test240State.S02)

        event is Test240Event.Failure -> TransitionResult.External(Test240State.Fail, Test240State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test240.scxml:8
    override fun onEntry(state: Test240State) {
        when (state) {
            is Test240State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test240State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test240State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test240Event.Timeout)
            }
            is Test240State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "Var1")) {
                        raiseInternal(Test240Event.Error.Execution)
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test240SceSynthInvokeInvoke0StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test240Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test240State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4: Param expr evaluation failure cancels invoke
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {
                        return@run  // C++ pattern: invoke cancelled on param error
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test240SceSynthInvokeInvoke1StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test240Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test240.scxml:8
    override fun onExit(state: Test240State) {
        when (state) {
            is Test240State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test240State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test240State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test240State.S01 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s01")
            }
            is Test240State.S02 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test240.scxml:8
    override fun executeTransitionActions(
        source: Test240State,
        event: Test240Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
