
// GENERATED CODE — DO NOT EDIT
// Source: resources/147/test147.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test147

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test147State : State {
    data object Fail : Test147State
    data object Pass : Test147State
    data object S0 : Test147State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test147Event : Event {
    data object Bar : Test147Event
    data object Bat : Test147Event
    data object Baz : Test147Event
    sealed interface Error : Test147Event {
        data object Execution : Error
    }
    data object Foo : Test147Event
}
// --- State Machine (W3C SCXML) ---

class Test147StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test147State, Test147Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test147State = Test147State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test147State? = when (stateId) {
        "fail" -> Test147State.Fail
        "pass" -> Test147State.Pass
        "s0" -> Test147State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test147State): String = when (state) {
        is Test147State.Fail -> "fail"
        is Test147State.Pass -> "pass"
        is Test147State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test147State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test147State): Int = when (state) {
        is Test147State.Fail -> 2
        is Test147State.Pass -> 1
        is Test147State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test147Event? = when (name) {
        "bar" -> Test147Event.Bar
        "bat" -> Test147Event.Bat
        "baz" -> Test147Event.Baz
        "error.execution" -> Test147Event.Error.Execution
        "foo" -> Test147Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test147Event): String? = when (event) {
        is Test147Event.Bar -> "bar"
        is Test147Event.Bat -> "bat"
        is Test147Event.Baz -> "baz"
        is Test147Event.Error.Execution -> "error.execution"
        is Test147Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test147")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test147Event.Error.Execution)
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
            raiseInternal(Test147Event.Error.Execution)
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
            raiseInternal(Test147Event.Error.Execution)
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
            raiseInternal(Test147Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test147Event) {
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
        state: Test147State,
        event: Test147Event
    ): TransitionResult<Test147State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test147State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test147Event
    ): TransitionResult<Test147State> = when {
        event is Test147Event.Bar && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test147State.Pass, Test147State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test147State.Fail, Test147State.S0)
    }


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test147State) {
        when (state) {
            is Test147State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test147State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test147State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            if (safeEvaluateGuard("false")) {

            raiseInternal(Test147Event.Foo)


            executeAssign("Var1", "Var1 + 1")
            } else if (safeEvaluateGuard("true")) {

            raiseInternal(Test147Event.Bar)


            executeAssign("Var1", "Var1 + 1")
            } else {

            raiseInternal(Test147Event.Baz)


            executeAssign("Var1", "Var1 + 1")
            }

            raiseInternal(Test147Event.Bat)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test147State) {
        when (state) {
            is Test147State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test147State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test147State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test147State,
        event: Test147Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
