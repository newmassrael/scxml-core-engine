// GENERATED CODE — DO NOT EDIT
// Source: resources/233/test233.scxml
// Generator: SCE Kotlin Code Generator v1.0

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
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test233State, Test233Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test233State = Test233State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
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
        val engine = scriptEngine ?: return false
        val sid = scriptSessionId ?: return false
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
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test233Event.Error.Execution)
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
            raiseInternal(Test233Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test233Event) {
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
        event is Test233Event.ChildToParent && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test233State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test233State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test233State) {
        when (state) {
            is Test233State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test233State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test233State.S0 -> {
            scheduleSend("__send_0", 3000L, Test233Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test233Child0StateMachine(scriptEngine), false, Test233Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test233State) {
        when (state) {
            is Test233State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test233State,
        event: Test233Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
