// GENERATED CODE — DO NOT EDIT
// Source: resources/314/test314.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test314

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test314State : State {
    data object Fail : Test314State
    data object Pass : Test314State
    data object S0 : Test314State
    data object S01 : Test314State
    data object S02 : Test314State
    data object S03 : Test314State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test314Event : Event {
    sealed interface Error : Test314Event {
        data object Execution : Error
    }
    data object Foo : Test314Event
}
// --- State Machine (W3C SCXML) ---

class Test314StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test314State, Test314Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test314State = Test314State.S01

    // W3C SCXML 3.2/3.4: Enter from top-level initial state (recursive descent
    // through compound/parallel hierarchy to populate activeStateIds)
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        onEntry(Test314State.S0)
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test314State): Test314State? = when (state) {
        is Test314State.S01 -> Test314State.S0
        is Test314State.S02 -> Test314State.S0
        is Test314State.S03 -> Test314State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test314State): Test314State = when (state) {
        is Test314State.S0 -> Test314State.S01
        else -> state
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test314Event? = when (name) {
        "error.execution" -> Test314Event.Error.Execution
        "foo" -> Test314Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test314Event): String? = when (event) {
        is Test314Event.Error.Execution -> "error.execution"
        is Test314Event.Foo -> "foo"
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
        engine.setupSystemVariables(sid, "test314")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
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
            raiseInternal(Test314Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test314Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: return
        val sid = scriptSessionId ?: return
        val eventName = eventNameOf(event) ?: return
        val meta = currentEventMetadata
        engine.setCurrentEvent(
            sid, eventName,
            data = meta.data,
            type = meta.type,
            sendId = meta.sendId,
            origin = meta.origin.ifEmpty { scriptSessionId ?: "" },
            originType = meta.originType.ifEmpty { "http://www.w3.org/TR/scxml/#SCXMLEventProcessor" },
            invokeId = meta.invokeId
        )
    }

    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test314State,
        event: Test314Event
    ): TransitionResult<Test314State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test314State.S0 -> processS0(event)
        // W3C SCXML 3.13: Ancestor-only routing (s01 has no own event transitions)
        is Test314State.S01 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (s02 has no own event transitions)
        is Test314State.S02 -> {
            val anc1 = processS0(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is Test314State.S03 -> {
            val result = processS03(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test314State
    ): TransitionResult<Test314State> = when (state) {
        is Test314State.S01 -> processNullS01()
        is Test314State.S02 -> processNullS02()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS01(
    ): TransitionResult<Test314State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test314State.S02)
    }

    private fun processNullS02(
    ): TransitionResult<Test314State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test314State.S03)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test314Event
    ): TransitionResult<Test314State> = when {
        event is Test314Event.Error.Execution -> TransitionResult.External(Test314State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test314Event
    ): TransitionResult<Test314State> = when {
        event is Test314Event.Error.Execution -> TransitionResult.External(Test314State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test314State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test314State) {
        when (state) {
            is Test314State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test314State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test314State.S0 -> {
                // W3C SCXML 3.3: Enter initial child of compound state
                onEntry(Test314State.S01)
            }
            is Test314State.S03 -> {
            executeAssign("Var1", "undefined.invalidProperty")
            raiseInternal(Test314Event.Foo)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test314State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test314State,
        event: Test314Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
