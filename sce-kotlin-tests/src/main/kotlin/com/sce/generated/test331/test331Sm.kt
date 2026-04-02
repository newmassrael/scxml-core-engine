// GENERATED CODE — DO NOT EDIT
// Source: resources/331/test331.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test331

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test331State : State {
    data object Fail : Test331State
    data object Pass : Test331State
    data object S0 : Test331State
    data object S1 : Test331State
    data object S2 : Test331State
    data object S3 : Test331State
    data object S4 : Test331State
    data object S5 : Test331State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test331Event : Event {
    sealed interface Error : Test331Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test331Event
}
// --- State Machine (W3C SCXML) ---

class Test331StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test331State, Test331Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test331State = Test331State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test331Event? = when (name) {
        "error" -> Test331Event.Error.Self
        "error.execution" -> Test331Event.Error.Execution
        "foo" -> Test331Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test331Event): String? = when (event) {
        is Test331Event.Error.Self -> "error"
        is Test331Event.Error.Execution -> "error.execution"
        is Test331Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test331")

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
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
            raiseInternal(Test331Event.Error.Execution)
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
            raiseInternal(Test331Event.Error.Execution)
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
            raiseInternal(Test331Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test331Event) {
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
        state: Test331State,
        event: Test331Event
    ): TransitionResult<Test331State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test331State.S0 -> processS0(event)
        is Test331State.S2 -> processS2(event)
        is Test331State.S4 -> processS4(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test331State
    ): TransitionResult<Test331State> = when (state) {
        is Test331State.S1 -> processNullS1()
        is Test331State.S3 -> processNullS3()
        is Test331State.S5 -> processNullS5()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'internal'") -> TransitionResult.External(Test331State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail)
    }

    private fun processNullS3(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'platform'") -> TransitionResult.External(Test331State.S4)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail)
    }

    private fun processNullS5(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'external'") -> TransitionResult.External(Test331State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        event is Test331Event.Foo -> TransitionResult.External(Test331State.S1)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail)
    }

    private fun processS2(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test331Event.Error || event is Test331Event.Error.Execution) -> TransitionResult.External(Test331State.S3)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail)
    }

    private fun processS4(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        event is Test331Event.Foo -> TransitionResult.External(Test331State.S5)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test331State) {
        when (state) {
            is Test331State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test331State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test331State.S0 -> {
            raiseInternal(Test331Event.Foo)
            }
            is Test331State.S2 -> {
            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raiseInternal(Test331Event.Error.Execution, EventMetadata.platform())
            }
            is Test331State.S4 -> {
            send(Test331Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test331State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test331State,
        event: Test331Event?
    ) {
        when (source) {
        is Test331State.S0 -> when {
            event is Test331Event.Foo -> {
            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        is Test331State.S2 -> when {
            (event is Test331Event.Error || event is Test331Event.Error.Execution) -> {
            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        is Test331State.S4 -> when {
            event is Test331Event.Foo -> {
            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
