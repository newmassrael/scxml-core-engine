// SCE-GENERATED — DO NOT EDIT
// source-hash: c528e32056e8246a79a9ab2618a850432ee939729f1f4a5827caa0163fd4cba3

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/an_assignment_keeps_the_document_it_is_given/an_assignment_keeps_the_document_it_is_given.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:32 :: _machine

package com.sce.integration.an_assignment_keeps_the_document_it_is_given

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AnAssignmentKeepsTheDocumentItIsGivenState : State {
    data object Kept : AnAssignmentKeepsTheDocumentItIsGivenState
    data object MemberLost : AnAssignmentKeepsTheDocumentItIsGivenState
    data object NameLost : AnAssignmentKeepsTheDocumentItIsGivenState
    data object ReadingMember : AnAssignmentKeepsTheDocumentItIsGivenState
    data object ReadingName : AnAssignmentKeepsTheDocumentItIsGivenState
    data object Sending : AnAssignmentKeepsTheDocumentItIsGivenState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AnAssignmentKeepsTheDocumentItIsGivenEvent : Event {
    data object Doc : AnAssignmentKeepsTheDocumentItIsGivenEvent
    sealed interface Error : AnAssignmentKeepsTheDocumentItIsGivenEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class AnAssignmentKeepsTheDocumentItIsGivenStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<AnAssignmentKeepsTheDocumentItIsGivenState, AnAssignmentKeepsTheDocumentItIsGivenEvent>(scriptEngine) {

    override val initialState: AnAssignmentKeepsTheDocumentItIsGivenState = AnAssignmentKeepsTheDocumentItIsGivenState.Sending

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
    override fun isFinalState(state: AnAssignmentKeepsTheDocumentItIsGivenState): Boolean = when (state) {
        is AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> true
        is AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost -> true
        is AnAssignmentKeepsTheDocumentItIsGivenState.NameLost -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>> =
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.Sending))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: readingMember's transition 0, as the microstep reads it.
        val transitionReadingMemberAt0 = EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>(
            AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember,
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.Kept)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: readingMember's transition 1, as the microstep reads it.
        val transitionReadingMemberAt1 = EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>(
            AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember,
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: readingName's transition 0, as the microstep reads it.
        val transitionReadingNameAt0 = EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>(
            AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName,
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: readingName's transition 1, as the microstep reads it.
        val transitionReadingNameAt1 = EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>(
            AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName,
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.NameLost)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: sending's transition 0, as the microstep reads it.
        val transitionSendingAt0 = EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>(
            AnAssignmentKeepsTheDocumentItIsGivenState.Sending,
            listOf(StateTarget(AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AnAssignmentKeepsTheDocumentItIsGivenState? = when (stateId) {
        "kept" -> AnAssignmentKeepsTheDocumentItIsGivenState.Kept
        "memberLost" -> AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost
        "nameLost" -> AnAssignmentKeepsTheDocumentItIsGivenState.NameLost
        "readingMember" -> AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember
        "readingName" -> AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName
        "sending" -> AnAssignmentKeepsTheDocumentItIsGivenState.Sending
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AnAssignmentKeepsTheDocumentItIsGivenState): String = when (state) {
        is AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> "kept"
        is AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost -> "memberLost"
        is AnAssignmentKeepsTheDocumentItIsGivenState.NameLost -> "nameLost"
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember -> "readingMember"
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName -> "readingName"
        is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> "sending"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AnAssignmentKeepsTheDocumentItIsGivenState): Int = when (state) {
        is AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> 3
        is AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost -> 5
        is AnAssignmentKeepsTheDocumentItIsGivenState.NameLost -> 4
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember -> 2
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName -> 1
        is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AnAssignmentKeepsTheDocumentItIsGivenEvent? = when (name) {
        "doc" -> AnAssignmentKeepsTheDocumentItIsGivenEvent.Doc
        "error.execution" -> AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AnAssignmentKeepsTheDocumentItIsGivenEvent): String? = when (event) {
        is AnAssignmentKeepsTheDocumentItIsGivenEvent.Doc -> "doc"
        is AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution -> "error.execution"
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
            "an_assignment_keeps_the_document_it_is_given",
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
        // W3C SCXML 5.2: Runtime variable 'v' (late binding, undefined)
        try {
            engine.evaluateExpr(sid, "undefined")
            engine.setVariable(sid, "v", null)
        } catch (_: Exception) {}
        // W3C SCXML 5.3: Initialize variable 'holder' with expr
        try {
            val initResult_holder = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("{}", "({})"))
            engine.setVariable(sid, "holder", initResult_holder)
        } catch (e: Exception) {
            raisePlatformError(AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution, "<data id='holder'> expr failed to evaluate")
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
            raisePlatformError(AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution, reason)
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
            raisePlatformError(AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(AnAssignmentKeepsTheDocumentItIsGivenEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: AnAssignmentKeepsTheDocumentItIsGivenEvent) {
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
    override fun bindCurrentEvent(event: AnAssignmentKeepsTheDocumentItIsGivenEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: AnAssignmentKeepsTheDocumentItIsGivenState, event: AnAssignmentKeepsTheDocumentItIsGivenEvent?): EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>? = when (state) {
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember -> firstEnabledTransitionOfReadingMember(state, event)
        is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName -> firstEnabledTransitionOfReadingName(state, event)
        is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> firstEnabledTransitionOfSending(state, event)
        else -> null
    }


    // readingMember's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfReadingMember(state: AnAssignmentKeepsTheDocumentItIsGivenState, event: AnAssignmentKeepsTheDocumentItIsGivenEvent?): EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(holder.doc) and (holder.doc:getElementsByTagName(\"book\")[1]:getAttribute(\"title\") == \"x\"))", "holder.doc && holder.doc.getElementsByTagName('book')[0].getAttribute('title') === 'x'")) -> TransitionsPart0.transitionReadingMemberAt0
        event == null -> TransitionsPart0.transitionReadingMemberAt1
        else -> null
    }


    // readingName's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfReadingName(state: AnAssignmentKeepsTheDocumentItIsGivenState, event: AnAssignmentKeepsTheDocumentItIsGivenEvent?): EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(v) and (#v:getElementsByTagName(\"book\") == 1))", "v && v.getElementsByTagName('book').length === 1")) -> TransitionsPart0.transitionReadingNameAt0
        event == null -> TransitionsPart0.transitionReadingNameAt1
        else -> null
    }


    // sending's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSending(state: AnAssignmentKeepsTheDocumentItIsGivenState, event: AnAssignmentKeepsTheDocumentItIsGivenEvent?): EnabledTransition<AnAssignmentKeepsTheDocumentItIsGivenState, HistoryId>? = when {
        event is AnAssignmentKeepsTheDocumentItIsGivenEvent.Doc -> TransitionsPart0.transitionSendingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:32 :: _machine
    override fun onEntry(state: AnAssignmentKeepsTheDocumentItIsGivenState, isDefaultEntry: Boolean) {
        when (state) {
            is AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> onEntryOfKept()
            is AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost -> onEntryOfMemberLost()
            is AnAssignmentKeepsTheDocumentItIsGivenState.NameLost -> onEntryOfNameLost()
            is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember -> onEntryOfReadingMember()
            is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName -> onEntryOfReadingName()
            is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> onEntryOfSending()
        }
    }


    private fun onEntryOfKept() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:64 :: kept :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfMemberLost() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:66 :: memberLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfNameLost() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:65 :: nameLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfReadingMember() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:58 :: readingMember :: _state_body
    }

    private fun onEntryOfReadingName() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:53 :: readingName :: _state_body
    }

    private fun onEntryOfSending() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:41 :: sending :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = "<books xmlns=\"\"><book title=\"x\"/></books>"
            // W3C SCXML 6.2: send to this session's external queue
            send(AnAssignmentKeepsTheDocumentItIsGivenEvent.Doc, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:32 :: _machine
    override fun onExit(state: AnAssignmentKeepsTheDocumentItIsGivenState) {
        when (state) {
            is AnAssignmentKeepsTheDocumentItIsGivenState.Kept -> onExitOfKept()
            is AnAssignmentKeepsTheDocumentItIsGivenState.MemberLost -> onExitOfMemberLost()
            is AnAssignmentKeepsTheDocumentItIsGivenState.NameLost -> onExitOfNameLost()
            is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingMember -> onExitOfReadingMember()
            is AnAssignmentKeepsTheDocumentItIsGivenState.ReadingName -> onExitOfReadingName()
            is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> onExitOfSending()
        }
    }


    private fun onExitOfKept() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:64 :: kept :: _state_body
    }

    private fun onExitOfMemberLost() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:66 :: memberLost :: _state_body
    }

    private fun onExitOfNameLost() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:65 :: nameLost :: _state_body
    }

    private fun onExitOfReadingMember() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:58 :: readingMember :: _state_body
    }

    private fun onExitOfReadingName() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:53 :: readingName :: _state_body
    }

    private fun onExitOfSending() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:41 :: sending :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:32 :: _machine
    override fun executeTransitionContent(source: AnAssignmentKeepsTheDocumentItIsGivenState, transitionIndex: Int) {
        when (source) {
            is AnAssignmentKeepsTheDocumentItIsGivenState.Sending -> transitionContentOfSending(transitionIndex)
            else -> {}
        }
    }


    // The content of `sending`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfSending(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfSendingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfSendingAt0() {
                // SCE-MAP: an_assignment_keeps_the_document_it_is_given.scxml:47 :: sending :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("v", "v"), com.sce.runtime.ScriptSource.lua("_event.data", "_event.data"))) {
                return
            }


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("holder.doc", "holder.doc"), com.sce.runtime.ScriptSource.lua("_event.data", "_event.data"))) {
                return
            }
    }
}
