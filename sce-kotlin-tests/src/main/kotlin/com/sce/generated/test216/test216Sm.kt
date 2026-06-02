// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: beb72c3a9cb76e61aa4916ff585cb6a1d22e66c189bf8cc96c5023dec391d982
// generated-at: 1780379958

// GENERATED CODE — DO NOT EDIT
// Source: resources/216/test216.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test216.scxml:7

package com.sce.generated.test216

import com.sce.runtime.*
import com.sce.interpreter.ScxmlRuntimeInterpreter


// --- States (W3C SCXML 3.2) ---

sealed interface Test216State : State {
    data object Fail : Test216State
    data object Pass : Test216State
    data object S0 : Test216State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test216Event : Event {
    sealed interface Done : Test216Event {
        data object Invoke : Done
    }
    sealed interface Error : Test216Event {
        data object Execution : Error
    }
    data object Timeout : Test216Event
}
// --- State Machine (W3C SCXML) ---

class Test216StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test216State, Test216Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test216State = Test216State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test216State? = when (stateId) {
        "fail" -> Test216State.Fail
        "pass" -> Test216State.Pass
        "s0" -> Test216State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test216State): String = when (state) {
        is Test216State.Fail -> "fail"
        is Test216State.Pass -> "pass"
        is Test216State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test216State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test216State): Int = when (state) {
        is Test216State.Fail -> 2
        is Test216State.Pass -> 1
        is Test216State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test216Event? = when (name) {
        "done.invoke" -> Test216Event.Done.Invoke
        "error.execution" -> Test216Event.Error.Execution
        "timeout" -> Test216Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test216Event): String? = when (event) {
        is Test216Event.Done.Invoke -> "done.invoke"
        is Test216Event.Error.Execution -> "error.execution"
        is Test216Event.Timeout -> "timeout"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test216")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "'foo'")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test216Event.Error.Execution)
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
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raiseInternal(Test216Event.Error.Execution)
            false
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    private fun executeAssign(location: String, expr: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raiseInternal(Test216Event.Error.Execution)
        }
    }

    // W3C SCXML 3.8.6: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test216Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test216Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
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
            sid,
            com.sce.runtime.SetCurrentEventArgs(
                name = eventName,
                data = meta.data,
                type = effectiveType,
                sendId = meta.sendId,
                origin = effectiveOrigin,
                originType = effectiveOriginType,
                invokeId = meta.invokeId
            )
        )
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test216State,
        event: Test216Event
    ): TransitionResult<Test216State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test216State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test216Event
    ): TransitionResult<Test216State> = when {
        event is Test216Event.Done.Invoke -> TransitionResult.External(Test216State.Pass, Test216State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test216State.Fail, Test216State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test216.scxml:7
    override fun onEntry(state: Test216State) {
        when (state) {
            is Test216State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test216State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test216State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test216Event.Timeout)


            executeAssign("Var1", "'file:test216sub1.scxml'")
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "s0.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        ensureScriptEngine()
                        val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        try {
                            // W3C SCXML 6.4.3: Evaluate srcexpr → file path → load SCXML → create child
                            val pathResult = eng.evaluateExpr(sid, "Var1")
                            val filePath = pathResult?.toString() ?: return@deferInvoke
                            val childSM = ScxmlRuntimeInterpreter.fromFile(filePath, "resources/216", scriptEngine)
                            startInvoke("_invoke_0", childSM, false, Test216Event.Done.Invoke, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: Expression evaluation or child creation failed (C++ parity)
                            raiseInternal(Test216Event.Error.Execution)
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test216.scxml:7
    override fun onExit(state: Test216State) {
        when (state) {
            is Test216State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test216State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test216State.S0 -> {
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test216.scxml:7
    override fun executeTransitionActions(
        source: Test216State,
        event: Test216Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
