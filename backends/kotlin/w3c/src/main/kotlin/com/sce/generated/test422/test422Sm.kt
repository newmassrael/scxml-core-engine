// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/422/test422.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422.scxml:10 :: _machine

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422State : State {
    data object Fail : Test422State
    data object Pass : Test422State
    data object S1 : Test422State
    data object S11 : Test422State
    data object S12 : Test422State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422Event : Event {
    sealed interface Done : Test422Event {
        data object Invoke : Done
    }
    sealed interface Error : Test422Event {
        data object Execution : Error
    }
    data object InvokeS1 : Test422Event
    data object InvokeS11 : Test422Event
    data object InvokeS12 : Test422Event
    data object Timeout : Test422Event
}
// --- State Machine (W3C SCXML) ---

class Test422StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test422State, Test422Event>(scriptEngine) {

    // ── §scxml-5.3: read the datamodel this machine is holding ──────────

    /**
     * §scxml-5.3: what the `Var1` datamodel variable is holding now.
     *
     * The live value, not the authored one: `<assign>` writes into the
     * session, so a reader frozen at generation time would answer the
     * document's literal for the whole run. `null` means the machine cannot
     * answer — no script engine is set, the session is not initialised yet,
     * `Var1` was assigned a value of another type, or the engine refused.
     */
    fun Var1(): Long? =
        com.sce.runtime.DatamodelRead.readInt(scriptEngine, scriptSessionId, "Var1")

    override val initialState: Test422State = Test422State.S11

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
    override fun parentOf(state: Test422State): Test422State? = when (state) {
        is Test422State.S11 -> Test422State.S1
        is Test422State.S12 -> Test422State.S1
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test422State): Boolean = when (state) {
        is Test422State.S1 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test422State): Boolean = when (state) {
        is Test422State.Fail -> true
        is Test422State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test422State): List<Test422State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test422State): List<EntryTarget<Test422State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test422State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test422State, List<Test422State>> = LinkedHashMap<Test422State, List<Test422State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test422State, List<Test422State>>) {
            table[Test422State.S1] = listOf(Test422State.S11, Test422State.S12)
        }


        val initialTargets: Map<Test422State, List<EntryTarget<Test422State, HistoryId>>> = LinkedHashMap<Test422State, List<EntryTarget<Test422State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test422State, List<EntryTarget<Test422State, HistoryId>>>) {
            table[Test422State.S1] = listOf(StateTarget(Test422State.S11))
        }


        val documentInitialTargetList: List<EntryTarget<Test422State, HistoryId>> =
            listOf(StateTarget(Test422State.S1))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test422State, HistoryId>(
            Test422State.S1,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test422State, HistoryId>(
            Test422State.S1,
            listOf(StateTarget(Test422State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 2, as the microstep reads it.
        val transitionS1At2 = EnabledTransition<Test422State, HistoryId>(
            Test422State.S1,
            listOf(StateTarget(Test422State.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 3, as the microstep reads it.
        val transitionS1At3 = EnabledTransition<Test422State, HistoryId>(
            Test422State.S1,
            listOf(StateTarget(Test422State.Fail)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11's transition 0, as the microstep reads it.
        val transitionS11At0 = EnabledTransition<Test422State, HistoryId>(
            Test422State.S11,
            listOf(StateTarget(Test422State.S12)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422State? = when (stateId) {
        "fail" -> Test422State.Fail
        "pass" -> Test422State.Pass
        "s1" -> Test422State.S1
        "s11" -> Test422State.S11
        "s12" -> Test422State.S12
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422State): String = when (state) {
        is Test422State.Fail -> "fail"
        is Test422State.Pass -> "pass"
        is Test422State.S1 -> "s1"
        is Test422State.S11 -> "s11"
        is Test422State.S12 -> "s12"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test422State): Int = when (state) {
        is Test422State.Fail -> 4
        is Test422State.Pass -> 3
        is Test422State.S1 -> 0
        is Test422State.S11 -> 1
        is Test422State.S12 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422Event? = when (name) {
        "done.invoke" -> Test422Event.Done.Invoke
        "error.execution" -> Test422Event.Error.Execution
        "invokeS1" -> Test422Event.InvokeS1
        "invokeS11" -> Test422Event.InvokeS11
        "invokeS12" -> Test422Event.InvokeS12
        "timeout" -> Test422Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422Event): String? = when (event) {
        is Test422Event.Done.Invoke -> "done.invoke"
        is Test422Event.Error.Execution -> "error.execution"
        is Test422Event.InvokeS1 -> "invokeS1"
        is Test422Event.InvokeS11 -> "invokeS11"
        is Test422Event.InvokeS12 -> "invokeS12"
        is Test422Event.Timeout -> "timeout"
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
            "test422",
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
        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("0", "0"))
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test422Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
            raisePlatformError(Test422Event.Error.Execution, reason)
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
            raisePlatformError(Test422Event.Error.Execution, "an expression could not be serialised to JSON")
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
            raisePlatformError(Test422Event.Error.Execution, "<assign> failed")
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
            raisePlatformError(Test422Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test422Event) {
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
    override fun bindCurrentEvent(event: Test422Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test422State, event: Test422Event?): EnabledTransition<Test422State, HistoryId>? = when (state) {
        is Test422State.S1 -> firstEnabledTransitionOfS1(state, event)
        is Test422State.S11 -> firstEnabledTransitionOfS11(state, event)
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test422State, event: Test422Event?): EnabledTransition<Test422State, HistoryId>? = when {
        (event is Test422Event.InvokeS1 || event is Test422Event.InvokeS12) -> TransitionsPart0.transitionS1At0
        event is Test422Event.InvokeS11 -> TransitionsPart0.transitionS1At1
        event is Test422Event.Timeout && safeEvaluateGuard(com.sce.runtime.ScriptSource.lua("_scxml_eq(Var1, 2)", "Var1 == 2")) -> TransitionsPart0.transitionS1At2
        event is Test422Event.Timeout -> TransitionsPart0.transitionS1At3
        else -> null
    }


    // s11's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS11(state: Test422State, event: Test422Event?): EnabledTransition<Test422State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS11At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422.scxml:10 :: _machine
    override fun onEntry(state: Test422State, isDefaultEntry: Boolean) {
        when (state) {
            is Test422State.Fail -> onEntryOfFail()
            is Test422State.Pass -> onEntryOfPass()
            is Test422State.S1 -> onEntryOfS1()
            is Test422State.S11 -> onEntryOfS11()
            is Test422State.S12 -> onEntryOfS12()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test422.scxml:77 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test422.scxml:76 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test422.scxml:14 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, Test422Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test422State.S1)
    }

    private fun onEntryOfS11() {
                // SCE-MAP: test422.scxml:40 :: s11 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_1", Test422State.S11)
    }

    private fun onEntryOfS12() {
                // SCE-MAP: test422.scxml:57 :: s12 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_2", Test422State.S12)
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
    private fun deferStaticInvoke(invokeId: String, state: Test422State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS1At0(state)
            "_invoke_1" -> startInvokeOfS11At0(state)
            "_invoke_2" -> startInvokeOfS12At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS1At0(state: Test422State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s1.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test422SceSynthInvokeInvoke0StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId, "", "test422__sce_synth_invoke__invoke_0", "_invoke_0")
                }
            }
    }

    private fun startInvokeOfS11At0(state: Test422State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s11.${nextInvokeStart("_invoke_1")}._invoke_1"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test422SceSynthInvokeInvoke1StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_1", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId, "", "test422__sce_synth_invoke__invoke_1", "_invoke_1")
                }
            }
    }

    private fun startInvokeOfS12At0(state: Test422State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s12.${nextInvokeStart("_invoke_2")}._invoke_2"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test422SceSynthInvokeInvoke2StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_2", childSM, false, Test422Event.Done.Invoke, "", generatedInvokeId, "", "test422__sce_synth_invoke__invoke_2", "_invoke_2")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422.scxml:10 :: _machine
    override fun onExit(state: Test422State) {
        when (state) {
            is Test422State.Fail -> onExitOfFail()
            is Test422State.Pass -> onExitOfPass()
            is Test422State.S1 -> onExitOfS1()
            is Test422State.S11 -> onExitOfS11()
            is Test422State.S12 -> onExitOfS12()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test422.scxml:77 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test422.scxml:76 :: pass :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test422.scxml:14 :: s1 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test422State.S1)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS11() {
                // SCE-MAP: test422.scxml:40 :: s11 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test422State.S11)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
    }

    private fun onExitOfS12() {
                // SCE-MAP: test422.scxml:57 :: s12 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test422State.S12)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test422.scxml:10 :: _machine
    override fun executeTransitionContent(source: Test422State, transitionIndex: Int) {
        when (source) {
            is Test422State.S1 -> transitionContentOfS1(transitionIndex)
            else -> {}
        }
    }


    // The content of `s1`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfS1(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfS1At0()
            else -> {}
        }
    }


    private fun transitionContentOfS1At0() {
                // SCE-MAP: test422.scxml:18 :: s1 :: _transition_0


            if (!executeAssign(com.sce.runtime.ScriptSource.lua("Var1", "Var1"), com.sce.runtime.ScriptSource.lua("_scxml_add(Var1, 1)", "Var1 + 1"))) {
                return
            }
    }
}
