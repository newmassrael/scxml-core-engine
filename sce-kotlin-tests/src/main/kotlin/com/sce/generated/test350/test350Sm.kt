// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 9faef2370910e1d1b12ff0b00a3d63d3578977b6f3f2045b8b014f47fa072349
// generated-at: 1778932425

// GENERATED CODE — DO NOT EDIT
// Source: resources/350/test350.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test350.scxml:6

package com.sce.generated.test350

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test350State : State {
    data object Fail : Test350State
    data object Pass : Test350State
    data object S0 : Test350State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test350Event : Event {
    sealed interface Error : Test350Event {
        data object Execution : Error
    }
    data object S0Event : Test350Event
    data object Timeout : Test350Event
}
// --- State Machine (W3C SCXML) ---

class Test350StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test350State, Test350Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test350State = Test350State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test350State? = when (stateId) {
        "fail" -> Test350State.Fail
        "pass" -> Test350State.Pass
        "s0" -> Test350State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test350State): String = when (state) {
        is Test350State.Fail -> "fail"
        is Test350State.Pass -> "pass"
        is Test350State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test350State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test350State): Int = when (state) {
        is Test350State.Fail -> 2
        is Test350State.Pass -> 1
        is Test350State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test350Event? = when (name) {
        "error.execution" -> Test350Event.Error.Execution
        "s0Event" -> Test350Event.S0Event
        "timeout" -> Test350Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test350Event): String? = when (event) {
        is Test350Event.Error.Execution -> "error.execution"
        is Test350Event.S0Event -> "s0Event"
        is Test350Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: return
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test350")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'#_scxml_'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test350Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "_sessionid")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test350Event.Error.Execution)
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
            raiseInternal(Test350Event.Error.Execution)
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
            raiseInternal(Test350Event.Error.Execution)
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
            raiseInternal(Test350Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test350Event) {
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
        state: Test350State,
        event: Test350Event
    ): TransitionResult<Test350State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test350State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test350Event
    ): TransitionResult<Test350State> = when {
        event is Test350Event.S0Event -> TransitionResult.External(Test350State.Pass, Test350State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test350State.Fail, Test350State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test350.scxml:6
    override fun onEntry(state: Test350State) {
        when (state) {
            is Test350State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test350State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test350State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test350Event.Timeout)


            send(Test350Event.S0Event, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test350.scxml:6
    override fun onExit(state: Test350State) {
        when (state) {
            is Test350State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test350State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test350State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test350.scxml:6
    override fun executeTransitionActions(
        source: Test350State,
        event: Test350Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
