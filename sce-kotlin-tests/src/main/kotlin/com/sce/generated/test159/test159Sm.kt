// GENERATED CODE — DO NOT EDIT
// Source: resources/159/test159.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test159

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test159State : State {
    data object Fail : Test159State
    data object Pass : Test159State
    data object S0 : Test159State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test159Event : Event {
    sealed interface Error : Test159Event {
        data object Execution : Error
    }
    data object ThisWillFail : Test159Event
}
// --- State Machine (W3C SCXML) ---

class Test159StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test159State, Test159Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test159State = Test159State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test159State? = when (stateId) {
        "fail" -> Test159State.Fail
        "pass" -> Test159State.Pass
        "s0" -> Test159State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test159State): String = when (state) {
        is Test159State.Fail -> "fail"
        is Test159State.Pass -> "pass"
        is Test159State.S0 -> "s0"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test159State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test159State): Int = when (state) {
        is Test159State.Fail -> 2
        is Test159State.Pass -> 1
        is Test159State.S0 -> 0
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test159Event? = when (name) {
        "error.execution" -> Test159Event.Error.Execution
        "thisWillFail" -> Test159Event.ThisWillFail
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test159Event): String? = when (event) {
        is Test159Event.Error.Execution -> "error.execution"
        is Test159Event.ThisWillFail -> "thisWillFail"
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
        engine.setupSystemVariables(sid, "test159")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test159Event.Error.Execution)
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
            raiseInternal(Test159Event.Error.Execution)
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
            raiseInternal(Test159Event.Error.Execution)
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
            raiseInternal(Test159Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test159Event) {
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
        state: Test159State,
        event: Test159Event
    ): TransitionResult<Test159State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test159State
    ): TransitionResult<Test159State> = when (state) {
        is Test159State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test159State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test159State.Fail)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test159State.Pass)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test159State) {
        when (state) {
            is Test159State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test159State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test159State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            // W3C SCXML 6.2 (test194): Invalid target raises error.execution
            raiseInternal(Test159Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            executeAssign("Var1", "Var1 + 1")
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test159State) {
        when (state) {
            is Test159State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test159State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test159State.S0 -> {
                activeStateIds.remove("s0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test159State,
        event: Test159Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
