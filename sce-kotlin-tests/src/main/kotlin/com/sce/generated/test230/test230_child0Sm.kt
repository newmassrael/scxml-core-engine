// GENERATED CODE — DO NOT EDIT
// Source: resources/230/test230_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test230

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test230Child0State : State {
    data object Sub0 : Test230Child0State
    data object SubFinal : Test230Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test230Child0Event : Event {
    data object ChildToParent : Test230Child0Event
    sealed interface Error : Test230Child0Event {
        data object Execution : Error
    }
    data object Timeout : Test230Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test230Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test230Child0State, Test230Child0Event>(scriptEngine) {

    override val initialState: Test230Child0State = Test230Child0State.Sub0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test230Child0Event? = when (name) {
        "childToParent" -> Test230Child0Event.ChildToParent
        "error.execution" -> Test230Child0Event.Error.Execution
        "timeout" -> Test230Child0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test230Child0Event): String? = when (event) {
        is Test230Child0Event.ChildToParent -> "childToParent"
        is Test230Child0Event.Error.Execution -> "error.execution"
        is Test230Child0Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test230_child0")




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
            raiseInternal(Test230Child0Event.Error.Execution)
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
            raiseInternal(Test230Child0Event.Error.Execution)
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
            raiseInternal(Test230Child0Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test230Child0Event) {
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
        state: Test230Child0State,
        event: Test230Child0Event
    ): TransitionResult<Test230Child0State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test230Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test230Child0Event
    ): TransitionResult<Test230Child0State> = when {
        event is Test230Child0Event.ChildToParent -> TransitionResult.External(Test230Child0State.SubFinal)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test230Child0State.SubFinal)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test230Child0State) {
        when (state) {
            is Test230Child0State.Sub0 -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            scheduleSend("__send_1", 2000L, Test230Child0Event.Timeout)
            }
            is Test230Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test230Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test230Child0State,
        event: Test230Child0Event?
    ) {
        when (source) {
        is Test230Child0State.Sub0 -> when {
            event is Test230Child0Event.ChildToParent -> {
            println("name is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.name")?.toString() ?: ""))
            println("type is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.type")?.toString() ?: ""))
            println("sendid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.sendid")?.toString() ?: ""))
            println("origin is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origin")?.toString() ?: ""))
            println("origintype is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.origintype")?.toString() ?: ""))
            println("invokeid is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.invokeid")?.toString() ?: ""))
            println("data is : " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "_event.data")?.toString() ?: ""))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
