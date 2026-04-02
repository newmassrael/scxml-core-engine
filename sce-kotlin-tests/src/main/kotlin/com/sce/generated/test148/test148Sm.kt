// GENERATED CODE — DO NOT EDIT
// Source: resources/148/test148.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test148

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test148State : State {
    data object Fail : Test148State
    data object Pass : Test148State
    data object S0 : Test148State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test148Event : Event {
    data object Bar : Test148Event
    data object Bat : Test148Event
    data object Baz : Test148Event
    sealed interface Error : Test148Event {
        data object Execution : Error
    }
    data object Foo : Test148Event
}
// --- State Machine (W3C SCXML) ---

class Test148StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test148State, Test148Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test148State = Test148State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test148Event? = when (name) {
        "bar" -> Test148Event.Bar
        "bat" -> Test148Event.Bat
        "baz" -> Test148Event.Baz
        "error.execution" -> Test148Event.Error.Execution
        "foo" -> Test148Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test148Event): String? = when (event) {
        is Test148Event.Bar -> "bar"
        is Test148Event.Bat -> "bat"
        is Test148Event.Baz -> "baz"
        is Test148Event.Error.Execution -> "error.execution"
        is Test148Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test148")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test148Event) {
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
        state: Test148State,
        event: Test148Event
    ): TransitionResult<Test148State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test148State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test148Event
    ): TransitionResult<Test148State> = when {
        event is Test148Event.Baz && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test148State.Pass, Test148State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test148State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test148State) {
        when (state) {
            is Test148State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test148State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test148State.S0 -> {
            if (safeEvaluateGuard("false")) {
            raiseInternal(Test148Event.Foo)
            executeAssign("Var1", "Var1 + 1")
            } else if (safeEvaluateGuard("false")) {
            raiseInternal(Test148Event.Bar)
            executeAssign("Var1", "Var1 + 1")
            } else {
            raiseInternal(Test148Event.Baz)
            executeAssign("Var1", "Var1 + 1")
            }
            raiseInternal(Test148Event.Bat)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test148State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test148State,
        event: Test148Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
