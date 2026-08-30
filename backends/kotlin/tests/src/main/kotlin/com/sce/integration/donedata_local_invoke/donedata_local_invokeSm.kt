// SCE-GENERATED — DO NOT EDIT
// source-hash: 7072491d11c203791302209b1bf9b82270fe7555d8209b82381d2a9f2ebc3c9f
// template-hash: 057f3064c2c620977191e86f67c1d505edec850a0d81b50b27d4b101952af703
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/donedata_local_invoke/donedata_local_invoke.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: donedata_local_invoke.scxml:28 :: _machine

package com.sce.integration.donedata_local_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface DonedataLocalInvokeState : State {
    data object Fail : DonedataLocalInvokeState
    data object Pass : DonedataLocalInvokeState
    data object PhaseContent : DonedataLocalInvokeState
    data object PhaseParam : DonedataLocalInvokeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface DonedataLocalInvokeEvent : Event {
    sealed interface Cancel : DonedataLocalInvokeEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : DonedataLocalInvokeEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object InvContent : Invoke
            data object InvParam : Invoke
        }
    }
    sealed interface Error : DonedataLocalInvokeEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class DonedataLocalInvokeStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<DonedataLocalInvokeState, DonedataLocalInvokeEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `param_ok` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `param_ok` was assigned a value of another type, or the engine refused.
     */
    fun paramOk(): Boolean? =
        com.sce.runtime.DatamodelRead.readBool(scriptEngine, scriptSessionId, "param_ok")

    override val initialState: DonedataLocalInvokeState = DonedataLocalInvokeState.PhaseParam

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
    override fun resolveState(stateId: String): DonedataLocalInvokeState? = when (stateId) {
        "fail" -> DonedataLocalInvokeState.Fail
        "pass" -> DonedataLocalInvokeState.Pass
        "phase_content" -> DonedataLocalInvokeState.PhaseContent
        "phase_param" -> DonedataLocalInvokeState.PhaseParam
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: DonedataLocalInvokeState): String = when (state) {
        is DonedataLocalInvokeState.Fail -> "fail"
        is DonedataLocalInvokeState.Pass -> "pass"
        is DonedataLocalInvokeState.PhaseContent -> "phase_content"
        is DonedataLocalInvokeState.PhaseParam -> "phase_param"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: DonedataLocalInvokeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: DonedataLocalInvokeState): Int = when (state) {
        is DonedataLocalInvokeState.Fail -> 3
        is DonedataLocalInvokeState.Pass -> 2
        is DonedataLocalInvokeState.PhaseContent -> 1
        is DonedataLocalInvokeState.PhaseParam -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): DonedataLocalInvokeEvent? = when (name) {
        "cancel.invoke" -> DonedataLocalInvokeEvent.Cancel.Invoke
        "done.invoke" -> DonedataLocalInvokeEvent.Done.Invoke.Self
        "done.invoke.inv_content" -> DonedataLocalInvokeEvent.Done.Invoke.InvContent
        "done.invoke.inv_param" -> DonedataLocalInvokeEvent.Done.Invoke.InvParam
        "error.execution" -> DonedataLocalInvokeEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: DonedataLocalInvokeEvent): String? = when (event) {
        is DonedataLocalInvokeEvent.Cancel.Invoke -> "cancel.invoke"
        is DonedataLocalInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is DonedataLocalInvokeEvent.Done.Invoke.InvContent -> "done.invoke.inv_content"
        is DonedataLocalInvokeEvent.Done.Invoke.InvParam -> "done.invoke.inv_param"
        is DonedataLocalInvokeEvent.Error.Execution -> "error.execution"
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
            "donedata_local_invoke",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'param_ok' with expr
        try {
            val initResult_paramOk = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.ecmascript("false"))
            engine.setVariable(sid, "param_ok", initResult_paramOk)
        } catch (e: Exception) {
            raisePlatformError(DonedataLocalInvokeEvent.Error.Execution, "<data id='param_ok'> expr failed to evaluate")
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
            raisePlatformError(DonedataLocalInvokeEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(DonedataLocalInvokeEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(DonedataLocalInvokeEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(DonedataLocalInvokeEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: DonedataLocalInvokeEvent) {
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
        state: DonedataLocalInvokeState,
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is DonedataLocalInvokeState.PhaseContent -> processPhaseContent(event)
        is DonedataLocalInvokeState.PhaseParam -> processPhaseParam(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processPhaseContent(
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> = when {
        event is DonedataLocalInvokeEvent.Done.Invoke.InvContent && safeEvaluateGuard(com.sce.runtime.ScriptSource.ecmascript("param_ok && _event.data === 'hello_content'")) -> TransitionResult.External(DonedataLocalInvokeState.Pass, DonedataLocalInvokeState.PhaseContent, 0)

        event is DonedataLocalInvokeEvent.Done.Invoke.InvContent -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseContent, 1)

        event is DonedataLocalInvokeEvent.Error.Execution -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseContent, 2)

        else -> TransitionResult.Ignored
    }

    private fun processPhaseParam(
        event: DonedataLocalInvokeEvent
    ): TransitionResult<DonedataLocalInvokeState> = when {
        event is DonedataLocalInvokeEvent.Done.Invoke.InvParam && safeEvaluateGuard(com.sce.runtime.ScriptSource.ecmascript("_event.data && _event.data.result === 42")) -> TransitionResult.External(DonedataLocalInvokeState.PhaseContent, DonedataLocalInvokeState.PhaseParam, 3)

        event is DonedataLocalInvokeEvent.Done.Invoke.InvParam -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseParam, 4)

        event is DonedataLocalInvokeEvent.Error.Execution -> TransitionResult.External(DonedataLocalInvokeState.Fail, DonedataLocalInvokeState.PhaseParam, 5)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: donedata_local_invoke.scxml:28 :: _machine
    override fun onEntry(state: DonedataLocalInvokeState, pathChild: DonedataLocalInvokeState?) {
        when (state) {
            is DonedataLocalInvokeState.Fail -> {
                // SCE-MAP: donedata_local_invoke.scxml:75 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLocalInvokeState.Pass -> {
                // SCE-MAP: donedata_local_invoke.scxml:74 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is DonedataLocalInvokeState.PhaseContent -> {
                // SCE-MAP: donedata_local_invoke.scxml:55 :: phase_content :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase_content")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase_content.${System.identityHashCode(this)}.inv_content"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = DonedataLocalInvokeSceSynthInvokeInvContentStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_content", childSM, false, DonedataLocalInvokeEvent.Done.Invoke.InvContent, "", generatedInvokeId)
                    }
                }
            }
            is DonedataLocalInvokeState.PhaseParam -> {
                // SCE-MAP: donedata_local_invoke.scxml:34 :: phase_param :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase_param")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase_param.${System.identityHashCode(this)}.inv_param"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = DonedataLocalInvokeSceSynthInvokeInvParamStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_param", childSM, false, DonedataLocalInvokeEvent.Done.Invoke.InvParam, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: donedata_local_invoke.scxml:28 :: _machine
    override fun onExit(state: DonedataLocalInvokeState) {
        when (state) {
            is DonedataLocalInvokeState.Fail -> {
                // SCE-MAP: donedata_local_invoke.scxml:75 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is DonedataLocalInvokeState.Pass -> {
                // SCE-MAP: donedata_local_invoke.scxml:74 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is DonedataLocalInvokeState.PhaseContent -> {
                // SCE-MAP: donedata_local_invoke.scxml:55 :: phase_content :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_content")
                activeStateIds.remove("phase_content")
            }
            is DonedataLocalInvokeState.PhaseParam -> {
                // SCE-MAP: donedata_local_invoke.scxml:34 :: phase_param :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_param")
                activeStateIds.remove("phase_param")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: donedata_local_invoke.scxml:28 :: _machine
    override fun executeTransitionActions(
        source: DonedataLocalInvokeState,
        event: DonedataLocalInvokeEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is DonedataLocalInvokeState.PhaseParam -> when (transitionIndex) {
            3 -> {
                // SCE-MAP: donedata_local_invoke.scxml:47 :: phase_param :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.ecmascript("param_ok"), com.sce.runtime.ScriptSource.ecmascript("true"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
