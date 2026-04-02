// GENERATED CODE — DO NOT EDIT
// Source: resources/342/test342.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test342

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test342State : State {
    data object Fail : Test342State
    data object Pass : Test342State
    data object S0 : Test342State
    data object S1 : Test342State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test342Event : Event {
    sealed interface Error : Test342Event {
        data object Execution : Error
    }
    data object Foo : Test342Event
}
// --- State Machine (W3C SCXML) ---

class Test342StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test342State, Test342Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test342State = Test342State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test342Event? = when (name) {
        "error.execution" -> Test342Event.Error.Execution
        "foo" -> Test342Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test342Event): String? = when (event) {
        is Test342Event.Error.Execution -> "error.execution"
        is Test342Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test342")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'foo'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test342Event.Error.Execution)
        }
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
            raiseInternal(Test342Event.Error.Execution)
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
            raiseInternal(Test342Event.Error.Execution)
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
            raiseInternal(Test342Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test342Event) {
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
        state: Test342State,
        event: Test342Event
    ): TransitionResult<Test342State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test342State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test342State
    ): TransitionResult<Test342State> = when (state) {
        is Test342State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test342State> = when {
        safeEvaluateGuard("Var1 === Var2") -> TransitionResult.External(Test342State.Pass)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test342State.Fail)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test342Event
    ): TransitionResult<Test342State> = when {
        event is Test342Event.Foo -> TransitionResult.External(Test342State.S1, Test342State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test342State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test342State) {
        when (state) {
            is Test342State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test342State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test342State.S0 -> {
            // W3C SCXML 6.2: Dynamic event name evaluation (test172)
            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: return@run
                val sid = scriptSessionId ?: return@run
                val dynamicEventName: String
                try {
                    val v = engine.evaluateExpr(sid, "Var1")
                    dynamicEventName = v?.toString() ?: ""
                } catch (_: Exception) {
                    raiseInternal(Test342Event.Error.Execution, EventMetadata.platform())
                    return@run
                }
                val resolvedEvent = resolveEventByName(dynamicEventName)
                if (resolvedEvent != null) {
                    send(resolvedEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
                }
            }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test342State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test342State,
        event: Test342Event?
    ) {
        when (source) {
        is Test342State.S0 -> when {
            event is Test342Event.Foo -> {
            executeAssign("Var2", "_event.name")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
