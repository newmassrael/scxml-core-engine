// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/241/test241.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test241.scxml:8 :: _machine

package com.sce.generated.test241

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test241State : State {
    data object Fail : Test241State
    data object Pass : Test241State
    data object S0 : Test241State
    data object S01 : Test241State
    data object S02 : Test241State
    data object S03 : Test241State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test241Event : Event {
    sealed interface Done : Test241Event {
        data object Invoke : Done
    }
    sealed interface Error : Test241Event {
        data object Execution : Error
    }
    data object Failure : Test241Event
    data object Success : Test241Event
    data object Timeout : Test241Event
}
// --- State Machine (W3C SCXML) ---

class Test241StateMachine(
    scriptEngine: ScxmlScriptEngine,
) : StateMachineEngine<Test241State, Test241Event>(scriptEngine) {

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

    override val initialState: Test241State = Test241State.S01

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // W3C SCXML B.1: Initialize script engine before entering initial state
    override fun enterInitialConfiguration() {
        ensureScriptEngine()
        super.enterInitialConfiguration()
    }

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test241State): Test241State? = when (state) {
        is Test241State.S01 -> Test241State.S0
        is Test241State.S02 -> Test241State.S0
        is Test241State.S03 -> Test241State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test241State): Boolean = when (state) {
        is Test241State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test241State): Boolean = when (state) {
        is Test241State.Fail -> true
        is Test241State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test241State): List<Test241State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test241State): List<EntryTarget<Test241State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test241State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test241State, List<Test241State>> = LinkedHashMap<Test241State, List<Test241State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test241State, List<Test241State>>) {
            table[Test241State.S0] = listOf(Test241State.S01, Test241State.S02, Test241State.S03)
        }


        val initialTargets: Map<Test241State, List<EntryTarget<Test241State, HistoryId>>> = LinkedHashMap<Test241State, List<EntryTarget<Test241State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test241State, List<EntryTarget<Test241State, HistoryId>>>) {
            table[Test241State.S0] = listOf(StateTarget(Test241State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test241State, HistoryId>> =
            listOf(StateTarget(Test241State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S0,
            listOf(StateTarget(Test241State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S01,
            listOf(StateTarget(Test241State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 1, as the microstep reads it.
        val transitionS01At1 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S01,
            listOf(StateTarget(Test241State.S03)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S02,
            listOf(StateTarget(Test241State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S02,
            listOf(StateTarget(Test241State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S03,
            listOf(StateTarget(Test241State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test241State, HistoryId>(
            Test241State.S03,
            listOf(StateTarget(Test241State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test241State? = when (stateId) {
        "fail" -> Test241State.Fail
        "pass" -> Test241State.Pass
        "s0" -> Test241State.S0
        "s01" -> Test241State.S01
        "s02" -> Test241State.S02
        "s03" -> Test241State.S03
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test241State): String = when (state) {
        is Test241State.Fail -> "fail"
        is Test241State.Pass -> "pass"
        is Test241State.S0 -> "s0"
        is Test241State.S01 -> "s01"
        is Test241State.S02 -> "s02"
        is Test241State.S03 -> "s03"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test241State): Int = when (state) {
        is Test241State.Fail -> 5
        is Test241State.Pass -> 4
        is Test241State.S0 -> 0
        is Test241State.S01 -> 1
        is Test241State.S02 -> 2
        is Test241State.S03 -> 3
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test241Event? = when (name) {
        "done.invoke" -> Test241Event.Done.Invoke
        "error.execution" -> Test241Event.Error.Execution
        "failure" -> Test241Event.Failure
        "success" -> Test241Event.Success
        "timeout" -> Test241Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test241Event): String? = when (event) {
        is Test241Event.Done.Invoke -> "done.invoke"
        is Test241Event.Error.Execution -> "error.execution"
        is Test241Event.Failure -> "failure"
        is Test241Event.Success -> "success"
        is Test241Event.Timeout -> "timeout"
    }




    // --- Script Engine Helpers (W3C SCXML B.1) ---

    // W3C SCXML 5.3: the declaration hook `enterAt` reaches. Every other caller
    // arrives through a guard, an assign or a script block, all of which run
    // `ensureScriptEngine()` on their own way in; a resume runs none of them,
    // and a host putting saved values back needs the variables to exist first.
    override fun declareDatamodel() {
        ensureScriptEngine()
    }

    // W3C SCXML B.1: Lazy script engine initialization
    private fun ensureScriptEngine() {
        if (scriptEngineInitialized) return
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = allocateScriptSession()
        engine.createSession(sid)

        // §scxml-C-1-1 / §scxml-C-2-3: the `_ioprocessors` entries come from the
        // same helper every other backend uses, so a machine reads the same
        // entry names and the same addresses whichever one runs it.
        engine.setupSystemVariables(
            sid,
            "test241",
            com.sce.runtime.IoProcessors.build(sid, basicHttpAccessUri),
        )

        declareVariablesPart0(engine, sid)




        // W3C SCXML 6.4: Apply pending invoke params from parent
        // Only set params matching child's declared datamodel variables (C++ DatamodelValidationHelper)
        if (pendingInvokeParams.isNotEmpty()) {
            for ((pName, pValue) in pendingInvokeParams) {
                if (engine.hasVariable(sid, pName)) {
                    try { engine.setVariable(sid, pName, pValue) } catch (_: Exception) {}
                }
            }
            pendingInvokeParams = emptyMap()
        }

        scriptEngineInitialized = true
    }

    @Suppress("UNUSED_PARAMETER")
    private fun declareVariablesPart0(engine: ScxmlScriptEngine, sid: String) {
        // W3C SCXML 5.3: Initialize variable 'Var1' with expr
        try {
            val initResult_Var1 = engine.evaluateExpr(sid, com.sce.runtime.ScriptSource.lua("1", "1"))
            engine.setVariable(sid, "Var1", initResult_Var1)
        } catch (e: Exception) {
            raisePlatformError(Test241Event.Error.Execution, "<data id='Var1'> expr failed to evaluate")
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
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.evaluateCondition(sid, guardExpr)
        } catch (e: Exception) {
            raisePlatformError(Test241Event.Error.Execution, reason)
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
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
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
            raisePlatformError(Test241Event.Error.Execution, "an expression could not be serialised to JSON")
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
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.assign(sid, location, expr)
            true
        } catch (e: Exception) {
            raisePlatformError(Test241Event.Error.Execution, "<assign> failed")
            false
        }
    }

    // W3C SCXML 5.8: Script block execution
    // Returns whether the script ran; on failure error.execution is already
    // raised, and the caller ends its block (W3C SCXML 4.9).
    private fun executeScriptBlock(script: com.sce.runtime.ScriptSource): Boolean {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
        return try {
            engine.executeScript(sid, script)
            true
        } catch (e: Exception) {
            raisePlatformError(Test241Event.Error.Execution, "<script> failed to execute")
            false
        }
    }

    // W3C SCXML 5.10: Set _event before event processing
    private fun setCurrentEventInScriptEngine(event: Test241Event) {
        ensureScriptEngine()
        val engine = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
        val sid = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
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
    override fun bindCurrentEvent(event: Test241Event) {
        setCurrentEventInScriptEngine(event)
    }

    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test241State, event: Test241Event?): EnabledTransition<Test241State, HistoryId>? = when (state) {
        is Test241State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test241State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test241State.S02 -> firstEnabledTransitionOfS02(state, event)
        is Test241State.S03 -> firstEnabledTransitionOfS03(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test241State, event: Test241Event?): EnabledTransition<Test241State, HistoryId>? = when {
        event is Test241Event.Timeout -> TransitionsPart0.transitionS0At0
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test241State, event: Test241Event?): EnabledTransition<Test241State, HistoryId>? = when {
        event is Test241Event.Success -> TransitionsPart0.transitionS01At0
        event is Test241Event.Failure -> TransitionsPart0.transitionS01At1
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test241State, event: Test241Event?): EnabledTransition<Test241State, HistoryId>? = when {
        event is Test241Event.Success -> TransitionsPart0.transitionS02At0
        event is Test241Event.Failure -> TransitionsPart0.transitionS02At1
        else -> null
    }


    // s03's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS03(state: Test241State, event: Test241Event?): EnabledTransition<Test241State, HistoryId>? = when {
        event is Test241Event.Failure -> TransitionsPart0.transitionS03At0
        event is Test241Event.Success -> TransitionsPart0.transitionS03At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test241.scxml:8 :: _machine
    override fun onEntry(state: Test241State, isDefaultEntry: Boolean) {
        when (state) {
            is Test241State.Fail -> onEntryOfFail()
            is Test241State.Pass -> onEntryOfPass()
            is Test241State.S0 -> onEntryOfS0()
            is Test241State.S01 -> onEntryOfS01()
            is Test241State.S02 -> onEntryOfS02()
            is Test241State.S03 -> onEntryOfS03()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test241.scxml:101 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test241.scxml:100 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test241.scxml:13 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, Test241Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test241.scxml:19 :: s01 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test241State.S01)
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test241.scxml:44 :: s02 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_1", Test241State.S02)
    }

    private fun onEntryOfS03() {
                // SCE-MAP: test241.scxml:71 :: s03 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_2", Test241State.S03)
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
    private fun deferStaticInvoke(invokeId: String, state: Test241State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS01At0(state)
            "_invoke_1" -> startInvokeOfS02At0(state)
            "_invoke_2" -> startInvokeOfS03At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS01At0(state: Test241State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s01.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        if (!(engineInv.hasVariable(sidInv, "Var1"))) {
                            raisePlatformError(Test241Event.Error.Execution, "<invoke> namelist names a location that cannot be read")
                            return@deferInvoke
                        }
                        invokeParams["Var1"] = engineInv.getVariable(sidInv, "Var1")

                    val childSM = adoptChild(Test241SceSynthInvokeInvoke0StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId, "", "test241__sce_synth_invoke__invoke_0", "_invoke_0")
                }
            }
    }

    private fun startInvokeOfS02At0(state: Test241State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s02.${nextInvokeStart("_invoke_1")}._invoke_1"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("1", "1"))
                        } catch (_: Exception) {
                            raisePlatformError(Test241Event.Error.Execution, "<invoke> <param name='Var1'> could not be read")
                        }

                    val childSM = adoptChild(Test241SceSynthInvokeInvoke1StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_1", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId, "", "test241__sce_synth_invoke__invoke_1", "_invoke_1")
                }
            }
    }

    private fun startInvokeOfS03At0(state: Test241State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s03.${nextInvokeStart("_invoke_2")}._invoke_2"
                deferInvoke(state, generatedInvokeId) {
                        // W3C SCXML 6.4: the arguments are evaluated when the <invoke>
                        // is executed — at macrostep end, where this deferred body
                        // runs — not when the state was entered.
                        ensureScriptEngine()
                        val engineInv = scriptEngine ?: error("scriptEngine is required (codegen invariant: needs_script_engine == true)")
                        val sidInv = scriptSessionId ?: error("scriptSessionId must be initialized after ensureScriptEngine() (codegen invariant)")
                        val invokeParams = mutableMapOf<String, Any?>()
                        // §scxml-5.7.1: a `<param>` that will not evaluate costs
                        // `error.execution` AND the name and value — nothing else: the
                        // child still starts. The map insert is inside the `try`, so a
                        // failure leaves the name absent.
                        try {
                            invokeParams["Var1"] = engineInv.evaluateExpr(sidInv, com.sce.runtime.ScriptSource.lua("1", "1"))
                        } catch (_: Exception) {
                            raisePlatformError(Test241Event.Error.Execution, "<invoke> <param name='Var1'> could not be read")
                        }

                    val childSM = adoptChild(Test241SceSynthInvokeInvoke2StateMachine(scriptEngine ?: error("scriptEngine is required for invoke (codegen invariant: parent needs_script_engine == true)")), generatedInvokeId)
                    setInvokeParams(childSM, invokeParams)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_2", childSM, false, Test241Event.Done.Invoke, "", generatedInvokeId, "", "test241__sce_synth_invoke__invoke_2", "_invoke_2")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test241.scxml:8 :: _machine
    override fun onExit(state: Test241State) {
        when (state) {
            is Test241State.Fail -> onExitOfFail()
            is Test241State.Pass -> onExitOfPass()
            is Test241State.S0 -> onExitOfS0()
            is Test241State.S01 -> onExitOfS01()
            is Test241State.S02 -> onExitOfS02()
            is Test241State.S03 -> onExitOfS03()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test241.scxml:101 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test241.scxml:100 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test241.scxml:13 :: s0 :: _state_body
    }

    private fun onExitOfS01() {
                // SCE-MAP: test241.scxml:19 :: s01 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test241State.S01)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS02() {
                // SCE-MAP: test241.scxml:44 :: s02 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test241State.S02)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
    }

    private fun onExitOfS03() {
                // SCE-MAP: test241.scxml:71 :: s03 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test241State.S03)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_2")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test241.scxml:8 :: _machine
    override fun executeTransitionContent(source: Test241State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
