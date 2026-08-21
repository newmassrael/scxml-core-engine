// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 84a841eae761d6fbf94d15cd646ae14f47646822f90559441b47e8f14bddfb19
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/331/test331.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test331.scxml:3 :: _machine

package com.sce.generated.test331

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test331State : State {
    data object Fail : Test331State
    data object Pass : Test331State
    data object S0 : Test331State
    data object S1 : Test331State
    data object S2 : Test331State
    data object S3 : Test331State
    data object S4 : Test331State
    data object S5 : Test331State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test331Event : Event {
    sealed interface Error : Test331Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Foo : Test331Event
}
// --- State Machine (W3C SCXML) ---

class Test331StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test331State, Test331Event>(scriptEngine) {

    override val initialState: Test331State = Test331State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test331State? = when (stateId) {
        "fail" -> Test331State.Fail
        "pass" -> Test331State.Pass
        "s0" -> Test331State.S0
        "s1" -> Test331State.S1
        "s2" -> Test331State.S2
        "s3" -> Test331State.S3
        "s4" -> Test331State.S4
        "s5" -> Test331State.S5
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test331State): String = when (state) {
        is Test331State.Fail -> "fail"
        is Test331State.Pass -> "pass"
        is Test331State.S0 -> "s0"
        is Test331State.S1 -> "s1"
        is Test331State.S2 -> "s2"
        is Test331State.S3 -> "s3"
        is Test331State.S4 -> "s4"
        is Test331State.S5 -> "s5"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test331State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test331State): Int = when (state) {
        is Test331State.Fail -> 7
        is Test331State.Pass -> 6
        is Test331State.S0 -> 0
        is Test331State.S1 -> 1
        is Test331State.S2 -> 2
        is Test331State.S3 -> 3
        is Test331State.S4 -> 4
        is Test331State.S5 -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test331Event? = when (name) {
        "error" -> Test331Event.Error.Self
        "error.execution" -> Test331Event.Error.Execution
        "foo" -> Test331Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test331Event): String? = when (event) {
        is Test331Event.Error.Self -> "error"
        is Test331Event.Error.Execution -> "error.execution"
        is Test331Event.Foo -> "foo"
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

        // W3C SCXML 5.2: Runtime variable 'Var1' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "Var1", null)
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
            raisePlatformError(Test331Event.Error.Execution, "a <transition> cond failed to evaluate")
            false
        }
    }

    // W3C SCXML B.2: the value of an inline `<content>` body, serialized
    // for transport.
    //
    // The reading is decided at build time — `source` is already the
    // expression or string literal the clause's ordered readings give —
    // and this evaluates it *here*, at send time, rather than handing the
    // expression to whatever reads `_event.data` later. That distinction
    // is not academic: the two engines this backend runs on disagree
    // about what a data string is. QuickJS tries a JS evaluation before
    // falling back; Rhino goes straight from JSON to the normalized
    // string, so an expression handed to it arrives as its own source
    // text. `JSON.stringify` is what both of them can read back, and it
    // is the same shape the C++ backend transports.
    private fun evaluateSendContent(source: String): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateExpr(sid, "JSON.stringify((" + source + "))")?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(Test331Event.Error.Execution, "an expression could not be serialised to JSON")
            ""
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
            raisePlatformError(Test331Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test331Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test331Event) {
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
        state: Test331State,
        event: Test331Event
    ): TransitionResult<Test331State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test331State.S0 -> processS0(event)
        is Test331State.S2 -> processS2(event)
        is Test331State.S4 -> processS4(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test331State
    ): TransitionResult<Test331State> = when (state) {
        is Test331State.S1 -> processNullS1()
        is Test331State.S3 -> processNullS3()
        is Test331State.S5 -> processNullS5()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'internal'") -> TransitionResult.External(Test331State.S2, Test331State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail, Test331State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'platform'") -> TransitionResult.External(Test331State.S4, Test331State.S3)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail, Test331State.S3)
    }

    private fun processNullS5(
    ): TransitionResult<Test331State> = when {
        safeEvaluateGuard("Var1 == 'external'") -> TransitionResult.External(Test331State.Pass, Test331State.S5)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test331State.Fail, Test331State.S5)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        event is Test331Event.Foo -> TransitionResult.External(Test331State.S1, Test331State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail, Test331State.S0)
    }

    private fun processS2(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test331Event.Error || event is Test331Event.Error.Execution) -> TransitionResult.External(Test331State.S3, Test331State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail, Test331State.S2)
    }

    private fun processS4(
        event: Test331Event
    ): TransitionResult<Test331State> = when {
        event is Test331Event.Foo -> TransitionResult.External(Test331State.S5, Test331State.S4)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test331State.Fail, Test331State.S4)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test331.scxml:3 :: _machine
    override fun onEntry(state: Test331State, pathChild: Test331State?) {
        when (state) {
            is Test331State.Fail -> {
                // SCE-MAP: test331.scxml:60 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test331State.Pass -> {
                // SCE-MAP: test331.scxml:59 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test331State.S0 -> {
                // SCE-MAP: test331.scxml:10 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return

            raiseInternal(Test331Event.Foo)
            }
            is Test331State.S1 -> {
                // SCE-MAP: test331.scxml:21 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test331State.S2 -> {
                // SCE-MAP: test331.scxml:26 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            // W3C SCXML 5.3: Empty location raises error.execution (C++ ActionExecutorImpl pattern)
            raisePlatformError(Test331Event.Error.Execution, "<assign> has an invalid or read-only location")
            }
            is Test331State.S3 -> {
                // SCE-MAP: test331.scxml:37 :: s3 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
            is Test331State.S4 -> {
                // SCE-MAP: test331.scxml:42 :: s4 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s4")) return


            send(Test331Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            is Test331State.S5 -> {
                // SCE-MAP: test331.scxml:53 :: s5 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s5")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test331.scxml:3 :: _machine
    override fun onExit(state: Test331State) {
        when (state) {
            is Test331State.Fail -> {
                // SCE-MAP: test331.scxml:60 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test331State.Pass -> {
                // SCE-MAP: test331.scxml:59 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test331State.S0 -> {
                // SCE-MAP: test331.scxml:10 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test331State.S1 -> {
                // SCE-MAP: test331.scxml:21 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test331State.S2 -> {
                // SCE-MAP: test331.scxml:26 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
            is Test331State.S3 -> {
                // SCE-MAP: test331.scxml:37 :: s3 :: _state_body
                activeStateIds.remove("s3")
            }
            is Test331State.S4 -> {
                // SCE-MAP: test331.scxml:42 :: s4 :: _state_body
                activeStateIds.remove("s4")
            }
            is Test331State.S5 -> {
                // SCE-MAP: test331.scxml:53 :: s5 :: _state_body
                activeStateIds.remove("s5")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test331.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test331State,
        event: Test331Event?
    ) {
        when (source) {
        is Test331State.S0 -> when {
            event is Test331Event.Foo -> {
                // SCE-MAP: test331.scxml:15 :: s0 :: _transition_0


            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        is Test331State.S2 -> when {
            (event is Test331Event.Error || event is Test331Event.Error.Execution) -> {
                // SCE-MAP: test331.scxml:31 :: s2 :: _transition_0


            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        is Test331State.S4 -> when {
            event is Test331Event.Foo -> {
                // SCE-MAP: test331.scxml:47 :: s4 :: _transition_0


            executeAssign("Var1", "_event.type")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
