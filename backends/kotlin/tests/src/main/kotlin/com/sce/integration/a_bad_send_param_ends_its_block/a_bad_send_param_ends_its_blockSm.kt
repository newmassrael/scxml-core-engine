// SCE-GENERATED — DO NOT EDIT
// source-hash: b74272086b58bcd1324d0bc10e40165be4cf9dfbba8c5603320f1127d00ae990

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_bad_send_param_ends_its_block/a_bad_send_param_ends_its_block.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_bad_send_param_ends_its_block.scxml:42 :: _machine

package com.sce.integration.a_bad_send_param_ends_its_block

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ABadSendParamEndsItsBlockState : State {
    data object Done : ABadSendParamEndsItsBlockState
    data object S0 : ABadSendParamEndsItsBlockState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ABadSendParamEndsItsBlockEvent : Event {
    data object Bare : ABadSendParamEndsItsBlockEvent
    data object Carried : ABadSendParamEndsItsBlockEvent
    sealed interface Error : ABadSendParamEndsItsBlockEvent {
        data object Execution : Error
    }
    data object Finish : ABadSendParamEndsItsBlockEvent
    data object Partial : ABadSendParamEndsItsBlockEvent
}
// --- State Machine (W3C SCXML) ---

class ABadSendParamEndsItsBlockStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<ABadSendParamEndsItsBlockState, ABadSendParamEndsItsBlockEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `errors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `errors` was assigned a value of another type, or the engine refused.
     */
    fun errors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "errors")

    /**
     * §scxml-5.3: what the `partials` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `partials` was assigned a value of another type, or the engine refused.
     */
    fun partials(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "partials")

    /**
     * §scxml-5.3: what the `bares` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `bares` was assigned a value of another type, or the engine refused.
     */
    fun bares(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "bares")

    /**
     * §scxml-5.3: what the `after` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `after` was assigned a value of another type, or the engine refused.
     */
    fun after(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "after")

    /**
     * §scxml-5.3: what the `carried` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `carried` was assigned a value of another type, or the engine refused.
     */
    fun carried(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "carried")

    /**
     * §scxml-5.3: what the `v` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `v` was assigned a value of another type, or the engine refused.
     */
    fun v(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "v")

    /**
     * §scxml-5.3: what the `obj` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `obj` was assigned a value of another type, or the engine refused.
     *
     * The value as JSON text, serialised by the engine's own `JSON.stringify`
     * (§scxml-B-2) so the key order is the document's.
     */
    fun obj(): String? =
        com.sce.runtime.DatamodelRead.readJson(scriptEngine, scriptSessionId, "obj")

    override val initialState: ABadSendParamEndsItsBlockState = ABadSendParamEndsItsBlockState.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ABadSendParamEndsItsBlockState): Boolean = when (state) {
        is ABadSendParamEndsItsBlockState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ABadSendParamEndsItsBlockState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ABadSendParamEndsItsBlockState, HistoryId>> =
            listOf(StateTarget(ABadSendParamEndsItsBlockState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>(
            ABadSendParamEndsItsBlockState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>(
            ABadSendParamEndsItsBlockState.S0,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>(
            ABadSendParamEndsItsBlockState.S0,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 3, as the microstep reads it.
        val transitionS0At3 = EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>(
            ABadSendParamEndsItsBlockState.S0,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 4, as the microstep reads it.
        val transitionS0At4 = EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>(
            ABadSendParamEndsItsBlockState.S0,
            listOf(StateTarget(ABadSendParamEndsItsBlockState.Done)),
            4,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ABadSendParamEndsItsBlockState? = when (stateId) {
        "done" -> ABadSendParamEndsItsBlockState.Done
        "s0" -> ABadSendParamEndsItsBlockState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ABadSendParamEndsItsBlockState): String = when (state) {
        is ABadSendParamEndsItsBlockState.Done -> "done"
        is ABadSendParamEndsItsBlockState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ABadSendParamEndsItsBlockState): Int = when (state) {
        is ABadSendParamEndsItsBlockState.Done -> 1
        is ABadSendParamEndsItsBlockState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ABadSendParamEndsItsBlockEvent? = when (name) {
        "bare" -> ABadSendParamEndsItsBlockEvent.Bare
        "carried" -> ABadSendParamEndsItsBlockEvent.Carried
        "error.execution" -> ABadSendParamEndsItsBlockEvent.Error.Execution
        "finish" -> ABadSendParamEndsItsBlockEvent.Finish
        "partial" -> ABadSendParamEndsItsBlockEvent.Partial
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ABadSendParamEndsItsBlockEvent): String? = when (event) {
        is ABadSendParamEndsItsBlockEvent.Bare -> "bare"
        is ABadSendParamEndsItsBlockEvent.Carried -> "carried"
        is ABadSendParamEndsItsBlockEvent.Error.Execution -> "error.execution"
        is ABadSendParamEndsItsBlockEvent.Finish -> "finish"
        is ABadSendParamEndsItsBlockEvent.Partial -> "partial"
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
            "a_bad_send_param_ends_its_block",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'errors' with expr
        try {
            val initResult_errors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "errors", initResult_errors)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='errors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'partials' with expr
        try {
            val initResult_partials = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "partials", initResult_partials)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='partials'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'bares' with expr
        try {
            val initResult_bares = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "bares", initResult_bares)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='bares'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'after' with expr
        try {
            val initResult_after = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "after", initResult_after)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='after'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'carried' with expr
        try {
            val initResult_carried = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "carried", initResult_carried)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='carried'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'v' with expr
        try {
            val initResult_v = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("7", "7"))
            engine.setVariable(sid, "v", initResult_v)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='v'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'obj' with expr
        try {
            val initResult_obj = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "obj", initResult_obj)
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<data id='obj'> expr failed to evaluate")
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
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "an expression could not be serialised to JSON")
            ""
        }
    }

    // W3C SCXML 5.3: Assignment via script engine
    //
    // Both halves carry a language: this engine's Lua arm splices the location
    // in front of `=` and runs the result, so a write target written in
    // ECMAScript has to have been lowered too. Same split as
    // `ScxmlScriptEngine.assign`.
    // Returns whether the assignment took place; on failure error.execution is
    // already raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeAssign(location: com.sce.runtime.ScriptSource, expr: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: ABadSendParamEndsItsBlockEvent) {
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



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: ABadSendParamEndsItsBlockEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ABadSendParamEndsItsBlockState,
        event: ABadSendParamEndsItsBlockEvent?
    ): EnabledTransition<ABadSendParamEndsItsBlockState, HistoryId>? = when (state) {
        is ABadSendParamEndsItsBlockState.S0 -> when {
            event is ABadSendParamEndsItsBlockEvent.Error.Execution -> transitionS0At0
            event is ABadSendParamEndsItsBlockEvent.Partial && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.good == 1)", "_event.data.good === 1")) -> transitionS0At1
            event is ABadSendParamEndsItsBlockEvent.Bare -> transitionS0At2
            event is ABadSendParamEndsItsBlockEvent.Carried && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.p == 7)", "_event.data.p === 7")) -> transitionS0At3
            event is ABadSendParamEndsItsBlockEvent.Finish -> transitionS0At4
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_bad_send_param_ends_its_block.scxml:42 :: _machine
    override fun onEntry(state: ABadSendParamEndsItsBlockState, isDefaultEntry: Boolean) {
        when (state) {
            is ABadSendParamEndsItsBlockState.Done -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:98 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ABadSendParamEndsItsBlockState.S0 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:56 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/4
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "bad", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "good", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("1", "1")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='good'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(ABadSendParamEndsItsBlockEvent.Partial, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 1)", "after + 1"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 2/4
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "good", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("1", "1")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='good'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "bad", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("obj.missing.deep", "obj.missing.deep")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='bad'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(ABadSendParamEndsItsBlockEvent.Partial, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 100)", "after + 100"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 3/4
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='p'> names no location")
            paramFailed = true

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(ABadSendParamEndsItsBlockEvent.Bare, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("after", "after"), com.sce.runtime.ScriptSource.lua("_scxml_add(after, 10)", "after + 10"))) {
                return@run
            }
                }
                // W3C SCXML 3.8: Onentry block 4/4
                run {


            if (run send@{
            var paramFailed = false
            ensureScriptEngine()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "p", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("v", "v")))
            } catch (_: Exception) {
                raisePlatformError(ABadSendParamEndsItsBlockEvent.Error.Execution, "<send> <param name='p'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(ABadSendParamEndsItsBlockEvent.Carried, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_bad_send_param_ends_its_block.scxml:42 :: _machine
    override fun onExit(state: ABadSendParamEndsItsBlockState) {
        when (state) {
            is ABadSendParamEndsItsBlockState.Done -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:98 :: done :: _state_body
            }
            is ABadSendParamEndsItsBlockState.S0 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:56 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_bad_send_param_ends_its_block.scxml:42 :: _machine
    override fun executeTransitionContent(source: ABadSendParamEndsItsBlockState, transitionIndex: Int) {
        when (source) {
        is ABadSendParamEndsItsBlockState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:83 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
            }
            1 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:86 :: s0 :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("partials", "partials"), com.sce.runtime.ScriptSource.lua("_scxml_add(partials, 1)", "partials + 1"))) {
                return
            }
            }
            2 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:89 :: s0 :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("bares", "bares"), com.sce.runtime.ScriptSource.lua("_scxml_add(bares, 1)", "bares + 1"))) {
                return
            }
            }
            3 -> {
                // SCE-MAP: a_bad_send_param_ends_its_block.scxml:92 :: s0 :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("carried", "carried"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
