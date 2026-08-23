// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f8628fc45ae1ba8d3b0272fbb37ab2b3fa73e6bcc8f28ed51f64ec3e41941c33
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/351/test351.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test351.scxml:5 :: _machine

package com.sce.generated.test351

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test351State : State {
    data object Fail : Test351State
    data object Pass : Test351State
    data object S0 : Test351State
    data object S1 : Test351State
    data object S2 : Test351State
    data object S3 : Test351State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test351Event : Event {
    sealed interface Error : Test351Event {
        data object Execution : Error
    }
    data object S0Event : Test351Event
    data object S0Event2 : Test351Event
    data object Timeout : Test351Event
}
// --- State Machine (W3C SCXML) ---

class Test351StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test351State, Test351Event>(scriptEngine) {

    override val initialState: Test351State = Test351State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test351State? = when (stateId) {
        "fail" -> Test351State.Fail
        "pass" -> Test351State.Pass
        "s0" -> Test351State.S0
        "s1" -> Test351State.S1
        "s2" -> Test351State.S2
        "s3" -> Test351State.S3
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test351State): String = when (state) {
        is Test351State.Fail -> "fail"
        is Test351State.Pass -> "pass"
        is Test351State.S0 -> "s0"
        is Test351State.S1 -> "s1"
        is Test351State.S2 -> "s2"
        is Test351State.S3 -> "s3"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test351State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test351State): Int = when (state) {
        is Test351State.Fail -> 5
        is Test351State.Pass -> 4
        is Test351State.S0 -> 0
        is Test351State.S1 -> 1
        is Test351State.S2 -> 2
        is Test351State.S3 -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test351Event? = when (name) {
        "error.execution" -> Test351Event.Error.Execution
        "s0Event" -> Test351Event.S0Event
        "s0Event2" -> Test351Event.S0Event2
        "timeout" -> Test351Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test351Event): String? = when (event) {
        is Test351Event.Error.Execution -> "error.execution"
        is Test351Event.S0Event -> "s0Event"
        is Test351Event.S0Event2 -> "s0Event2"
        is Test351Event.Timeout -> "timeout"
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
            "test351",
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
            raisePlatformError(Test351Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test351Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test351Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test351Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test351Event) {
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
        // §scxml-B-2-8-1: the binding answers which rung the payload got, and
        // that answer used to end here. The ladder decided between a DOM, a
        // value and a space-normalized string, and the decision was dropped —
        // so a payload that announced structure and would not parse reached
        // the document as raw characters, every `_event.data.<field>` read
        // empty, and nothing anywhere could say so.
        //
        // Recorded on the spot rather than returned up: this class extends
        // `StateMachineEngine`, so the frame that binds already holds both the
        // reading and the event it belongs to — which is the pairing the count
        // needs.
        val payloadReading = engine.setCurrentEvent(
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
        notePayloadReading(event, payloadReading)
    }


    // W3C SCXML 3.12: Event processing with script engine condition evaluation
    override fun processEvent(
        state: Test351State,
        event: Test351Event
    ): TransitionResult<Test351State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test351State.S0 -> processS0(event)
        is Test351State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test351State
    ): TransitionResult<Test351State> = when (state) {
        is Test351State.S1 -> processNullS1()
        is Test351State.S3 -> processNullS3()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test351State> = when {
        safeEvaluateGuard("Var1 == 'send1'") -> TransitionResult.External(Test351State.S2, Test351State.S1)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test351State.Fail, Test351State.S1)
    }

    private fun processNullS3(
    ): TransitionResult<Test351State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test351State.Pass, Test351State.S3)
    }

    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test351Event
    ): TransitionResult<Test351State> = when {
        event is Test351Event.S0Event -> TransitionResult.External(Test351State.S1, Test351State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test351State.Fail, Test351State.S0)
    }

    private fun processS2(
        event: Test351Event
    ): TransitionResult<Test351State> = when {
        event is Test351Event.S0Event2 -> TransitionResult.External(Test351State.S3, Test351State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test351State.Fail, Test351State.S2)
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test351.scxml:5 :: _machine
    override fun onEntry(state: Test351State, pathChild: Test351State?) {
        when (state) {
            is Test351State.Fail -> {
                // SCE-MAP: test351.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test351State.Pass -> {
                // SCE-MAP: test351.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test351State.S0 -> {
                // SCE-MAP: test351.scxml:12 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 5000L, Test351Event.Timeout)


            send(Test351Event.S0Event, EventMetadata.external(sendId = "send1", origin = scriptSessionId ?: ""))
            }
            is Test351State.S1 -> {
                // SCE-MAP: test351.scxml:26 :: s1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s1")) return
            }
            is Test351State.S2 -> {
                // SCE-MAP: test351.scxml:31 :: s2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s2")) return


            scheduleSend("__send_1", 5000L, Test351Event.Timeout)


            send(Test351Event.S0Event2, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: ""))
            }
            is Test351State.S3 -> {
                // SCE-MAP: test351.scxml:43 :: s3 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s3")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test351.scxml:5 :: _machine
    override fun onExit(state: Test351State) {
        when (state) {
            is Test351State.Fail -> {
                // SCE-MAP: test351.scxml:50 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test351State.Pass -> {
                // SCE-MAP: test351.scxml:49 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test351State.S0 -> {
                // SCE-MAP: test351.scxml:12 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test351State.S1 -> {
                // SCE-MAP: test351.scxml:26 :: s1 :: _state_body
                activeStateIds.remove("s1")
            }
            is Test351State.S2 -> {
                // SCE-MAP: test351.scxml:31 :: s2 :: _state_body
                activeStateIds.remove("s2")
            }
            is Test351State.S3 -> {
                // SCE-MAP: test351.scxml:43 :: s3 :: _state_body
                activeStateIds.remove("s3")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test351.scxml:5 :: _machine
    override fun executeTransitionActions(
        source: Test351State,
        event: Test351Event?
    ) {
        when (source) {
        is Test351State.S0 -> when {
            event is Test351Event.S0Event -> {
                // SCE-MAP: test351.scxml:18 :: s0 :: _transition_0


            executeAssign("Var1", "_event.sendid")
            }
            else -> {}
        }
        is Test351State.S2 -> when {
            event is Test351Event.S0Event2 -> {
                // SCE-MAP: test351.scxml:37 :: s2 :: _transition_0


            executeAssign("Var2", "_event.sendid")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
