// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/347/test347.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test347.scxml:6 :: _machine

package com.sce.generated.test347

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test347State : State {
    data object Fail : Test347State
    data object Pass : Test347State
    data object S0 : Test347State
    data object S01 : Test347State
    data object S02 : Test347State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test347Event : Event {
    data object ChildToParent : Test347Event
    sealed interface Done : Test347Event {
        data object Invoke : Done
    }
    sealed interface Error : Test347Event {
        data object Self : Error
        data object Communication : Error
        data object Execution : Error
    }
    data object ParentToChild : Test347Event
    data object Timeout : Test347Event
}
// --- State Machine (W3C SCXML) ---

class Test347StateMachine(
) : StateMachineEngine<Test347State, Test347Event>() {

    override val initialState: Test347State = Test347State.S01

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
    override fun parentOf(state: Test347State): Test347State? = when (state) {
        is Test347State.S01 -> Test347State.S0
        is Test347State.S02 -> Test347State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test347State): Boolean = when (state) {
        is Test347State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test347State): Boolean = when (state) {
        is Test347State.Fail -> true
        is Test347State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test347State): List<Test347State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test347State): List<EntryTarget<Test347State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test347State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test347State, List<Test347State>> = LinkedHashMap<Test347State, List<Test347State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test347State, List<Test347State>>) {
            table[Test347State.S0] = listOf(Test347State.S01, Test347State.S02)
        }


        val initialTargets: Map<Test347State, List<EntryTarget<Test347State, HistoryId>>> = LinkedHashMap<Test347State, List<EntryTarget<Test347State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test347State, List<EntryTarget<Test347State, HistoryId>>>) {
            table[Test347State.S0] = listOf(StateTarget(Test347State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test347State, HistoryId>> =
            listOf(StateTarget(Test347State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S0,
            listOf(StateTarget(Test347State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S01,
            listOf(StateTarget(Test347State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S02,
            listOf(StateTarget(Test347State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test347State, HistoryId>(
            Test347State.S02,
            listOf(StateTarget(Test347State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test347State? = when (stateId) {
        "fail" -> Test347State.Fail
        "pass" -> Test347State.Pass
        "s0" -> Test347State.S0
        "s01" -> Test347State.S01
        "s02" -> Test347State.S02
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test347State): String = when (state) {
        is Test347State.Fail -> "fail"
        is Test347State.Pass -> "pass"
        is Test347State.S0 -> "s0"
        is Test347State.S01 -> "s01"
        is Test347State.S02 -> "s02"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test347State): Int = when (state) {
        is Test347State.Fail -> 4
        is Test347State.Pass -> 3
        is Test347State.S0 -> 0
        is Test347State.S01 -> 1
        is Test347State.S02 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test347Event? = when (name) {
        "childToParent" -> Test347Event.ChildToParent
        "done.invoke" -> Test347Event.Done.Invoke
        "error" -> Test347Event.Error.Self
        "error.communication" -> Test347Event.Error.Communication
        "error.execution" -> Test347Event.Error.Execution
        "parentToChild" -> Test347Event.ParentToChild
        "timeout" -> Test347Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test347Event): String? = when (event) {
        is Test347Event.ChildToParent -> "childToParent"
        is Test347Event.Done.Invoke -> "done.invoke"
        is Test347Event.Error.Self -> "error"
        is Test347Event.Error.Communication -> "error.communication"
        is Test347Event.Error.Execution -> "error.execution"
        is Test347Event.ParentToChild -> "parentToChild"
        is Test347Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test347State, event: Test347Event?): EnabledTransition<Test347State, HistoryId>? = when (state) {
        is Test347State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test347State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test347State.S02 -> firstEnabledTransitionOfS02(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test347State, event: Test347Event?): EnabledTransition<Test347State, HistoryId>? = when {
        event is Test347Event.Timeout -> TransitionsPart0.transitionS0At0
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test347State, event: Test347Event?): EnabledTransition<Test347State, HistoryId>? = when {
        event is Test347Event.ChildToParent -> TransitionsPart0.transitionS01At0
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test347State, event: Test347Event?): EnabledTransition<Test347State, HistoryId>? = when {
        event is Test347Event.Done.Invoke -> TransitionsPart0.transitionS02At0
        (event is Test347Event.Error || event is Test347Event.Error.Communication || event is Test347Event.Error.Execution) -> TransitionsPart0.transitionS02At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun onEntry(state: Test347State, isDefaultEntry: Boolean) {
        when (state) {
            is Test347State.Fail -> onEntryOfFail()
            is Test347State.Pass -> onEntryOfPass()
            is Test347State.S0 -> onEntryOfS0()
            is Test347State.S01 -> onEntryOfS01()
            is Test347State.S02 -> onEntryOfS02()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test347.scxml:42 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test347.scxml:41 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test347.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 20000L, Test347Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("child", Test347State.S0)
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test347.scxml:28 :: s01 :: _state_body
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test347.scxml:32 :: s02 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("child", "parentToChild", sendData)) {
                raisePlatformError(Test347Event.Error.Communication, "<send target='#_child'> names an invocation that is not running", "__send_1")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
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
    private fun deferStaticInvoke(invokeId: String, state: Test347State) {
        when (invokeId) {
            "child" -> startInvokeOfS0At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS0At0(state: Test347State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s0.${nextInvokeStart("child")}.child"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test347SceSynthInvokeChildStateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("child", childSM, false, Test347Event.Done.Invoke, "", generatedInvokeId, "", "test347__sce_synth_invoke__child", "child")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun onExit(state: Test347State) {
        when (state) {
            is Test347State.Fail -> onExitOfFail()
            is Test347State.Pass -> onExitOfPass()
            is Test347State.S0 -> onExitOfS0()
            is Test347State.S01 -> onExitOfS01()
            is Test347State.S02 -> onExitOfS02()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test347.scxml:42 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test347.scxml:41 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test347.scxml:9 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test347State.S0)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("child")
    }

    private fun onExitOfS01() {
                // SCE-MAP: test347.scxml:28 :: s01 :: _state_body
    }

    private fun onExitOfS02() {
                // SCE-MAP: test347.scxml:32 :: s02 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test347.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test347State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
