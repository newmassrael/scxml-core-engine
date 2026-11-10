// SCE-GENERATED — DO NOT EDIT
// source-hash: c97edcb094613d8138825758fc943d853d23ad4854f2fa7dcf6ff6f58539b674

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/empty_finalize_updates_the_location/empty_finalize_updates_the_location.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine

package com.sce.integration.empty_finalize_updates_the_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EmptyFinalizeUpdatesTheLocationState : State {
    data object AbsentPhase : EmptyFinalizeUpdatesTheLocationState
    data object EmptyPhase : EmptyFinalizeUpdatesTheLocationState
    data object FailAbsentChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailEmptyChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailNotUpdated : EmptyFinalizeUpdatesTheLocationState
    data object FailUnmatchedChildSilent : EmptyFinalizeUpdatesTheLocationState
    data object FailUnmatchedNameWrote : EmptyFinalizeUpdatesTheLocationState
    data object FailUpdatedWithoutFinalize : EmptyFinalizeUpdatesTheLocationState
    data object Pass : EmptyFinalizeUpdatesTheLocationState
    data object UnmatchedPhase : EmptyFinalizeUpdatesTheLocationState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EmptyFinalizeUpdatesTheLocationEvent : Event {
    sealed interface Done : EmptyFinalizeUpdatesTheLocationEvent {
        data object Invoke : Done
    }
    sealed interface Error : EmptyFinalizeUpdatesTheLocationEvent {
        data object Execution : Error
    }
    data object FromAbsentChild : EmptyFinalizeUpdatesTheLocationEvent
    data object FromEmptyChild : EmptyFinalizeUpdatesTheLocationEvent
    data object FromUnmatchedChild : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutAbsent : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutEmpty : EmptyFinalizeUpdatesTheLocationEvent
    data object TimeoutUnmatched : EmptyFinalizeUpdatesTheLocationEvent
}
// --- State Machine (W3C SCXML) ---

class EmptyFinalizeUpdatesTheLocationStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EmptyFinalizeUpdatesTheLocationState, EmptyFinalizeUpdatesTheLocationEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `tally` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `tally` was assigned a value of another type, or the engine refused.
     */
    fun tally(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "tally")

    /**
     * §scxml-5.3: what the `guard` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `guard` was assigned a value of another type, or the engine refused.
     */
    fun guard(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "guard")

    /**
     * §scxml-5.3: what the `keeper` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `keeper` was assigned a value of another type, or the engine refused.
     */
    fun keeper(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "keeper")

    override val initialState: EmptyFinalizeUpdatesTheLocationState = EmptyFinalizeUpdatesTheLocationState.EmptyPhase

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: EmptyFinalizeUpdatesTheLocationState): Boolean = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> true
        is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> true
        is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> true
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> true
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> true
        is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> true
        is EmptyFinalizeUpdatesTheLocationState.Pass -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EmptyFinalizeUpdatesTheLocationState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<EmptyFinalizeUpdatesTheLocationState, HistoryId>> =
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.EmptyPhase))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: absentPhase's transition 0, as the microstep reads it.
        val transitionAbsentPhaseAt0 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.AbsentPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: absentPhase's transition 1, as the microstep reads it.
        val transitionAbsentPhaseAt1 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.AbsentPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: absentPhase's transition 2, as the microstep reads it.
        val transitionAbsentPhaseAt2 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.AbsentPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: emptyPhase's transition 0, as the microstep reads it.
        val transitionEmptyPhaseAt0 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.EmptyPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.AbsentPhase)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: emptyPhase's transition 1, as the microstep reads it.
        val transitionEmptyPhaseAt1 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.EmptyPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailNotUpdated)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: emptyPhase's transition 2, as the microstep reads it.
        val transitionEmptyPhaseAt2 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.EmptyPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: unmatchedPhase's transition 0, as the microstep reads it.
        val transitionUnmatchedPhaseAt0 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: unmatchedPhase's transition 1, as the microstep reads it.
        val transitionUnmatchedPhaseAt1 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: unmatchedPhase's transition 2, as the microstep reads it.
        val transitionUnmatchedPhaseAt2 = EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>(
            EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase,
            listOf(StateTarget(EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EmptyFinalizeUpdatesTheLocationState? = when (stateId) {
        "absentPhase" -> EmptyFinalizeUpdatesTheLocationState.AbsentPhase
        "emptyPhase" -> EmptyFinalizeUpdatesTheLocationState.EmptyPhase
        "failAbsentChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent
        "failEmptyChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent
        "failNotUpdated" -> EmptyFinalizeUpdatesTheLocationState.FailNotUpdated
        "failUnmatchedChildSilent" -> EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent
        "failUnmatchedNameWrote" -> EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote
        "failUpdatedWithoutFinalize" -> EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize
        "pass" -> EmptyFinalizeUpdatesTheLocationState.Pass
        "unmatchedPhase" -> EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EmptyFinalizeUpdatesTheLocationState): String = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> "absentPhase"
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> "emptyPhase"
        is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> "failAbsentChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> "failEmptyChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> "failNotUpdated"
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> "failUnmatchedChildSilent"
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> "failUnmatchedNameWrote"
        is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> "failUpdatedWithoutFinalize"
        is EmptyFinalizeUpdatesTheLocationState.Pass -> "pass"
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> "unmatchedPhase"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: EmptyFinalizeUpdatesTheLocationState): Int = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> 1
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> 0
        is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> 8
        is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> 7
        is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> 4
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> 9
        is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> 6
        is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> 5
        is EmptyFinalizeUpdatesTheLocationState.Pass -> 3
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EmptyFinalizeUpdatesTheLocationEvent? = when (name) {
        "done.invoke" -> EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke
        "error.execution" -> EmptyFinalizeUpdatesTheLocationEvent.Error.Execution
        "fromAbsentChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild
        "fromEmptyChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild
        "fromUnmatchedChild" -> EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild
        "timeoutAbsent" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent
        "timeoutEmpty" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty
        "timeoutUnmatched" -> EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EmptyFinalizeUpdatesTheLocationEvent): String? = when (event) {
        is EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke -> "done.invoke"
        is EmptyFinalizeUpdatesTheLocationEvent.Error.Execution -> "error.execution"
        is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild -> "fromAbsentChild"
        is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild -> "fromEmptyChild"
        is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild -> "fromUnmatchedChild"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent -> "timeoutAbsent"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty -> "timeoutEmpty"
        is EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched -> "timeoutUnmatched"
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
            "empty_finalize_updates_the_location",
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
        // W3C SCXML 5.3: Initialize variable 'tally' with expr
        try {
            val initResult_tally = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "tally", initResult_tally)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='tally'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'guard' with expr
        try {
            val initResult_guard = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "guard", initResult_guard)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='guard'> expr failed to evaluate")
        }
        // W3C SCXML 5.3: Initialize variable 'keeper' with expr
        try {
            val initResult_keeper = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("3", "3"))
            engine.setVariable(sid, "keeper", initResult_keeper)
        } catch (e: Exception) {
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<data id='keeper'> expr failed to evaluate")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, reason)
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EmptyFinalizeUpdatesTheLocationEvent) {
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
    override fun bindCurrentEvent(event: EmptyFinalizeUpdatesTheLocationEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: EmptyFinalizeUpdatesTheLocationState, event: EmptyFinalizeUpdatesTheLocationEvent?): EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>? = when (state) {
        is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> firstEnabledTransitionOfAbsentPhase(state, event)
        is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> firstEnabledTransitionOfEmptyPhase(state, event)
        is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> firstEnabledTransitionOfUnmatchedPhase(state, event)
        else -> null
    }


    // absentPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAbsentPhase(state: EmptyFinalizeUpdatesTheLocationState, event: EmptyFinalizeUpdatesTheLocationEvent?): EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>? = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(guard ~= 1)", "guard !== 1")) -> TransitionsPart0.transitionAbsentPhaseAt0
        event is EmptyFinalizeUpdatesTheLocationEvent.FromAbsentChild -> TransitionsPart0.transitionAbsentPhaseAt1
        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent -> TransitionsPart0.transitionAbsentPhaseAt2
        else -> null
    }


    // emptyPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfEmptyPhase(state: EmptyFinalizeUpdatesTheLocationState, event: EmptyFinalizeUpdatesTheLocationEvent?): EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>? = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(tally == 7)", "tally === 7")) -> TransitionsPart0.transitionEmptyPhaseAt0
        event is EmptyFinalizeUpdatesTheLocationEvent.FromEmptyChild -> TransitionsPart0.transitionEmptyPhaseAt1
        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty -> TransitionsPart0.transitionEmptyPhaseAt2
        else -> null
    }


    // unmatchedPhase's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfUnmatchedPhase(state: EmptyFinalizeUpdatesTheLocationState, event: EmptyFinalizeUpdatesTheLocationEvent?): EnabledTransition<EmptyFinalizeUpdatesTheLocationState, HistoryId>? = when {
        event is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(keeper ~= 3)", "keeper !== 3")) -> TransitionsPart0.transitionUnmatchedPhaseAt0
        event is EmptyFinalizeUpdatesTheLocationEvent.FromUnmatchedChild -> TransitionsPart0.transitionUnmatchedPhaseAt1
        event is EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched -> TransitionsPart0.transitionUnmatchedPhaseAt2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun onEntry(state: EmptyFinalizeUpdatesTheLocationState, isDefaultEntry: Boolean) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> onEntryOfAbsentPhase()
            is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> onEntryOfEmptyPhase()
            is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> onEntryOfFailAbsentChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> onEntryOfFailEmptyChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> onEntryOfFailNotUpdated()
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> onEntryOfFailUnmatchedChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> onEntryOfFailUnmatchedNameWrote()
            is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> onEntryOfFailUpdatedWithoutFinalize()
            is EmptyFinalizeUpdatesTheLocationState.Pass -> onEntryOfPass()
            is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> onEntryOfUnmatchedPhase()
        }
    }


    private fun onEntryOfAbsentPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:105 :: absentPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_1", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutAbsent, EventMetadata.sentBy(sessionId, sendId = "__send_1", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inv_absent", EmptyFinalizeUpdatesTheLocationState.AbsentPhase)
    }

    private fun onEntryOfEmptyPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:71 :: emptyPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutEmpty, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inv_empty", EmptyFinalizeUpdatesTheLocationState.EmptyPhase)
    }

    private fun onEntryOfFailAbsentChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:183 :: failAbsentChildSilent :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailEmptyChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:182 :: failEmptyChildSilent :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNotUpdated() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:179 :: failNotUpdated :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailUnmatchedChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:184 :: failUnmatchedChildSilent :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailUnmatchedNameWrote() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:181 :: failUnmatchedNameWrote :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailUpdatedWithoutFinalize() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:180 :: failUpdatedWithoutFinalize :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:178 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfUnmatchedPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:144 :: unmatchedPhase :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_2", 3000L, EmptyFinalizeUpdatesTheLocationEvent.TimeoutUnmatched, EventMetadata.sentBy(sessionId, sendId = "__send_2", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inv_unmatched", EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    //
    // Each invoke's start is a method of its own (`startInvokeOf<State>At<n>`,
    // `n` its place among its state's invokes), and this dispatches to it with
    // one call an arm, for the reason `onEntry` does.
    private fun deferStaticInvoke(invokeId: String, state: EmptyFinalizeUpdatesTheLocationState) {
        when (invokeId) {
            "inv_absent" -> startInvokeOfAbsentPhaseAt0(state)
            "inv_empty" -> startInvokeOfEmptyPhaseAt0(state)
            "inv_unmatched" -> startInvokeOfUnmatchedPhaseAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfAbsentPhaseAt0(state: EmptyFinalizeUpdatesTheLocationState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "absentPhase.${nextInvokeStart("inv_absent")}.inv_absent"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "guard"))) {
                            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["guard"] = engineInv.getVariable(sidInv, "guard")

                    val childSM = adoptChild(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvAbsentStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inv_absent", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "", generatedInvokeId, "", "empty_finalize_updates_the_location__sce_synth_invoke__inv_absent", "inv_absent")
                }
            }
    }

    private fun startInvokeOfEmptyPhaseAt0(state: EmptyFinalizeUpdatesTheLocationState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "emptyPhase.${nextInvokeStart("inv_empty")}.inv_empty"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "tally"))) {
                            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["tally"] = engineInv.getVariable(sidInv, "tally")

                    val childSM = adoptChild(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvEmptyStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inv_empty", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "if (_event.data && _event.data.tally !== undefined) { tally = _event.data.tally; }", generatedInvokeId, "", "empty_finalize_updates_the_location__sce_synth_invoke__inv_empty", "inv_empty")
                }
            }
    }

    private fun startInvokeOfUnmatchedPhaseAt0(state: EmptyFinalizeUpdatesTheLocationState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "unmatchedPhase.${nextInvokeStart("inv_unmatched")}.inv_unmatched"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        assertScriptSession()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "keeper"))) {
                            raisePlatformError(EmptyFinalizeUpdatesTheLocationEvent.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["keeper"] = engineInv.getVariable(sidInv, "keeper")

                    val childSM = adoptChild(EmptyFinalizeUpdatesTheLocationSceSynthInvokeInvUnmatchedStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inv_unmatched", childSM, false, EmptyFinalizeUpdatesTheLocationEvent.Done.Invoke, "if (_event.data && _event.data.keeper !== undefined) { keeper = _event.data.keeper; }", generatedInvokeId, "", "empty_finalize_updates_the_location__sce_synth_invoke__inv_unmatched", "inv_unmatched")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun onExit(state: EmptyFinalizeUpdatesTheLocationState) {
        when (state) {
            is EmptyFinalizeUpdatesTheLocationState.AbsentPhase -> onExitOfAbsentPhase()
            is EmptyFinalizeUpdatesTheLocationState.EmptyPhase -> onExitOfEmptyPhase()
            is EmptyFinalizeUpdatesTheLocationState.FailAbsentChildSilent -> onExitOfFailAbsentChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailEmptyChildSilent -> onExitOfFailEmptyChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailNotUpdated -> onExitOfFailNotUpdated()
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedChildSilent -> onExitOfFailUnmatchedChildSilent()
            is EmptyFinalizeUpdatesTheLocationState.FailUnmatchedNameWrote -> onExitOfFailUnmatchedNameWrote()
            is EmptyFinalizeUpdatesTheLocationState.FailUpdatedWithoutFinalize -> onExitOfFailUpdatedWithoutFinalize()
            is EmptyFinalizeUpdatesTheLocationState.Pass -> onExitOfPass()
            is EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase -> onExitOfUnmatchedPhase()
        }
    }


    private fun onExitOfAbsentPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:105 :: absentPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(EmptyFinalizeUpdatesTheLocationState.AbsentPhase)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_absent")
    }

    private fun onExitOfEmptyPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:71 :: emptyPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(EmptyFinalizeUpdatesTheLocationState.EmptyPhase)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_empty")
    }

    private fun onExitOfFailAbsentChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:183 :: failAbsentChildSilent :: _state_body
    }

    private fun onExitOfFailEmptyChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:182 :: failEmptyChildSilent :: _state_body
    }

    private fun onExitOfFailNotUpdated() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:179 :: failNotUpdated :: _state_body
    }

    private fun onExitOfFailUnmatchedChildSilent() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:184 :: failUnmatchedChildSilent :: _state_body
    }

    private fun onExitOfFailUnmatchedNameWrote() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:181 :: failUnmatchedNameWrote :: _state_body
    }

    private fun onExitOfFailUpdatedWithoutFinalize() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:180 :: failUpdatedWithoutFinalize :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:178 :: pass :: _state_body
    }

    private fun onExitOfUnmatchedPhase() {
                // SCE-MAP: empty_finalize_updates_the_location.scxml:144 :: unmatchedPhase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(EmptyFinalizeUpdatesTheLocationState.UnmatchedPhase)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_unmatched")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: empty_finalize_updates_the_location.scxml:52 :: _machine
    override fun executeTransitionContent(source: EmptyFinalizeUpdatesTheLocationState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
