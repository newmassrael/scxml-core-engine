// SCE-GENERATED — DO NOT EDIT
// source-hash: aba7884293df72194a909ee8240f8376d0d98958e0d17eb2bec8d76ccd227f77

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_payload_rides_on_its_own_event/a_payload_rides_on_its_own_event.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine

package com.sce.integration.a_payload_rides_on_its_own_event

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface APayloadRidesOnItsOwnEventState : State {
    data object Done : APayloadRidesOnItsOwnEventState
    data object S0 : APayloadRidesOnItsOwnEventState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface APayloadRidesOnItsOwnEventEvent : Event {
    sealed interface Error : APayloadRidesOnItsOwnEventEvent {
        data object Execution : Error
    }
    data object ExtC : APayloadRidesOnItsOwnEventEvent
    data object ExtN : APayloadRidesOnItsOwnEventEvent
    data object ExtV : APayloadRidesOnItsOwnEventEvent
    data object Finish : APayloadRidesOnItsOwnEventEvent
    data object Plain1 : APayloadRidesOnItsOwnEventEvent
    data object Plain2 : APayloadRidesOnItsOwnEventEvent
    data object Plain3 : APayloadRidesOnItsOwnEventEvent
    data object Plain4 : APayloadRidesOnItsOwnEventEvent
    data object WithV : APayloadRidesOnItsOwnEventEvent
}
// --- State Machine (W3C SCXML) ---

class APayloadRidesOnItsOwnEventStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<APayloadRidesOnItsOwnEventState, APayloadRidesOnItsOwnEventEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `got` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `got` was assigned a value of another type, or the engine refused.
     */
    fun got(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "got")

    /**
     * §scxml-5.3: what the `stolen` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `stolen` was assigned a value of another type, or the engine refused.
     */
    fun stolen(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "stolen")

    /**
     * §scxml-5.3: what the `plains` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `plains` was assigned a value of another type, or the engine refused.
     */
    fun plains(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "plains")

    /**
     * §scxml-5.3: what the `v9` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `v9` was assigned a value of another type, or the engine refused.
     */
    fun v9(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "v9")

    override val initialState: APayloadRidesOnItsOwnEventState = APayloadRidesOnItsOwnEventState.S0

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
    override fun isFinalState(state: APayloadRidesOnItsOwnEventState): Boolean = when (state) {
        is APayloadRidesOnItsOwnEventState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<APayloadRidesOnItsOwnEventState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<APayloadRidesOnItsOwnEventState, HistoryId>> =
            listOf(StateTarget(APayloadRidesOnItsOwnEventState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 3, as the microstep reads it.
        val transitionS0At3 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 4, as the microstep reads it.
        val transitionS0At4 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 5, as the microstep reads it.
        val transitionS0At5 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            emptyList(),
            5,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 6, as the microstep reads it.
        val transitionS0At6 = EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>(
            APayloadRidesOnItsOwnEventState.S0,
            listOf(StateTarget(APayloadRidesOnItsOwnEventState.Done)),
            6,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): APayloadRidesOnItsOwnEventState? = when (stateId) {
        "done" -> APayloadRidesOnItsOwnEventState.Done
        "s0" -> APayloadRidesOnItsOwnEventState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: APayloadRidesOnItsOwnEventState): String = when (state) {
        is APayloadRidesOnItsOwnEventState.Done -> "done"
        is APayloadRidesOnItsOwnEventState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: APayloadRidesOnItsOwnEventState): Int = when (state) {
        is APayloadRidesOnItsOwnEventState.Done -> 1
        is APayloadRidesOnItsOwnEventState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): APayloadRidesOnItsOwnEventEvent? = when (name) {
        "error.execution" -> APayloadRidesOnItsOwnEventEvent.Error.Execution
        "extC" -> APayloadRidesOnItsOwnEventEvent.ExtC
        "extN" -> APayloadRidesOnItsOwnEventEvent.ExtN
        "extV" -> APayloadRidesOnItsOwnEventEvent.ExtV
        "finish" -> APayloadRidesOnItsOwnEventEvent.Finish
        "plain1" -> APayloadRidesOnItsOwnEventEvent.Plain1
        "plain2" -> APayloadRidesOnItsOwnEventEvent.Plain2
        "plain3" -> APayloadRidesOnItsOwnEventEvent.Plain3
        "plain4" -> APayloadRidesOnItsOwnEventEvent.Plain4
        "withV" -> APayloadRidesOnItsOwnEventEvent.WithV
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: APayloadRidesOnItsOwnEventEvent): String? = when (event) {
        is APayloadRidesOnItsOwnEventEvent.Error.Execution -> "error.execution"
        is APayloadRidesOnItsOwnEventEvent.ExtC -> "extC"
        is APayloadRidesOnItsOwnEventEvent.ExtN -> "extN"
        is APayloadRidesOnItsOwnEventEvent.ExtV -> "extV"
        is APayloadRidesOnItsOwnEventEvent.Finish -> "finish"
        is APayloadRidesOnItsOwnEventEvent.Plain1 -> "plain1"
        is APayloadRidesOnItsOwnEventEvent.Plain2 -> "plain2"
        is APayloadRidesOnItsOwnEventEvent.Plain3 -> "plain3"
        is APayloadRidesOnItsOwnEventEvent.Plain4 -> "plain4"
        is APayloadRidesOnItsOwnEventEvent.WithV -> "withV"
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
            "a_payload_rides_on_its_own_event",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'got' with expr
        try {
            val initResult_got = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "got", initResult_got)
        } catch (e: Exception) {
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<data id='got'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'stolen' with expr
        try {
            val initResult_stolen = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "stolen", initResult_stolen)
        } catch (e: Exception) {
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<data id='stolen'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'plains' with expr
        try {
            val initResult_plains = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "plains", initResult_plains)
        } catch (e: Exception) {
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<data id='plains'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'v9' with expr
        try {
            val initResult_v9 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("9", "9"))
            engine.setVariable(sid, "v9", initResult_v9)
        } catch (e: Exception) {
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<data id='v9'> expr failed to evaluate")
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: APayloadRidesOnItsOwnEventEvent) {
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
    override fun bindCurrentEvent(event: APayloadRidesOnItsOwnEventEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: APayloadRidesOnItsOwnEventState,
        event: APayloadRidesOnItsOwnEventEvent?
    ): EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>? = when (state) {
        is APayloadRidesOnItsOwnEventState.S0 -> when {
            (event is APayloadRidesOnItsOwnEventEvent.Plain1 || event is APayloadRidesOnItsOwnEventEvent.Plain2 || event is APayloadRidesOnItsOwnEventEvent.Plain3 || event is APayloadRidesOnItsOwnEventEvent.Plain4) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_event.data)", "_event.data")) -> transitionS0At0
            (event is APayloadRidesOnItsOwnEventEvent.Plain1 || event is APayloadRidesOnItsOwnEventEvent.Plain2 || event is APayloadRidesOnItsOwnEventEvent.Plain3 || event is APayloadRidesOnItsOwnEventEvent.Plain4) -> transitionS0At1
            event is APayloadRidesOnItsOwnEventEvent.WithV && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v == 7)", "_event.data.v === 7")) -> transitionS0At2
            event is APayloadRidesOnItsOwnEventEvent.ExtV && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v == 8)", "_event.data.v === 8")) -> transitionS0At3
            event is APayloadRidesOnItsOwnEventEvent.ExtN && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v9 == 9)", "_event.data.v9 === 9")) -> transitionS0At4
            event is APayloadRidesOnItsOwnEventEvent.ExtC && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 10)", "_event.data === 10")) -> transitionS0At5
            event is APayloadRidesOnItsOwnEventEvent.Finish -> transitionS0At6
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun onEntry(state: APayloadRidesOnItsOwnEventState, isDefaultEntry: Boolean) {
        when (state) {
            is APayloadRidesOnItsOwnEventState.Done -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:82 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is APayloadRidesOnItsOwnEventState.S0 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:37 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/4
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain1)


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                ensureScriptEngine()
                val engineI = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidI = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsI = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsI, "v", engineI.evaluateExpr(sidI, com.sce.runtime.ScriptSource.lua("7", "7")))
                } catch (_: Exception) {
                    raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> <param name='v'> could not be read")
                    paramFailed = true
                }


                raiseInternal(APayloadRidesOnItsOwnEventEvent.WithV, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/4
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {


            if (run send@{
            var paramFailed = false
            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsE, "v", engineE.evaluateExpr(sidE, com.sce.runtime.ScriptSource.lua("8", "8")))
                } catch (_: Exception) {
                    raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> <param name='v'> could not be read")
                    paramFailed = true
                }


                val eventDataE = buildJsonFromParams(paramsE)
                send(APayloadRidesOnItsOwnEventEvent.ExtV, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = eventDataE))
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain2)
                }
                // W3C SCXML 3.8: Onentry block 3/4
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {


            if (run send@{
            // W3C SCXML 5.10: Evaluate params/namelist for event data
            run {
                ensureScriptEngine()
                val engineE = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidE = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsE = mutableMapOf<String, Any?>()
                // W3C SCXML C.1: Evaluate namelist — abort send on error (C++ NamelistHelper pattern, test553)
                if (!engineE.hasVariable(sidE, "v9")) {
                    raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> namelist names 'v9', which is not declared")
                    return@send false
                }
                try { paramsE["v9"] = engineE.getVariable(sidE, "v9") } catch (_: Exception) {
                    raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> namelist entry 'v9' failed to evaluate")
                    return@send false
                }

                val eventDataE = buildJsonFromParams(paramsE)
                send(APayloadRidesOnItsOwnEventEvent.ExtN, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = eventDataE))
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain3)
                }
                // W3C SCXML 3.8: Onentry block 4/4
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {


            if (run send@{
            // W3C SCXML B.2: the reading is decided at build time; a value
            // is evaluated here and serialized, XML is handed on as source.
            send(APayloadRidesOnItsOwnEventEvent.ExtC, EventMetadata.external(sendId = "__send_3", origin = scriptSessionId ?: "", data = evaluateSendContent(com.sce.runtime.ScriptSource.lua("10", "10"))))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain4)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun onExit(state: APayloadRidesOnItsOwnEventState) {
        when (state) {
            is APayloadRidesOnItsOwnEventState.Done -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:82 :: done :: _state_body
            }
            is APayloadRidesOnItsOwnEventState.S0 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:37 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun executeTransitionContent(source: APayloadRidesOnItsOwnEventState, transitionIndex: Int) {
        when (source) {
        is APayloadRidesOnItsOwnEventState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:61 :: s0 :: _transition_0


            executeAssign(com.sce.runtime.ScriptSource.lua("stolen", "stolen"), com.sce.runtime.ScriptSource.lua("_scxml_add(stolen, 1)", "stolen + 1"))
            }
            1 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:64 :: s0 :: _transition_1


            executeAssign(com.sce.runtime.ScriptSource.lua("plains", "plains"), com.sce.runtime.ScriptSource.lua("_scxml_add(plains, 1)", "plains + 1"))
            }
            2 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:67 :: s0 :: _transition_2


            executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))
            }
            3 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:70 :: s0 :: _transition_3


            executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))
            }
            4 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:73 :: s0 :: _transition_4


            executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))
            }
            5 -> {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:76 :: s0 :: _transition_5


            executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))
            }
            else -> {}
        }
        else -> {}
        }
    }
}
