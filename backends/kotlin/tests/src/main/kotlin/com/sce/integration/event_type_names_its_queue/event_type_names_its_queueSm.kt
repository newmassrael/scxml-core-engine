// SCE-GENERATED — DO NOT EDIT
// source-hash: 0e632ea18e896bea24cf74313657aba02d906a276b3b64be501fc7738588903c

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_type_names_its_queue/event_type_names_its_queue.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_type_names_its_queue.scxml:29 :: _machine

package com.sce.integration.event_type_names_its_queue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventTypeNamesItsQueueState : State {
    data object Done : EventTypeNamesItsQueueState
    data object S0 : EventTypeNamesItsQueueState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventTypeNamesItsQueueEvent : Event {
    sealed interface Error : EventTypeNamesItsQueueEvent {
        data object Execution : Error
    }
    data object Ext : EventTypeNamesItsQueueEvent
    data object Int : EventTypeNamesItsQueueEvent
    data object ViaInternalSend : EventTypeNamesItsQueueEvent
}
// --- State Machine (W3C SCXML) ---

class EventTypeNamesItsQueueStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventTypeNamesItsQueueState, EventTypeNamesItsQueueEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `intCode` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `intCode` was assigned a value of another type, or the engine refused.
     */
    fun intCode(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "intCode")

    /**
     * §scxml-5.3: what the `sendCode` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `sendCode` was assigned a value of another type, or the engine refused.
     */
    fun sendCode(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "sendCode")

    /**
     * §scxml-5.3: what the `extCode` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `extCode` was assigned a value of another type, or the engine refused.
     */
    fun extCode(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "extCode")

    override val initialState: EventTypeNamesItsQueueState = EventTypeNamesItsQueueState.S0

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
    override fun isFinalState(state: EventTypeNamesItsQueueState): Boolean = when (state) {
        is EventTypeNamesItsQueueState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EventTypeNamesItsQueueState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<EventTypeNamesItsQueueState, HistoryId>> =
            listOf(StateTarget(EventTypeNamesItsQueueState.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<EventTypeNamesItsQueueState, HistoryId>(
            EventTypeNamesItsQueueState.S0,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<EventTypeNamesItsQueueState, HistoryId>(
            EventTypeNamesItsQueueState.S0,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<EventTypeNamesItsQueueState, HistoryId>(
            EventTypeNamesItsQueueState.S0,
            listOf(StateTarget(EventTypeNamesItsQueueState.Done)),
            2,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EventTypeNamesItsQueueState? = when (stateId) {
        "done" -> EventTypeNamesItsQueueState.Done
        "s0" -> EventTypeNamesItsQueueState.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventTypeNamesItsQueueState): String = when (state) {
        is EventTypeNamesItsQueueState.Done -> "done"
        is EventTypeNamesItsQueueState.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: EventTypeNamesItsQueueState): Int = when (state) {
        is EventTypeNamesItsQueueState.Done -> 1
        is EventTypeNamesItsQueueState.S0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventTypeNamesItsQueueEvent? = when (name) {
        "error.execution" -> EventTypeNamesItsQueueEvent.Error.Execution
        "ext" -> EventTypeNamesItsQueueEvent.Ext
        "int" -> EventTypeNamesItsQueueEvent.Int
        "viaInternalSend" -> EventTypeNamesItsQueueEvent.ViaInternalSend
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventTypeNamesItsQueueEvent): String? = when (event) {
        is EventTypeNamesItsQueueEvent.Error.Execution -> "error.execution"
        is EventTypeNamesItsQueueEvent.Ext -> "ext"
        is EventTypeNamesItsQueueEvent.Int -> "int"
        is EventTypeNamesItsQueueEvent.ViaInternalSend -> "viaInternalSend"
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
            "event_type_names_its_queue",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        // W3C SCXML 5.3: Initialize variable 'intCode' with expr
        try {
            val initResult_intCode = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "intCode", initResult_intCode)
        } catch (e: Exception) {
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<data id='intCode'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'sendCode' with expr
        try {
            val initResult_sendCode = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "sendCode", initResult_sendCode)
        } catch (e: Exception) {
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<data id='sendCode'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'extCode' with expr
        try {
            val initResult_extCode = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "extCode", initResult_extCode)
        } catch (e: Exception) {
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<data id='extCode'> expr failed to evaluate")
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
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "a <transition> cond failed to evaluate")
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
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<script> failed to execute")
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventTypeNamesItsQueueEvent) {
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
    override fun bindCurrentEvent(event: EventTypeNamesItsQueueEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: EventTypeNamesItsQueueState,
        event: EventTypeNamesItsQueueEvent?
    ): EnabledTransition<EventTypeNamesItsQueueState, HistoryId>? = when (state) {
        is EventTypeNamesItsQueueState.S0 -> when {
            event is EventTypeNamesItsQueueEvent.Int -> transitionS0At0
            event is EventTypeNamesItsQueueEvent.ViaInternalSend -> transitionS0At1
            event is EventTypeNamesItsQueueEvent.Ext -> transitionS0At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_type_names_its_queue.scxml:29 :: _machine
    override fun onEntry(state: EventTypeNamesItsQueueState, isDefaultEntry: Boolean) {
        when (state) {
            is EventTypeNamesItsQueueState.Done -> {
                // SCE-MAP: event_type_names_its_queue.scxml:79 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is EventTypeNamesItsQueueState.S0 -> {
                // SCE-MAP: event_type_names_its_queue.scxml:39 :: s0 :: _state_body


            send(EventTypeNamesItsQueueEvent.Ext, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))

            raiseInternal(EventTypeNamesItsQueueEvent.Int)


            // W3C SCXML 5.10: An internal send carries `_event.data` just as
            // an external one does. Before this the payload was dropped
            // silently — the event was queued with no data at all.
            run {
                ensureScriptEngine()
                val engineI = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                val sidI = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                val paramsI = mutableMapOf<String, Any?>()
                try {
                    putParam(paramsI, "a", engineI.evaluateExpr(sidI, com.sce.runtime.ScriptSource.lua("1", "1")))
                } catch (_: Exception) {
                    // W3C SCXML 5.7.1: report the failure and omit the name and value.
                    raisePlatformError(EventTypeNamesItsQueueEvent.Error.Execution, "<send> <param name='a'> expr failed to evaluate")
                }

                raiseInternal(EventTypeNamesItsQueueEvent.ViaInternalSend, EventMetadata.internal(buildJsonFromParams(paramsI)))
            }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_type_names_its_queue.scxml:29 :: _machine
    override fun onExit(state: EventTypeNamesItsQueueState) {
        when (state) {
            is EventTypeNamesItsQueueState.Done -> {
                // SCE-MAP: event_type_names_its_queue.scxml:79 :: done :: _state_body
            }
            is EventTypeNamesItsQueueState.S0 -> {
                // SCE-MAP: event_type_names_its_queue.scxml:39 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: event_type_names_its_queue.scxml:29 :: _machine
    override fun executeTransitionContent(source: EventTypeNamesItsQueueState, transitionIndex: Int) {
        when (source) {
        is EventTypeNamesItsQueueState.S0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: event_type_names_its_queue.scxml:48 :: s0 :: _transition_0


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"internal\")", "_event.type == 'internal'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("intCode", "intCode"), com.sce.runtime.ScriptSource.lua("1", "1"))
            } else if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"external\")", "_event.type == 'external'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("intCode", "intCode"), com.sce.runtime.ScriptSource.lua("2", "2"))
            } else {


            executeAssign(com.sce.runtime.ScriptSource.lua("intCode", "intCode"), com.sce.runtime.ScriptSource.lua("3", "3"))
            }
            }
            1 -> {
                // SCE-MAP: event_type_names_its_queue.scxml:58 :: s0 :: _transition_1


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"internal\")", "_event.type == 'internal'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("sendCode", "sendCode"), com.sce.runtime.ScriptSource.lua("1", "1"))
            } else if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"external\")", "_event.type == 'external'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("sendCode", "sendCode"), com.sce.runtime.ScriptSource.lua("2", "2"))
            } else {


            executeAssign(com.sce.runtime.ScriptSource.lua("sendCode", "sendCode"), com.sce.runtime.ScriptSource.lua("3", "3"))
            }
            }
            2 -> {
                // SCE-MAP: event_type_names_its_queue.scxml:68 :: s0 :: _transition_2


            if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"internal\")", "_event.type == 'internal'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("extCode", "extCode"), com.sce.runtime.ScriptSource.lua("1", "1"))
            } else if (safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.type, \"external\")", "_event.type == 'external'"))) {


            executeAssign(com.sce.runtime.ScriptSource.lua("extCode", "extCode"), com.sce.runtime.ScriptSource.lua("2", "2"))
            } else {


            executeAssign(com.sce.runtime.ScriptSource.lua("extCode", "extCode"), com.sce.runtime.ScriptSource.lua("3", "3"))
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
