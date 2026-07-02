// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: b5e91c83753cb468c86997c5541ac646288562f682111eb4bbd825060d84bc2e
// generated-at: 1782963882

// GENERATED CODE — DO NOT EDIT
// Source: resources/457/test457.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test457.scxml:5

package com.sce.generated.test457

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test457State : State {
    data object Fail : Test457State
    data object Pass : Test457State
    data object S0 : Test457State
    data object S1 : Test457State
    data object S2 : Test457State
    data object S3 : Test457State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test457Event : Event {
    data object Bar : Test457Event
    sealed interface Error : Test457Event {
        data object Execution : Error
    }
    data object Foo : Test457Event
}
// --- State Machine (W3C SCXML) ---

class Test457StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test457State, Test457Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test457State = Test457State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test457State? = when (stateId) {
        "fail" -> Test457State.Fail
        "pass" -> Test457State.Pass
        "s0" -> Test457State.S0
        "s1" -> Test457State.S1
        "s2" -> Test457State.S2
        "s3" -> Test457State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test457State): String = when (state) {
        is Test457State.Fail -> "fail"
        is Test457State.Pass -> "pass"
        is Test457State.S0 -> "s0"
        is Test457State.S1 -> "s1"
        is Test457State.S2 -> "s2"
        is Test457State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test457State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test457State): Int = when (state) {
        is Test457State.Fail -> 5
        is Test457State.Pass -> 4
        is Test457State.S0 -> 0
        is Test457State.S1 -> 1
        is Test457State.S2 -> 2
        is Test457State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test457Event? = when (name) {
        "bar" -> Test457Event.Bar
        "error.execution" -> Test457Event.Error.Execution
        "foo" -> Test457Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test457Event): String? = when (event) {
        is Test457Event.Bar -> "bar"
        is Test457Event.Error.Execution -> "error.execution"
        is Test457Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // W3C SCXML 5.10: Setup system variables (_sessionid, _name, _ioprocessors)
        engine.setupSystemVariables(sid, "test457")

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test457Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.2: Runtime variable 'Var3' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var3", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.3: Initialize variable 'Var4' with expr
        try {
            val initResult_Var4 = engine.evaluateExpr(sid, "7")
            engine.setVariable(sid, "Var4", initResult_Var4)
        } catch (e: Exception) {
            raiseInternal(Test457Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var5' with expr
        try {
            val initResult_Var5 = engine.evaluateExpr(sid, "[1,2,3]")
            engine.setVariable(sid, "Var5", initResult_Var5)
        } catch (e: Exception) {
            raiseInternal(Test457Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'Var6' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var6", null)
        } catch (_: Exception) {}




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
            raiseInternal(Test457Event.Error.Execution)
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
            raiseInternal(Test457Event.Error.Execution)
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
            raiseInternal(Test457Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test457Event) {
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
        state: Test457State,
        event: Test457Event
    ): TransitionResult<Test457State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test457State.S0 -> processS0(event)
        is Test457State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test457State
    ): TransitionResult<Test457State> = when (state) {
        is Test457State.S2 -> processNullS2()
        is Test457State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS2(
    ): TransitionResult<Test457State> = when {
        safeEvaluateGuard("Var1==0") -> TransitionResult.External(Test457State.S3, Test457State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test457State.Fail, Test457State.S2)
    }

    private fun processNullS3(
    ): TransitionResult<Test457State> = when {
        safeEvaluateGuard("Var6==6") -> TransitionResult.External(Test457State.Pass, Test457State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test457State.Fail, Test457State.S3)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test457Event
    ): TransitionResult<Test457State> = when {
        event is Test457Event.Error.Execution -> TransitionResult.External(Test457State.S1, Test457State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test457State.Fail, Test457State.S0)
    }

    private fun processS1(
        event: Test457Event
    ): TransitionResult<Test457State> = when {
        event is Test457Event.Error.Execution -> TransitionResult.External(Test457State.S2, Test457State.S1)

        event is Test457Event.Bar -> TransitionResult.External(Test457State.Fail, Test457State.S1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test457.scxml:5
    override fun onEntry(state: Test457State) {
        when (state) {
            is Test457State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return

            // W3C SCXML 3.8.8: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'fail'")?.toString() ?: ""))
            } catch (_: Exception) {}
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test457State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return

            // W3C SCXML 3.8.8: Log expression evaluation (non-fatal on error, C++ pattern)
            try {
                println("Outcome: " + (scriptEngine?.evaluateExpr(scriptSessionId ?: "", "'pass'")?.toString() ?: ""))
            } catch (_: Exception) {}
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test457State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, "Var4", "Var2", "Var3") {


            engine.assign(sid, "Var1", "Var1 + 1")
                    }
                } catch (e: Exception) {
                    raiseInternal(Test457Event.Error.Execution)
                }
            }

            raiseInternal(Test457Event.Foo)
            }
            is Test457State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, "Var5", "'continue'", "Var3") {


            engine.assign(sid, "Var1", "Var1 + 1")
                    }
                } catch (e: Exception) {
                    raiseInternal(Test457Event.Error.Execution)
                }
            }

            raiseInternal(Test457Event.Bar)
            }
            is Test457State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test457State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return


            executeAssign("Var6", "0")


            run {
                ensureScriptEngine()
                val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    engine.executeForeach(sid, "Var5", "Var2", "") {


            engine.assign(sid, "Var6", "Var6 + Var2")
                    }
                } catch (e: Exception) {
                    raiseInternal(Test457Event.Error.Execution)
                }
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test457.scxml:5
    override fun onExit(state: Test457State) {
        when (state) {
            is Test457State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test457State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test457State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test457State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test457State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test457State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test457.scxml:5
    override fun executeTransitionActions(
        source: Test457State,
        event: Test457Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
