// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/239/test239.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test239.scxml:5 :: _machine

package com.sce.generated.test239

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test239State : State {
    data object Fail : Test239State
    data object Pass : Test239State
    data object S0 : Test239State
    data object S01 : Test239State
    data object S02 : Test239State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test239Event : Event {
    sealed interface Done : Test239Event {
        data object Invoke : Done
    }
    sealed interface Error : Test239Event {
        data object Execution : Error
    }
    data object Timeout : Test239Event
}
// --- State Machine (W3C SCXML) ---

class Test239StateMachine(
) : StateMachineEngine<Test239State, Test239Event>() {

    override val initialState: Test239State = Test239State.S01

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
    override fun parentOf(state: Test239State): Test239State? = when (state) {
        is Test239State.S01 -> Test239State.S0
        is Test239State.S02 -> Test239State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test239State): Boolean = when (state) {
        is Test239State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test239State): Boolean = when (state) {
        is Test239State.Fail -> true
        is Test239State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test239State): List<Test239State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test239State): List<EntryTarget<Test239State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test239State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test239State, List<Test239State>> = LinkedHashMap<Test239State, List<Test239State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test239State, List<Test239State>>) {
            table[Test239State.S0] = listOf(Test239State.S01, Test239State.S02)
        }


        val initialTargets: Map<Test239State, List<EntryTarget<Test239State, HistoryId>>> = LinkedHashMap<Test239State, List<EntryTarget<Test239State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test239State, List<EntryTarget<Test239State, HistoryId>>>) {
            table[Test239State.S0] = listOf(StateTarget(Test239State.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test239State, HistoryId>> =
            listOf(StateTarget(Test239State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test239State, HistoryId>(
            Test239State.S0,
            listOf(StateTarget(Test239State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test239State, HistoryId>(
            Test239State.S01,
            listOf(StateTarget(Test239State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test239State, HistoryId>(
            Test239State.S02,
            listOf(StateTarget(Test239State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test239State? = when (stateId) {
        "fail" -> Test239State.Fail
        "pass" -> Test239State.Pass
        "s0" -> Test239State.S0
        "s01" -> Test239State.S01
        "s02" -> Test239State.S02
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test239State): String = when (state) {
        is Test239State.Fail -> "fail"
        is Test239State.Pass -> "pass"
        is Test239State.S0 -> "s0"
        is Test239State.S01 -> "s01"
        is Test239State.S02 -> "s02"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test239State): Int = when (state) {
        is Test239State.Fail -> 4
        is Test239State.Pass -> 3
        is Test239State.S0 -> 0
        is Test239State.S01 -> 1
        is Test239State.S02 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test239Event? = when (name) {
        "done.invoke" -> Test239Event.Done.Invoke
        "error.execution" -> Test239Event.Error.Execution
        "timeout" -> Test239Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test239Event): String? = when (event) {
        is Test239Event.Done.Invoke -> "done.invoke"
        is Test239Event.Error.Execution -> "error.execution"
        is Test239Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test239State, event: Test239Event?): EnabledTransition<Test239State, HistoryId>? = when (state) {
        is Test239State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test239State.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test239State.S02 -> firstEnabledTransitionOfS02(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test239State, event: Test239Event?): EnabledTransition<Test239State, HistoryId>? = when {
        event is Test239Event.Timeout -> TransitionsPart0.transitionS0At0
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test239State, event: Test239Event?): EnabledTransition<Test239State, HistoryId>? = when {
        event is Test239Event.Done.Invoke -> TransitionsPart0.transitionS01At0
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test239State, event: Test239Event?): EnabledTransition<Test239State, HistoryId>? = when {
        event is Test239Event.Done.Invoke -> TransitionsPart0.transitionS02At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test239.scxml:5 :: _machine
    override fun onEntry(state: Test239State, isDefaultEntry: Boolean) {
        when (state) {
            is Test239State.Fail -> onEntryOfFail()
            is Test239State.Pass -> onEntryOfPass()
            is Test239State.S0 -> onEntryOfS0()
            is Test239State.S01 -> onEntryOfS01()
            is Test239State.S02 -> onEntryOfS02()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test239.scxml:35 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test239.scxml:34 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test239.scxml:8 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, Test239Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test239.scxml:14 :: s01 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_0", Test239State.S01)
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test239.scxml:19 :: s02 :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("_invoke_1", Test239State.S02)
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
    private fun deferStaticInvoke(invokeId: String, state: Test239State) {
        when (invokeId) {
            "_invoke_0" -> startInvokeOfS01At0(state)
            "_invoke_1" -> startInvokeOfS02At0(state)
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }


    private fun startInvokeOfS01At0(state: Test239State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s01.${nextInvokeStart("_invoke_0")}._invoke_0"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test239sub1StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_0", childSM, false, Test239Event.Done.Invoke, "", generatedInvokeId, "test239sub1", "test239sub1", "_invoke_0")
                }
            }
    }

    private fun startInvokeOfS02At0(state: Test239State) {
            run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format;
                // the platform part is the number of this start (docs/adr/0045).
                val generatedInvokeId = "s02.${nextInvokeStart("_invoke_1")}._invoke_1"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = adoptChild(Test239SceSynthInvokeInvoke1StateMachine(), generatedInvokeId)
                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("_invoke_1", childSM, false, Test239Event.Done.Invoke, "", generatedInvokeId, "", "test239__sce_synth_invoke__invoke_1", "_invoke_1")
                }
            }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test239.scxml:5 :: _machine
    override fun onExit(state: Test239State) {
        when (state) {
            is Test239State.Fail -> onExitOfFail()
            is Test239State.Pass -> onExitOfPass()
            is Test239State.S0 -> onExitOfS0()
            is Test239State.S01 -> onExitOfS01()
            is Test239State.S02 -> onExitOfS02()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test239.scxml:35 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test239.scxml:34 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test239.scxml:8 :: s0 :: _state_body
    }

    private fun onExitOfS01() {
                // SCE-MAP: test239.scxml:14 :: s01 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test239State.S01)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
    }

    private fun onExitOfS02() {
                // SCE-MAP: test239.scxml:19 :: s02 :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(Test239State.S02)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_1")
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test239.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test239State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
