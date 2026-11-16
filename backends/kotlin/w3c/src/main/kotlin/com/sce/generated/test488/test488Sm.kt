// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/488/test488.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test488.scxml:4 :: _machine

package com.sce.generated.test488

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test488State : State {
    data object Fail : Test488State
    data object Pass : Test488State
    data object S0 : Test488State
    data object S01 : Test488State
    data object S02 : Test488State
    data object S1 : Test488State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test488Event : Event {
    data object Wildcard : Test488Event
    sealed interface Done : Test488Event {
        sealed interface State : Done {
            data object S0 : State
        }
    }
    sealed interface Error : Test488Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test488StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test488State, Test488Event>(scriptEngine) {

    override val initialState: Test488State = Test488State.S01

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test488State): Test488State? = when (state) {
        is Test488State.S01 -> Test488State.S0
        is Test488State.S02 -> Test488State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test488State): Boolean = when (state) {
        is Test488State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test488State): Boolean = when (state) {
        is Test488State.Fail -> true
        is Test488State.Pass -> true
        is Test488State.S02 -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test488State): List<Test488State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test488State): List<EntryTarget<Test488State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test488State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test488State, List<Test488State>> = LinkedHashMap<Test488State, List<Test488State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test488State, List<Test488State>>) {
            table[Test488State.S0] = listOf(Test488State.S01, Test488State.S02)
        }


        val initialTargets: Map<Test488State, List<EntryTarget<Test488State, HistoryId>>> = LinkedHashMap<Test488State, List<EntryTarget<Test488State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test488State, List<EntryTarget<Test488State, HistoryId>>>) {
            table[Test488State.S0] = listOf(StateTarget(Test488State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test488State, HistoryId>> =
            listOf(StateTarget(Test488State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S0,
            listOf(StateTarget(Test488State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S0,
            listOf(StateTarget(Test488State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S0,
            listOf(StateTarget(Test488State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S01,
            listOf(StateTarget(Test488State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S1,
            listOf(StateTarget(Test488State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test488State, HistoryId>(
            Test488State.S1,
            listOf(StateTarget(Test488State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test488State? = when (stateId) {
        "fail" -> Test488State.Fail
        "pass" -> Test488State.Pass
        "s0" -> Test488State.S0
        "s01" -> Test488State.S01
        "s02" -> Test488State.S02
        "s1" -> Test488State.S1
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test488State): String = when (state) {
        is Test488State.Fail -> "fail"
        is Test488State.Pass -> "pass"
        is Test488State.S0 -> "s0"
        is Test488State.S01 -> "s01"
        is Test488State.S02 -> "s02"
        is Test488State.S1 -> "s1"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test488State): Int = when (state) {
        is Test488State.Fail -> 5
        is Test488State.Pass -> 4
        is Test488State.S0 -> 0
        is Test488State.S01 -> 1
        is Test488State.S02 -> 2
        is Test488State.S1 -> 3
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test488Event? = when (name) {
        "done.state.s0" -> Test488Event.Done.State.S0
        "error.execution" -> Test488Event.Error.Execution
        "*" -> Test488Event.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test488Event): String? = when (event) {
        is Test488Event.Done.State.S0 -> "done.state.s0"
        is Test488Event.Error.Execution -> "error.execution"
        is Test488Event.Wildcard -> "*"
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
            "test488",
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
            raisePlatformError(Test488Event.Error.Execution, reason)
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
            raisePlatformError(Test488Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test488Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test488Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test488Event) {
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
    override fun bindCurrentEvent(event: Test488Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test488State, event: Test488Event?): EnabledTransition<Test488State, HistoryId>? = when (state) {
        is Test488State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test488State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test488State.S1 -> firstEnabledTransitionOfS1(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test488State, event: Test488Event?): EnabledTransition<Test488State, HistoryId>? = when {
        event is Test488Event.Error.Execution -> TransitionsPart0.transitionS0At0
        event is Test488Event.Done.State.S0 -> TransitionsPart0.transitionS0At1
        event is Test488Event.Done.State.S0 -> TransitionsPart0.transitionS0At2
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test488State, event: Test488Event?): EnabledTransition<Test488State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS01At0
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test488State, event: Test488Event?): EnabledTransition<Test488State, HistoryId>? = when {
        event is Test488Event.Done.State.S0 -> TransitionsPart0.transitionS1At0
        event != null -> TransitionsPart0.transitionS1At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test488.scxml:4 :: _machine
    override fun onEntry(state: Test488State, isDefaultEntry: Boolean) {
        when (state) {
            is Test488State.Fail -> onEntryOfFail()
            is Test488State.Pass -> onEntryOfPass()
            is Test488State.S0 -> onEntryOfS0()
            is Test488State.S01 -> onEntryOfS01()
            is Test488State.S02 -> onEntryOfS02()
            is Test488State.S1 -> onEntryOfS1()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test488.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test488.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test488.scxml:7 :: s0 :: _state_body
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test488.scxml:15 :: s01 :: _state_body
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test488.scxml:18 :: s02 :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    assertScriptSession()
                    val engineDD = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                    val sidDD = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
                    var doneEventData = ""
                    // W3C SCXML 5.5: Evaluate <param> elements (C++ DoneDataHelper::evaluateParams pattern)
                    val doneParams = mutableMapOf<String, Any?>()
                    try {
                        doneParams["someParam"] = engineDD.evaluateExpr(sidDD, com.sce.runtime.ScriptSource.lua("nil.invalidProperty", "undefined.invalidProperty"))
                    } catch (_: Exception) {
                        // §scxml-5.7: error.execution, and this pair is ignored.
                        raisePlatformError(Test488Event.Error.Execution, "<donedata> <param name='someParam'> failed to evaluate")
                    }
                    // §scxml-5.5: the pairs that survived, `{}` when none did
                    // (C++ DoneDataHelper::evaluateParams). Not left to
                    // buildJsonFromParams, whose empty answer is a <send>'s.
                    doneEventData = if (doneParams.isEmpty()) "{}" else buildJsonFromParams(doneParams)
                    // W3C SCXML 3.7: Final child state reached, raise done.state with data
                    raiseInternal(Test488Event.Done.State.S0, EventMetadata.platform(doneEventData))
                }
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test488.scxml:29 :: s1 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test488.scxml:4 :: _machine
    override fun onExit(state: Test488State) {
        when (state) {
            is Test488State.Fail -> onExitOfFail()
            is Test488State.Pass -> onExitOfPass()
            is Test488State.S0 -> onExitOfS0()
            is Test488State.S01 -> onExitOfS01()
            is Test488State.S02 -> onExitOfS02()
            is Test488State.S1 -> onExitOfS1()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test488.scxml:35 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test488.scxml:34 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test488.scxml:7 :: s0 :: _state_body
    }

    private fun onExitOfS01() {
                // SCE-MAP: test488.scxml:15 :: s01 :: _state_body
    }

    private fun onExitOfS02() {
                // SCE-MAP: test488.scxml:18 :: s02 :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test488.scxml:29 :: s1 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test488.scxml:4 :: _machine
    override fun executeTransitionContent(source: Test488State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
