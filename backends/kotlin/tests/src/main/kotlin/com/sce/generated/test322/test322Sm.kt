// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 74ba562b33766da248288b5dadec1e79a0ebb46a66e38786f6a7a4b2ccd653e3
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/322/test322.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test322.scxml:6 :: _machine

package com.sce.generated.test322

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test322State : State {
    data object Fail : Test322State
    data object Pass : Test322State
    data object S0 : Test322State
    data object S1 : Test322State
    data object S2 : Test322State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test322Event : Event {
    sealed interface Error : Test322Event {
        data object Execution : Error
    }
    data object Foo : Test322Event
}
// --- State Machine (W3C SCXML) ---

class Test322StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test322State, Test322Event>(scriptEngine) {

    // Datamodel (W3C SCXML 5.3)

    override val initialState: Test322State = Test322State.S0

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test322State? = when (stateId) {
        "fail" -> Test322State.Fail
        "pass" -> Test322State.Pass
        "s0" -> Test322State.S0
        "s1" -> Test322State.S1
        "s2" -> Test322State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test322State): String = when (state) {
        is Test322State.Fail -> "fail"
        is Test322State.Pass -> "pass"
        is Test322State.S0 -> "s0"
        is Test322State.S1 -> "s1"
        is Test322State.S2 -> "s2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test322State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test322State): Int = when (state) {
        is Test322State.Fail -> 4
        is Test322State.Pass -> 3
        is Test322State.S0 -> 0
        is Test322State.S1 -> 1
        is Test322State.S2 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test322Event? = when (name) {
        "error.execution" -> Test322Event.Error.Execution
        "foo" -> Test322Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test322Event): String? = when (event) {
        is Test322Event.Error.Execution -> "error.execution"
        is Test322Event.Foo -> "foo"
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
            "machineName",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "_sessionid")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raiseInternal(Test322Event.Error.Execution)
        }
        // W3C SCXML 5.2: Runtime variable 'Var2' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var2", null)
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
            raiseInternal(Test322Event.Error.Execution)
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
            raiseInternal(Test322Event.Error.Execution)
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
            raiseInternal(Test322Event.Error.Execution)
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test322Event) {
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
        // W3C SCXML C.1: `_event.origin` is the sender's published
        // `_ioprocessors` location, not its bare session id — and this is the
        // one place that publishes `_event` to the document, so this is where
        // the id becomes a location. The engine keeps the bare id in
        // `EventMetadata.origin` because its session-keyed lookups (`<finalize>`
        // dispatch, cancelled-invoke filtering) match on it; converting at the
        // raise would make one value serve two consumers that need different
        // spellings. The conversion itself lives in
        // `com.sce.runtime.IoProcessors.publishedOrigin`, the port of the
        // `IOProcessorHelper::publishedOrigin` the C++ engines share: a second
        // spelling of the rule is how the backends would stop agreeing.
        val effectiveOrigin = com.sce.runtime.IoProcessors.publishedOrigin(
            if (meta.type == "external") meta.origin.ifEmpty { scriptSessionId ?: "" } else meta.origin
        )
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
        state: Test322State,
        event: Test322Event
    ): TransitionResult<Test322State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test322State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test322State
    ): TransitionResult<Test322State> = when (state) {
        is Test322State.S0 -> processNullS0()
        is Test322State.S2 -> processNullS2()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test322State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test322State.S1, Test322State.S0)
    }

    private fun processNullS2(
    ): TransitionResult<Test322State> = when {
        safeEvaluateGuard("Var1 == _sessionid") -> TransitionResult.External(Test322State.Pass, Test322State.S2)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test322State.Fail, Test322State.S2)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test322Event
    ): TransitionResult<Test322State> = when {
        event is Test322Event.Error.Execution -> TransitionResult.External(Test322State.S2, Test322State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test322State.Fail, Test322State.S1)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test322.scxml:6 :: _machine
    override fun onEntry(state: Test322State) {
        when (state) {
            is Test322State.Fail -> {
                // SCE-MAP: test322.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test322State.Pass -> {
                // SCE-MAP: test322.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test322State.S0 -> {
                // SCE-MAP: test322.scxml:12 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return
            }
            is Test322State.S1 -> {
                // SCE-MAP: test322.scxml:17 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return


            executeAssign("_sessionid", "'otherName'")

            raiseInternal(Test322Event.Foo)
            }
            is Test322State.S2 -> {
                // SCE-MAP: test322.scxml:27 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test322.scxml:6 :: _machine
    override fun onExit(state: Test322State) {
        when (state) {
            is Test322State.Fail -> {
                // SCE-MAP: test322.scxml:35 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test322State.Pass -> {
                // SCE-MAP: test322.scxml:34 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test322State.S0 -> {
                // SCE-MAP: test322.scxml:12 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test322State.S1 -> {
                // SCE-MAP: test322.scxml:17 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test322State.S2 -> {
                // SCE-MAP: test322.scxml:27 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test322.scxml:6 :: _machine
    override fun executeTransitionActions(
        source: Test322State,
        event: Test322Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
