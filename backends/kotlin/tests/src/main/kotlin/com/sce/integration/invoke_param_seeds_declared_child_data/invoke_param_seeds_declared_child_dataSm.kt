// SCE-GENERATED — DO NOT EDIT
// source-hash: a9b3d7b7ea8a5bd6001a98d04817a6efb870e7f83add64eb3bb769017877144d
// template-hash: 082e347ab97b9b491598f98d263b24d185e7e030b1c1600c8a0939850d86f8db
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_param_seeds_declared_child_data/invoke_param_seeds_declared_child_data.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_param_seeds_declared_child_data.scxml:84 :: _machine

package com.sce.integration.invoke_param_seeds_declared_child_data

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokeParamSeedsDeclaredChildDataState : State {
    data object FailChildEvaluatedTheExpression : InvokeParamSeedsDeclaredChildDataState
    data object FailDeclaredParamLost : InvokeParamSeedsDeclaredChildDataState
    data object FailInfiniteParamCollapsed : InvokeParamSeedsDeclaredChildDataState
    data object FailInfiniteParamLost : InvokeParamSeedsDeclaredChildDataState
    data object FailNamelistValueLost : InvokeParamSeedsDeclaredChildDataState
    data object FailParentOnlyExprLost : InvokeParamSeedsDeclaredChildDataState
    data object FailShadowSeedLost : InvokeParamSeedsDeclaredChildDataState
    data object FailUnmatchedParamEnteredTheChild : InvokeParamSeedsDeclaredChildDataState
    data object Infinite : InvokeParamSeedsDeclaredChildDataState
    data object NamelistPhase : InvokeParamSeedsDeclaredChildDataState
    data object Pass : InvokeParamSeedsDeclaredChildDataState
    data object Shadowed : InvokeParamSeedsDeclaredChildDataState
    data object SoleName : InvokeParamSeedsDeclaredChildDataState
    data object Unmatched : InvokeParamSeedsDeclaredChildDataState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokeParamSeedsDeclaredChildDataEvent : Event {
    sealed interface Cancel : InvokeParamSeedsDeclaredChildDataEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : InvokeParamSeedsDeclaredChildDataEvent {
        data object Invoke : Done
    }
    sealed interface Error : InvokeParamSeedsDeclaredChildDataEvent {
        data object Execution : Error
    }
    sealed interface Seed : InvokeParamSeedsDeclaredChildDataEvent {
        data object Collapsed : Seed
        data object Leaked : Seed
        data object Missing : Seed
        data object Ok : Seed
        data object Shadowed : Seed
    }
}
// --- State Machine (W3C SCXML) ---

class InvokeParamSeedsDeclaredChildDataStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InvokeParamSeedsDeclaredChildDataState, InvokeParamSeedsDeclaredChildDataEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `token` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `token` was assigned a value of another type, or the engine refused.
     */
    fun token(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "token")

    /**
     * §scxml-5.3: what the `only_here` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `only_here` was assigned a value of another type, or the engine refused.
     */
    fun onlyHere(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "only_here")

    override val initialState: InvokeParamSeedsDeclaredChildDataState = InvokeParamSeedsDeclaredChildDataState.Shadowed

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
    override fun resolveState(stateId: String): InvokeParamSeedsDeclaredChildDataState? = when (stateId) {
        "failChildEvaluatedTheExpression" -> InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression
        "failDeclaredParamLost" -> InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost
        "failInfiniteParamCollapsed" -> InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed
        "failInfiniteParamLost" -> InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost
        "failNamelistValueLost" -> InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost
        "failParentOnlyExprLost" -> InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost
        "failShadowSeedLost" -> InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost
        "failUnmatchedParamEnteredTheChild" -> InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild
        "infinite" -> InvokeParamSeedsDeclaredChildDataState.Infinite
        "namelistPhase" -> InvokeParamSeedsDeclaredChildDataState.NamelistPhase
        "pass" -> InvokeParamSeedsDeclaredChildDataState.Pass
        "shadowed" -> InvokeParamSeedsDeclaredChildDataState.Shadowed
        "soleName" -> InvokeParamSeedsDeclaredChildDataState.SoleName
        "unmatched" -> InvokeParamSeedsDeclaredChildDataState.Unmatched
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokeParamSeedsDeclaredChildDataState): String = when (state) {
        is InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression -> "failChildEvaluatedTheExpression"
        is InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost -> "failDeclaredParamLost"
        is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed -> "failInfiniteParamCollapsed"
        is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost -> "failInfiniteParamLost"
        is InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost -> "failNamelistValueLost"
        is InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost -> "failParentOnlyExprLost"
        is InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost -> "failShadowSeedLost"
        is InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild -> "failUnmatchedParamEnteredTheChild"
        is InvokeParamSeedsDeclaredChildDataState.Infinite -> "infinite"
        is InvokeParamSeedsDeclaredChildDataState.NamelistPhase -> "namelistPhase"
        is InvokeParamSeedsDeclaredChildDataState.Pass -> "pass"
        is InvokeParamSeedsDeclaredChildDataState.Shadowed -> "shadowed"
        is InvokeParamSeedsDeclaredChildDataState.SoleName -> "soleName"
        is InvokeParamSeedsDeclaredChildDataState.Unmatched -> "unmatched"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokeParamSeedsDeclaredChildDataState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokeParamSeedsDeclaredChildDataState): Int = when (state) {
        is InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression -> 6
        is InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost -> 10
        is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed -> 13
        is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost -> 12
        is InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost -> 11
        is InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost -> 8
        is InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost -> 7
        is InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild -> 9
        is InvokeParamSeedsDeclaredChildDataState.Infinite -> 4
        is InvokeParamSeedsDeclaredChildDataState.NamelistPhase -> 3
        is InvokeParamSeedsDeclaredChildDataState.Pass -> 5
        is InvokeParamSeedsDeclaredChildDataState.Shadowed -> 0
        is InvokeParamSeedsDeclaredChildDataState.SoleName -> 1
        is InvokeParamSeedsDeclaredChildDataState.Unmatched -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokeParamSeedsDeclaredChildDataEvent? = when (name) {
        "cancel.invoke" -> InvokeParamSeedsDeclaredChildDataEvent.Cancel.Invoke
        "done.invoke" -> InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke
        "error.execution" -> InvokeParamSeedsDeclaredChildDataEvent.Error.Execution
        "seed.collapsed" -> InvokeParamSeedsDeclaredChildDataEvent.Seed.Collapsed
        "seed.leaked" -> InvokeParamSeedsDeclaredChildDataEvent.Seed.Leaked
        "seed.missing" -> InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing
        "seed.ok" -> InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok
        "seed.shadowed" -> InvokeParamSeedsDeclaredChildDataEvent.Seed.Shadowed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokeParamSeedsDeclaredChildDataEvent): String? = when (event) {
        is InvokeParamSeedsDeclaredChildDataEvent.Cancel.Invoke -> "cancel.invoke"
        is InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke -> "done.invoke"
        is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> "error.execution"
        is InvokeParamSeedsDeclaredChildDataEvent.Seed.Collapsed -> "seed.collapsed"
        is InvokeParamSeedsDeclaredChildDataEvent.Seed.Leaked -> "seed.leaked"
        is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> "seed.missing"
        is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> "seed.ok"
        is InvokeParamSeedsDeclaredChildDataEvent.Seed.Shadowed -> "seed.shadowed"
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
            "invoke_param_seeds_declared_child_data",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'token' with expr
        try {
            val initResult_token = engine.evaluateExpr(sid, "'parent'")
            engine.setVariable(sid, "token", initResult_token)
        } catch (e: Exception) {
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<data id='token'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'only_here' with expr
        try {
            val initResult_onlyHere = engine.evaluateExpr(sid, "'sole'")
            engine.setVariable(sid, "only_here", initResult_onlyHere)
        } catch (e: Exception) {
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<data id='only_here'> expr failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InvokeParamSeedsDeclaredChildDataEvent) {
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
        state: InvokeParamSeedsDeclaredChildDataState,
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is InvokeParamSeedsDeclaredChildDataState.Infinite -> processInfinite(event)
        is InvokeParamSeedsDeclaredChildDataState.NamelistPhase -> processNamelistPhase(event)
        is InvokeParamSeedsDeclaredChildDataState.Shadowed -> processShadowed(event)
        is InvokeParamSeedsDeclaredChildDataState.SoleName -> processSoleName(event)
        is InvokeParamSeedsDeclaredChildDataState.Unmatched -> processUnmatched(event)
        else -> TransitionResult.Ignored
    }
    }


    // --- Per-State Event Handlers ---

    private fun processInfinite(
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> = when {
        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.Pass, InvokeParamSeedsDeclaredChildDataState.Infinite)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost, InvokeParamSeedsDeclaredChildDataState.Infinite)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Collapsed -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed, InvokeParamSeedsDeclaredChildDataState.Infinite)

        event is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost, InvokeParamSeedsDeclaredChildDataState.Infinite)

        else -> TransitionResult.Ignored
    }

    private fun processNamelistPhase(
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> = when {
        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.Infinite, InvokeParamSeedsDeclaredChildDataState.NamelistPhase)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost, InvokeParamSeedsDeclaredChildDataState.NamelistPhase)

        event is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost, InvokeParamSeedsDeclaredChildDataState.NamelistPhase)

        else -> TransitionResult.Ignored
    }

    private fun processShadowed(
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> = when {
        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.SoleName, InvokeParamSeedsDeclaredChildDataState.Shadowed)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Shadowed -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression, InvokeParamSeedsDeclaredChildDataState.Shadowed)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost, InvokeParamSeedsDeclaredChildDataState.Shadowed)

        event is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost, InvokeParamSeedsDeclaredChildDataState.Shadowed)

        else -> TransitionResult.Ignored
    }

    private fun processSoleName(
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> = when {
        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.Unmatched, InvokeParamSeedsDeclaredChildDataState.SoleName)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost, InvokeParamSeedsDeclaredChildDataState.SoleName)

        event is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost, InvokeParamSeedsDeclaredChildDataState.SoleName)

        else -> TransitionResult.Ignored
    }

    private fun processUnmatched(
        event: InvokeParamSeedsDeclaredChildDataEvent
    ): TransitionResult<InvokeParamSeedsDeclaredChildDataState> = when {
        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Ok -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.NamelistPhase, InvokeParamSeedsDeclaredChildDataState.Unmatched)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Leaked -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild, InvokeParamSeedsDeclaredChildDataState.Unmatched)

        event is InvokeParamSeedsDeclaredChildDataEvent.Seed.Missing -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost, InvokeParamSeedsDeclaredChildDataState.Unmatched)

        event is InvokeParamSeedsDeclaredChildDataEvent.Error.Execution -> TransitionResult.External(InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost, InvokeParamSeedsDeclaredChildDataState.Unmatched)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:84 :: _machine
    override fun onEntry(state: InvokeParamSeedsDeclaredChildDataState, pathChild: InvokeParamSeedsDeclaredChildDataState?) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:266 :: failChildEvaluatedTheExpression :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failChildEvaluatedTheExpression")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:270 :: failDeclaredParamLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failDeclaredParamLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:273 :: failInfiniteParamCollapsed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failInfiniteParamCollapsed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:272 :: failInfiniteParamLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failInfiniteParamLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:271 :: failNamelistValueLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNamelistValueLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:268 :: failParentOnlyExprLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failParentOnlyExprLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:267 :: failShadowSeedLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failShadowSeedLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:269 :: failUnmatchedParamEnteredTheChild :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failUnmatchedParamEnteredTheChild")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.Infinite -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:235 :: infinite :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("infinite")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "infinite.${System.identityHashCode(this)}.inv_infinite"
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
                        invokeParams["seen"] = engineInv.evaluateExpr(sidInv, "1/0")
                    } catch (_: Exception) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> <param name='seen'> expr failed to evaluate")
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvInfiniteStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_infinite", childSM, false, InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is InvokeParamSeedsDeclaredChildDataState.NamelistPhase -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:193 :: namelistPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("namelistPhase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "namelistPhase.${System.identityHashCode(this)}.inv_namelist"
                    // W3C SCXML 6.4: Evaluate params at defer time (parent context)
                    ensureScriptEngine()
                    val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                    val invokeParams = mutableMapOf<String, Any?>()
                    // W3C SCXML 6.4.1: Namelist variable must exist in parent (C++ NamelistHelper pattern)
                    if (!engineInv.hasVariable(sidInv, "token")) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> namelist names 'token', which the parent does not declare")
                        return@run  // C++ pattern: invoke cancelled on namelist error
                    }
                    invokeParams["token"] = engineInv.getVariable(sidInv, "token")
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvNamelistStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_namelist", childSM, false, InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is InvokeParamSeedsDeclaredChildDataState.Pass -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:265 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokeParamSeedsDeclaredChildDataState.Shadowed -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:92 :: shadowed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("shadowed")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "shadowed.${System.identityHashCode(this)}.inv_shadow"
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
                        invokeParams["seen"] = engineInv.evaluateExpr(sidInv, "token")
                    } catch (_: Exception) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> <param name='seen'> expr failed to evaluate")
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvShadowStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_shadow", childSM, false, InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is InvokeParamSeedsDeclaredChildDataState.SoleName -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:123 :: soleName :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("soleName")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "soleName.${System.identityHashCode(this)}.inv_sole"
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
                        invokeParams["seen"] = engineInv.evaluateExpr(sidInv, "only_here")
                    } catch (_: Exception) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> <param name='seen'> expr failed to evaluate")
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvSoleStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_sole", childSM, false, InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is InvokeParamSeedsDeclaredChildDataState.Unmatched -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:149 :: unmatched :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("unmatched")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "unmatched.${System.identityHashCode(this)}.inv_unmatched"
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
                        invokeParams["declared"] = engineInv.evaluateExpr(sidInv, "'carried'")
                    } catch (_: Exception) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> <param name='declared'> expr failed to evaluate")
                    }
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
                        invokeParams["nowhere"] = engineInv.evaluateExpr(sidInv, "'leaked'")
                    } catch (_: Exception) {
                        raisePlatformError(InvokeParamSeedsDeclaredChildDataEvent.Error.Execution, "<invoke> <param name='nowhere'> expr failed to evaluate")
                    }
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = InvokeParamSeedsDeclaredChildDataSceSynthInvokeInvUnmatchedStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)"))
                        setInvokeParams(childSM, invokeParams)
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_unmatched", childSM, false, InvokeParamSeedsDeclaredChildDataEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:84 :: _machine
    override fun onExit(state: InvokeParamSeedsDeclaredChildDataState) {
        when (state) {
            is InvokeParamSeedsDeclaredChildDataState.FailChildEvaluatedTheExpression -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:266 :: failChildEvaluatedTheExpression :: _state_body
                activeStateIds.remove("failChildEvaluatedTheExpression")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailDeclaredParamLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:270 :: failDeclaredParamLost :: _state_body
                activeStateIds.remove("failDeclaredParamLost")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamCollapsed -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:273 :: failInfiniteParamCollapsed :: _state_body
                activeStateIds.remove("failInfiniteParamCollapsed")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailInfiniteParamLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:272 :: failInfiniteParamLost :: _state_body
                activeStateIds.remove("failInfiniteParamLost")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailNamelistValueLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:271 :: failNamelistValueLost :: _state_body
                activeStateIds.remove("failNamelistValueLost")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailParentOnlyExprLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:268 :: failParentOnlyExprLost :: _state_body
                activeStateIds.remove("failParentOnlyExprLost")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailShadowSeedLost -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:267 :: failShadowSeedLost :: _state_body
                activeStateIds.remove("failShadowSeedLost")
            }
            is InvokeParamSeedsDeclaredChildDataState.FailUnmatchedParamEnteredTheChild -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:269 :: failUnmatchedParamEnteredTheChild :: _state_body
                activeStateIds.remove("failUnmatchedParamEnteredTheChild")
            }
            is InvokeParamSeedsDeclaredChildDataState.Infinite -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:235 :: infinite :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_infinite")
                activeStateIds.remove("infinite")
            }
            is InvokeParamSeedsDeclaredChildDataState.NamelistPhase -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:193 :: namelistPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_namelist")
                activeStateIds.remove("namelistPhase")
            }
            is InvokeParamSeedsDeclaredChildDataState.Pass -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:265 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is InvokeParamSeedsDeclaredChildDataState.Shadowed -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:92 :: shadowed :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_shadow")
                activeStateIds.remove("shadowed")
            }
            is InvokeParamSeedsDeclaredChildDataState.SoleName -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:123 :: soleName :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_sole")
                activeStateIds.remove("soleName")
            }
            is InvokeParamSeedsDeclaredChildDataState.Unmatched -> {
                // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:149 :: unmatched :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_unmatched")
                activeStateIds.remove("unmatched")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_param_seeds_declared_child_data.scxml:84 :: _machine
    override fun executeTransitionActions(
        source: InvokeParamSeedsDeclaredChildDataState,
        event: InvokeParamSeedsDeclaredChildDataEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
