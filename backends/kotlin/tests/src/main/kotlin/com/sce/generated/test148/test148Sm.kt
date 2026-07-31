// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: c6c9654e14987bf9fee21998d111ca1385c48c09f2deb9cc862525d124525214
// generated-at: 1785480867

// GENERATED CODE — DO NOT EDIT
// Source: resources/148/test148.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test148.scxml:7

package com.sce.generated.test148

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test148State : State {
    data object Fail : Test148State
    data object Pass : Test148State
    data object S0 : Test148State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test148Event : Event {
    data object Bar : Test148Event
    data object Bat : Test148Event
    data object Baz : Test148Event
    sealed interface Error : Test148Event {
        data object Execution : Error
    }
    data object Foo : Test148Event
}
// --- State Machine (W3C SCXML) ---

class Test148StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test148State, Test148Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test148State = Test148State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test148State? = when (stateId) {
        "fail" -> Test148State.Fail
        "pass" -> Test148State.Pass
        "s0" -> Test148State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test148State): String = when (state) {
        is Test148State.Fail -> "fail"
        is Test148State.Pass -> "pass"
        is Test148State.S0 -> "s0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test148State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test148State): Int = when (state) {
        is Test148State.Fail -> 2
        is Test148State.Pass -> 1
        is Test148State.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test148Event? = when (name) {
        "bar" -> Test148Event.Bar
        "bat" -> Test148Event.Bat
        "baz" -> Test148Event.Baz
        "error.execution" -> Test148Event.Error.Execution
        "foo" -> Test148Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test148Event): String? = when (event) {
        is Test148Event.Bar -> "bar"
        is Test148Event.Bat -> "bat"
        is Test148Event.Baz -> "baz"
        is Test148Event.Error.Execution -> "error.execution"
        is Test148Event.Foo -> "foo"
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
            "test148",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
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
            raiseInternal(Test148Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test148Event) {
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
        state: Test148State,
        event: Test148Event
    ): TransitionResult<Test148State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test148State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test148Event
    ): TransitionResult<Test148State> = when {
        event is Test148Event.Baz && safeEvaluateGuard("Var1 == 1") -> TransitionResult.External(Test148State.Pass, Test148State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test148State.Fail, Test148State.S0)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test148.scxml:7
    override fun onEntry(state: Test148State) {
        when (state) {
            is Test148State.Fail -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test148State.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test148State.S0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            if (safeEvaluateGuard("false")) {

            raiseInternal(Test148Event.Foo)


            executeAssign("Var1", "Var1 + 1")
            } else if (safeEvaluateGuard("false")) {

            raiseInternal(Test148Event.Bar)


            executeAssign("Var1", "Var1 + 1")
            } else {

            raiseInternal(Test148Event.Baz)


            executeAssign("Var1", "Var1 + 1")
            }

            raiseInternal(Test148Event.Bat)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test148.scxml:7
    override fun onExit(state: Test148State) {
        when (state) {
            is Test148State.Fail -> {
                activeStateIds.remove("fail")
            }
            is Test148State.Pass -> {
                activeStateIds.remove("pass")
            }
            is Test148State.S0 -> {
                activeStateIds.remove("s0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test148.scxml:7
    override fun executeTransitionActions(
        source: Test148State,
        event: Test148Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
