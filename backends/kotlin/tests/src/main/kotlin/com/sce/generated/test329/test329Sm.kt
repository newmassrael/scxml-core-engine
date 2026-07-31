// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 7aab3b29aa8f5ef17f1c8730c3954aecc89c78aabf4a2226d70ddd8c24038efe
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/329/test329.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test329.scxml:3

package com.sce.generated.test329

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test329State : State {
    data object Fail : Test329State
    data object Pass : Test329State
    data object S0 : Test329State
    data object S1 : Test329State
    data object S2 : Test329State
    data object S3 : Test329State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test329Event : Event {
    sealed interface Error : Test329Event {
        data object Execution : Error
    }
    data object Foo : Test329Event
}
// --- State Machine (W3C SCXML) ---

class Test329StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test329State, Test329Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test329State = Test329State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test329State? = when (stateId) {
        "fail" -> Test329State.Fail
        "pass" -> Test329State.Pass
        "s0" -> Test329State.S0
        "s1" -> Test329State.S1
        "s2" -> Test329State.S2
        "s3" -> Test329State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test329State): String = when (state) {
        is Test329State.Fail -> "fail"
        is Test329State.Pass -> "pass"
        is Test329State.S0 -> "s0"
        is Test329State.S1 -> "s1"
        is Test329State.S2 -> "s2"
        is Test329State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test329State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test329State): Int = when (state) {
        is Test329State.Fail -> 5
        is Test329State.Pass -> 4
        is Test329State.S0 -> 0
        is Test329State.S1 -> 1
        is Test329State.S2 -> 2
        is Test329State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test329Event? = when (name) {
        "error.execution" -> Test329Event.Error.Execution
        "foo" -> Test329Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test329Event): String? = when (event) {
        is Test329Event.Error.Execution -> "error.execution"
        is Test329Event.Foo -> "foo"
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
            "test329",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
        } catch (_: Exception) {}
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
        // W3C SCXML 5.2: Runtime variable 'Var4' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var4", null)
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
            raiseInternal(Test329Event.Error.Execution)
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
            raiseInternal(Test329Event.Error.Execution)
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
            raiseInternal(Test329Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test329Event) {
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
        state: Test329State,
        event: Test329Event
    ): TransitionResult<Test329State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test329State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test329State
    ): TransitionResult<Test329State> = when (state) {
        is Test329State.S1 -> processNullS1()
        is Test329State.S2 -> processNullS2()
        is Test329State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var2 == _event") -> TransitionResult.External(Test329State.S2, Test329State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail, Test329State.S1)
    }

    private fun processNullS2(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var3 == _name") -> TransitionResult.External(Test329State.S3, Test329State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail, Test329State.S2)
    }

    private fun processNullS3(
    ): TransitionResult<Test329State> = when {
        safeEvaluateGuard("Var4 == _ioprocessors") -> TransitionResult.External(Test329State.Pass, Test329State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test329State.Fail, Test329State.S3)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test329Event
    ): TransitionResult<Test329State> = when {
        event is Test329Event.Foo && safeEvaluateGuard("Var1 == _sessionid") -> TransitionResult.External(Test329State.S1, Test329State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test329State.Fail, Test329State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test329.scxml:3
    override fun onEntry(state: Test329State) {
        when (state) {
            is Test329State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test329State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test329State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test329Event.Foo)


            executeAssign("Var1", "_sessionid")


            executeAssign("_sessionid", "'invalid_session_id'")
            }
            is Test329State.S1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            executeAssign("Var2", "_event")


            executeAssign("_event", "27")
            }
            is Test329State.S2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            executeAssign("Var3", "_name")


            executeAssign("_name", "27")
            }
            is Test329State.S3 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return


            executeAssign("Var4", "_ioprocessors")


            executeAssign("_ioprocessors", "27")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test329.scxml:3
    override fun onExit(state: Test329State) {
        when (state) {
            is Test329State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test329State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test329State.S0 -> {
                activeStateIds.remove("s0")
            }
            is Test329State.S1 -> {
                activeStateIds.remove("s1")
            }
            is Test329State.S2 -> {
                activeStateIds.remove("s2")
            }
            is Test329State.S3 -> {
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test329.scxml:3
    override fun executeTransitionActions(
        source: Test329State,
        event: Test329Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
