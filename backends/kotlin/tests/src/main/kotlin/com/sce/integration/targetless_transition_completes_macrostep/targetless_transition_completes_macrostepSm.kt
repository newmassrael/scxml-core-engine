// SCE-GENERATED — DO NOT EDIT
// source-hash: 23347f5c092342ad5655a09f8c78eecec8de3c0705a0affd88f1ecbcd658f869

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/targetless_transition_completes_macrostep/targetless_transition_completes_macrostep.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine

package com.sce.integration.targetless_transition_completes_macrostep

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface TargetlessTransitionCompletesMacrostepState : State {
    data object Idle : TargetlessTransitionCompletesMacrostepState
    data object Recycled : TargetlessTransitionCompletesMacrostepState
    data object Settled : TargetlessTransitionCompletesMacrostepState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface TargetlessTransitionCompletesMacrostepEvent : Event {
    data object Arm : TargetlessTransitionCompletesMacrostepEvent
    sealed interface Error : TargetlessTransitionCompletesMacrostepEvent {
        data object Execution : Error
    }
    data object Ping : TargetlessTransitionCompletesMacrostepEvent
    data object Pong : TargetlessTransitionCompletesMacrostepEvent
    data object Quiet : TargetlessTransitionCompletesMacrostepEvent
    data object Recycle : TargetlessTransitionCompletesMacrostepEvent
}
// --- State Machine (W3C SCXML) ---

class TargetlessTransitionCompletesMacrostepStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<TargetlessTransitionCompletesMacrostepState, TargetlessTransitionCompletesMacrostepEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `quiet` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `quiet` was assigned a value of another type, or the engine refused.
     */
    fun quiet(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "quiet")

    /**
     * §scxml-5.3: what the `armed` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `armed` was assigned a value of another type, or the engine refused.
     */
    fun armed(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "armed")

    /**
     * §scxml-5.3: what the `chained` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `chained` was assigned a value of another type, or the engine refused.
     */
    fun chained(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "chained")

    /**
     * §scxml-5.3: what the `polished` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `polished` was assigned a value of another type, or the engine refused.
     */
    fun polished(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "polished")

    /**
     * §scxml-5.3: what the `answered` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `answered` was assigned a value of another type, or the engine refused.
     */
    fun answered(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "answered")

    /**
     * §scxml-5.3: what the `entries` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `entries` was assigned a value of another type, or the engine refused.
     */
    fun entries(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "entries")

    override val initialState: TargetlessTransitionCompletesMacrostepState = TargetlessTransitionCompletesMacrostepState.Idle

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
    override val documentInitialTargets: List<EntryTarget<TargetlessTransitionCompletesMacrostepState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<TargetlessTransitionCompletesMacrostepState, HistoryId>> =
            listOf(StateTarget(TargetlessTransitionCompletesMacrostepState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            listOf(StateTarget(TargetlessTransitionCompletesMacrostepState.Settled)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Idle,
            listOf(StateTarget(TargetlessTransitionCompletesMacrostepState.Recycled)),
            5,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: recycled's transition 0, as the microstep reads it.
        val transitionRecycledAt0 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Recycled,
            listOf(StateTarget(TargetlessTransitionCompletesMacrostepState.Recycled)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: settled's transition 0, as the microstep reads it.
        val transitionSettledAt0 = EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>(
            TargetlessTransitionCompletesMacrostepState.Settled,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): TargetlessTransitionCompletesMacrostepState? = when (stateId) {
        "idle" -> TargetlessTransitionCompletesMacrostepState.Idle
        "recycled" -> TargetlessTransitionCompletesMacrostepState.Recycled
        "settled" -> TargetlessTransitionCompletesMacrostepState.Settled
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: TargetlessTransitionCompletesMacrostepState): String = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> "idle"
        is TargetlessTransitionCompletesMacrostepState.Recycled -> "recycled"
        is TargetlessTransitionCompletesMacrostepState.Settled -> "settled"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: TargetlessTransitionCompletesMacrostepState): Int = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> 0
        is TargetlessTransitionCompletesMacrostepState.Recycled -> 2
        is TargetlessTransitionCompletesMacrostepState.Settled -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): TargetlessTransitionCompletesMacrostepEvent? = when (name) {
        "arm" -> TargetlessTransitionCompletesMacrostepEvent.Arm
        "error.execution" -> TargetlessTransitionCompletesMacrostepEvent.Error.Execution
        "ping" -> TargetlessTransitionCompletesMacrostepEvent.Ping
        "pong" -> TargetlessTransitionCompletesMacrostepEvent.Pong
        "quiet" -> TargetlessTransitionCompletesMacrostepEvent.Quiet
        "recycle" -> TargetlessTransitionCompletesMacrostepEvent.Recycle
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TargetlessTransitionCompletesMacrostepEvent): String? = when (event) {
        is TargetlessTransitionCompletesMacrostepEvent.Arm -> "arm"
        is TargetlessTransitionCompletesMacrostepEvent.Error.Execution -> "error.execution"
        is TargetlessTransitionCompletesMacrostepEvent.Ping -> "ping"
        is TargetlessTransitionCompletesMacrostepEvent.Pong -> "pong"
        is TargetlessTransitionCompletesMacrostepEvent.Quiet -> "quiet"
        is TargetlessTransitionCompletesMacrostepEvent.Recycle -> "recycle"
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
            "targetless_transition_completes_macrostep",
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
        // W3C SCXML 5.3: Initialize variable 'quiet' with expr
        try {
            val initResult_quiet = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "quiet", initResult_quiet)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='quiet'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'armed' with expr
        try {
            val initResult_armed = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "armed", initResult_armed)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='armed'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'chained' with expr
        try {
            val initResult_chained = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "chained", initResult_chained)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='chained'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'polished' with expr
        try {
            val initResult_polished = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "polished", initResult_polished)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='polished'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'answered' with expr
        try {
            val initResult_answered = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "answered", initResult_answered)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='answered'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'entries' with expr
        try {
            val initResult_entries = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "entries", initResult_entries)
        } catch (e: Exception) {
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<data id='entries'> expr failed to evaluate")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, reason)
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(TargetlessTransitionCompletesMacrostepEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: TargetlessTransitionCompletesMacrostepEvent) {
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
    override fun bindCurrentEvent(event: TargetlessTransitionCompletesMacrostepEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: TargetlessTransitionCompletesMacrostepState, event: TargetlessTransitionCompletesMacrostepEvent?): EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>? = when (state) {
        is TargetlessTransitionCompletesMacrostepState.Idle -> firstEnabledTransitionOfIdle(state, event)
        is TargetlessTransitionCompletesMacrostepState.Recycled -> firstEnabledTransitionOfRecycled(state, event)
        is TargetlessTransitionCompletesMacrostepState.Settled -> firstEnabledTransitionOfSettled(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: TargetlessTransitionCompletesMacrostepState, event: TargetlessTransitionCompletesMacrostepEvent?): EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(armed, 1)", "armed == 1")) -> TransitionsPart0.transitionIdleAt0
        event is TargetlessTransitionCompletesMacrostepEvent.Quiet -> TransitionsPart0.transitionIdleAt1
        event is TargetlessTransitionCompletesMacrostepEvent.Arm -> TransitionsPart0.transitionIdleAt2
        event is TargetlessTransitionCompletesMacrostepEvent.Ping -> TransitionsPart0.transitionIdleAt3
        event is TargetlessTransitionCompletesMacrostepEvent.Pong -> TransitionsPart0.transitionIdleAt4
        event is TargetlessTransitionCompletesMacrostepEvent.Recycle -> TransitionsPart0.transitionIdleAt5
        else -> null
    }


    // recycled's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRecycled(state: TargetlessTransitionCompletesMacrostepState, event: TargetlessTransitionCompletesMacrostepEvent?): EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(entries < 2)", "entries < 2")) -> TransitionsPart0.transitionRecycledAt0
        else -> null
    }


    // settled's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSettled(state: TargetlessTransitionCompletesMacrostepState, event: TargetlessTransitionCompletesMacrostepEvent?): EnabledTransition<TargetlessTransitionCompletesMacrostepState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(polished, 0)", "polished == 0")) -> TransitionsPart0.transitionSettledAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun onEntry(state: TargetlessTransitionCompletesMacrostepState, isDefaultEntry: Boolean) {
        when (state) {
            is TargetlessTransitionCompletesMacrostepState.Idle -> onEntryOfIdle()
            is TargetlessTransitionCompletesMacrostepState.Recycled -> onEntryOfRecycled()
            is TargetlessTransitionCompletesMacrostepState.Settled -> onEntryOfSettled()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:81 :: idle :: _state_body
    }

    private fun onEntryOfRecycled() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:147 :: recycled :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("entries", "entries"), com.sce.runtime.ScriptSource.lua("_scxml_add(entries, 1)", "entries + 1"))) {
                return@run
            }
                }
    }

    private fun onEntryOfSettled() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:125 :: settled :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun onExit(state: TargetlessTransitionCompletesMacrostepState) {
        when (state) {
            is TargetlessTransitionCompletesMacrostepState.Idle -> onExitOfIdle()
            is TargetlessTransitionCompletesMacrostepState.Recycled -> onExitOfRecycled()
            is TargetlessTransitionCompletesMacrostepState.Settled -> onExitOfSettled()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:81 :: idle :: _state_body
    }

    private fun onExitOfRecycled() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:147 :: recycled :: _state_body
    }

    private fun onExitOfSettled() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:125 :: settled :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: targetless_transition_completes_macrostep.scxml:51 :: _machine
    override fun executeTransitionContent(source: TargetlessTransitionCompletesMacrostepState, transitionIndex: Int) {
        when (source) {
            is TargetlessTransitionCompletesMacrostepState.Idle -> transitionContentOfIdle(transitionIndex)
            is TargetlessTransitionCompletesMacrostepState.Settled -> transitionContentOfSettled(transitionIndex)
            else -> {}
        }
    }


    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            3 -> transitionContentOfIdleAt3()
            4 -> transitionContentOfIdleAt4()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:86 :: idle :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("chained", "chained"), com.sce.runtime.ScriptSource.lua("_scxml_add(chained, 1)", "chained + 1"))) {
                return
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:93 :: idle :: _transition_1


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("quiet", "quiet"), com.sce.runtime.ScriptSource.lua("_scxml_add(quiet, 1)", "quiet + 1"))) {
                return
            }
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:100 :: idle :: _transition_2


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("armed", "armed"), com.sce.runtime.ScriptSource.lua("1", "1"))) {
                return
            }
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:107 :: idle :: _transition_3

            raiseInternal(TargetlessTransitionCompletesMacrostepEvent.Pong)
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:111 :: idle :: _transition_4


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("answered", "answered"), com.sce.runtime.ScriptSource.lua("_scxml_add(answered, 1)", "answered + 1"))) {
                return
            }
    }

    // The content of `settled`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSettled(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSettledAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSettledAt0() {
                // SCE-MAP: targetless_transition_completes_macrostep.scxml:126 :: settled :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("polished", "polished"), com.sce.runtime.ScriptSource.lua("_scxml_add(polished, 1)", "polished + 1"))) {
                return
            }
    }
}
