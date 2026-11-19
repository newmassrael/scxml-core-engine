// SCE-GENERATED — DO NOT EDIT
// source-hash: 428dfedc40362bcde893d2a13aaf88295d66fbb3c5dc619cc8fae908cc42df31
// source: a_child_takes_a_handed_value_in_place_of_its_data.scxml

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_child_takes_a_handed_value_in_place_of_its_data/a_child_takes_a_handed_value_in_place_of_its_data.scxml#by_namelist
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:184 :: _machine

package com.sce.integration.a_child_takes_a_handed_value_in_place_of_its_data

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState : State {
    data object Collect : AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState
    data object Reported : AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent : Event {
    sealed interface Error : AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent {
        data object Communication : Error
        data object Execution : Error
    }
    sealed interface Names : AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent {
        data object Report : Names
    }
    data object Report : AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent
}
// --- State Machine (W3C SCXML) ---

class AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `record` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `record` was assigned a value of another type, or the engine refused.
     */
    fun record(): String? =
        com.sce.runtime.DatamodelRead.readString(scriptEngine, scriptSessionId, "record")

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

    override val initialState: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState = AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState): Boolean = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>> =
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: collect's transition 0, as the microstep reads it.
        val transitionCollectAt0 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: collect's transition 1, as the microstep reads it.
        val transitionCollectAt1 = EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>(
            AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect,
            listOf(StateTarget(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState? = when (stateId) {
        "collect" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect
        "reported" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState): String = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> "collect"
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported -> "reported"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState): Int = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> 0
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent? = when (name) {
        "error.communication" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Communication
        "error.execution" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution
        "names.report" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Names.Report
        "report" -> AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Report
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent): String? = when (event) {
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Communication -> "error.communication"
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution -> "error.execution"
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Names.Report -> "names.report"
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Report -> "report"
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
            "a_child_takes_a_handed_value_in_place_of_its_data__sce_synth_invoke__by_namelist",
        )?.let { return it }
        return null
    }

    // W3C SCXML 5.3: initialize the datamodel in the session the start
    // established, before any state is entered: the document's <data>, its
    // states' early-bound <data>, then the top-level <script>s, the order of
    // interpret in Appendix D. §scxml-6.4: a value the invoke that started
    // this session handed it under the id of a top-level <data> is that
    // <data>'s initial value, declared in place of its own, so the scripts,
    // the <onentry>s and the conditions of the first macrostep read it; a
    // state's <data> never takes one, and a name no top-level <data> has is
    // not added (docs/adr/0098).
    override fun declareDatamodel() {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")

        declareVariablesPart0(engine, sid)

    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // §scxml-6.4: 'listed' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "listed", AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'listed' with expr
            try {
                engine.setVariable(sid, "listed", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("nil.invalidProperty", "undefined.invalidProperty")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='listed'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'record' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "record", AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'record' with expr
            try {
                engine.setVariable(sid, "record", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("\"own record\"", "'own record'")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='record'> expr failed to evaluate")
            }
        }
        // §scxml-6.4: 'errors' takes a value its invoke handed it, in place of its own
        if (!declareHandedValue(engine, sid, "errors", AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution)) {
            // W3C SCXML 5.3: Initialize variable 'errors' with expr
            try {
                engine.setVariable(sid, "errors", engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0")))
            } catch (e: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "<data id='errors'> expr failed to evaluate")
            }
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
    private fun evaluateGuardRaising(guardExpr: com.sce.runtime.ScriptSource, detail: String): Boolean? {
        assertScriptSession()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, detail)
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
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.EXPRESSION_FAILED, "an expression could not be serialised to JSON")
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
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.ASSIGNMENT_FAILED, "<assign> failed")
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
            raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.SCRIPT_FAILED, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent) {
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
    override fun bindCurrentEvent(event: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, event: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>? = when (state) {
        is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> firstEnabledTransitionOfCollect(state, event)
        else -> null
    }


    // collect's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfCollect(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, event: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent?): EnabledTransition<AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, HistoryId>? = when {
        event is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution -> TransitionsPart0.transitionCollectAt0
        event is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Report -> TransitionsPart0.transitionCollectAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:184 :: _machine
    override fun onEntry(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, isDefaultEntry: Boolean) {
        when (state) {
            is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> onEntryOfCollect()
            is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported -> onEntryOfReported()
        }
    }


    private fun onEntryOfCollect() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:191 :: collect :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Report)
                }
    }

    private fun onEntryOfReported() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:207 :: reported :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:184 :: _machine
    override fun onExit(state: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState) {
        when (state) {
            is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> onExitOfCollect()
            is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Reported -> onExitOfReported()
        }
    }


    private fun onExitOfCollect() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:191 :: collect :: _state_body
    }

    private fun onExitOfReported() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:207 :: reported :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:184 :: _machine
    override fun executeTransitionContent(source: AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState, transitionIndex: Int) {
        when (source) {
            is AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistState.Collect -> transitionContentOfCollect(transitionIndex)
            else -> {}
        }
    }


    // The content of `collect`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfCollect(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfCollectAt0()
            1 -> transitionContentOfCollectAt1()
            else -> {}
        }
    }


    private fun transitionContentOfCollectAt0() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:195 :: collect :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("errors", "errors"), com.sce.runtime.ScriptSource.lua("_scxml_add(errors, 1)", "errors + 1"))) {
                return
            }
    }

    private fun transitionContentOfCollectAt1() {
                // SCE-MAP: a_child_takes_a_handed_value_in_place_of_its_data.scxml:198 :: collect :: _transition_1


            if (run send@{
            var paramFailed = false
            assertScriptSession()
            val payloadEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val payloadSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "listed", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("listed", "listed")))
            } catch (_: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<send> <param name='listed'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "stray", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("_typeof(stray)", "typeof stray")))
            } catch (_: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<send> <param name='stray'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "record", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("(function() if (((((_typeof(record) == \"object\") and (record ~= nil)) and (record.name == \"whole\")) and (record.inner.depth == 2)) and (#record.list == 3)) then return \"whole\" end return \"broken\" end)()", "typeof record === 'object' && record !== null && record.name === 'whole' && record.inner.depth === 2 && record.list.length === 3 ? 'whole' : 'broken'")))
            } catch (_: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<send> <param name='record'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "errors", payloadEngine.evaluateExpr(payloadSid, com.sce.runtime.ScriptSource.lua("errors", "errors")))
            } catch (_: Exception) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Execution, ErrorReason.LOCATION_INVALID, "<send> <param name='errors'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(AChildTakesAHandedValueInPlaceOfItsDataSceSynthInvokeByNamelistEvent.Error.Communication, ErrorReason.TARGET_UNREACHABLE, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("names.report", sendData)
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
