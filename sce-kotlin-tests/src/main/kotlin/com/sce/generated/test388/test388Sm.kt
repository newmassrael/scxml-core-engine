// GENERATED CODE — DO NOT EDIT
// Source: resources/388/test388.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test388

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test388State : State {
    data object Fail : Test388State
    data object Pass : Test388State
    data object S0 : Test388State
    data object S01 : Test388State
    data object S011 : Test388State
    data object S012 : Test388State
    data object S02 : Test388State
    data object S021 : Test388State
    data object S022 : Test388State
    data object S1 : Test388State
    data object S2 : Test388State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test388Event : Event {
    sealed interface Entering : Test388Event {
        data object Self : Entering
        data object S011 : Entering
        data object S012 : Entering
        data object S021 : Entering
        data object S022 : Entering
    }
    sealed interface Error : Test388Event {
        data object Execution : Error
    }
    data object Timeout : Test388Event
}
// --- State Machine (W3C SCXML) ---

class Test388StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test388State, Test388Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test388State = Test388State.S012

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test388State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test388State): Test388State? = when (state) {
        is Test388State.S01 -> Test388State.S0
        is Test388State.S011 -> Test388State.S01
        is Test388State.S012 -> Test388State.S01
        is Test388State.S02 -> Test388State.S0
        is Test388State.S021 -> Test388State.S02
        is Test388State.S022 -> Test388State.S02
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test388State): Test388State = when (state) {
        is Test388State.S0 -> Test388State.S011
        is Test388State.S01 -> Test388State.S011
        is Test388State.S02 -> Test388State.S021
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test388Event? = when (name) {
        "entering" -> Test388Event.Entering.Self
        "entering.s011" -> Test388Event.Entering.S011
        "entering.s012" -> Test388Event.Entering.S012
        "entering.s021" -> Test388Event.Entering.S021
        "entering.s022" -> Test388Event.Entering.S022
        "error.execution" -> Test388Event.Error.Execution
        "timeout" -> Test388Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test388Event): String? = when (event) {
        is Test388Event.Entering.Self -> "entering"
        is Test388Event.Entering.S011 -> "entering.s011"
        is Test388Event.Entering.S012 -> "entering.s012"
        is Test388Event.Entering.S021 -> "entering.s021"
        is Test388Event.Entering.S022 -> "entering.s022"
        is Test388Event.Error.Execution -> "error.execution"
        is Test388Event.Timeout -> "timeout"
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
        engine.setupSystemVariables(sid, "test388")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
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
            raiseInternal(Test388Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test388Event) {
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
        state: Test388State,
        event: Test388Event
    ): TransitionResult<Test388State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test388State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test388State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s011 has no own event transitions)
        is Test388State.S011 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s012 has no own event transitions)
        is Test388State.S012 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test388State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s021 has no own event transitions)
        is Test388State.S021 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s022 has no own event transitions)
        is Test388State.S022 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test388State
    ): TransitionResult<Test388State> = when (state) {
        is Test388State.S1 -> processNullS1()
        is Test388State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test388State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test388State.S022)
    }

    private fun processNullS2(
    ): TransitionResult<Test388State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test388State.S021)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test388Event
    ): TransitionResult<Test388State> = when {
        event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test388State.S1, Test388State.S0)

        event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test388State.S2, Test388State.S0)

        // W3C SCXML 3.12.1: Prefix match for "entering"
        (event is Test388Event.Entering || event is Test388Event.Entering.S011 || event is Test388Event.Entering.S012 || event is Test388Event.Entering.S021 || event is Test388Event.Entering.S022) && safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        event is Test388Event.Entering.S011 && safeEvaluateGuard("Var1 == 3") -> TransitionResult.External(Test388State.Pass, Test388State.S0)

        // W3C SCXML 3.12.1: Prefix match for "entering"
        (event is Test388Event.Entering || event is Test388Event.Entering.S011 || event is Test388Event.Entering.S012 || event is Test388Event.Entering.S021 || event is Test388Event.Entering.S022) && safeEvaluateGuard("Var1 == 3") -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        event is Test388Event.Timeout -> TransitionResult.External(Test388State.Fail, Test388State.S0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test388State) {
        when (state) {
            is Test388State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test388State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test388State.S0 -> {
            executeAssign("Var1", "Var1 + 1")
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test388State.S01)
            }
            is Test388State.S01 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test388State.S011)
            }
            is Test388State.S011 -> {
            raiseInternal(Test388Event.Entering.S011)
            }
            is Test388State.S012 -> {
            raiseInternal(Test388Event.Entering.S012)
            }
            is Test388State.S02 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test388State.S021)
            }
            is Test388State.S021 -> {
            raiseInternal(Test388Event.Entering.S021)
            }
            is Test388State.S022 -> {
            raiseInternal(Test388Event.Entering.S022)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test388State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test388State,
        event: Test388Event?
    ) {
        when (source) {
        is Test388State.S0 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S01 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S011 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S012 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S02 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S021 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        is Test388State.S022 -> when {
            event is Test388Event.Entering.S012 && safeEvaluateGuard("Var1 == 1") -> {
            scheduleSend("__send_0", 2000L, Test388Event.Timeout)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
