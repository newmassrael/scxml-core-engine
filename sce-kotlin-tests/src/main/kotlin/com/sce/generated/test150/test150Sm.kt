// GENERATED CODE — DO NOT EDIT
// Source: resources/150/test150.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test150

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test150State : State {
    data object Fail : Test150State
    data object Pass : Test150State
    data object S0 : Test150State
    data object S1 : Test150State
    data object S2 : Test150State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test150Event : Event {
    data object Bar : Test150Event
    sealed interface Error : Test150Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test150Event
}
// --- State Machine (W3C SCXML) ---

class Test150StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test150State, Test150Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test150State = Test150State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test150Event? = when (name) {
        "bar" -> Test150Event.Bar
        "error" -> Test150Event.Error.Self
        "error.execution" -> Test150Event.Error.Execution
        "foo" -> Test150Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test150Event): String? = when (event) {
        is Test150Event.Bar -> "bar"
        is Test150Event.Error.Self -> "error"
        is Test150Event.Error.Execution -> "error.execution"
        is Test150Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test150")

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
        // W3C SCXML B.2: Initialize variable 'Var3' with inline content
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, "[1,2,3]")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raiseInternal(Test150Event.Error.Execution)
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
            raiseInternal(Test150Event.Error.Execution)
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
            raiseInternal(Test150Event.Error.Execution)
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
            raiseInternal(Test150Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test150Event) {
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
        state: Test150State,
        event: Test150Event
    ): TransitionResult<Test150State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test150State.S0 -> processS0(event)
        is Test150State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test150State
    ): TransitionResult<Test150State> = when (state) {
        is Test150State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS2(
    ): TransitionResult<Test150State> = when {
        safeEvaluateGuard("typeof Var4 !== 'undefined'") -> TransitionResult.External(Test150State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test150State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test150Event
    ): TransitionResult<Test150State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test150Event.Error || event is Test150Event.Error.Execution) -> TransitionResult.External(Test150State.Fail)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test150State.S1)
    }

    private fun processS1(
        event: Test150Event
    ): TransitionResult<Test150State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test150Event.Error || event is Test150Event.Error.Execution) -> TransitionResult.External(Test150State.Fail)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test150State.S2)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test150State) {
        when (state) {
            is Test150State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test150State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test150State.S0 -> {
            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: return@run
                val sid = scriptSessionId ?: return@run
                try {
                    engine.executeForeach(sid, "Var3", "Var1", "Var2") {
                    }
                } catch (e: Exception) {
                    raiseInternal(Test150Event.Error.Execution)
                }
            }
            raiseInternal(Test150Event.Foo)
            }
            is Test150State.S1 -> {
            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: return@run
                val sid = scriptSessionId ?: return@run
                try {
                    engine.executeForeach(sid, "Var3", "Var4", "Var5") {
                    }
                } catch (e: Exception) {
                    raiseInternal(Test150Event.Error.Execution)
                }
            }
            raiseInternal(Test150Event.Bar)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test150State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test150State,
        event: Test150Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
