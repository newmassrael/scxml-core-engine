// GENERATED CODE — DO NOT EDIT
// Source: resources/505/test505.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test505

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test505State : State {
    data object Fail : Test505State
    data object Pass : Test505State
    data object S1 : Test505State
    data object S11 : Test505State
    data object S2 : Test505State
    data object S3 : Test505State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test505Event : Event {
    data object Bar : Test505Event
    sealed interface Error : Test505Event {
        data object Execution : Error
    }
    data object Foo : Test505Event
}
// --- State Machine (W3C SCXML) ---

class Test505StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test505State, Test505Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test505State = Test505State.S11

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test505State.S1)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test505State): Test505State? = when (state) {
        is Test505State.S11 -> Test505State.S1
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test505State): Test505State = when (state) {
        is Test505State.S1 -> Test505State.S11
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test505Event? = when (name) {
        "bar" -> Test505Event.Bar
        "error.execution" -> Test505Event.Error.Execution
        "foo" -> Test505Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test505Event): String? = when (event) {
        is Test505Event.Bar -> "bar"
        is Test505Event.Error.Execution -> "error.execution"
        is Test505Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test505")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test505Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test505Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var3' with expr
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raiseInternal(Test505Event.Error.Execution)
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
            raiseInternal(Test505Event.Error.Execution)
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
            raiseInternal(Test505Event.Error.Execution)
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
            raiseInternal(Test505Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test505Event) {
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
        state: Test505State,
        event: Test505Event
    ): TransitionResult<Test505State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test505State.S1 -> processS1(event)
        // W3C SCXML 3.13: Ancestor-only routing (s11 has no own event transitions)
        is Test505State.S11 -> {
            val anc1 = processS1(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test505State
    ): TransitionResult<Test505State> = when (state) {
        is Test505State.S2 -> processNullS2()
        is Test505State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS2(
    ): TransitionResult<Test505State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test505State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test505State.Fail)
    }

    private fun processNullS3(
    ): TransitionResult<Test505State> = when {
        safeEvaluateGuard("Var2 == 2") -> TransitionResult.External(Test505State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test505State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test505Event
    ): TransitionResult<Test505State> = when {
        event is Test505Event.Foo -> TransitionResult.Internal
        event is Test505Event.Bar && safeEvaluateGuard("Var3 == 1") -> TransitionResult.External(Test505State.S2)
        event is Test505Event.Bar -> TransitionResult.External(Test505State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test505State) {
        when (state) {
            is Test505State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test505State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test505State.S1 -> {
            raiseInternal(Test505Event.Foo)
            raiseInternal(Test505Event.Bar)
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test505State.S11)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test505State) {
        when (state) {
            is Test505State.S1 -> {
            executeAssign("Var1", "Var1 + 1")
            }
            is Test505State.S11 -> {
            executeAssign("Var2", "Var2 + 1")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test505State,
        event: Test505Event?
    ) {
        when (source) {
        is Test505State.S1 -> when {
            event is Test505Event.Foo -> {
            executeAssign("Var3", "Var3 + 1")
            }
            else -> {}
        }
        is Test505State.S11 -> when {
            event is Test505Event.Foo -> {
            executeAssign("Var3", "Var3 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
