// GENERATED CODE — DO NOT EDIT
// Source: resources/332/test332.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test332

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test332State : State {
    data object Fail : Test332State
    data object Pass : Test332State
    data object S0 : Test332State
    data object S1 : Test332State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test332Event : Event {
    sealed interface Error : Test332Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test332Event
}
// --- State Machine (W3C SCXML) ---

class Test332StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test332State, Test332Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test332State = Test332State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test332Event? = when (name) {
        "error" -> Test332Event.Error.Self
        "error.execution" -> Test332Event.Error.Execution
        "foo" -> Test332Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test332Event): String? = when (event) {
        is Test332Event.Error.Self -> "error"
        is Test332Event.Error.Execution -> "error.execution"
        is Test332Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test332")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}



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
            raiseInternal(Test332Event.Error.Execution)
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
            raiseInternal(Test332Event.Error.Execution)
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
            raiseInternal(Test332Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test332Event) {
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
        state: Test332State,
        event: Test332Event
    ): TransitionResult<Test332State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test332State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test332State
    ): TransitionResult<Test332State> = when (state) {
        is Test332State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test332State> = when {
        safeEvaluateGuard("Var1 === Var2") -> TransitionResult.External(Test332State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test332State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test332Event
    ): TransitionResult<Test332State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test332Event.Error || event is Test332Event.Error.Execution) -> TransitionResult.External(Test332State.S1)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test332State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test332State) {
        when (state) {
            is Test332State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test332State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test332State.S0 -> {
            // W3C SCXML 6.2.4: Store sendid in idlocation (test183, test332)
            run {
                ensureScriptEngine()
                val eng = scriptEngine ?: return@run
                val sid = scriptSessionId ?: return@run
                try { eng.setVariable(sid, "Var1", "__send_0") } catch (_: Exception) {}
            }
            // W3C SCXML 6.2 (test194): Invalid target raises error.execution
            raiseInternal(Test332Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test332State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test332State,
        event: Test332Event?
    ) {
        when (source) {
        is Test332State.S0 -> when {
            (event is Test332Event.Error || event is Test332Event.Error.Execution) -> {
            executeAssign("Var2", "_event.sendid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
