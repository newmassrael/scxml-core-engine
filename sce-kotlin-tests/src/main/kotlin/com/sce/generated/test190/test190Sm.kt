// GENERATED CODE — DO NOT EDIT
// Source: resources/190/test190.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test190

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test190State : State {
    data object Fail : Test190State
    data object Pass : Test190State
    data object S0 : Test190State
    data object S1 : Test190State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test190Event : Event {
    sealed interface Error : Test190Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Event1 : Test190Event
    data object Event2 : Test190Event
    data object Timeout : Test190Event
}
// --- State Machine (W3C SCXML) ---

class Test190StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test190State, Test190Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test190State = Test190State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(initialState)
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test190State? = when (stateId) {
        "fail" -> Test190State.Fail
        "pass" -> Test190State.Pass
        "s0" -> Test190State.S0
        "s1" -> Test190State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test190State): String = when (state) {
        is Test190State.Fail -> "fail"
        is Test190State.Pass -> "pass"
        is Test190State.S0 -> "s0"
        is Test190State.S1 -> "s1"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test190State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test190State): Int = when (state) {
        is Test190State.Fail -> 3
        is Test190State.Pass -> 2
        is Test190State.S0 -> 0
        is Test190State.S1 -> 1
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test190Event? = when (name) {
        "error.communication" -> Test190Event.Error.Communication
        "error.execution" -> Test190Event.Error.Execution
        "event1" -> Test190Event.Event1
        "event2" -> Test190Event.Event2
        "timeout" -> Test190Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test190Event): String? = when (event) {
        is Test190Event.Error.Communication -> "error.communication"
        is Test190Event.Error.Execution -> "error.execution"
        is Test190Event.Event1 -> "event1"
        is Test190Event.Event2 -> "event2"
        is Test190Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test190")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'#_scxml_'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test190Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "_sessionid")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test190Event.Error.Execution)
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
            raiseInternal(Test190Event.Error.Execution)
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
            raiseInternal(Test190Event.Error.Execution)
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
            raiseInternal(Test190Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test190Event) {
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
        state: Test190State,
        event: Test190Event
    ): TransitionResult<Test190State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test190State.S0 -> processS0(event)
        is Test190State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test190Event
    ): TransitionResult<Test190State> = when {
        event is Test190Event.Event1 -> TransitionResult.External(Test190State.S1, Test190State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test190State.Fail)
    }

    private fun processS1(
        event: Test190Event
    ): TransitionResult<Test190State> = when {
        event is Test190Event.Event2 -> TransitionResult.External(Test190State.Pass, Test190State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test190State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test190State) {
        when (state) {
            is Test190State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test190State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test190State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            // W3C SCXML 6.2: Dynamic target evaluation (test173)
            run {
                ensureScriptEngine()
                val engineT = scriptEngine ?: return@run
                val sidT = scriptSessionId ?: return@run
                val dynamicTarget: String
                try {
                    val v = engineT.evaluateExpr(sidT, "Var1")
                    dynamicTarget = v?.toString() ?: ""
                } catch (_: Exception) {
                    raiseInternal(Test190Event.Error.Execution, EventMetadata.platform())
                    return@run
                }
                // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                if (dynamicTarget.startsWith("!")) {
                    raiseInternal(Test190Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
                    return@run
                }
                // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                if (dynamicTarget.isEmpty() || dynamicTarget == "undefined") {
                    raiseInternal(Test190Event.Error.Communication, EventMetadata.platform())
                    return@run
                }
                if (dynamicTarget == "#_internal") {
                    raiseInternal(Test190Event.Event2)
                } else if (dynamicTarget == "#_parent") {
                    onSendToParent?.invoke("event2", "")
                } else {
                    send(Test190Event.Event2, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
                }
            }
            raiseInternal(Test190Event.Event1)
            send(Test190Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            is Test190State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test190State) {
        when (state) {
            is Test190State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test190State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test190State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test190State.S1 -> {
                activeStateIds.remove("s1")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test190State,
        event: Test190Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
