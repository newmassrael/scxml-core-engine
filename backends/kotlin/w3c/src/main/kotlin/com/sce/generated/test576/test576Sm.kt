// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/576/test576.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test576.scxml:6 :: _machine

package com.sce.generated.test576

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test576State : State {
    data object Fail : Test576State
    data object Pass : Test576State
    data object S0 : Test576State
    data object S1 : Test576State
    data object S11 : Test576State
    data object S111 : Test576State
    data object S11p1 : Test576State
    data object S11p11 : Test576State
    data object S11p111 : Test576State
    data object S11p112 : Test576State
    data object S11p12 : Test576State
    data object S11p121 : Test576State
    data object S11p122 : Test576State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test576Event : Event {
    data object InS11p112 : Test576Event
    sealed interface Error : Test576Event {
        data object Execution : Error
    }
    data object Timeout : Test576Event
}
// --- State Machine (W3C SCXML) ---

class Test576StateMachine(
) : StateMachineEngine<Test576State, Test576Event>() {

    override val initialState: Test576State = Test576State.S11p112

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
    override fun parentOf(state: Test576State): Test576State? = when (state) {
        is Test576State.S11 -> Test576State.S1
        is Test576State.S111 -> Test576State.S11
        is Test576State.S11p1 -> Test576State.S11
        is Test576State.S11p11 -> Test576State.S11p1
        is Test576State.S11p111 -> Test576State.S11p11
        is Test576State.S11p112 -> Test576State.S11p11
        is Test576State.S11p12 -> Test576State.S11p1
        is Test576State.S11p121 -> Test576State.S11p12
        is Test576State.S11p122 -> Test576State.S11p12
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test576State): Boolean = when (state) {
        is Test576State.S1 -> true
        is Test576State.S11 -> true
        is Test576State.S11p11 -> true
        is Test576State.S11p12 -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test576State): Boolean = when (state) {
        is Test576State.S11p1 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test576State): Boolean = when (state) {
        is Test576State.Fail -> true
        is Test576State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test576State): List<Test576State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test576State): List<EntryTarget<Test576State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test576State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test576State, List<Test576State>> = LinkedHashMap<Test576State, List<Test576State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test576State, List<Test576State>>) {
            table[Test576State.S1] = listOf(Test576State.S11)
            table[Test576State.S11] = listOf(Test576State.S111, Test576State.S11p1)
            table[Test576State.S11p1] = listOf(Test576State.S11p11, Test576State.S11p12)
            table[Test576State.S11p11] = listOf(Test576State.S11p111, Test576State.S11p112)
            table[Test576State.S11p12] = listOf(Test576State.S11p121, Test576State.S11p122)
        }


        val initialTargets: Map<Test576State, List<EntryTarget<Test576State, HistoryId>>> = LinkedHashMap<Test576State, List<EntryTarget<Test576State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test576State, List<EntryTarget<Test576State, HistoryId>>>) {
            table[Test576State.S1] = listOf(StateTarget(Test576State.S11))
            table[Test576State.S11] = listOf(StateTarget(Test576State.S111))
            table[Test576State.S11p11] = listOf(StateTarget(Test576State.S11p111))
            table[Test576State.S11p12] = listOf(StateTarget(Test576State.S11p121))
        }


        val documentInitialTargetList: List<EntryTarget<Test576State, HistoryId>> =
            listOf(StateTarget(Test576State.S11p112), StateTarget(Test576State.S11p122))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test576State, HistoryId>(
            Test576State.S0,
            listOf(StateTarget(Test576State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test576State, HistoryId>(
            Test576State.S1,
            listOf(StateTarget(Test576State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11p122's transition 0, as the microstep reads it.
        val transitionS11p122At0 = EnabledTransition<Test576State, HistoryId>(
            Test576State.S11p122,
            listOf(StateTarget(Test576State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test576State? = when (stateId) {
        "fail" -> Test576State.Fail
        "pass" -> Test576State.Pass
        "s0" -> Test576State.S0
        "s1" -> Test576State.S1
        "s11" -> Test576State.S11
        "s111" -> Test576State.S111
        "s11p1" -> Test576State.S11p1
        "s11p11" -> Test576State.S11p11
        "s11p111" -> Test576State.S11p111
        "s11p112" -> Test576State.S11p112
        "s11p12" -> Test576State.S11p12
        "s11p121" -> Test576State.S11p121
        "s11p122" -> Test576State.S11p122
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test576State): String = when (state) {
        is Test576State.Fail -> "fail"
        is Test576State.Pass -> "pass"
        is Test576State.S0 -> "s0"
        is Test576State.S1 -> "s1"
        is Test576State.S11 -> "s11"
        is Test576State.S111 -> "s111"
        is Test576State.S11p1 -> "s11p1"
        is Test576State.S11p11 -> "s11p11"
        is Test576State.S11p111 -> "s11p111"
        is Test576State.S11p112 -> "s11p112"
        is Test576State.S11p12 -> "s11p12"
        is Test576State.S11p121 -> "s11p121"
        is Test576State.S11p122 -> "s11p122"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test576State): Int = when (state) {
        is Test576State.Fail -> 12
        is Test576State.Pass -> 11
        is Test576State.S0 -> 0
        is Test576State.S1 -> 1
        is Test576State.S11 -> 2
        is Test576State.S111 -> 3
        is Test576State.S11p1 -> 4
        is Test576State.S11p11 -> 5
        is Test576State.S11p111 -> 6
        is Test576State.S11p112 -> 7
        is Test576State.S11p12 -> 8
        is Test576State.S11p121 -> 9
        is Test576State.S11p122 -> 10
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test576Event? = when (name) {
        "error.execution" -> Test576Event.Error.Execution
        "In-s11p112" -> Test576Event.InS11p112
        "timeout" -> Test576Event.Timeout
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test576Event): String? = when (event) {
        is Test576Event.Error.Execution -> "error.execution"
        is Test576Event.InS11p112 -> "In-s11p112"
        is Test576Event.Timeout -> "timeout"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test576State, event: Test576Event?): EnabledTransition<Test576State, HistoryId>? = when (state) {
        is Test576State.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test576State.S1 -> firstEnabledTransitionOfS1(state, event)
        is Test576State.S11p122 -> firstEnabledTransitionOfS11p122(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test576State, event: Test576Event?): EnabledTransition<Test576State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS0At0
        else -> null
    }


    // s1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS1(state: Test576State, event: Test576Event?): EnabledTransition<Test576State, HistoryId>? = when {
        event is Test576Event.Timeout -> TransitionsPart0.transitionS1At0
        else -> null
    }


    // s11p122's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS11p122(state: Test576State, event: Test576Event?): EnabledTransition<Test576State, HistoryId>? = when {
        event is Test576Event.InS11p112 -> TransitionsPart0.transitionS11p122At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun onEntry(state: Test576State, isDefaultEntry: Boolean) {
        when (state) {
            is Test576State.Fail -> onEntryOfFail()
            is Test576State.Pass -> onEntryOfPass()
            is Test576State.S0 -> onEntryOfS0()
            is Test576State.S1 -> onEntryOfS1()
            is Test576State.S11 -> onEntryOfS11()
            is Test576State.S111 -> onEntryOfS111()
            is Test576State.S11p1 -> onEntryOfS11p1()
            is Test576State.S11p11 -> onEntryOfS11p11()
            is Test576State.S11p111 -> onEntryOfS11p111()
            is Test576State.S11p112 -> onEntryOfS11p112()
            is Test576State.S11p12 -> onEntryOfS11p12()
            is Test576State.S11p121 -> onEntryOfS11p121()
            is Test576State.S11p122 -> onEntryOfS11p122()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test576.scxml:40 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test576.scxml:39 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test576.scxml:9 :: s0 :: _state_body
    }

    private fun onEntryOfS1() {
                // SCE-MAP: test576.scxml:13 :: s1 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 1000L, Test576Event.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS11() {
                // SCE-MAP: test576.scxml:18 :: s11 :: _state_body
    }

    private fun onEntryOfS111() {
                // SCE-MAP: test576.scxml:19 :: s111 :: _state_body
    }

    private fun onEntryOfS11p1() {
                // SCE-MAP: test576.scxml:20 :: s11p1 :: _state_body
    }

    private fun onEntryOfS11p11() {
                // SCE-MAP: test576.scxml:21 :: s11p11 :: _state_body
    }

    private fun onEntryOfS11p111() {
                // SCE-MAP: test576.scxml:22 :: s11p111 :: _state_body
    }

    private fun onEntryOfS11p112() {
                // SCE-MAP: test576.scxml:23 :: s11p112 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(Test576Event.InS11p112)
                }
    }

    private fun onEntryOfS11p12() {
                // SCE-MAP: test576.scxml:29 :: s11p12 :: _state_body
    }

    private fun onEntryOfS11p121() {
                // SCE-MAP: test576.scxml:30 :: s11p121 :: _state_body
    }

    private fun onEntryOfS11p122() {
                // SCE-MAP: test576.scxml:31 :: s11p122 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun onExit(state: Test576State) {
        when (state) {
            is Test576State.Fail -> onExitOfFail()
            is Test576State.Pass -> onExitOfPass()
            is Test576State.S0 -> onExitOfS0()
            is Test576State.S1 -> onExitOfS1()
            is Test576State.S11 -> onExitOfS11()
            is Test576State.S111 -> onExitOfS111()
            is Test576State.S11p1 -> onExitOfS11p1()
            is Test576State.S11p11 -> onExitOfS11p11()
            is Test576State.S11p111 -> onExitOfS11p111()
            is Test576State.S11p112 -> onExitOfS11p112()
            is Test576State.S11p12 -> onExitOfS11p12()
            is Test576State.S11p121 -> onExitOfS11p121()
            is Test576State.S11p122 -> onExitOfS11p122()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test576.scxml:40 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test576.scxml:39 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test576.scxml:9 :: s0 :: _state_body
    }

    private fun onExitOfS1() {
                // SCE-MAP: test576.scxml:13 :: s1 :: _state_body
    }

    private fun onExitOfS11() {
                // SCE-MAP: test576.scxml:18 :: s11 :: _state_body
    }

    private fun onExitOfS111() {
                // SCE-MAP: test576.scxml:19 :: s111 :: _state_body
    }

    private fun onExitOfS11p1() {
                // SCE-MAP: test576.scxml:20 :: s11p1 :: _state_body
    }

    private fun onExitOfS11p11() {
                // SCE-MAP: test576.scxml:21 :: s11p11 :: _state_body
    }

    private fun onExitOfS11p111() {
                // SCE-MAP: test576.scxml:22 :: s11p111 :: _state_body
    }

    private fun onExitOfS11p112() {
                // SCE-MAP: test576.scxml:23 :: s11p112 :: _state_body
    }

    private fun onExitOfS11p12() {
                // SCE-MAP: test576.scxml:29 :: s11p12 :: _state_body
    }

    private fun onExitOfS11p121() {
                // SCE-MAP: test576.scxml:30 :: s11p121 :: _state_body
    }

    private fun onExitOfS11p122() {
                // SCE-MAP: test576.scxml:31 :: s11p122 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test576.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test576State, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
