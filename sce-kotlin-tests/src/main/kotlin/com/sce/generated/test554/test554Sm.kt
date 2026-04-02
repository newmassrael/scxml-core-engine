// GENERATED CODE — DO NOT EDIT
// Source: resources/554/test554.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test554

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test554State : State {
    data object Fail : Test554State
    data object Pass : Test554State
    data object S0 : Test554State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test554Event : Event {
    sealed interface Cancel : Test554Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test554Event {
        data object Invoke : Done
    }
    sealed interface Error : Test554Event {
        data object Execution : Error
    }
    data object Timer : Test554Event
}
// --- State Machine (W3C SCXML) ---

class Test554StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test554State, Test554Event>(scriptEngine) {

    override val initialState: Test554State = Test554State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test554Event? = when (name) {
        "cancel.invoke" -> Test554Event.Cancel.Invoke
        "done.invoke" -> Test554Event.Done.Invoke
        "error.execution" -> Test554Event.Error.Execution
        "timer" -> Test554Event.Timer
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test554Event): String? = when (event) {
        is Test554Event.Cancel.Invoke -> "cancel.invoke"
        is Test554Event.Done.Invoke -> "done.invoke"
        is Test554Event.Error.Execution -> "error.execution"
        is Test554Event.Timer -> "timer"
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
        engine.setupSystemVariables(sid, "test554")




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
            raiseInternal(Test554Event.Error.Execution)
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
            raiseInternal(Test554Event.Error.Execution)
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
            raiseInternal(Test554Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test554Event) {
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
        state: Test554State,
        event: Test554Event
    ): TransitionResult<Test554State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test554State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test554Event
    ): TransitionResult<Test554State> = when {
        event is Test554Event.Timer -> TransitionResult.External(Test554State.Pass)
        event is Test554Event.Done.Invoke -> TransitionResult.External(Test554State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test554State) {
        when (state) {
            is Test554State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test554State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test554State.S0 -> {
            scheduleSend("__send_0", 1000L, Test554Event.Timer)
                // W3C SCXML 6.4: Start invoked child state machine
                run {
                    val childSM = Test554Child0StateMachine(scriptEngine)
                    // W3C SCXML 6.4: Evaluate and pass invoke params to child before start
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: return@run
                    val sidInv = scriptSessionId ?: return@run
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4: Namelist variable must exist in parent
                    try {
                        invokeParams["__undefined_variable_for_error__"] = engineInv.getVariable(sidInv, "__undefined_variable_for_error__")
                    } catch (_: Exception) {
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    setInvokeParams(childSM, invokeParams)
                    startInvoke("_invoke_0", childSM, false, Test554Event.Done.Invoke)
                }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test554State) {
        when (state) {
            is Test554State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test554State,
        event: Test554Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
