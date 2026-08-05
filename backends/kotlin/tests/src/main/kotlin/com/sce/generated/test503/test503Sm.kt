// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 59f8d3cf0f729f691caba7296b1e49d1e9a1888fee49dbe7c62233edc3993473
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/503/test503.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test503.scxml:5

package com.sce.generated.test503

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test503State : State {
    data object Fail : Test503State
    data object Pass : Test503State
    data object S1 : Test503State
    data object S2 : Test503State
    data object S3 : Test503State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test503Event : Event {
    data object Bar : Test503Event
    sealed interface Error : Test503Event {
        data object Execution : Error
    }
    data object Foo : Test503Event
}
// --- State Machine (W3C SCXML) ---

class Test503StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test503State, Test503Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test503State = Test503State.S1

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test503State? = when (stateId) {
        "fail" -> Test503State.Fail
        "pass" -> Test503State.Pass
        "s1" -> Test503State.S1
        "s2" -> Test503State.S2
        "s3" -> Test503State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test503State): String = when (state) {
        is Test503State.Fail -> "fail"
        is Test503State.Pass -> "pass"
        is Test503State.S1 -> "s1"
        is Test503State.S2 -> "s2"
        is Test503State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test503State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test503State): Int = when (state) {
        is Test503State.Fail -> 4
        is Test503State.Pass -> 3
        is Test503State.S1 -> 0
        is Test503State.S2 -> 1
        is Test503State.S3 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test503Event? = when (name) {
        "bar" -> Test503Event.Bar
        "error.execution" -> Test503Event.Error.Execution
        "foo" -> Test503Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test503Event): String? = when (event) {
        is Test503Event.Bar -> "bar"
        is Test503Event.Error.Execution -> "error.execution"
        is Test503Event.Foo -> "foo"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "test503",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test503Event.Error.Execution)
        }
        // W3C SCXML 5.3: Initialize variable 'Var2' with expr
        try {
            val initResult_Var2 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var2", initResult_Var2)
        } catch (e: Exception) {
            raiseInternal(Test503Event.Error.Execution)
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
            raiseInternal(Test503Event.Error.Execution)
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
            raiseInternal(Test503Event.Error.Execution)
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: String) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raiseInternal(Test503Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test503Event) {
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
        state: Test503State,
        event: Test503Event
    ): TransitionResult<Test503State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test503State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test503State
    ): TransitionResult<Test503State> = when (state) {
        is Test503State.S1 -> processNullS1()
        is Test503State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test503State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test503State.S2, Test503State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test503State> = when {
        safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test503State.Pass, Test503State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test503State.Fail, Test503State.S3)
    }

    // --- Per-State Event Handlers ---

    private fun processS2(
        event: Test503Event
    ): TransitionResult<Test503State> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is Test503Event.Foo -> TransitionResult.Internal
        event is Test503Event.Bar && safeEvaluateGuard("Var2 == 1") -> TransitionResult.External(Test503State.S3, Test503State.S2)

        event is Test503Event.Bar -> TransitionResult.External(Test503State.Fail, Test503State.S2)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test503.scxml:5
    override fun onEntry(state: Test503State) {
        when (state) {
            is Test503State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test503State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test503State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return

            raiseInternal(Test503Event.Foo)

            raiseInternal(Test503Event.Bar)
            }
            is Test503State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
            is Test503State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test503.scxml:5
    override fun onExit(state: Test503State) {
        when (state) {
            is Test503State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test503State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test503State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test503State.S2 -> {
                activeStateIds.remove("s2")


            executeAssign("Var1", "Var1 + 1")
            }
            is Test503State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test503.scxml:5
    override fun executeTransitionActions(
        source: Test503State,
        event: Test503Event?
    ) {
        when (source) {
        is Test503State.S2 -> when {
            event is Test503Event.Foo -> {


            executeAssign("Var2", "Var2 + 1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
