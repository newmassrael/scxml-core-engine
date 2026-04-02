// GENERATED CODE — DO NOT EDIT
// Source: resources/503/test503.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test503

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test503State : State {
    data object Fail : Test503State
    data object Pass : Test503State
    data object S1 : Test503State
    data object S2 : Test503State
    data object S3 : Test503State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test503Event : Event {
    data object Bar : Test503Event
    sealed interface Error : Test503Event {
        data object Execution : Error
    }
    data object Foo : Test503Event
}
// --- State Machine (W3C SCXML) ---

class Test503StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test503State, Test503Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test503State = Test503State.S1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test503Event? = when (name) {
        "bar" -> Test503Event.Bar
        "error.execution" -> Test503Event.Error.Execution
        "foo" -> Test503Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test503Event): String? = when (event) {
        is Test503Event.Bar -> "bar"
        is Test503Event.Error.Execution -> "error.execution"
        is Test503Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test503")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test503Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test503Event.Error.Execution)
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
            raiseInternal(Test503Event.Error.Execution)
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
            raiseInternal(Test503Event.Error.Execution)
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
            raiseInternal(Test503Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test503Event) {
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
        state: Test503State,
        event: Test503Event
    ): TransitionResult<Test503State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test503State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test503State
    ): TransitionResult<Test503State> = when (state) {
        is Test503State.S1 -> processNullS1()
        is Test503State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test503State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test503State.S2)
    }

    private fun processNullS3(
    ): TransitionResult<Test503State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test503State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test503State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS2(
        event: Test503Event
    ): TransitionResult<Test503State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test503Event.Foo -> TransitionResult.Internal
        event is Test503Event.Bar && safeEvaluateGuard("Var2 == 1") -> TransitionResult.External(Test503State.S3, Test503State.S2)

        event is Test503Event.Bar -> TransitionResult.External(Test503State.Fail, Test503State.S2)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test503State) {
        when (state) {
            is Test503State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test503State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test503State.S1 -> {
            raiseInternal(Test503Event.Foo)
            raiseInternal(Test503Event.Bar)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test503State) {
        when (state) {
            is Test503State.S2 -> {
            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test503State,
        event: Test503Event?
    ) {
        when (source) {
        is Test503State.S2 -> when {
            event is Test503Event.Foo -> {
            executeAssign("Var2", "Var2 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
