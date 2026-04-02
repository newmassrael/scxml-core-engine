// GENERATED CODE — DO NOT EDIT
// Source: resources/241/test241.scxml
// Generator: SCE Kotlin Code Generator v1.0

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

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test241State.S0)
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
        else -> null
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
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = meta.type,
            sendId = meta.sendId,
            origin = meta.origin.ifEmpty { scriptSessionId ?: "" },
            originType = meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" },
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
        event is Test241Event.Timeout -> TransitionResult.External(Test241State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Success -> TransitionResult.External(Test241State.S02)
        event is Test241Event.Failure -> TransitionResult.External(Test241State.S03)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Success -> TransitionResult.External(Test241State.Pass)
        event is Test241Event.Failure -> TransitionResult.External(Test241State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test241Event
    ): TransitionResult<Test241State> = when {
        event is Test241Event.Failure -> TransitionResult.External(Test241State.Pass)
        event is Test241Event.Success -> TransitionResult.External(Test241State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test241State) {
        when (state) {
            is Test241State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test241State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test241State.S0 -> {
            scheduleSend("__send_0", 2000L, Test241Event.Timeout)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test241State.S01)
            }
            is Test241State.S01 -> {
                // W3C SCXML 6.4: Start invoked child state machine
                run {
                    val childSM = Test241Child0StateMachine(scriptEngine)
                    // W3C SCXML 6.4: Evaluate and pass invoke params to child before start
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    try {
                        invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")
                    } catch (_: Exception) {}
                    setInvokeParams(childSM, invokeParams)
                    startInvoke("_invoke_0", childSM, false, Test241Event.Done.Invoke)
                }
            }
            is Test241State.S02 -> {
                // W3C SCXML 6.4: Start invoked child state machine
                run {
                    val childSM = Test241Child1StateMachine(scriptEngine)
                    // W3C SCXML 6.4: Evaluate and pass invoke params to child before start
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {}
                    setInvokeParams(childSM, invokeParams)
                    startInvoke("_invoke_1", childSM, false, Test241Event.Done.Invoke)
                }
            }
            is Test241State.S03 -> {
                // W3C SCXML 6.4: Start invoked child state machine
                run {
                    val childSM = Test241Child2StateMachine(scriptEngine)
                    // W3C SCXML 6.4: Evaluate and pass invoke params to child before start
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {}
                    setInvokeParams(childSM, invokeParams)
                    startInvoke("_invoke_2", childSM, false, Test241Event.Done.Invoke)
                }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test241State) {
        when (state) {
            is Test241State.S01 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test241State.S02 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_1")
            }
            is Test241State.S03 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_2")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test241State,
        event: Test241Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
