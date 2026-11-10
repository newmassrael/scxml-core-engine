// SCE-GENERATED — DO NOT EDIT
// source-hash: c56e8b2e82b26aafed117bfaa06905c41b2c8e5d207725d3f84b7293eb1eb4ee

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/event_origin_is_a_location/event_origin_is_a_location.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine

package com.sce.integration.event_origin_is_a_location

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface EventOriginIsALocationState : State {
    data object AwaitReply : EventOriginIsALocationState
    data object Fail : EventOriginIsALocationState
    data object Pass : EventOriginIsALocationState
    data object Phase : EventOriginIsALocationState
    data object Waiting : EventOriginIsALocationState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface EventOriginIsALocationEvent : Event {
    sealed interface Done : EventOriginIsALocationEvent {
        data object Invoke : Done
    }
    sealed interface Error : EventOriginIsALocationEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object FromChild : EventOriginIsALocationEvent
    data object Reply : EventOriginIsALocationEvent
    data object ReplyArrived : EventOriginIsALocationEvent
}
// --- State Machine (W3C SCXML) ---

class EventOriginIsALocationStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<EventOriginIsALocationState, EventOriginIsALocationEvent>(scriptEngine) {

    override val initialState: EventOriginIsALocationState = EventOriginIsALocationState.Waiting

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: EventOriginIsALocationState): EventOriginIsALocationState? = when (state) {
        is EventOriginIsALocationState.AwaitReply -> EventOriginIsALocationState.Phase
        is EventOriginIsALocationState.Waiting -> EventOriginIsALocationState.Phase
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: EventOriginIsALocationState): Boolean = when (state) {
        is EventOriginIsALocationState.Phase -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: EventOriginIsALocationState): Boolean = when (state) {
        is EventOriginIsALocationState.Fail -> true
        is EventOriginIsALocationState.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: EventOriginIsALocationState): List<EventOriginIsALocationState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: EventOriginIsALocationState): List<EntryTarget<EventOriginIsALocationState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<EventOriginIsALocationState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<EventOriginIsALocationState, List<EventOriginIsALocationState>> = LinkedHashMap<EventOriginIsALocationState, List<EventOriginIsALocationState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<EventOriginIsALocationState, List<EventOriginIsALocationState>>) {
            table[EventOriginIsALocationState.Phase] = listOf(EventOriginIsALocationState.Waiting, EventOriginIsALocationState.AwaitReply)
        }


        val initialTargets: Map<EventOriginIsALocationState, List<EntryTarget<EventOriginIsALocationState, HistoryId>>> = LinkedHashMap<EventOriginIsALocationState, List<EntryTarget<EventOriginIsALocationState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<EventOriginIsALocationState, List<EntryTarget<EventOriginIsALocationState, HistoryId>>>) {
            table[EventOriginIsALocationState.Phase] = listOf(StateTarget(EventOriginIsALocationState.Waiting))
        }


        val documentInitialTargetList: List<EntryTarget<EventOriginIsALocationState, HistoryId>> =
            listOf(StateTarget(EventOriginIsALocationState.Phase))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: await_reply's transition 0, as the microstep reads it.
        val transitionAwaitReplyAt0 = EnabledTransition<EventOriginIsALocationState, HistoryId>(
            EventOriginIsALocationState.AwaitReply,
            listOf(StateTarget(EventOriginIsALocationState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<EventOriginIsALocationState, HistoryId>(
            EventOriginIsALocationState.Waiting,
            listOf(StateTarget(EventOriginIsALocationState.AwaitReply)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<EventOriginIsALocationState, HistoryId>(
            EventOriginIsALocationState.Waiting,
            listOf(StateTarget(EventOriginIsALocationState.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): EventOriginIsALocationState? = when (stateId) {
        "await_reply" -> EventOriginIsALocationState.AwaitReply
        "fail" -> EventOriginIsALocationState.Fail
        "pass" -> EventOriginIsALocationState.Pass
        "phase" -> EventOriginIsALocationState.Phase
        "waiting" -> EventOriginIsALocationState.Waiting
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: EventOriginIsALocationState): String = when (state) {
        is EventOriginIsALocationState.AwaitReply -> "await_reply"
        is EventOriginIsALocationState.Fail -> "fail"
        is EventOriginIsALocationState.Pass -> "pass"
        is EventOriginIsALocationState.Phase -> "phase"
        is EventOriginIsALocationState.Waiting -> "waiting"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: EventOriginIsALocationState): Int = when (state) {
        is EventOriginIsALocationState.AwaitReply -> 2
        is EventOriginIsALocationState.Fail -> 4
        is EventOriginIsALocationState.Pass -> 3
        is EventOriginIsALocationState.Phase -> 0
        is EventOriginIsALocationState.Waiting -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): EventOriginIsALocationEvent? = when (name) {
        "done.invoke" -> EventOriginIsALocationEvent.Done.Invoke
        "error.communication" -> EventOriginIsALocationEvent.Error.Communication
        "error.execution" -> EventOriginIsALocationEvent.Error.Execution
        "fromChild" -> EventOriginIsALocationEvent.FromChild
        "reply" -> EventOriginIsALocationEvent.Reply
        "replyArrived" -> EventOriginIsALocationEvent.ReplyArrived
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: EventOriginIsALocationEvent): String? = when (event) {
        is EventOriginIsALocationEvent.Done.Invoke -> "done.invoke"
        is EventOriginIsALocationEvent.Error.Communication -> "error.communication"
        is EventOriginIsALocationEvent.Error.Execution -> "error.execution"
        is EventOriginIsALocationEvent.FromChild -> "fromChild"
        is EventOriginIsALocationEvent.Reply -> "reply"
        is EventOriginIsALocationEvent.ReplyArrived -> "replyArrived"
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
            "event_origin_is_a_location",
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, reason)
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: EventOriginIsALocationEvent) {
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
    override fun bindCurrentEvent(event: EventOriginIsALocationEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: EventOriginIsALocationState, event: EventOriginIsALocationEvent?): EnabledTransition<EventOriginIsALocationState, HistoryId>? = when (state) {
        is EventOriginIsALocationState.AwaitReply -> firstEnabledTransitionOfAwaitReply(state, event)
        is EventOriginIsALocationState.Waiting -> firstEnabledTransitionOfWaiting(state, event)
        else -> null
    }


    // await_reply's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitReply(state: EventOriginIsALocationState, event: EventOriginIsALocationEvent?): EnabledTransition<EventOriginIsALocationState, HistoryId>? = when {
        event is EventOriginIsALocationEvent.ReplyArrived -> TransitionsPart0.transitionAwaitReplyAt0
        else -> null
    }


    // waiting's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWaiting(state: EventOriginIsALocationState, event: EventOriginIsALocationEvent?): EnabledTransition<EventOriginIsALocationState, HistoryId>? = when {
        event is EventOriginIsALocationEvent.FromChild && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(_event.origin, _event.data.myLocation)", "_event.origin == _event.data.myLocation")) -> TransitionsPart0.transitionWaitingAt0
        event is EventOriginIsALocationEvent.FromChild -> TransitionsPart0.transitionWaitingAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun onEntry(state: EventOriginIsALocationState, isDefaultEntry: Boolean) {
        when (state) {
            is EventOriginIsALocationState.AwaitReply -> onEntryOfAwaitReply()
            is EventOriginIsALocationState.Fail -> onEntryOfFail()
            is EventOriginIsALocationState.Pass -> onEntryOfPass()
            is EventOriginIsALocationState.Phase -> onEntryOfPhase()
            is EventOriginIsALocationState.Waiting -> onEntryOfWaiting()
        }
    }


    private fun onEntryOfAwaitReply() {
                // SCE-MAP: event_origin_is_a_location.scxml:83 :: await_reply :: _state_body
    }

    private fun onEntryOfFail() {
                // SCE-MAP: event_origin_is_a_location.scxml:89 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: event_origin_is_a_location.scxml:88 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPhase() {
                // SCE-MAP: event_origin_is_a_location.scxml:49 :: phase :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("inv_peer", EventOriginIsALocationState.Phase)
    }

    private fun onEntryOfWaiting() {
                // SCE-MAP: event_origin_is_a_location.scxml:74 :: waiting :: _state_body
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
    private fun deferStaticInvoke(invokeId: String, state: EventOriginIsALocationState) {
        when (invokeId) {
            "inv_peer" -> startInvokeOfPhaseAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfPhaseAt0(state: EventOriginIsALocationState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "phase.${nextInvokeStart("inv_peer")}.inv_peer"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(EventOriginIsALocationSceSynthInvokeInvPeerStateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("inv_peer", childSM, false, EventOriginIsALocationEvent.Done.Invoke, "", generatedInvokeId, "", "event_origin_is_a_location__sce_synth_invoke__inv_peer", "inv_peer")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun onExit(state: EventOriginIsALocationState) {
        when (state) {
            is EventOriginIsALocationState.AwaitReply -> onExitOfAwaitReply()
            is EventOriginIsALocationState.Fail -> onExitOfFail()
            is EventOriginIsALocationState.Pass -> onExitOfPass()
            is EventOriginIsALocationState.Phase -> onExitOfPhase()
            is EventOriginIsALocationState.Waiting -> onExitOfWaiting()
        }
    }


    private fun onExitOfAwaitReply() {
                // SCE-MAP: event_origin_is_a_location.scxml:83 :: await_reply :: _state_body
    }

    private fun onExitOfFail() {
                // SCE-MAP: event_origin_is_a_location.scxml:89 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: event_origin_is_a_location.scxml:88 :: pass :: _state_body
    }

    private fun onExitOfPhase() {
                // SCE-MAP: event_origin_is_a_location.scxml:49 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(EventOriginIsALocationState.Phase)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_peer")
    }

    private fun onExitOfWaiting() {
                // SCE-MAP: event_origin_is_a_location.scxml:74 :: waiting :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: event_origin_is_a_location.scxml:40 :: _machine
    override fun executeTransitionContent(source: EventOriginIsALocationState, transitionIndex: Int) {
        when (source) {
            is EventOriginIsALocationState.Waiting -> transitionContentOfWaiting(transitionIndex)
            else -> {}
        }
    }


    // The content of `waiting`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfWaiting(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfWaitingAt0()
            else -> {}
        }
    }


    private fun transitionContentOfWaitingAt0() {
                // SCE-MAP: event_origin_is_a_location.scxml:75 :: waiting :: _transition_0


            if (run send@{
            assertScriptSession()
            val argEngine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
            val argSid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
            // §scxml-C-1: a target expression's value is read as text — the
            // same reading C++ `resultToString` gives it.
            val _rt = try {
                valueToWireString(argEngine.evaluateExpr(argSid, com.sce.runtime.ScriptSource.lua("_event.origin", "_event.origin")))
            } catch (_: Exception) {
                raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send> targetexpr could not be evaluated", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isInvalidTarget(_rt)) {
                // W3C SCXML 6.2 (test194): refused as a static one is.
                raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send> targetexpr produced a target this processor cannot address", "__send_0")
                return@send true
            }
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(EventOriginIsALocationEvent.Error.Communication, "<send> targetexpr evaluated to nothing, so there is no target to reach", "__send_0")
                return@send true
            }
            val sendData = ""
            val sendWireParams = emptyMap<String, List<String>>()
            if (com.sce.runtime.SendHelper.isMeshTarget(_rt)) {
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "reply",
                target = _rt,
                content = "",
                params = sendWireParams,
                sendId = "__send_0",
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
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_0")
            }
            } else {
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
EventOriginIsALocationEvent.Reply,
"reply",
                sendData,
0L,
                "__send_0",
                EventOriginIsALocationEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(EventOriginIsALocationEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(EventOriginIsALocationEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_0")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            } // end of the Mesh-peer choice (SCE_MESH.md §mesh-19)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }
}
