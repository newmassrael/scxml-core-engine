// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425

// GENERATED CODE — DO NOT EDIT
// Source: resources/241/test241.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test241.scxml:8

package com.sce.generated.test241

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test241State : State {
    data object Fail : Test241State
    data object Pass : Test241State
    data object S0 : Test241State
    data object S01 : Test241State
    data object S02 : Test241State
    data object S03 : Test241State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test241Event : Event {
    sealed interface Cancel : Test241Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test241Event {
        data object Invoke : Done
    }
    sealed interface Error : Test241Event {
        data object Execution : Error
    }
    data object Failure : Test241Event
    data object Success : Test241Event
    data object Timeout : Test241Event
}
// --- State Machine (W3C SCXML) ---

class Test241StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test241State, Test241Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test241State = Test241State.S01

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test241State): Test241State? = when (state) {
        is Test241State.S01 -> Test241State.S0
        is Test241State.S02 -> Test241State.S0
        is Test241State.S03 -> Test241State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test241State): Test241State = when (state) {
        is Test241State.S0 -> Test241State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test241State? = when (stateId) {
        "fail" -> Test241State.Fail
        "pass" -> Test241State.Pass
        "s0" -> Test241State.S0
        "s01" -> Test241State.S01
        "s02" -> Test241State.S02
        "s03" -> Test241State.S03
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test241State): String = when (state) {
        is Test241State.Fail -> "fail"
        is Test241State.Pass -> "pass"
        is Test241State.S0 -> "s0"
        is Test241State.S01 -> "s01"
        is Test241State.S02 -> "s02"
        is Test241State.S03 -> "s03"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test241State): Boolean = when (state) {
        is Test241State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test241State): Int = when (state) {
        is Test241State.Fail -> 5
        is Test241State.Pass -> 4
        is Test241State.S0 -> 0
        is Test241State.S01 -> 1
        is Test241State.S02 -> 2
        is Test241State.S03 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test241Event? = when (name) {
        "cancel.invoke" -> Test241Event.Cancel.Invoke
        "done.invoke" -> Test241Event.Done.Invoke
        "error.execution" -> Test241Event.Error.Execution
        "failure" -> Test241Event.Failure
        "success" -> Test241Event.Success
        "timeout" -> Test241Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test241Event): String? = when (event) {
        is Test241Event.Cancel.Invoke -> "cancel.invoke"
        is Test241Event.Done.Invoke -> "done.invoke"
        is Test241Event.Error.Execution -> "error.execution"
        is Test241Event.Failure -> "failure"
        is Test241Event.Success -> "success"
        is Test241Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test241")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test241Event.Error.Execution)
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
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test241Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test241Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test241Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test241Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
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
            sid, eventName,
            data = meta.data,
            type = effectiveType,
            sendId = meta.sendId,
            origin = effectiveOrigin,
            originType = effectiveOriginType,
            invokeId = meta.invokeId
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test241State,
        event: Test241Event
    ): TransitionResult<Test241State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test241State.S0 -> processS0(event)
        is Test241State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test241State.S02 -> {
            val result = processS02(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test241State.S03 -> {
            val result = processS03(event)
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
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Timeout -> TransitionResult.External(Test241State.Fail, Test241State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Success -> TransitionResult.External(Test241State.S02, Test241State.S01)

        event is Test241Event.Failure -> TransitionResult.External(Test241State.S03, Test241State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Success -> TransitionResult.External(Test241State.Pass, Test241State.S02)

        event is Test241Event.Failure -> TransitionResult.External(Test241State.Fail, Test241State.S02)

        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Failure -> TransitionResult.External(Test241State.Pass, Test241State.S03)

        event is Test241Event.Success -> TransitionResult.External(Test241State.Fail, Test241State.S03)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test241.scxml:8
    override fun onEntry(state: Test241State) {
        when (state) {
            is Test241State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test241State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test241State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test241Event.Timeout)
            }
            is Test241State.S01 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "Var1")) {
                        raiseInternal(Test241Event.Error.Execution)
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test241SceSynthInvokeInvoke0StateMachine(scriptEngine)
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test241State.S02 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4: Param expr evaluation failure cancels invoke
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {
                        return@run  // C++ pattern: invoke cancelled on param error
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test241SceSynthInvokeInvoke1StateMachine(scriptEngine)
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test241State.S03 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s03")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s03.${System.identityHashCode(this)}._invoke_2"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4: Param expr evaluation failure cancels invoke
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {
                        return@run  // C++ pattern: invoke cancelled on param error
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test241SceSynthInvokeInvoke2StateMachine(scriptEngine)
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_2", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test241.scxml:8
    override fun onExit(state: Test241State) {
        when (state) {
            is Test241State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test241State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test241State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test241State.S01 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s01")
            }
            is Test241State.S02 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s02")
            }
            is Test241State.S03 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
                activeStateIds.remove("s03")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test241.scxml:8
    override fun executeTransitionActions(
        source: Test241State,
        event: Test241Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
