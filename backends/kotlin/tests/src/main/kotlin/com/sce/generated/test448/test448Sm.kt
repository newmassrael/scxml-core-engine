// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/448/test448.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test448.scxml:5 :: _machine

package com.sce.generated.test448

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test448State : State {
    data object Fail : Test448State
    data object Pass : Test448State
    data object S0 : Test448State
    data object S01 : Test448State
    data object S01p : Test448State
    data object S01p1 : Test448State
    data object S01p2 : Test448State
    data object S1 : Test448State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test448Event : Event {
    sealed interface Error : Test448Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test448StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test448State, Test448Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `var1` was assigned a value of another type, or the engine refused.
     */
    fun var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "var1")

    /**
     * §scxml-5.3: what the `var2` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `var2` was assigned a value of another type, or the engine refused.
     */
    fun var2(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "var2")

    override val initialState: Test448State = Test448State.S01

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
    override fun parentOf(state: Test448State): Test448State? = when (state) {
        is Test448State.S01 -> Test448State.S0
        is Test448State.S01p -> Test448State.S1
        is Test448State.S01p1 -> Test448State.S01p
        is Test448State.S01p2 -> Test448State.S01p
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test448State): Boolean = when (state) {
        is Test448State.S0 -> true
        is Test448State.S1 -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test448State): Boolean = when (state) {
        is Test448State.S01p -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test448State): Boolean = when (state) {
        is Test448State.Fail -> true
        is Test448State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test448State): List<Test448State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test448State): List<EntryTarget<Test448State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test448State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test448State, List<Test448State>> = LinkedHashMap<Test448State, List<Test448State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test448State, List<Test448State>>) {
            table[Test448State.S0] = listOf(Test448State.S01)
            table[Test448State.S01p] = listOf(Test448State.S01p1, Test448State.S01p2)
            table[Test448State.S1] = listOf(Test448State.S01p)
        }


        val initialTargets: Map<Test448State, List<EntryTarget<Test448State, HistoryId>>> = LinkedHashMap<Test448State, List<EntryTarget<Test448State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test448State, List<EntryTarget<Test448State, HistoryId>>>) {
            table[Test448State.S0] = listOf(StateTarget(Test448State.S01))
            table[Test448State.S1] = listOf(StateTarget(Test448State.S01p))
        }


        val documentInitialTargetList: List<EntryTarget<Test448State, HistoryId>> =
            listOf(StateTarget(Test448State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test448State, HistoryId>(
            Test448State.S0,
            listOf(StateTarget(Test448State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test448State, HistoryId>(
            Test448State.S0,
            listOf(StateTarget(Test448State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01p1's transition 0, as the microstep reads it.
        val transitionS01p1At0 = EnabledTransition<Test448State, HistoryId>(
            Test448State.S01p1,
            listOf(StateTarget(Test448State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01p1's transition 1, as the microstep reads it.
        val transitionS01p1At1 = EnabledTransition<Test448State, HistoryId>(
            Test448State.S01p1,
            listOf(StateTarget(Test448State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test448State? = when (stateId) {
        "fail" -> Test448State.Fail
        "pass" -> Test448State.Pass
        "s0" -> Test448State.S0
        "s01" -> Test448State.S01
        "s01p" -> Test448State.S01p
        "s01p1" -> Test448State.S01p1
        "s01p2" -> Test448State.S01p2
        "s1" -> Test448State.S1
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test448State): String = when (state) {
        is Test448State.Fail -> "fail"
        is Test448State.Pass -> "pass"
        is Test448State.S0 -> "s0"
        is Test448State.S01 -> "s01"
        is Test448State.S01p -> "s01p"
        is Test448State.S01p1 -> "s01p1"
        is Test448State.S01p2 -> "s01p2"
        is Test448State.S1 -> "s1"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test448State): Int = when (state) {
        is Test448State.Fail -> 7
        is Test448State.Pass -> 6
        is Test448State.S0 -> 0
        is Test448State.S01 -> 1
        is Test448State.S01p -> 3
        is Test448State.S01p1 -> 4
        is Test448State.S01p2 -> 5
        is Test448State.S1 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test448Event? = when (name) {
        "error.execution" -> Test448Event.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test448Event): String? = when (event) {
        is Test448Event.Error.Execution -> "error.execution"
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
            "test448",
        )?.let { return it }

        // W3C SCXML 5.9.2: Register In() predicate callback, before any
        // `<data>` or top-level `<script>` can ask it
        val sid = scriptSessionId ?: error("scriptSessionId is established by the start (codegen invariant)")
        engine.setStateQueryCallback(sid) { stateId -> isStateActive(stateId) }
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


        // W3C SCXML 5.3: Early binding — initialize state-level datamodel variables at startup
        declareStateVariablesPart0(engine, sid)

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

    private fun declareStateVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // State 's01' variable 'var1'
        try {
            val initResult_var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "var1", initResult_var1)
        } catch (e: Exception) {
            raisePlatformError(Test448Event.Error.Execution, "<data id='var1'> expr failed to evaluate")
        }
        // State 's01p2' variable 'var2'
        try {
            val initResult_var2 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "var2", initResult_var2)
        } catch (e: Exception) {
            raisePlatformError(Test448Event.Error.Execution, "<data id='var2'> expr failed to evaluate")
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
            raisePlatformError(Test448Event.Error.Execution, reason)
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
            raisePlatformError(Test448Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test448Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test448Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test448Event) {
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
    override fun bindCurrentEvent(event: Test448Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test448State, event: Test448Event?): EnabledTransition<Test448State, HistoryId>? = when (state) {
        is Test448State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test448State.S01p1 -> firstEnabledTransitionOfS01p1(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test448State, event: Test448Event?): EnabledTransition<Test448State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(var1, 1)", "var1==1")) -> TransitionsPart0.transitionS0At0
        event == null -> TransitionsPart0.transitionS0At1
        else -> null
    }


    // s01p1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01p1(state: Test448State, event: Test448Event?): EnabledTransition<Test448State, HistoryId>? = when {
        event == null && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(var2, 1)", "var2==1")) -> TransitionsPart0.transitionS01p1At0
        event == null -> TransitionsPart0.transitionS01p1At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun onEntry(state: Test448State, isDefaultEntry: Boolean) {
        when (state) {
            is Test448State.Fail -> onEntryOfFail()
            is Test448State.Pass -> onEntryOfPass()
            is Test448State.S0 -> onEntryOfS0()
            is Test448State.S01 -> onEntryOfS01()
            is Test448State.S01p -> onEntryOfS01p()
            is Test448State.S01p1 -> onEntryOfS01p1()
            is Test448State.S01p2 -> onEntryOfS01p2()
            is Test448State.S1 -> onEntryOfS1()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test448.scxml:34 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test448.scxml:33 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test448.scxml:8 :: s0 :: _state_body
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test448.scxml:12 :: s01 :: _state_body
    }

    private fun onEntryOfS01p() {
                // SCE-MAP: test448.scxml:19 :: s01p :: _state_body
    }

    private fun onEntryOfS01p1() {
                // SCE-MAP: test448.scxml:20 :: s01p1 :: _state_body
    }

    private fun onEntryOfS01p2() {
                // SCE-MAP: test448.scxml:25 :: s01p2 :: _state_body
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test448.scxml:18 :: s1 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun onExit(state: Test448State) {
        when (state) {
            is Test448State.Fail -> onExitOfFail()
            is Test448State.Pass -> onExitOfPass()
            is Test448State.S0 -> onExitOfS0()
            is Test448State.S01 -> onExitOfS01()
            is Test448State.S01p -> onExitOfS01p()
            is Test448State.S01p1 -> onExitOfS01p1()
            is Test448State.S01p2 -> onExitOfS01p2()
            is Test448State.S1 -> onExitOfS1()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test448.scxml:34 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test448.scxml:33 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test448.scxml:8 :: s0 :: _state_body
    }

    private fun onExitOfS01() {
                // SCE-MAP: test448.scxml:12 :: s01 :: _state_body
    }

    private fun onExitOfS01p() {
                // SCE-MAP: test448.scxml:19 :: s01p :: _state_body
    }

    private fun onExitOfS01p1() {
                // SCE-MAP: test448.scxml:20 :: s01p1 :: _state_body
    }

    private fun onExitOfS01p2() {
                // SCE-MAP: test448.scxml:25 :: s01p2 :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test448.scxml:18 :: s1 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test448.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test448State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
