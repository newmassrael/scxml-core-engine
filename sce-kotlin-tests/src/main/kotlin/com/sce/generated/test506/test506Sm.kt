// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 2c4f76809986b4347703e89a8e901379e8391f815371b53c5a7eecbe187e1cf5
// generated-at: 1781081955

// GENERATED CODE — DO NOT EDIT
// Source: resources/506/test506.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test506.scxml:6

package com.sce.generated.test506

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test506State : State {
    data object Fail : Test506State
    data object Pass : Test506State
    data object S1 : Test506State
    data object S2 : Test506State
    data object S21 : Test506State
    data object S3 : Test506State
    data object S4 : Test506State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test506Event : Event {
    data object Bar : Test506Event
    sealed interface Error : Test506Event {
        data object Execution : Error
    }
    data object Foo : Test506Event
}
// --- State Machine (W3C SCXML) ---

class Test506StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test506State, Test506Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test506State = Test506State.S1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test506State): Test506State? = when (state) {
        is Test506State.S21 -> Test506State.S2
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test506State): Test506State = when (state) {
        is Test506State.S2 -> Test506State.S21
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test506State? = when (stateId) {
        "fail" -> Test506State.Fail
        "pass" -> Test506State.Pass
        "s1" -> Test506State.S1
        "s2" -> Test506State.S2
        "s21" -> Test506State.S21
        "s3" -> Test506State.S3
        "s4" -> Test506State.S4
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test506State): String = when (state) {
        is Test506State.Fail -> "fail"
        is Test506State.Pass -> "pass"
        is Test506State.S1 -> "s1"
        is Test506State.S2 -> "s2"
        is Test506State.S21 -> "s21"
        is Test506State.S3 -> "s3"
        is Test506State.S4 -> "s4"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test506State): Boolean = when (state) {
        is Test506State.S2 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test506State): Int = when (state) {
        is Test506State.Fail -> 6
        is Test506State.Pass -> 5
        is Test506State.S1 -> 0
        is Test506State.S2 -> 1
        is Test506State.S21 -> 2
        is Test506State.S3 -> 3
        is Test506State.S4 -> 4
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test506Event? = when (name) {
        "bar" -> Test506Event.Bar
        "error.execution" -> Test506Event.Error.Execution
        "foo" -> Test506Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test506Event): String? = when (event) {
        is Test506Event.Bar -> "bar"
        is Test506Event.Error.Execution -> "error.execution"
        is Test506Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test506")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test506Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test506Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var3' with expr
        try {
            val initResult_Var3 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var3", initResult_Var3)
        } catch (e: Exception) {
            raiseInternal(Test506Event.Error.Execution)
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
            raiseInternal(Test506Event.Error.Execution)
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
            raiseInternal(Test506Event.Error.Execution)
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
            raiseInternal(Test506Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test506Event) {
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
        state: Test506State,
        event: Test506Event
    ): TransitionResult<Test506State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test506State.S2 -> processS2(event)
        // W3C SCXML 3.13: Ancestor-only routing (s21 has no own event transitions)
        is Test506State.S21 -> {
            val anc1 = processS2(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test506State
    ): TransitionResult<Test506State> = when (state) {
        is Test506State.S1 -> processNullS1()
        is Test506State.S3 -> processNullS3()
        is Test506State.S4 -> processNullS4()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test506State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test506State.S2, Test506State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test506State> = when {
        safeEvaluateGuard("Var1 == 2") -> TransitionResult.External(Test506State.S4, Test506State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test506State.Fail, Test506State.S3)
    }

    private fun processNullS4(
    ): TransitionResult<Test506State> = when {
        safeEvaluateGuard("Var2 == 2") -> TransitionResult.External(Test506State.Pass, Test506State.S4)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test506State.Fail, Test506State.S4)
    }

    // --- Per-State Event Handlers ---

    private fun processS2(
        event: Test506Event
    ): TransitionResult<Test506State> = when {
        event is Test506Event.Foo -> TransitionResult.External(Test506State.S2, Test506State.S2)

        event is Test506Event.Bar && safeEvaluateGuard("Var3 == 1") -> TransitionResult.External(Test506State.S3, Test506State.S2)

        event is Test506Event.Bar -> TransitionResult.External(Test506State.Fail, Test506State.S2)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test506.scxml:6
    override fun onEntry(state: Test506State) {
        when (state) {
            is Test506State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test506State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test506State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test506Event.Foo)

            raiseInternal(Test506Event.Bar)
            }
            is Test506State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test506State.S21 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s21")) return
            }
            is Test506State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
            is Test506State.S4 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s4")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test506.scxml:6
    override fun onExit(state: Test506State) {
        when (state) {
            is Test506State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test506State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test506State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test506State.S2 -> {
                activeStateIds.remove("s2")


            executeAssign("Var1", "Var1 + 1")
            }
            is Test506State.S21 -> {
                activeStateIds.remove("s21")


            executeAssign("Var2", "Var2 + 1")
            }
            is Test506State.S3 -> {
                activeStateIds.remove("s3")
            }
            is Test506State.S4 -> {
                activeStateIds.remove("s4")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test506.scxml:6
    override fun executeTransitionActions(
        source: Test506State,
        event: Test506Event?
    ) {
        when (source) {
        is Test506State.S2 -> when {
            event is Test506Event.Foo -> {


            executeAssign("Var3", "Var3 + 1")
            }
            else -> {}
        }
        is Test506State.S21 -> when {
            event is Test506Event.Foo -> {


            executeAssign("Var3", "Var3 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
