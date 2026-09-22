// SCE-GENERATED — DO NOT EDIT
// source-hash: 34a3aa3a202a7ed359ee0ab4d1bade13a9630715ad1b32496ec4d19b29db3f4f
// template-hash: cfb1f54fd630bae878f5b70ad09507a7bb80ff2d52a4a44c9d3c51d12db9137b
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_candidate_selects_the_child/invoke_candidate_selects_the_child.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_candidate_selects_the_child.scxml:34 :: _machine

package com.sce.integration.invoke_candidate_selects_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeCandidateSelectsTheChildState : State {
    data object NoChild : InvokeCandidateSelectsTheChildState
    data object Pass : InvokeCandidateSelectsTheChildState
    data object Probe : InvokeCandidateSelectsTheChildState
    data object WrongChild : InvokeCandidateSelectsTheChildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeCandidateSelectsTheChildEvent : Event {
    sealed interface Error : InvokeCandidateSelectsTheChildEvent {
        data object Execution : Error
    }
    sealed interface From : InvokeCandidateSelectsTheChildEvent {
        data object Chosen : From
        data object Other : From
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeCandidateSelectsTheChildStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeCandidateSelectsTheChildState, InvokeCandidateSelectsTheChildEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `pick` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `pick` was assigned a value of another type, or the engine refused.
     */
    fun pick(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "pick")

    override val initialState: InvokeCandidateSelectsTheChildState = InvokeCandidateSelectsTheChildState.Probe

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
    override fun resolveState(stateId: String): InvokeCandidateSelectsTheChildState? = when (stateId) {
        "noChild" -> InvokeCandidateSelectsTheChildState.NoChild
        "pass" -> InvokeCandidateSelectsTheChildState.Pass
        "probe" -> InvokeCandidateSelectsTheChildState.Probe
        "wrongChild" -> InvokeCandidateSelectsTheChildState.WrongChild
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeCandidateSelectsTheChildState): String = when (state) {
        is InvokeCandidateSelectsTheChildState.NoChild -> "noChild"
        is InvokeCandidateSelectsTheChildState.Pass -> "pass"
        is InvokeCandidateSelectsTheChildState.Probe -> "probe"
        is InvokeCandidateSelectsTheChildState.WrongChild -> "wrongChild"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeCandidateSelectsTheChildState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeCandidateSelectsTheChildState): Int = when (state) {
        is InvokeCandidateSelectsTheChildState.NoChild -> 3
        is InvokeCandidateSelectsTheChildState.Pass -> 1
        is InvokeCandidateSelectsTheChildState.Probe -> 0
        is InvokeCandidateSelectsTheChildState.WrongChild -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeCandidateSelectsTheChildEvent? = when (name) {
        "error.execution" -> InvokeCandidateSelectsTheChildEvent.Error.Execution
        "from.chosen" -> InvokeCandidateSelectsTheChildEvent.From.Chosen
        "from.other" -> InvokeCandidateSelectsTheChildEvent.From.Other
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeCandidateSelectsTheChildEvent): String? = when (event) {
        is InvokeCandidateSelectsTheChildEvent.Error.Execution -> "error.execution"
        is InvokeCandidateSelectsTheChildEvent.From.Chosen -> "from.chosen"
        is InvokeCandidateSelectsTheChildEvent.From.Other -> "from.other"
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
            "invoke_candidate_selects_the_child",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'pick' with expr
        try {
            val initResult_pick = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"file:chosen.scxml\"", "'file:chosen.scxml'"))
            engine.setVariable(sid, "pick", initResult_pick)
        } catch (e: Exception) {
            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<data id='pick'> expr failed to evaluate")
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
            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeCandidateSelectsTheChildEvent) {
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
        state: InvokeCandidateSelectsTheChildState,
        event: InvokeCandidateSelectsTheChildEvent
    ): TransitionResult<InvokeCandidateSelectsTheChildState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is InvokeCandidateSelectsTheChildState.Probe -> processProbe(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processProbe(
        event: InvokeCandidateSelectsTheChildEvent
    ): TransitionResult<InvokeCandidateSelectsTheChildState> = when {
        event is InvokeCandidateSelectsTheChildEvent.From.Chosen -> TransitionResult.External(InvokeCandidateSelectsTheChildState.Pass, InvokeCandidateSelectsTheChildState.Probe, 0)

        event is InvokeCandidateSelectsTheChildEvent.From.Other -> TransitionResult.External(InvokeCandidateSelectsTheChildState.WrongChild, InvokeCandidateSelectsTheChildState.Probe, 1)

        event is InvokeCandidateSelectsTheChildEvent.Error.Execution -> TransitionResult.External(InvokeCandidateSelectsTheChildState.NoChild, InvokeCandidateSelectsTheChildState.Probe, 2)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_candidate_selects_the_child.scxml:34 :: _machine
    override fun onEntry(state: InvokeCandidateSelectsTheChildState, pathChild: InvokeCandidateSelectsTheChildState?) {
        when (state) {
            is InvokeCandidateSelectsTheChildState.NoChild -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:55 :: noChild :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("noChild")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeCandidateSelectsTheChildState.Pass -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:48 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeCandidateSelectsTheChildState.Probe -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:41 :: probe :: _state_body
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
                                eng.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("pick", "pick"))?.toString()
                            } catch (_: Exception) {
                                null
                            }
                            if (filePath == null) {
                                raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<invoke srcexpr='pick'> could not be evaluated")
                                return@deferInvoke
                            }
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            val __sceSelected = filePath
                                .substringAfterLast('/')
                                .substringAfterLast('\\')
                                .removePrefix("file:")
                                .substringBeforeLast('.')
                            val childSM = when (__sceSelected) {
                                "chosen" -> ChosenStateMachine()
                                "other" -> OtherStateMachine()
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<invoke srcexpr='pick'> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }
                            startInvoke("_invoke_0", childSM, false, null, "", generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(InvokeCandidateSelectsTheChildEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            is InvokeCandidateSelectsTheChildState.WrongChild -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:54 :: wrongChild :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("wrongChild")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_candidate_selects_the_child.scxml:34 :: _machine
    override fun onExit(state: InvokeCandidateSelectsTheChildState) {
        when (state) {
            is InvokeCandidateSelectsTheChildState.NoChild -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:55 :: noChild :: _state_body
                activeStateIds.remove("noChild")
            }
            is InvokeCandidateSelectsTheChildState.Pass -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:48 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokeCandidateSelectsTheChildState.Probe -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:41 :: probe :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                activeStateIds.remove("probe")
            }
            is InvokeCandidateSelectsTheChildState.WrongChild -> {
                // SCE-MAP: invoke_candidate_selects_the_child.scxml:54 :: wrongChild :: _state_body
                activeStateIds.remove("wrongChild")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_candidate_selects_the_child.scxml:34 :: _machine
    override fun executeTransitionActions(
        source: InvokeCandidateSelectsTheChildState,
        event: InvokeCandidateSelectsTheChildEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
