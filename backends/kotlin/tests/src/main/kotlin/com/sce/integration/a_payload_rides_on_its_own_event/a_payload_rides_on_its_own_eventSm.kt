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

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

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
    }

    private object TransitionsPart0 {

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
            "a_payload_rides_on_its_own_event",
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, reason)
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: APayloadRidesOnItsOwnEventEvent) {
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
    override fun bindCurrentEvent(event: APayloadRidesOnItsOwnEventEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: APayloadRidesOnItsOwnEventState, event: APayloadRidesOnItsOwnEventEvent?): EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>? = when (state) {
        is APayloadRidesOnItsOwnEventState.S0 -> firstEnabledTransitionOfS0(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: APayloadRidesOnItsOwnEventState, event: APayloadRidesOnItsOwnEventEvent?): EnabledTransition<APayloadRidesOnItsOwnEventState, HistoryId>? = when {
        (event is APayloadRidesOnItsOwnEventEvent.Plain1 || event is APayloadRidesOnItsOwnEventEvent.Plain2 || event is APayloadRidesOnItsOwnEventEvent.Plain3 || event is APayloadRidesOnItsOwnEventEvent.Plain4) && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_truthy(_event.data)", "_event.data")) -> TransitionsPart0.transitionS0At0
        (event is APayloadRidesOnItsOwnEventEvent.Plain1 || event is APayloadRidesOnItsOwnEventEvent.Plain2 || event is APayloadRidesOnItsOwnEventEvent.Plain3 || event is APayloadRidesOnItsOwnEventEvent.Plain4) -> TransitionsPart0.transitionS0At1
        event is APayloadRidesOnItsOwnEventEvent.WithV && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v == 7)", "_event.data.v === 7")) -> TransitionsPart0.transitionS0At2
        event is APayloadRidesOnItsOwnEventEvent.ExtV && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v == 8)", "_event.data.v === 8")) -> TransitionsPart0.transitionS0At3
        event is APayloadRidesOnItsOwnEventEvent.ExtN && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data.v9 == 9)", "_event.data.v9 === 9")) -> TransitionsPart0.transitionS0At4
        event is APayloadRidesOnItsOwnEventEvent.ExtC && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 10)", "_event.data === 10")) -> TransitionsPart0.transitionS0At5
        event is APayloadRidesOnItsOwnEventEvent.Finish -> TransitionsPart0.transitionS0At6
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun onEntry(state: APayloadRidesOnItsOwnEventState, isDefaultEntry: Boolean) {
        when (state) {
            is APayloadRidesOnItsOwnEventState.Done -> onEntryOfDone()
            is APayloadRidesOnItsOwnEventState.S0 -> onEntryOfS0()
        }
    }


    private fun onEntryOfDone() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:82 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:37 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/4
                run {

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain1)


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "v", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("7", "7")))
            } catch (_: Exception) {
                raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> <param name='v'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(APayloadRidesOnItsOwnEventEvent.WithV, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 3.8: Onentry block 2/4
                run {


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "v", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("8", "8")))
            } catch (_: Exception) {
                raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> <param name='v'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(APayloadRidesOnItsOwnEventEvent.ExtV, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain2)
                }
                // W3C SCXML 3.8: Onentry block 3/4
                run {


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            // W3C SCXML 6.2 + B.2 (test553): a namelist names locations, and one
            // that is not declared is an argument that cannot be evaluated —
            // one error however many of its names are bad.
            val sendNamelist = mutableListOf<Pair<String, Any?>>()
            if (!argEngine.hasVariable(argSid, "v9")) {
                raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> namelist names 'v9', which is not declared", "__send_2")
                return@send true
            }
            try {
                sendNamelist.add("v9" to argEngine.getVariable(argSid, "v9"))
            } catch (_: Exception) {
                raisePlatformError(APayloadRidesOnItsOwnEventEvent.Error.Execution, "<send> namelist entry 'v9' could not be read", "__send_2")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            // W3C SCXML C.1: namelist variables become top-level keys in the
            // data table — the values the prologue read, after the params.
            for ((name, value) in sendNamelist) sendPayload[name] = value
            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(APayloadRidesOnItsOwnEventEvent.ExtN, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain3)
                }
                // W3C SCXML 3.8: Onentry block 4/4
                run {


            if (run send@{
            val sendData = "10"
            // W3C SCXML 6.2: send to this session's external queue
            send(APayloadRidesOnItsOwnEventEvent.ExtC, EventMetadata.sentBy(sessionId, sendId = "__send_3", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            raiseInternal(APayloadRidesOnItsOwnEventEvent.Plain4)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun onExit(state: APayloadRidesOnItsOwnEventState) {
        when (state) {
            is APayloadRidesOnItsOwnEventState.Done -> onExitOfDone()
            is APayloadRidesOnItsOwnEventState.S0 -> onExitOfS0()
        }
    }


    private fun onExitOfDone() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:82 :: done :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:37 :: s0 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_payload_rides_on_its_own_event.scxml:26 :: _machine
    override fun executeTransitionContent(source: APayloadRidesOnItsOwnEventState, transitionIndex: Int) {
        when (source) {
            is APayloadRidesOnItsOwnEventState.S0 -> transitionContentOfS0(transitionIndex)
            else -> {}
        }
    }


    // The content of `s0`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfS0(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfS0At0()
            1 -> transitionContentOfS0At1()
            2 -> transitionContentOfS0At2()
            3 -> transitionContentOfS0At3()
            4 -> transitionContentOfS0At4()
            5 -> transitionContentOfS0At5()
            else -> {}
        }
    }


    private fun transitionContentOfS0At0() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:61 :: s0 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("stolen", "stolen"), com.sce.runtime.ScriptSource.lua("_scxml_add(stolen, 1)", "stolen + 1"))) {
                return
            }
    }

    private fun transitionContentOfS0At1() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:64 :: s0 :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("plains", "plains"), com.sce.runtime.ScriptSource.lua("_scxml_add(plains, 1)", "plains + 1"))) {
                return
            }
    }

    private fun transitionContentOfS0At2() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:67 :: s0 :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))) {
                return
            }
    }

    private fun transitionContentOfS0At3() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:70 :: s0 :: _transition_3


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))) {
                return
            }
    }

    private fun transitionContentOfS0At4() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:73 :: s0 :: _transition_4


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))) {
                return
            }
    }

    private fun transitionContentOfS0At5() {
                // SCE-MAP: a_payload_rides_on_its_own_event.scxml:76 :: s0 :: _transition_5


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("got", "got"), com.sce.runtime.ScriptSource.lua("_scxml_add(got, 1)", "got + 1"))) {
                return
            }
    }
}
