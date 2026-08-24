// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 2a328c6a2c55f2d381ea947b66337ce444ad937a90838cfa9cbdecc92a89b987
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/240/test240.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test240.scxml:8 :: _machine

package com.sce.generated.test240

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test240State : State {
    data object Fail : Test240State
    data object Pass : Test240State
    data object S0 : Test240State
    data object S01 : Test240State
    data object S02 : Test240State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test240Event : Event {
    sealed interface Cancel : Test240Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test240Event {
        data object Invoke : Done
    }
    sealed interface Error : Test240Event {
        data object Execution : Error
    }
    data object Failure : Test240Event
    data object Success : Test240Event
    data object Timeout : Test240Event
}
// --- State Machine (W3C SCXML) ---

class Test240StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test240State, Test240Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test240State = Test240State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test240State): Test240State? = when (state) {
        is Test240State.S01 -> Test240State.S0
        is Test240State.S02 -> Test240State.S0
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: Test240State): Test240State = when (state) {
        is Test240State.S0 -> Test240State.S01
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test240State? = when (stateId) {
        "fail" -> Test240State.Fail
        "pass" -> Test240State.Pass
        "s0" -> Test240State.S0
        "s01" -> Test240State.S01
        "s02" -> Test240State.S02
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test240State): String = when (state) {
        is Test240State.Fail -> "fail"
        is Test240State.Pass -> "pass"
        is Test240State.S0 -> "s0"
        is Test240State.S01 -> "s01"
        is Test240State.S02 -> "s02"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test240State): Boolean = when (state) {
        is Test240State.S0 -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test240State): Int = when (state) {
        is Test240State.Fail -> 4
        is Test240State.Pass -> 3
        is Test240State.S0 -> 0
        is Test240State.S01 -> 1
        is Test240State.S02 -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test240Event? = when (name) {
        "cancel.invoke" -> Test240Event.Cancel.Invoke
        "done.invoke" -> Test240Event.Done.Invoke
        "error.execution" -> Test240Event.Error.Execution
        "failure" -> Test240Event.Failure
        "success" -> Test240Event.Success
        "timeout" -> Test240Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test240Event): String? = when (event) {
        is Test240Event.Cancel.Invoke -> "cancel.invoke"
        is Test240Event.Done.Invoke -> "done.invoke"
        is Test240Event.Error.Execution -> "error.execution"
        is Test240Event.Failure -> "failure"
        is Test240Event.Success -> "success"
        is Test240Event.Timeout -> "timeout"
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
            "test240",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "1")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test240Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test240Event.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(Test240Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test240Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test240Event.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test240Event) {
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
        state: Test240State,
        event: Test240Event
    ): TransitionResult<Test240State> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is Test240State.S0 -> processS0(event)
        is Test240State.S01 -> {
            val result = processS01(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processS0(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is Test240State.S02 -> {
            val result = processS02(event)
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


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Timeout -> TransitionResult.External(Test240State.Fail, Test240State.S0)

        else -> TransitionResult.Ignored
    }

    private fun processS01(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Success -> TransitionResult.External(Test240State.S02, Test240State.S01)

        event is Test240Event.Failure -> TransitionResult.External(Test240State.Fail, Test240State.S01)

        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test240Event
    ): TransitionResult<Test240State> = when {
        event is Test240Event.Success -> TransitionResult.External(Test240State.Pass, Test240State.S02)

        event is Test240Event.Failure -> TransitionResult.External(Test240State.Fail, Test240State.S02)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test240.scxml:8 :: _machine
    override fun onEntry(state: Test240State, pathChild: Test240State?) {
        when (state) {
            is Test240State.Fail -> {
                // SCE-MAP: test240.scxml:70 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test240State.Pass -> {
                // SCE-MAP: test240.scxml:69 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test240State.S0 -> {
                // SCE-MAP: test240.scxml:13 :: s0 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s0")) return


            scheduleSend("__send_0", 2000L, Test240Event.Timeout)
            }
            is Test240State.S01 -> {
                // SCE-MAP: test240.scxml:19 :: s01 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s01")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s01.${System.identityHashCode(this)}._invoke_0"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "Var1")) {
                        raisePlatformError(Test240Event.Error.Execution, "<invoke> namelist names 'Var1', which the parent does not declare")
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test240SceSynthInvokeInvoke0StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_0", childSM, false, Test240Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is Test240State.S02 -> {
                // SCE-MAP: test240.scxml:42 :: s02 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("s02")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "s02.${System.identityHashCode(this)}._invoke_1"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // §scxml-5.7.1: a `<param>` whose expr will not evaluate costs
                    // `error.execution` on the internal queue AND the name and
                    // value — and nothing else. The clause delegates only the
                    // SUCCESSFUL name and value to the context ("Otherwise the use
                    // of the name and value depends on the context in which the
                    // <param> element occurs. See 5.5 <donedata>, 6.2 <send> and
                    // 6.4 <invoke>"), so §scxml-6.4.2's "terminate the processing
                    // of the element" is not reached by a failing `<param>`.
                    //
                    // This arm used to `return@run`, cancelling the whole invoke
                    // and raising nothing — the strictest reading of 6.4.2 with
                    // 5.7.1's reporting half dropped, so a document lost the child
                    // AND the event that would have explained why. The comment
                    // called that "the C++ pattern"; C++ does not cancel. The map
                    // insert is inside the `try`, so a failure leaves the name
                    // absent, which is the clause's other half.
                    try {
                        invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, "1")
                    } catch (_: Exception) {
                        raisePlatformError(Test240Event.Error.Execution, "<invoke> <param name='Var1'> expr failed to evaluate")
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = Test240SceSynthInvokeInvoke1StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("_invoke_1", childSM, false, Test240Event.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test240.scxml:8 :: _machine
    override fun onExit(state: Test240State) {
        when (state) {
            is Test240State.Fail -> {
                // SCE-MAP: test240.scxml:70 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is Test240State.Pass -> {
                // SCE-MAP: test240.scxml:69 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is Test240State.S0 -> {
                // SCE-MAP: test240.scxml:13 :: s0 :: _state_body
                activeStateIds.remove("s0")
            }
            is Test240State.S01 -> {
                // SCE-MAP: test240.scxml:19 :: s01 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("s01")
            }
            is Test240State.S02 -> {
                // SCE-MAP: test240.scxml:42 :: s02 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
                activeStateIds.remove("s02")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test240.scxml:8 :: _machine
    override fun executeTransitionActions(
        source: Test240State,
        event: Test240Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
