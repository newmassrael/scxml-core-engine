// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/207/test207.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test207.scxml:8 :: _machine

package com.sce.generated.test207

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test207State : State {
    data object Fail : Test207State
    data object Pass : Test207State
    data object S0 : Test207State
    data object S01 : Test207State
    data object S02 : Test207State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test207Event : Event {
    data object ChildToParent : Test207Event
    sealed interface Done : Test207Event {
        data object Invoke : Done
    }
    sealed interface Error : Test207Event {
        data object Execution : Error
    }
    data object Fail : Test207Event
    data object Pass : Test207Event
    data object Timeout : Test207Event
}
// --- State Machine (W3C SCXML) ---

class Test207StateMachine(
) : StateMachineEngine<Test207State, Test207Event>() {

    override val initialState: Test207State = Test207State.S01

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
    override fun parentOf(state: Test207State): Test207State? = when (state) {
        is Test207State.S01 -> Test207State.S0
        is Test207State.S02 -> Test207State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test207State): Boolean = when (state) {
        is Test207State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test207State): Boolean = when (state) {
        is Test207State.Fail -> true
        is Test207State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test207State): List<Test207State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test207State): List<EntryTarget<Test207State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test207State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test207State, List<Test207State>> = LinkedHashMap<Test207State, List<Test207State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test207State, List<Test207State>>) {
            table[Test207State.S0] = listOf(Test207State.S01, Test207State.S02)
        }


        val initialTargets: Map<Test207State, List<EntryTarget<Test207State, HistoryId>>> = LinkedHashMap<Test207State, List<EntryTarget<Test207State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test207State, List<EntryTarget<Test207State, HistoryId>>>) {
            table[Test207State.S0] = listOf(StateTarget(Test207State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test207State, HistoryId>> =
            listOf(StateTarget(Test207State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test207State, HistoryId>(
            Test207State.S01,
            listOf(StateTarget(Test207State.S02)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test207State, HistoryId>(
            Test207State.S02,
            listOf(StateTarget(Test207State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test207State, HistoryId>(
            Test207State.S02,
            listOf(StateTarget(Test207State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 2, as the microstep reads it.
        val transitionS02At2 = EnabledTransition<Test207State, HistoryId>(
            Test207State.S02,
            listOf(StateTarget(Test207State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test207State? = when (stateId) {
        "fail" -> Test207State.Fail
        "pass" -> Test207State.Pass
        "s0" -> Test207State.S0
        "s01" -> Test207State.S01
        "s02" -> Test207State.S02
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test207State): String = when (state) {
        is Test207State.Fail -> "fail"
        is Test207State.Pass -> "pass"
        is Test207State.S0 -> "s0"
        is Test207State.S01 -> "s01"
        is Test207State.S02 -> "s02"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test207State): Int = when (state) {
        is Test207State.Fail -> 4
        is Test207State.Pass -> 3
        is Test207State.S0 -> 0
        is Test207State.S01 -> 1
        is Test207State.S02 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test207Event? = when (name) {
        "childToParent" -> Test207Event.ChildToParent
        "done.invoke" -> Test207Event.Done.Invoke
        "error.execution" -> Test207Event.Error.Execution
        "fail" -> Test207Event.Fail
        "pass" -> Test207Event.Pass
        "timeout" -> Test207Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test207Event): String? = when (event) {
        is Test207Event.ChildToParent -> "childToParent"
        is Test207Event.Done.Invoke -> "done.invoke"
        is Test207Event.Error.Execution -> "error.execution"
        is Test207Event.Fail -> "fail"
        is Test207Event.Pass -> "pass"
        is Test207Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test207State, event: Test207Event?): EnabledTransition<Test207State, HistoryId>? = when (state) {
        is Test207State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test207State.S02 -> firstEnabledTransitionOfS02(state, event)
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test207State, event: Test207Event?): EnabledTransition<Test207State, HistoryId>? = when {
        event is Test207Event.ChildToParent -> TransitionsPart0.transitionS01At0
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test207State, event: Test207Event?): EnabledTransition<Test207State, HistoryId>? = when {
        event is Test207Event.Pass -> TransitionsPart0.transitionS02At0
        event is Test207Event.Fail -> TransitionsPart0.transitionS02At1
        event is Test207Event.Timeout -> TransitionsPart0.transitionS02At2
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun onEntry(state: Test207State, isDefaultEntry: Boolean) {
        when (state) {
            is Test207State.Fail -> onEntryOfFail()
            is Test207State.Pass -> onEntryOfPass()
            is Test207State.S0 -> onEntryOfS0()
            is Test207State.S01 -> onEntryOfS01()
            is Test207State.S02 -> onEntryOfS02()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test207.scxml:58 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test207.scxml:57 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test207.scxml:11 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, Test207Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test207State.S0)
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test207.scxml:43 :: s01 :: _state_body
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test207.scxml:49 :: s02 :: _state_body
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
    private fun deferStaticInvoke(invokeId: String, state: Test207State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS0At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS0At0(state: Test207State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s0.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test207SceSynthInvokeInvoke0StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test207Event.Done.Invoke, "", generatedInvokeId, "", "test207__sce_synth_invoke__invoke_0", "_invoke_0")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun onExit(state: Test207State) {
        when (state) {
            is Test207State.Fail -> onExitOfFail()
            is Test207State.Pass -> onExitOfPass()
            is Test207State.S0 -> onExitOfS0()
            is Test207State.S01 -> onExitOfS01()
            is Test207State.S02 -> onExitOfS02()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test207.scxml:58 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test207.scxml:57 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test207.scxml:11 :: s0 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test207State.S0)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS01() {
                // SCE-MAP: test207.scxml:43 :: s01 :: _state_body
    }

    private fun onExitOfS02() {
                // SCE-MAP: test207.scxml:49 :: s02 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test207.scxml:8 :: _machine
    override fun executeTransitionContent(source: Test207State, transitionIndex: Int) {
        when (source) {
            is Test207State.S01 -> transitionContentOfS01(transitionIndex)
            else -> {}
        }
    }


    // The content of `s01`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfS01(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfS01At0()
            else -> {}
        }
    }


    private fun transitionContentOfS01At0() {
                // SCE-MAP: test207.scxml:44 :: s01 :: _transition_0


            cancelSend("foo")
    }
}
