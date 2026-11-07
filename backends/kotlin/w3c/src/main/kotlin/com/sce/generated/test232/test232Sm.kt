// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/232/test232.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test232.scxml:5 :: _machine

package com.sce.generated.test232

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test232State : State {
    data object Fail : Test232State
    data object Pass : Test232State
    data object S0 : Test232State
    data object S01 : Test232State
    data object S02 : Test232State
    data object S03 : Test232State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232Event : Event {
    data object ChildToParent1 : Test232Event
    data object ChildToParent2 : Test232Event
    sealed interface Done : Test232Event {
        data object Invoke : Done
    }
    sealed interface Error : Test232Event {
        data object Execution : Error
    }
    data object Timeout : Test232Event
}
// --- State Machine (W3C SCXML) ---

class Test232StateMachine(
) : StateMachineEngine<Test232State, Test232Event>() {

    override val initialState: Test232State = Test232State.S01

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
    override fun parentOf(state: Test232State): Test232State? = when (state) {
        is Test232State.S01 -> Test232State.S0
        is Test232State.S02 -> Test232State.S0
        is Test232State.S03 -> Test232State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test232State): Boolean = when (state) {
        is Test232State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test232State): Boolean = when (state) {
        is Test232State.Fail -> true
        is Test232State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test232State): List<Test232State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test232State): List<EntryTarget<Test232State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test232State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test232State, List<Test232State>> = LinkedHashMap<Test232State, List<Test232State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test232State, List<Test232State>>) {
            table[Test232State.S0] = listOf(Test232State.S01, Test232State.S02, Test232State.S03)
        }


        val initialTargets: Map<Test232State, List<EntryTarget<Test232State, HistoryId>>> = LinkedHashMap<Test232State, List<EntryTarget<Test232State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test232State, List<EntryTarget<Test232State, HistoryId>>>) {
            table[Test232State.S0] = listOf(StateTarget(Test232State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test232State, HistoryId>> =
            listOf(StateTarget(Test232State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S0,
            listOf(StateTarget(Test232State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S01,
            listOf(StateTarget(Test232State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S02,
            listOf(StateTarget(Test232State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test232State, HistoryId>(
            Test232State.S03,
            listOf(StateTarget(Test232State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test232State? = when (stateId) {
        "fail" -> Test232State.Fail
        "pass" -> Test232State.Pass
        "s0" -> Test232State.S0
        "s01" -> Test232State.S01
        "s02" -> Test232State.S02
        "s03" -> Test232State.S03
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test232State): String = when (state) {
        is Test232State.Fail -> "fail"
        is Test232State.Pass -> "pass"
        is Test232State.S0 -> "s0"
        is Test232State.S01 -> "s01"
        is Test232State.S02 -> "s02"
        is Test232State.S03 -> "s03"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test232State): Int = when (state) {
        is Test232State.Fail -> 5
        is Test232State.Pass -> 4
        is Test232State.S0 -> 0
        is Test232State.S01 -> 1
        is Test232State.S02 -> 2
        is Test232State.S03 -> 3
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232Event? = when (name) {
        "childToParent1" -> Test232Event.ChildToParent1
        "childToParent2" -> Test232Event.ChildToParent2
        "done.invoke" -> Test232Event.Done.Invoke
        "error.execution" -> Test232Event.Error.Execution
        "timeout" -> Test232Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232Event): String? = when (event) {
        is Test232Event.ChildToParent1 -> "childToParent1"
        is Test232Event.ChildToParent2 -> "childToParent2"
        is Test232Event.Done.Invoke -> "done.invoke"
        is Test232Event.Error.Execution -> "error.execution"
        is Test232Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test232State, event: Test232Event?): EnabledTransition<Test232State, HistoryId>? = when (state) {
        is Test232State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test232State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test232State.S02 -> firstEnabledTransitionOfS02(state, event)
        is Test232State.S03 -> firstEnabledTransitionOfS03(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test232State, event: Test232Event?): EnabledTransition<Test232State, HistoryId>? = when {
        event is Test232Event.Timeout -> TransitionsPart0.transitionS0At0
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test232State, event: Test232Event?): EnabledTransition<Test232State, HistoryId>? = when {
        event is Test232Event.ChildToParent1 -> TransitionsPart0.transitionS01At0
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test232State, event: Test232Event?): EnabledTransition<Test232State, HistoryId>? = when {
        event is Test232Event.ChildToParent2 -> TransitionsPart0.transitionS02At0
        else -> null
    }


    // s03's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS03(state: Test232State, event: Test232Event?): EnabledTransition<Test232State, HistoryId>? = when {
        event is Test232Event.Done.Invoke -> TransitionsPart0.transitionS03At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun onEntry(state: Test232State, isDefaultEntry: Boolean) {
        when (state) {
            is Test232State.Fail -> onEntryOfFail()
            is Test232State.Pass -> onEntryOfPass()
            is Test232State.S0 -> onEntryOfS0()
            is Test232State.S01 -> onEntryOfS01()
            is Test232State.S02 -> onEntryOfS02()
            is Test232State.S03 -> onEntryOfS03()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test232.scxml:42 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test232.scxml:41 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test232.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 3000L, Test232Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test232State.S0)
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test232.scxml:27 :: s01 :: _state_body
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test232.scxml:31 :: s02 :: _state_body
    }

    private fun onEntryOfS03() {
                // SCE-MAP: test232.scxml:35 :: s03 :: _state_body
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
    private fun deferStaticInvoke(invokeId: String, state: Test232State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS0At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS0At0(state: Test232State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s0.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test232SceSynthInvokeInvoke0StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test232Event.Done.Invoke, "", generatedInvokeId, "", "test232__sce_synth_invoke__invoke_0", "_invoke_0")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun onExit(state: Test232State) {
        when (state) {
            is Test232State.Fail -> onExitOfFail()
            is Test232State.Pass -> onExitOfPass()
            is Test232State.S0 -> onExitOfS0()
            is Test232State.S01 -> onExitOfS01()
            is Test232State.S02 -> onExitOfS02()
            is Test232State.S03 -> onExitOfS03()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test232.scxml:42 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test232.scxml:41 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test232.scxml:8 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test232State.S0)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS01() {
                // SCE-MAP: test232.scxml:27 :: s01 :: _state_body
    }

    private fun onExitOfS02() {
                // SCE-MAP: test232.scxml:31 :: s02 :: _state_body
    }

    private fun onExitOfS03() {
                // SCE-MAP: test232.scxml:35 :: s03 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test232.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test232State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
