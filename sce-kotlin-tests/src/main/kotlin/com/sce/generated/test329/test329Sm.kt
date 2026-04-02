// GENERATED CODE — DO NOT EDIT
// Source: resources/329/test329.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test329

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test329State : State {
    data object Fail : Test329State
    data object Pass : Test329State
    data object S0 : Test329State
    data object S1 : Test329State
    data object S2 : Test329State
    data object S3 : Test329State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test329Event : Event {
    sealed interface Error : Test329Event {
        data object Execution : Error
    }
    data object Foo : Test329Event
}
// --- State Machine (W3C SCXML) ---

class Test329StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test329State, Test329Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test329State = Test329State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test329Event? = when (name) {
        "error.execution" -> Test329Event.Error.Execution
        "foo" -> Test329Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test329Event): String? = when (event) {
        is Test329Event.Error.Execution -> "error.execution"
        is Test329Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test329")

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
        // W3C SCXML 5.2: Runtime variable 'Var3' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var3", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var4' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var4", null)
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
            raiseInternal(Test329Event.Error.Execution)
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
            raiseInternal(Test329Event.Error.Execution)
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
            raiseInternal(Test329Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test329Event) {
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
        state: Test329State,
        event: Test329Event
    ): TransitionResult<Test329State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test329State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test329State
    ): TransitionResult<Test329State> = when (state) {
        is Test329State.S1 -> processNullS1()
        is Test329State.S2 -> processNullS2()
        is Test329State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var2 == _event") -> TransitionResult.External(Test329State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail)
    }

    private fun processNullS2(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var3 == _name") -> TransitionResult.External(Test329State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail)
    }

    private fun processNullS3(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var4 == _ioprocessors") -> TransitionResult.External(Test329State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test329Event
    ): TransitionResult<Test329State> = when {
        event is Test329Event.Foo && safeEvaluateGuard("Var1 == _sessionid") -> TransitionResult.External(Test329State.S1)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test329State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test329State) {
        when (state) {
            is Test329State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test329State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test329State.S0 -> {
            raiseInternal(Test329Event.Foo)
            executeAssign("Var1", "_sessionid")
            executeAssign("_sessionid", "'invalid_session_id'")
            }
            is Test329State.S1 -> {
            executeAssign("Var2", "_event")
            executeAssign("_event", "27")
            }
            is Test329State.S2 -> {
            executeAssign("Var3", "_name")
            executeAssign("_name", "27")
            }
            is Test329State.S3 -> {
            executeAssign("Var4", "_ioprocessors")
            executeAssign("_ioprocessors", "27")
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test329State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test329State,
        event: Test329Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
