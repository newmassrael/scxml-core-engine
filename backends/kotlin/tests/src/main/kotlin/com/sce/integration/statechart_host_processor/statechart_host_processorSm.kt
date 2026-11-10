// SCE-GENERATED — DO NOT EDIT
// source-hash: 7e9246f9bdadc8c18a240b365b4cc726cf6b8bab9981f448fd674ebdd45b3460

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_host_processor.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_host_processor.scxml:44 :: _machine

package com.sce.integration.statechart_host_processor

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartHostProcessorState : State {
    data object Dispatching : StatechartHostProcessorState
    data object Pairs : StatechartHostProcessorState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartHostProcessorEvent : Event {
    data object Advance : StatechartHostProcessorEvent
    sealed interface Answer : StatechartHostProcessorEvent {
        data object Please : Answer
    }
    sealed interface Error : StatechartHostProcessorEvent {
        data object Execution : Error
    }
    sealed interface Plain : StatechartHostProcessorEvent {
        data object Arrived : Plain
    }
    sealed interface Turn : StatechartHostProcessorEvent {
        data object Done : Turn
    }
    sealed interface Watch : StatechartHostProcessorEvent {
        data object Answer : Watch
        data object Pairs : Watch
        data object Turn : Watch
    }
}
// --- State Machine (W3C SCXML) ---

class StatechartHostProcessorStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<StatechartHostProcessorState, StatechartHostProcessorEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `served` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `served` was assigned a value of another type, or the engine refused.
     */
    fun served(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "served")

    /**
     * §scxml-5.3: what the `refused` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `refused` was assigned a value of another type, or the engine refused.
     */
    fun refused(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "refused")

    /**
     * §scxml-5.3: what the `plain` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `plain` was assigned a value of another type, or the engine refused.
     */
    fun plain(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "plain")

    /**
     * §scxml-5.3: what the `paramErrors` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `paramErrors` was assigned a value of another type, or the engine refused.
     */
    fun paramErrors(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "paramErrors")

    /**
     * §scxml-5.3: what the `mode` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `mode` was assigned a value of another type, or the engine refused.
     */
    fun mode(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "mode")

    override val initialState: StatechartHostProcessorState = StatechartHostProcessorState.Dispatching

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartHostProcessorState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartHostProcessorState, HistoryId>> =
            listOf(StateTarget(StatechartHostProcessorState.Dispatching))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: dispatching's transition 0, as the microstep reads it.
        val transitionDispatchingAt0 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Dispatching,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: dispatching's transition 1, as the microstep reads it.
        val transitionDispatchingAt1 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Dispatching,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: dispatching's transition 2, as the microstep reads it.
        val transitionDispatchingAt2 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Dispatching,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: dispatching's transition 3, as the microstep reads it.
        val transitionDispatchingAt3 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Dispatching,
            listOf(StateTarget(StatechartHostProcessorState.Pairs)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: pairs's transition 0, as the microstep reads it.
        val transitionPairsAt0 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Pairs,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: pairs's transition 1, as the microstep reads it.
        val transitionPairsAt1 = EnabledTransition<StatechartHostProcessorState, HistoryId>(
            StatechartHostProcessorState.Pairs,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartHostProcessorState? = when (stateId) {
        "dispatching" -> StatechartHostProcessorState.Dispatching
        "pairs" -> StatechartHostProcessorState.Pairs
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartHostProcessorState): String = when (state) {
        is StatechartHostProcessorState.Dispatching -> "dispatching"
        is StatechartHostProcessorState.Pairs -> "pairs"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartHostProcessorState): Int = when (state) {
        is StatechartHostProcessorState.Dispatching -> 0
        is StatechartHostProcessorState.Pairs -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartHostProcessorEvent? = when (name) {
        "advance" -> StatechartHostProcessorEvent.Advance
        "answer.please" -> StatechartHostProcessorEvent.Answer.Please
        "error.execution" -> StatechartHostProcessorEvent.Error.Execution
        "plain.arrived" -> StatechartHostProcessorEvent.Plain.Arrived
        "turn.done" -> StatechartHostProcessorEvent.Turn.Done
        "watch.answer" -> StatechartHostProcessorEvent.Watch.Answer
        "watch.pairs" -> StatechartHostProcessorEvent.Watch.Pairs
        "watch.turn" -> StatechartHostProcessorEvent.Watch.Turn
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartHostProcessorEvent): String? = when (event) {
        is StatechartHostProcessorEvent.Advance -> "advance"
        is StatechartHostProcessorEvent.Answer.Please -> "answer.please"
        is StatechartHostProcessorEvent.Error.Execution -> "error.execution"
        is StatechartHostProcessorEvent.Plain.Arrived -> "plain.arrived"
        is StatechartHostProcessorEvent.Turn.Done -> "turn.done"
        is StatechartHostProcessorEvent.Watch.Answer -> "watch.answer"
        is StatechartHostProcessorEvent.Watch.Pairs -> "watch.pairs"
        is StatechartHostProcessorEvent.Watch.Turn -> "watch.turn"
    }




    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // docs/adr/0096: make this machine's script session, the first step of
    // every start (`initialize`, `start`, `enterAt`, and the start of a child
    // this machine is). The runtime's `makeScriptSession` takes the three steps
    // (create the session, which loads the shared library into it, then bind
    // the system variables), releases what a failed step made and answers the
    // failure, which refuses the start with nothing entered; a later start
    // asks for the session again.
    override fun establishScriptSession(): com.sce.runtime.ScriptSessionFailure? {
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        makeScriptSession(
            engine,
            "statechart_host_processor",
        )?.let { return it }
        return null
    }

    // W3C SCXML 5.3: initialize the datamodel in the session the start
    // established, before any state is entered: the document's <data>, its
    // states' early-bound <data>, the invoke params the parent handed this
    // session, then the top-level <script>s. The params are applied before the
    // scripts, which read the datamodel the invocation started it with: the
    // order of interpret in Appendix D, and that of every other backend.
    override fun declareDatamodel() {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")

        declareVariablesPart0(engine, sid)


        // W3C SCXML 6.4: Apply pending invoke params from parent, overriding the
        // <data> default just bound, and only for a name this session declares
        // (C++ DatamodelValidationHelper). The parent hands them before this
        // session's start (`setInvokeParams`) and this session applies them, so
        // a child's session is made by the child.
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }
    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // W3C SCXML 5.3: Initialize variable 'served' with expr
        try {
            val initResult_served = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "served", initResult_served)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='served'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'refused' with expr
        try {
            val initResult_refused = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "refused", initResult_refused)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='refused'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'plain' with expr
        try {
            val initResult_plain = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "plain", initResult_plain)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='plain'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'paramErrors' with expr
        try {
            val initResult_paramErrors = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "paramErrors", initResult_paramErrors)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='paramErrors'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'mode' with expr
        try {
            val initResult_mode = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"pairs\"", "'pairs'"))
            engine.setVariable(sid, "mode", initResult_mode)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='mode'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'nothing' with expr
        try {
            val initResult_nothing = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("nil", "null"))
            engine.setVariable(sid, "nothing", initResult_nothing)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<data id='nothing'> expr failed to evaluate")
        }
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
    private fun safeEvaluateGuard(guardExpr: com.sce.runtime.ScriptSource): Boolean =
        evaluateGuardRaising(guardExpr, "a <transition> cond failed to evaluate") ?: false

    // W3C SCXML 5.9.1 + 4.9: an <if> or <elseif> cond, evaluated and reported
    // as a transition guard is. A failure also runs [onFailure]: the <if>
    // still selects on `false`, and is then the element whose processing
    // raised, so its block ends after it.
    @Suppress("unused")
    private inline fun evaluateIfCond(guardExpr: com.sce.runtime.ScriptSource, onFailure: () -> Unit): Boolean {
        val result = evaluateGuardRaising(guardExpr, "an <if> cond failed to evaluate")
        if (result == null) onFailure()
        return result ?: false
    }

    // W3C SCXML 5.9.1: a cond that cannot be evaluated raises error.execution;
    // `null` says so, where a bare `false` could not.
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, reason: String): Boolean? {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, reason)
            null
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
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
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "an expression could not be serialised to JSON")
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
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: StatechartHostProcessorEvent) {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        val meta = currentEventMetadata
        // §scxml-5.10: the name the event ARRIVED under, which is longer than its
        // member's when a name the document does not write was matched through a
        // prefix (§scxml-3.12.1).
        val eventName = meta.name.ifEmpty { eventNameOf(event) ?: return }
        // W3C SCXML 5.10.1: C++ classifyEventType — platform events override type
        val effectiveType = when {
            eventName.startsWith("done.") || eventName.startsWith("error.") -> "platform"
            else -> meta.type
        }
        // W3C SCXML 5.10.1 + C.1: `_event.origin` and `_event.origintype` are the
        // reply address the processor that delivered the event wrote where it
        // built it, as one value (docs/adr/0090): the SCXML processor's type and
        // the sender's published `_ioprocessors` location for an event a session
        // sent, a host's own pair for what a host named, the Mesh transport's
        // for a peer. Read here as written, with no conversion. An event no
        // processor stamped — an internal or platform event, a host-posted event
        // or reply that names no party, a BasicHTTP request — has none and is
        // handed over as the empty string, which the binding surfaces as blank
        // (undefined, §scxml-B-2-8): nothing is filled in for it, neither this
        // session's own address nor the SCXML processor's type.
        val eventOrigin = meta.origin
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
                origin = eventOrigin?.address ?: "",
                originType = eventOrigin?.processor ?: "",
                invokeId = meta.invokeId
            )
        )
        notePayloadReading(event, payloadReading)
    }



    // W3C SCXML 5.10: bind the event as the `_event` its transitions' guards
    // read — once, before the first guard runs, and not for an eventless
    // selection, which has no event of its own.
    override fun bindCurrentEvent(event: StatechartHostProcessorEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StatechartHostProcessorState, event: StatechartHostProcessorEvent?): EnabledTransition<StatechartHostProcessorState, HistoryId>? = when (state) {
        is StatechartHostProcessorState.Dispatching -> firstEnabledTransitionOfDispatching(state, event)
        is StatechartHostProcessorState.Pairs -> firstEnabledTransitionOfPairs(state, event)
    }


    // dispatching's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfDispatching(state: StatechartHostProcessorState, event: StatechartHostProcessorEvent?): EnabledTransition<StatechartHostProcessorState, HistoryId>? = when {
        event is StatechartHostProcessorEvent.Plain.Arrived -> TransitionsPart0.transitionDispatchingAt0
        event is StatechartHostProcessorEvent.Turn.Done -> TransitionsPart0.transitionDispatchingAt1
        event is StatechartHostProcessorEvent.Error.Execution -> TransitionsPart0.transitionDispatchingAt2
        event is StatechartHostProcessorEvent.Advance && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(served > 0)", "served > 0")) -> TransitionsPart0.transitionDispatchingAt3
        else -> null
    }


    // pairs's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfPairs(state: StatechartHostProcessorState, event: StatechartHostProcessorEvent?): EnabledTransition<StatechartHostProcessorState, HistoryId>? = when {
        event is StatechartHostProcessorEvent.Error.Execution -> TransitionsPart0.transitionPairsAt0
        event is StatechartHostProcessorEvent.Answer.Please -> TransitionsPart0.transitionPairsAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_host_processor.scxml:44 :: _machine
    override fun onEntry(state: StatechartHostProcessorState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartHostProcessorState.Dispatching -> onEntryOfDispatching()
            is StatechartHostProcessorState.Pairs -> onEntryOfPairs()
        }
    }


    private fun onEntryOfDispatching() {
                // SCE-MAP: statechart_host_processor.scxml:59 :: dispatching :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(StatechartHostProcessorEvent.Plain.Arrived, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "within", "2500")
            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "watch.turn",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: send to this session's external queue
            send(StatechartHostProcessorEvent.Advance, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfPairs() {
                // SCE-MAP: statechart_host_processor.scxml:88 :: pairs :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            // W3C SCXML 6.2 + B.2 (test553): a namelist names locations, and one
            // that is not declared is an argument that cannot be evaluated —
            // one error however many of its names are bad.
            val sendNamelist = mutableListOf<Pair<String, Any?>>()
            if (!argEngine.hasVariable(argSid, "mode")) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send> namelist names 'mode', which is not declared", "__send_4")
                return@send true
            }
            try {
                sendNamelist.add("mode" to argEngine.getVariable(argSid, "mode"))
            } catch (_: Exception) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send> namelist entry 'mode' could not be read", "__send_4")
                return@send true
            }
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kept", "here")
            try {
                putParam(sendPayload, "broken", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("nothing.deep", "nothing.deep")))
            } catch (_: Exception) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send> <param name='broken'> could not be read")
                paramFailed = true
            }

            // W3C SCXML C.1: namelist variables become top-level keys in the
            // data table — the values the prologue read, after the params.
            for ((name, value) in sendNamelist) sendPayload[name] = value
            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "watch.pairs",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_4",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_4")
            }
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_host_processor.scxml:44 :: _machine
    override fun onExit(state: StatechartHostProcessorState) {
        when (state) {
            is StatechartHostProcessorState.Dispatching -> onExitOfDispatching()
            is StatechartHostProcessorState.Pairs -> onExitOfPairs()
        }
    }


    private fun onExitOfDispatching() {
                // SCE-MAP: statechart_host_processor.scxml:59 :: dispatching :: _state_body
    }

    private fun onExitOfPairs() {
                // SCE-MAP: statechart_host_processor.scxml:88 :: pairs :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_host_processor.scxml:44 :: _machine
    override fun executeTransitionContent(source: StatechartHostProcessorState, transitionIndex: Int) {
        when (source) {
            is StatechartHostProcessorState.Dispatching -> transitionContentOfDispatching(transitionIndex)
            is StatechartHostProcessorState.Pairs -> transitionContentOfPairs(transitionIndex)
        }
    }


    // The content of `dispatching`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfDispatching(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfDispatchingAt0()
            1 -> transitionContentOfDispatchingAt1()
            2 -> transitionContentOfDispatchingAt2()
            else -> {}
        }
    }


    private fun transitionContentOfDispatchingAt0() {
                // SCE-MAP: statechart_host_processor.scxml:73 :: dispatching :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("plain", "plain"), com.sce.runtime.ScriptSource.lua("_scxml_add(plain, 1)", "plain + 1"))) {
                return
            }
    }

    private fun transitionContentOfDispatchingAt1() {
                // SCE-MAP: statechart_host_processor.scxml:76 :: dispatching :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("served", "served"), com.sce.runtime.ScriptSource.lua("_scxml_add(served, 1)", "served + 1"))) {
                return
            }
    }

    private fun transitionContentOfDispatchingAt2() {
                // SCE-MAP: statechart_host_processor.scxml:79 :: dispatching :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("refused", "refused"), com.sce.runtime.ScriptSource.lua("_scxml_add(refused, 1)", "refused + 1"))) {
                return
            }
    }

    // The content of `pairs`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfPairs(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfPairsAt0()
            1 -> transitionContentOfPairsAt1()
            else -> {}
        }
    }


    private fun transitionContentOfPairsAt0() {
                // SCE-MAP: statechart_host_processor.scxml:95 :: pairs :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("paramErrors", "paramErrors"), com.sce.runtime.ScriptSource.lua("_scxml_add(paramErrors, 1)", "paramErrors + 1"))) {
                return
            }
    }

    private fun transitionContentOfPairsAt1() {
                // SCE-MAP: statechart_host_processor.scxml:103 :: pairs :: _transition_1


            if (run send@{
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "watch.answer",
                target = "",
                content = "",
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("x-sce-host")) {
                raisePlatformError(StatechartHostProcessorEvent.Error.Execution, "<send type='x-sce-host'> names a processor the host declared but never registered", "__send_3")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
