// SCE-GENERATED — DO NOT EDIT
// source-hash: 330474c9d384762034a0ce81e85f7fab16d80ad68caac74a931eac551a42e48f
// template-hash: c7fa1bace9cc09130fe34c6bb613ca8da547abc5200c95b444deca8a9309196b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_expression_failure_is_reported/invoke_expression_failure_is_reported.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_expression_failure_is_reported.scxml:38 :: _machine

package com.sce.integration.invoke_expression_failure_is_reported

import com.sce.runtime.*
import com.sce.interpreter.ScxmlRuntimeInterpreter


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeExpressionFailureIsReportedState : State {
    data object Fail : InvokeExpressionFailureIsReportedState
    data object Pass : InvokeExpressionFailureIsReportedState
    data object Probe : InvokeExpressionFailureIsReportedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeExpressionFailureIsReportedEvent : Event {
    sealed interface Done : InvokeExpressionFailureIsReportedEvent {
        data object Invoke : Done
    }
    sealed interface Error : InvokeExpressionFailureIsReportedEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeExpressionFailureIsReportedStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeExpressionFailureIsReportedState, InvokeExpressionFailureIsReportedEvent>(scriptEngine) {

    override val initialState: InvokeExpressionFailureIsReportedState = InvokeExpressionFailureIsReportedState.Probe

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
    override fun resolveState(stateId: String): InvokeExpressionFailureIsReportedState? = when (stateId) {
        "fail" -> InvokeExpressionFailureIsReportedState.Fail
        "pass" -> InvokeExpressionFailureIsReportedState.Pass
        "probe" -> InvokeExpressionFailureIsReportedState.Probe
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeExpressionFailureIsReportedState): String = when (state) {
        is InvokeExpressionFailureIsReportedState.Fail -> "fail"
        is InvokeExpressionFailureIsReportedState.Pass -> "pass"
        is InvokeExpressionFailureIsReportedState.Probe -> "probe"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeExpressionFailureIsReportedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeExpressionFailureIsReportedState): Int = when (state) {
        is InvokeExpressionFailureIsReportedState.Fail -> 2
        is InvokeExpressionFailureIsReportedState.Pass -> 1
        is InvokeExpressionFailureIsReportedState.Probe -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeExpressionFailureIsReportedEvent? = when (name) {
        "done.invoke" -> InvokeExpressionFailureIsReportedEvent.Done.Invoke
        "error.execution" -> InvokeExpressionFailureIsReportedEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeExpressionFailureIsReportedEvent): String? = when (event) {
        is InvokeExpressionFailureIsReportedEvent.Done.Invoke -> "done.invoke"
        is InvokeExpressionFailureIsReportedEvent.Error.Execution -> "error.execution"
    }



    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

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
            "invoke_expression_failure_is_reported",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'target' with expr
        try {
            val initResult_target = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("nil", "null"))
            engine.setVariable(sid, "target", initResult_target)
        } catch (e: Exception) {
            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "<data id='target'> expr failed to evaluate")
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
    //
    // The guard arrives as a `ScriptSource`, not a `String`: it carries the
    // language its text is in, so a machine generated for a Lua engine hands
    // over Lua the build-time frontend produced and one generated for an
    // ECMAScript engine hands over the author's own text — and the engine is
    // never left to guess which it got. The C++ sibling
    // (`process_transition.jinja2`) takes the same argument for the same
    // reason.
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
    //
    // The serialization wraps BOTH halves, in each half's own language. A
    // wrapper composed around one of them only would build a `ScriptSource`
    // whose two strings no longer say the same thing, and the diagnostic that
    // reads `source` would name an expression the engine never ran. `JSON` is
    // a §scxml-B-2-9 name both engines carry, so the wrapper is the same eight
    // characters on either arm — what differs is what it wraps.
    private fun evaluateSendContent(source: com.sce.runtime.ScriptSource): String {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        val serialized = when (source.language) {
            com.sce.runtime.ScriptLanguage.ECMAScript ->
                com.sce.runtime.ScriptSource.ecmascript("JSON.stringify((" + source.source + "))")
            com.sce.runtime.ScriptLanguage.Lua ->
                com.sce.runtime.ScriptSource.lua(
                    "JSON.stringify((" + source.text + "))",
                    "JSON.stringify((" + source.source + "))",
                )
        }
        return try {
            engine.evaluateExpr(sid, serialized)?.toString() ?: ""
        } catch (e: Exception) {
            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.assign(sid, location, expr)
        } catch (e: Exception) {
            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "<assign> failed")
        }
    }

    // W3C SCXML 5.8: Script block execution
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        try {
            engine.executeScript(sid, script)
        } catch (e: Exception) {
            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeExpressionFailureIsReportedEvent) {
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
        state: InvokeExpressionFailureIsReportedState,
        event: InvokeExpressionFailureIsReportedEvent
    ): TransitionResult<InvokeExpressionFailureIsReportedState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is InvokeExpressionFailureIsReportedState.Probe -> processProbe(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processProbe(
        event: InvokeExpressionFailureIsReportedEvent
    ): TransitionResult<InvokeExpressionFailureIsReportedState> = when {
        event is InvokeExpressionFailureIsReportedEvent.Error.Execution -> TransitionResult.External(InvokeExpressionFailureIsReportedState.Pass, InvokeExpressionFailureIsReportedState.Probe, 0)

        event is InvokeExpressionFailureIsReportedEvent.Done.Invoke -> TransitionResult.External(InvokeExpressionFailureIsReportedState.Fail, InvokeExpressionFailureIsReportedState.Probe, 1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_expression_failure_is_reported.scxml:38 :: _machine
    override fun onEntry(state: InvokeExpressionFailureIsReportedState, pathChild: InvokeExpressionFailureIsReportedState?) {
        when (state) {
            is InvokeExpressionFailureIsReportedState.Fail -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:50 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeExpressionFailureIsReportedState.Pass -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:49 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeExpressionFailureIsReportedState.Probe -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:44 :: probe :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("probe")) return
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    val generatedInvokeId = "probe.${System.identityHashCode(this)}._invoke_0"
                    deferInvoke(state, generatedInvokeId) {
                        ensureScriptEngine()
                        val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        try {
                            // W3C SCXML 6.4.3: Evaluate srcexpr → file path → load SCXML → create child.
                            // The evaluation is its own try: the clause it answers is
                            // "the expression could not be evaluated", and until this
                            // split that fact shared a catch — and a wording — with
                            // "the child could not be started". A `null` result took
                            // neither path and returned silently.
                            val filePath = try {
                                eng.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("target.path", "target.path"))?.toString()
                            } catch (_: Exception) {
                                null
                            }
                            if (filePath == null) {
                                raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "<invoke srcexpr='target.path'> could not be evaluated")
                                return@deferInvoke
                            }
                            val childSM = ScxmlRuntimeInterpreter.fromFile(filePath, "integration_resources/invoke_expression_failure_is_reported", scriptEngine)
                            startInvoke("_invoke_0", childSM, false, InvokeExpressionFailureIsReportedEvent.Done.Invoke, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(InvokeExpressionFailureIsReportedEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_expression_failure_is_reported.scxml:38 :: _machine
    override fun onExit(state: InvokeExpressionFailureIsReportedState) {
        when (state) {
            is InvokeExpressionFailureIsReportedState.Fail -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:50 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is InvokeExpressionFailureIsReportedState.Pass -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:49 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokeExpressionFailureIsReportedState.Probe -> {
                // SCE-MAP: invoke_expression_failure_is_reported.scxml:44 :: probe :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("probe")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_expression_failure_is_reported.scxml:38 :: _machine
    override fun executeTransitionActions(
        source: InvokeExpressionFailureIsReportedState,
        event: InvokeExpressionFailureIsReportedEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
