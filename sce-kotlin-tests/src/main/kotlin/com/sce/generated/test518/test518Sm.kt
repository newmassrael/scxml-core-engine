// GENERATED CODE — DO NOT EDIT
// Source: resources/518/test518.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test518

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test518State : State {
    data object Fail : Test518State
    data object Pass : Test518State
    data object S0 : Test518State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test518Event : Event {
    sealed interface Error : Test518Event {
        data object Execution : Error
    }
    data object Test : Test518Event
    data object Timeout : Test518Event
}
// --- State Machine (W3C SCXML) ---

class Test518StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test518State, Test518Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test518State = Test518State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test518State? = when (stateId) {
        "fail" -> Test518State.Fail
        "pass" -> Test518State.Pass
        "s0" -> Test518State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test518State): String = when (state) {
        is Test518State.Fail -> "fail"
        is Test518State.Pass -> "pass"
        is Test518State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test518State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test518State): Int = when (state) {
        is Test518State.Fail -> 2
        is Test518State.Pass -> 1
        is Test518State.S0 -> 0
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test518Event? = when (name) {
        "error.execution" -> Test518Event.Error.Execution
        "test" -> Test518Event.Test
        "timeout" -> Test518Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test518Event): String? = when (event) {
        is Test518Event.Error.Execution -> "error.execution"
        is Test518Event.Test -> "test"
        is Test518Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test518")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "2")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test518Event.Error.Execution)
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
            raiseInternal(Test518Event.Error.Execution)
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
            raiseInternal(Test518Event.Error.Execution)
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
            raiseInternal(Test518Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test518Event) {
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
        state: Test518State,
        event: Test518Event
    ): TransitionResult<Test518State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test518State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test518Event
    ): TransitionResult<Test518State> = when {
        event is Test518Event.Test -> TransitionResult.External(Test518State.Pass, Test518State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test518State.Fail, Test518State.S0)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test518State) {
        when (state) {
            is Test518State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test518State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test518State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            scheduleSend("__send_0", 30000L, Test518Event.Timeout)
            // W3C SCXML C.2: BasicHTTP send with script engine evaluation
            run {
                ensureScriptEngine()
                val engineH = scriptEngine ?: return@run
                val sidH = scriptSessionId ?: return@run
                val httpParams = mutableMapOf<String, List<String>>()
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern)
                if (!engineH.hasVariable(sidH, "Var1")) {
                    raiseInternal(Test518Event.Error.Execution)
                    return@run
                }
                try {
                    val v = engineH.getVariable(sidH, "Var1")
                    httpParams["Var1"] = listOf(v?.toString() ?: "")
                } catch (_: Exception) {
                    raiseInternal(Test518Event.Error.Execution)
                    return@run
                }
                val httpContent = ""
                performHttpSend("http://localhost:8080/test", "test", httpContent, httpParams, "__send_1")
            }
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test518State) {
        when (state) {
            is Test518State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test518State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test518State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test518State,
        event: Test518Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
