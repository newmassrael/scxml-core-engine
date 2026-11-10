// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d102d83be3372e77ae8dde1c74b18f608301e96c6945cff9b996a5dc929e6ec

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: inline_content_is_read_by_its_data_model.scxml:39 :: _machine

package com.sce.integration.inline_content_is_read_by_its_data_model

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InlineContentIsReadByItsDataModelState : State {
    data object AwaitDocument : InlineContentIsReadByItsDataModelState
    data object AwaitNumber : InlineContentIsReadByItsDataModelState
    data object AwaitObject : InlineContentIsReadByItsDataModelState
    data object AwaitText : InlineContentIsReadByItsDataModelState
    data object FailDocument : InlineContentIsReadByItsDataModelState
    data object FailNumber : InlineContentIsReadByItsDataModelState
    data object FailObject : InlineContentIsReadByItsDataModelState
    data object FailText : InlineContentIsReadByItsDataModelState
    data object Pass : InlineContentIsReadByItsDataModelState
    data object Run : InlineContentIsReadByItsDataModelState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InlineContentIsReadByItsDataModelEvent : Event {
    sealed interface Done : InlineContentIsReadByItsDataModelEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Reporter : Invoke
        }
    }
    sealed interface Error : InlineContentIsReadByItsDataModelEvent {
        data object Execution : Error
    }
    data object Number : InlineContentIsReadByItsDataModelEvent
    data object Object : InlineContentIsReadByItsDataModelEvent
    data object Text : InlineContentIsReadByItsDataModelEvent
}
// --- State Machine (W3C SCXML) ---

class InlineContentIsReadByItsDataModelStateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<InlineContentIsReadByItsDataModelState, InlineContentIsReadByItsDataModelEvent>(scriptEngine) {

    override val initialState: InlineContentIsReadByItsDataModelState = InlineContentIsReadByItsDataModelState.AwaitNumber

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
    override fun parentOf(state: InlineContentIsReadByItsDataModelState): InlineContentIsReadByItsDataModelState? = when (state) {
        is InlineContentIsReadByItsDataModelState.AwaitNumber -> InlineContentIsReadByItsDataModelState.Run
        is InlineContentIsReadByItsDataModelState.AwaitObject -> InlineContentIsReadByItsDataModelState.Run
        is InlineContentIsReadByItsDataModelState.AwaitText -> InlineContentIsReadByItsDataModelState.Run
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: InlineContentIsReadByItsDataModelState): Boolean = when (state) {
        is InlineContentIsReadByItsDataModelState.Run -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InlineContentIsReadByItsDataModelState): Boolean = when (state) {
        is InlineContentIsReadByItsDataModelState.FailDocument -> true
        is InlineContentIsReadByItsDataModelState.FailNumber -> true
        is InlineContentIsReadByItsDataModelState.FailObject -> true
        is InlineContentIsReadByItsDataModelState.FailText -> true
        is InlineContentIsReadByItsDataModelState.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: InlineContentIsReadByItsDataModelState): List<InlineContentIsReadByItsDataModelState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: InlineContentIsReadByItsDataModelState): List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<InlineContentIsReadByItsDataModelState, List<InlineContentIsReadByItsDataModelState>> = LinkedHashMap<InlineContentIsReadByItsDataModelState, List<InlineContentIsReadByItsDataModelState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<InlineContentIsReadByItsDataModelState, List<InlineContentIsReadByItsDataModelState>>) {
            table[InlineContentIsReadByItsDataModelState.Run] = listOf(InlineContentIsReadByItsDataModelState.AwaitNumber, InlineContentIsReadByItsDataModelState.AwaitText, InlineContentIsReadByItsDataModelState.AwaitObject)
        }


        val initialTargets: Map<InlineContentIsReadByItsDataModelState, List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>>> = LinkedHashMap<InlineContentIsReadByItsDataModelState, List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<InlineContentIsReadByItsDataModelState, List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>>>) {
            table[InlineContentIsReadByItsDataModelState.Run] = listOf(StateTarget(InlineContentIsReadByItsDataModelState.AwaitNumber))
        }


        val documentInitialTargetList: List<EntryTarget<InlineContentIsReadByItsDataModelState, HistoryId>> =
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.Run))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: awaitDocument's transition 0, as the microstep reads it.
        val transitionAwaitDocumentAt0 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitDocument,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitDocument's transition 1, as the microstep reads it.
        val transitionAwaitDocumentAt1 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitDocument,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.FailDocument)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitNumber's transition 0, as the microstep reads it.
        val transitionAwaitNumberAt0 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitNumber,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.AwaitText)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitNumber's transition 1, as the microstep reads it.
        val transitionAwaitNumberAt1 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitNumber,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.FailNumber)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitObject's transition 0, as the microstep reads it.
        val transitionAwaitObjectAt0 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitObject,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.AwaitDocument)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitObject's transition 1, as the microstep reads it.
        val transitionAwaitObjectAt1 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitObject,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.FailObject)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitText's transition 0, as the microstep reads it.
        val transitionAwaitTextAt0 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitText,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.AwaitObject)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: awaitText's transition 1, as the microstep reads it.
        val transitionAwaitTextAt1 = EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>(
            InlineContentIsReadByItsDataModelState.AwaitText,
            listOf(StateTarget(InlineContentIsReadByItsDataModelState.FailText)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InlineContentIsReadByItsDataModelState? = when (stateId) {
        "awaitDocument" -> InlineContentIsReadByItsDataModelState.AwaitDocument
        "awaitNumber" -> InlineContentIsReadByItsDataModelState.AwaitNumber
        "awaitObject" -> InlineContentIsReadByItsDataModelState.AwaitObject
        "awaitText" -> InlineContentIsReadByItsDataModelState.AwaitText
        "failDocument" -> InlineContentIsReadByItsDataModelState.FailDocument
        "failNumber" -> InlineContentIsReadByItsDataModelState.FailNumber
        "failObject" -> InlineContentIsReadByItsDataModelState.FailObject
        "failText" -> InlineContentIsReadByItsDataModelState.FailText
        "pass" -> InlineContentIsReadByItsDataModelState.Pass
        "run" -> InlineContentIsReadByItsDataModelState.Run
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InlineContentIsReadByItsDataModelState): String = when (state) {
        is InlineContentIsReadByItsDataModelState.AwaitDocument -> "awaitDocument"
        is InlineContentIsReadByItsDataModelState.AwaitNumber -> "awaitNumber"
        is InlineContentIsReadByItsDataModelState.AwaitObject -> "awaitObject"
        is InlineContentIsReadByItsDataModelState.AwaitText -> "awaitText"
        is InlineContentIsReadByItsDataModelState.FailDocument -> "failDocument"
        is InlineContentIsReadByItsDataModelState.FailNumber -> "failNumber"
        is InlineContentIsReadByItsDataModelState.FailObject -> "failObject"
        is InlineContentIsReadByItsDataModelState.FailText -> "failText"
        is InlineContentIsReadByItsDataModelState.Pass -> "pass"
        is InlineContentIsReadByItsDataModelState.Run -> "run"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InlineContentIsReadByItsDataModelState): Int = when (state) {
        is InlineContentIsReadByItsDataModelState.AwaitDocument -> 4
        is InlineContentIsReadByItsDataModelState.AwaitNumber -> 1
        is InlineContentIsReadByItsDataModelState.AwaitObject -> 3
        is InlineContentIsReadByItsDataModelState.AwaitText -> 2
        is InlineContentIsReadByItsDataModelState.FailDocument -> 9
        is InlineContentIsReadByItsDataModelState.FailNumber -> 6
        is InlineContentIsReadByItsDataModelState.FailObject -> 8
        is InlineContentIsReadByItsDataModelState.FailText -> 7
        is InlineContentIsReadByItsDataModelState.Pass -> 5
        is InlineContentIsReadByItsDataModelState.Run -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InlineContentIsReadByItsDataModelEvent? = when (name) {
        "done.invoke" -> InlineContentIsReadByItsDataModelEvent.Done.Invoke.Self
        "done.invoke.reporter" -> InlineContentIsReadByItsDataModelEvent.Done.Invoke.Reporter
        "error.execution" -> InlineContentIsReadByItsDataModelEvent.Error.Execution
        "number" -> InlineContentIsReadByItsDataModelEvent.Number
        "object" -> InlineContentIsReadByItsDataModelEvent.Object
        "text" -> InlineContentIsReadByItsDataModelEvent.Text
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InlineContentIsReadByItsDataModelEvent): String? = when (event) {
        is InlineContentIsReadByItsDataModelEvent.Done.Invoke.Self -> "done.invoke"
        is InlineContentIsReadByItsDataModelEvent.Done.Invoke.Reporter -> "done.invoke.reporter"
        is InlineContentIsReadByItsDataModelEvent.Error.Execution -> "error.execution"
        is InlineContentIsReadByItsDataModelEvent.Number -> "number"
        is InlineContentIsReadByItsDataModelEvent.Object -> "object"
        is InlineContentIsReadByItsDataModelEvent.Text -> "text"
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
            "inline_content_is_read_by_its_data_model",
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
            raisePlatformError(InlineContentIsReadByItsDataModelEvent.Error.Execution, reason)
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
            raisePlatformError(InlineContentIsReadByItsDataModelEvent.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(InlineContentIsReadByItsDataModelEvent.Error.Execution, "<assign> failed")
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
            raisePlatformError(InlineContentIsReadByItsDataModelEvent.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: InlineContentIsReadByItsDataModelEvent) {
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
    override fun bindCurrentEvent(event: InlineContentIsReadByItsDataModelEvent) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InlineContentIsReadByItsDataModelState, event: InlineContentIsReadByItsDataModelEvent?): EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>? = when (state) {
        is InlineContentIsReadByItsDataModelState.AwaitDocument -> firstEnabledTransitionOfAwaitDocument(state, event)
        is InlineContentIsReadByItsDataModelState.AwaitNumber -> firstEnabledTransitionOfAwaitNumber(state, event)
        is InlineContentIsReadByItsDataModelState.AwaitObject -> firstEnabledTransitionOfAwaitObject(state, event)
        is InlineContentIsReadByItsDataModelState.AwaitText -> firstEnabledTransitionOfAwaitText(state, event)
        else -> null
    }


    // awaitDocument's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitDocument(state: InlineContentIsReadByItsDataModelState, event: InlineContentIsReadByItsDataModelEvent?): EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>? = when {
        event is InlineContentIsReadByItsDataModelEvent.Done.Invoke.Reporter && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(_event.data) and (_event.data:getElementsByTagName(\"book\")[1]:getAttribute(\"title\") == \"x\"))", "_event.data && _event.data.getElementsByTagName('book')[0].getAttribute('title') === 'x'")) -> TransitionsPart0.transitionAwaitDocumentAt0
        event is InlineContentIsReadByItsDataModelEvent.Done.Invoke.Reporter -> TransitionsPart0.transitionAwaitDocumentAt1
        else -> null
    }


    // awaitNumber's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitNumber(state: InlineContentIsReadByItsDataModelState, event: InlineContentIsReadByItsDataModelEvent?): EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>? = when {
        event is InlineContentIsReadByItsDataModelEvent.Number && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == 21)", "_event.data === 21")) -> TransitionsPart0.transitionAwaitNumberAt0
        event is InlineContentIsReadByItsDataModelEvent.Number -> TransitionsPart0.transitionAwaitNumberAt1
        else -> null
    }


    // awaitObject's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitObject(state: InlineContentIsReadByItsDataModelState, event: InlineContentIsReadByItsDataModelEvent?): EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>? = when {
        event is InlineContentIsReadByItsDataModelEvent.Object && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_scxml_truthy(_event.data) and (_event.data.k == 7))", "_event.data && _event.data.k === 7")) -> TransitionsPart0.transitionAwaitObjectAt0
        event is InlineContentIsReadByItsDataModelEvent.Object -> TransitionsPart0.transitionAwaitObjectAt1
        else -> null
    }


    // awaitText's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfAwaitText(state: InlineContentIsReadByItsDataModelState, event: InlineContentIsReadByItsDataModelEvent?): EnabledTransition<InlineContentIsReadByItsDataModelState, HistoryId>? = when {
        event is InlineContentIsReadByItsDataModelEvent.Text && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("(_event.data == \"two words\")", "_event.data === 'two words'")) -> TransitionsPart0.transitionAwaitTextAt0
        event is InlineContentIsReadByItsDataModelEvent.Text -> TransitionsPart0.transitionAwaitTextAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:39 :: _machine
    override fun onEntry(state: InlineContentIsReadByItsDataModelState, isDefaultEntry: Boolean) {
        when (state) {
            is InlineContentIsReadByItsDataModelState.AwaitDocument -> onEntryOfAwaitDocument()
            is InlineContentIsReadByItsDataModelState.AwaitNumber -> onEntryOfAwaitNumber()
            is InlineContentIsReadByItsDataModelState.AwaitObject -> onEntryOfAwaitObject()
            is InlineContentIsReadByItsDataModelState.AwaitText -> onEntryOfAwaitText()
            is InlineContentIsReadByItsDataModelState.FailDocument -> onEntryOfFailDocument()
            is InlineContentIsReadByItsDataModelState.FailNumber -> onEntryOfFailNumber()
            is InlineContentIsReadByItsDataModelState.FailObject -> onEntryOfFailObject()
            is InlineContentIsReadByItsDataModelState.FailText -> onEntryOfFailText()
            is InlineContentIsReadByItsDataModelState.Pass -> onEntryOfPass()
            is InlineContentIsReadByItsDataModelState.Run -> onEntryOfRun()
        }
    }


    private fun onEntryOfAwaitDocument() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:83 :: awaitDocument :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("reporter", InlineContentIsReadByItsDataModelState.AwaitDocument)
    }

    private fun onEntryOfAwaitNumber() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:64 :: awaitNumber :: _state_body
    }

    private fun onEntryOfAwaitObject() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:77 :: awaitObject :: _state_body
    }

    private fun onEntryOfAwaitText() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:69 :: awaitText :: _state_body
    }

    private fun onEntryOfFailDocument() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:106 :: failDocument :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailNumber() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:103 :: failNumber :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailObject() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:105 :: failObject :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfFailText() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:104 :: failText :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:102 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfRun() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:43 :: run :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("emitter", InlineContentIsReadByItsDataModelState.Run)
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
    private fun deferStaticInvoke(invokeId: String, state: InlineContentIsReadByItsDataModelState) {
        when (invokeId) {
            "reporter" -> startInvokeOfAwaitDocumentAt0(state)
            "emitter" -> startInvokeOfRunAt0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfAwaitDocumentAt0(state: InlineContentIsReadByItsDataModelState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "awaitDocument.${nextInvokeStart("reporter")}.reporter"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(InlineContentIsReadByItsDataModelSceSynthInvokeReporterStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("reporter", childSM, false, InlineContentIsReadByItsDataModelEvent.Done.Invoke.Reporter, "", generatedInvokeId, "", "inline_content_is_read_by_its_data_model__sce_synth_invoke__reporter", "reporter")
                }
            }
    }

    private fun startInvokeOfRunAt0(state: InlineContentIsReadByItsDataModelState) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "run.${nextInvokeStart("emitter")}.emitter"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("emitter", childSM, false, InlineContentIsReadByItsDataModelEvent.Done.Invoke.Self, "", generatedInvokeId, "", "inline_content_is_read_by_its_data_model__sce_synth_invoke__emitter", "emitter")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:39 :: _machine
    override fun onExit(state: InlineContentIsReadByItsDataModelState) {
        when (state) {
            is InlineContentIsReadByItsDataModelState.AwaitDocument -> onExitOfAwaitDocument()
            is InlineContentIsReadByItsDataModelState.AwaitNumber -> onExitOfAwaitNumber()
            is InlineContentIsReadByItsDataModelState.AwaitObject -> onExitOfAwaitObject()
            is InlineContentIsReadByItsDataModelState.AwaitText -> onExitOfAwaitText()
            is InlineContentIsReadByItsDataModelState.FailDocument -> onExitOfFailDocument()
            is InlineContentIsReadByItsDataModelState.FailNumber -> onExitOfFailNumber()
            is InlineContentIsReadByItsDataModelState.FailObject -> onExitOfFailObject()
            is InlineContentIsReadByItsDataModelState.FailText -> onExitOfFailText()
            is InlineContentIsReadByItsDataModelState.Pass -> onExitOfPass()
            is InlineContentIsReadByItsDataModelState.Run -> onExitOfRun()
        }
    }


    private fun onExitOfAwaitDocument() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:83 :: awaitDocument :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(InlineContentIsReadByItsDataModelState.AwaitDocument)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("reporter")
    }

    private fun onExitOfAwaitNumber() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:64 :: awaitNumber :: _state_body
    }

    private fun onExitOfAwaitObject() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:77 :: awaitObject :: _state_body
    }

    private fun onExitOfAwaitText() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:69 :: awaitText :: _state_body
    }

    private fun onExitOfFailDocument() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:106 :: failDocument :: _state_body
    }

    private fun onExitOfFailNumber() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:103 :: failNumber :: _state_body
    }

    private fun onExitOfFailObject() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:105 :: failObject :: _state_body
    }

    private fun onExitOfFailText() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:104 :: failText :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:102 :: pass :: _state_body
    }

    private fun onExitOfRun() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:43 :: run :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(InlineContentIsReadByItsDataModelState.Run)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("emitter")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:39 :: _machine
    override fun executeTransitionContent(source: InlineContentIsReadByItsDataModelState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
