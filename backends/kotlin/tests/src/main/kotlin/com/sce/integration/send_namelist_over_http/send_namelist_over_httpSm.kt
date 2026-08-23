// SCE-GENERATED — DO NOT EDIT
// source-hash: 4ee53cfb47d1fc9305452f53c97089f2edfbc8978dab68d1b49e898f5eb29582
// template-hash: 90a17b01a07fa6e2db248839e7823280de5374963642bfa559a2f033a153c586
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/send_namelist_over_http/send_namelist_over_http.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: send_namelist_over_http.scxml:51 :: _machine

package com.sce.integration.send_namelist_over_http

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface SendNamelistOverHttpState : State {
    data object DiscardPhase : SendNamelistOverHttpState
    data object FailMessageNotDiscarded : SendNamelistOverHttpState
    data object FailNamelistNeverArrived : SendNamelistOverHttpState
    data object FailNamelistNotPosted : SendNamelistOverHttpState
    data object FailNoNamelistError : SendNamelistOverHttpState
    data object MapPhase : SendNamelistOverHttpState
    data object MapVerdict : SendNamelistOverHttpState
    data object Pass : SendNamelistOverHttpState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SendNamelistOverHttpEvent : Event {
    sealed interface Error : SendNamelistOverHttpEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Mapped : SendNamelistOverHttpEvent
    data object ShouldNotArrive : SendNamelistOverHttpEvent
    data object TimeoutDiscard : SendNamelistOverHttpEvent
    data object TimeoutMap : SendNamelistOverHttpEvent
}
// --- State Machine (W3C SCXML) ---

class SendNamelistOverHttpStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<SendNamelistOverHttpState, SendNamelistOverHttpEvent>(scriptEngine) {

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

    /**
     * §scxml-5.3: what the `echoed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `echoed` was assigned a value of another type, or the engine refused.
     */
    fun echoed(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "echoed")

    /**
     * §scxml-5.3: what the `sawNamelistError` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sawNamelistError` was assigned a value of another type, or the engine refused.
     */
    fun sawNamelistError(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sawNamelistError")

    override val initialState: SendNamelistOverHttpState = SendNamelistOverHttpState.MapPhase

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
    override fun resolveState(stateId: String): SendNamelistOverHttpState? = when (stateId) {
        "discardPhase" -> SendNamelistOverHttpState.DiscardPhase
        "failMessageNotDiscarded" -> SendNamelistOverHttpState.FailMessageNotDiscarded
        "failNamelistNeverArrived" -> SendNamelistOverHttpState.FailNamelistNeverArrived
        "failNamelistNotPosted" -> SendNamelistOverHttpState.FailNamelistNotPosted
        "failNoNamelistError" -> SendNamelistOverHttpState.FailNoNamelistError
        "mapPhase" -> SendNamelistOverHttpState.MapPhase
        "mapVerdict" -> SendNamelistOverHttpState.MapVerdict
        "pass" -> SendNamelistOverHttpState.Pass
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SendNamelistOverHttpState): String = when (state) {
        is SendNamelistOverHttpState.DiscardPhase -> "discardPhase"
        is SendNamelistOverHttpState.FailMessageNotDiscarded -> "failMessageNotDiscarded"
        is SendNamelistOverHttpState.FailNamelistNeverArrived -> "failNamelistNeverArrived"
        is SendNamelistOverHttpState.FailNamelistNotPosted -> "failNamelistNotPosted"
        is SendNamelistOverHttpState.FailNoNamelistError -> "failNoNamelistError"
        is SendNamelistOverHttpState.MapPhase -> "mapPhase"
        is SendNamelistOverHttpState.MapVerdict -> "mapVerdict"
        is SendNamelistOverHttpState.Pass -> "pass"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: SendNamelistOverHttpState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: SendNamelistOverHttpState): Int = when (state) {
        is SendNamelistOverHttpState.DiscardPhase -> 2
        is SendNamelistOverHttpState.FailMessageNotDiscarded -> 6
        is SendNamelistOverHttpState.FailNamelistNeverArrived -> 4
        is SendNamelistOverHttpState.FailNamelistNotPosted -> 5
        is SendNamelistOverHttpState.FailNoNamelistError -> 7
        is SendNamelistOverHttpState.MapPhase -> 0
        is SendNamelistOverHttpState.MapVerdict -> 1
        is SendNamelistOverHttpState.Pass -> 3
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SendNamelistOverHttpEvent? = when (name) {
        "error.communication" -> SendNamelistOverHttpEvent.Error.Communication
        "error.execution" -> SendNamelistOverHttpEvent.Error.Execution
        "mapped" -> SendNamelistOverHttpEvent.Mapped
        "shouldNotArrive" -> SendNamelistOverHttpEvent.ShouldNotArrive
        "timeoutDiscard" -> SendNamelistOverHttpEvent.TimeoutDiscard
        "timeoutMap" -> SendNamelistOverHttpEvent.TimeoutMap
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SendNamelistOverHttpEvent): String? = when (event) {
        is SendNamelistOverHttpEvent.Error.Communication -> "error.communication"
        is SendNamelistOverHttpEvent.Error.Execution -> "error.execution"
        is SendNamelistOverHttpEvent.Mapped -> "mapped"
        is SendNamelistOverHttpEvent.ShouldNotArrive -> "shouldNotArrive"
        is SendNamelistOverHttpEvent.TimeoutDiscard -> "timeoutDiscard"
        is SendNamelistOverHttpEvent.TimeoutMap -> "timeoutMap"
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
            "send_namelist_over_http",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, "2")
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<data id='Var1'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'echoed' with expr
        try {
            val initResult_echoed = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "echoed", initResult_echoed)
        } catch (e: Exception) {
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<data id='echoed'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sawNamelistError' with expr
        try {
            val initResult_sawNamelistError = engine.evaluateExpr(sid, "0")
            engine.setVariable(sid, "sawNamelistError", initResult_sawNamelistError)
        } catch (e: Exception) {
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<data id='sawNamelistError'> expr failed to evaluate")
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
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: SendNamelistOverHttpEvent) {
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
        state: SendNamelistOverHttpState,
        event: SendNamelistOverHttpEvent
    ): TransitionResult<SendNamelistOverHttpState> {
        // W3C SCXML 5.10: Set _event before guard evaluation
        setCurrentEventInScriptEngine(event)
        return when (state) {
        is SendNamelistOverHttpState.DiscardPhase -> processDiscardPhase(event)
        is SendNamelistOverHttpState.MapPhase -> processMapPhase(event)
        else -> TransitionResult.Ignored
    }
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: SendNamelistOverHttpState
    ): TransitionResult<SendNamelistOverHttpState> = when (state) {
        is SendNamelistOverHttpState.MapVerdict -> processNullMapVerdict()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullMapVerdict(
    ): TransitionResult<SendNamelistOverHttpState> = when {
        safeEvaluateGuard("echoed == 2") -> TransitionResult.External(SendNamelistOverHttpState.DiscardPhase, SendNamelistOverHttpState.MapVerdict)
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(SendNamelistOverHttpState.FailNamelistNotPosted, SendNamelistOverHttpState.MapVerdict)
    }

    // --- Per-State Event Handlers ---

    private fun processDiscardPhase(
        event: SendNamelistOverHttpEvent
    ): TransitionResult<SendNamelistOverHttpState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is SendNamelistOverHttpEvent.Error.Execution -> TransitionResult.Internal
        event is SendNamelistOverHttpEvent.ShouldNotArrive -> TransitionResult.External(SendNamelistOverHttpState.FailMessageNotDiscarded, SendNamelistOverHttpState.DiscardPhase)

        event is SendNamelistOverHttpEvent.TimeoutDiscard && safeEvaluateGuard("sawNamelistError !== 1") -> TransitionResult.External(SendNamelistOverHttpState.FailNoNamelistError, SendNamelistOverHttpState.DiscardPhase)

        event is SendNamelistOverHttpEvent.TimeoutDiscard -> TransitionResult.External(SendNamelistOverHttpState.Pass, SendNamelistOverHttpState.DiscardPhase)

        else -> TransitionResult.Ignored
    }

    private fun processMapPhase(
        event: SendNamelistOverHttpEvent
    ): TransitionResult<SendNamelistOverHttpState> = when {
        event is SendNamelistOverHttpEvent.Mapped -> TransitionResult.External(SendNamelistOverHttpState.MapVerdict, SendNamelistOverHttpState.MapPhase)

        event is SendNamelistOverHttpEvent.TimeoutMap -> TransitionResult.External(SendNamelistOverHttpState.FailNamelistNeverArrived, SendNamelistOverHttpState.MapPhase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: send_namelist_over_http.scxml:51 :: _machine
    override fun onEntry(state: SendNamelistOverHttpState, pathChild: SendNamelistOverHttpState?) {
        when (state) {
            is SendNamelistOverHttpState.DiscardPhase -> {
                // SCE-MAP: send_namelist_over_http.scxml:95 :: discardPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("discardPhase")) return


            scheduleSend("__send_2", 2000L, SendNamelistOverHttpEvent.TimeoutDiscard)


            // W3C SCXML 6.2: Resolve dynamic target (targetexpr="_ioprocessors['basichttp'].location")
            var _resolvedTarget: String? = null
            run resolveTarget@{
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = eng.evaluateExpr(sid, "_ioprocessors['basichttp'].location")
                    val target = v?.toString() ?: ""
                    // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                    if (target.startsWith("!")) {
                        raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_3")
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raisePlatformError(SendNamelistOverHttpEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach")
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> targetexpr failed to evaluate")
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raisePlatformError(SendNamelistOverHttpEvent.Error.Communication, "<send> over BasicHTTPEventProcessor resolved a target that is not an http(s) URL")
            } else {

            // W3C SCXML C.2: BasicHTTP send with script engine evaluation
            run {
                ensureScriptEngine()
                val engineH = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidH = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val httpParams = mutableMapOf<String, List<String>>()
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern)
                if (!engineH.hasVariable(sidH, "__sce_not_declared__")) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> namelist names '__sce_not_declared__', which is not declared")
                    return@run
                }
                try {
                    val v = engineH.getVariable(sidH, "__sce_not_declared__")
                    httpParams["__sce_not_declared__"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> namelist entry '__sce_not_declared__' failed to evaluate")
                    return@run
                }
                val httpContent = ""
                performHttpSend(_rt, "shouldNotArrive", httpContent, httpParams, "__send_3")
            }
            }
            } // end of _resolvedTarget?.let
            }
            is SendNamelistOverHttpState.FailMessageNotDiscarded -> {
                // SCE-MAP: send_namelist_over_http.scxml:118 :: failMessageNotDiscarded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failMessageNotDiscarded")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendNamelistOverHttpState.FailNamelistNeverArrived -> {
                // SCE-MAP: send_namelist_over_http.scxml:116 :: failNamelistNeverArrived :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNamelistNeverArrived")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendNamelistOverHttpState.FailNamelistNotPosted -> {
                // SCE-MAP: send_namelist_over_http.scxml:117 :: failNamelistNotPosted :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNamelistNotPosted")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendNamelistOverHttpState.FailNoNamelistError -> {
                // SCE-MAP: send_namelist_over_http.scxml:119 :: failNoNamelistError :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("failNoNamelistError")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is SendNamelistOverHttpState.MapPhase -> {
                // SCE-MAP: send_namelist_over_http.scxml:71 :: mapPhase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("mapPhase")) return


            scheduleSend("__send_0", 3000L, SendNamelistOverHttpEvent.TimeoutMap)


            // W3C SCXML 6.2: Resolve dynamic target (targetexpr="_ioprocessors['basichttp'].location")
            var _resolvedTarget: String? = null
            run resolveTarget@{
                ensureScriptEngine()
                val eng = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                try {
                    val v = eng.evaluateExpr(sid, "_ioprocessors['basichttp'].location")
                    val target = v?.toString() ?: ""
                    // W3C SCXML 6.2 (test194): Invalid target (C++ SendHelper::isInvalidTarget)
                    if (target.startsWith("!")) {
                        raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_1")
                        return@resolveTarget
                    }
                    // W3C SCXML C.1 (test496): Unreachable target (C++ SendHelper::isUnreachableTarget)
                    if (target.isEmpty() || target == "undefined") {
                        raisePlatformError(SendNamelistOverHttpEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach")
                        return@resolveTarget
                    }
                    _resolvedTarget = target
                } catch (_: Exception) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> targetexpr failed to evaluate")
                }
            }
            _resolvedTarget?.let { _rt ->
            // W3C SCXML C.2: Validate dynamic target is HTTP URL
            if (!_rt.startsWith("http://") && !_rt.startsWith("https://")) {
                raisePlatformError(SendNamelistOverHttpEvent.Error.Communication, "<send> over BasicHTTPEventProcessor resolved a target that is not an http(s) URL")
            } else {

            // W3C SCXML C.2: BasicHTTP send with script engine evaluation
            run {
                ensureScriptEngine()
                val engineH = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidH = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val httpParams = mutableMapOf<String, List<String>>()
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern)
                if (!engineH.hasVariable(sidH, "Var1")) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> namelist names 'Var1', which is not declared")
                    return@run
                }
                try {
                    val v = engineH.getVariable(sidH, "Var1")
                    httpParams["Var1"] = listOf(valueToWireString(v))
                } catch (_: Exception) {
                    raisePlatformError(SendNamelistOverHttpEvent.Error.Execution, "<send> namelist entry 'Var1' failed to evaluate")
                    return@run
                }
                val httpContent = ""
                performHttpSend(_rt, "mapped", httpContent, httpParams, "__send_1")
            }
            }
            } // end of _resolvedTarget?.let
            }
            is SendNamelistOverHttpState.MapVerdict -> {
                // SCE-MAP: send_namelist_over_http.scxml:88 :: mapVerdict :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("mapVerdict")) return
            }
            is SendNamelistOverHttpState.Pass -> {
                // SCE-MAP: send_namelist_over_http.scxml:115 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: send_namelist_over_http.scxml:51 :: _machine
    override fun onExit(state: SendNamelistOverHttpState) {
        when (state) {
            is SendNamelistOverHttpState.DiscardPhase -> {
                // SCE-MAP: send_namelist_over_http.scxml:95 :: discardPhase :: _state_body
                activeStateIds.remove("discardPhase")
            }
            is SendNamelistOverHttpState.FailMessageNotDiscarded -> {
                // SCE-MAP: send_namelist_over_http.scxml:118 :: failMessageNotDiscarded :: _state_body
                activeStateIds.remove("failMessageNotDiscarded")
            }
            is SendNamelistOverHttpState.FailNamelistNeverArrived -> {
                // SCE-MAP: send_namelist_over_http.scxml:116 :: failNamelistNeverArrived :: _state_body
                activeStateIds.remove("failNamelistNeverArrived")
            }
            is SendNamelistOverHttpState.FailNamelistNotPosted -> {
                // SCE-MAP: send_namelist_over_http.scxml:117 :: failNamelistNotPosted :: _state_body
                activeStateIds.remove("failNamelistNotPosted")
            }
            is SendNamelistOverHttpState.FailNoNamelistError -> {
                // SCE-MAP: send_namelist_over_http.scxml:119 :: failNoNamelistError :: _state_body
                activeStateIds.remove("failNoNamelistError")
            }
            is SendNamelistOverHttpState.MapPhase -> {
                // SCE-MAP: send_namelist_over_http.scxml:71 :: mapPhase :: _state_body
                activeStateIds.remove("mapPhase")
            }
            is SendNamelistOverHttpState.MapVerdict -> {
                // SCE-MAP: send_namelist_over_http.scxml:88 :: mapVerdict :: _state_body
                activeStateIds.remove("mapVerdict")
            }
            is SendNamelistOverHttpState.Pass -> {
                // SCE-MAP: send_namelist_over_http.scxml:115 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: send_namelist_over_http.scxml:51 :: _machine
    override fun executeTransitionActions(
        source: SendNamelistOverHttpState,
        event: SendNamelistOverHttpEvent?
    ) {
        when (source) {
        is SendNamelistOverHttpState.DiscardPhase -> when {
            event is SendNamelistOverHttpEvent.Error.Execution -> {
                // SCE-MAP: send_namelist_over_http.scxml:106 :: discardPhase :: _transition_0


            executeAssign("sawNamelistError", "1")
            }
            else -> {}
        }
        is SendNamelistOverHttpState.MapPhase -> when {
            event is SendNamelistOverHttpEvent.Mapped -> {
                // SCE-MAP: send_namelist_over_http.scxml:82 :: mapPhase :: _transition_0


            executeAssign("echoed", "_event.data.Var1")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
