// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/403/test403a.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test403a.scxml:11 :: _machine

package com.sce.generated.test403a

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test403aState : State {
    data object Fail : Test403aState
    data object Pass : Test403aState
    data object S0 : Test403aState
    data object S01 : Test403aState
    data object S02 : Test403aState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test403aEvent : Event {
    data object Wildcard : Test403aEvent
    sealed interface Error : Test403aEvent {
        data object Execution : Error
    }
    data object Event1 : Test403aEvent
    data object Event2 : Test403aEvent
    data object Timeout : Test403aEvent
}
// --- State Machine (W3C SCXML) ---

class Test403aStateMachine(
) : StateMachineEngine<Test403aState, Test403aEvent>() {

    override val initialState: Test403aState = Test403aState.S01

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
    override fun parentOf(state: Test403aState): Test403aState? = when (state) {
        is Test403aState.S01 -> Test403aState.S0
        is Test403aState.S02 -> Test403aState.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test403aState): Boolean = when (state) {
        is Test403aState.S0 -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test403aState): Boolean = when (state) {
        is Test403aState.Fail -> true
        is Test403aState.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test403aState): List<Test403aState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test403aState): List<EntryTarget<Test403aState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test403aState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test403aState, List<Test403aState>> = LinkedHashMap<Test403aState, List<Test403aState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test403aState, List<Test403aState>>) {
            table[Test403aState.S0] = listOf(Test403aState.S01, Test403aState.S02)
        }


        val initialTargets: Map<Test403aState, List<EntryTarget<Test403aState, HistoryId>>> = LinkedHashMap<Test403aState, List<EntryTarget<Test403aState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test403aState, List<EntryTarget<Test403aState, HistoryId>>>) {
            table[Test403aState.S0] = listOf(StateTarget(Test403aState.S01))
        }


        val documentInitialTargetList: List<EntryTarget<Test403aState, HistoryId>> =
            listOf(StateTarget(Test403aState.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S0,
            listOf(StateTarget(Test403aState.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S0,
            listOf(StateTarget(Test403aState.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S0,
            listOf(StateTarget(Test403aState.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S01,
            listOf(StateTarget(Test403aState.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 1, as the microstep reads it.
        val transitionS01At1 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S01,
            listOf(StateTarget(Test403aState.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S02,
            listOf(StateTarget(Test403aState.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test403aState, HistoryId>(
            Test403aState.S02,
            listOf(StateTarget(Test403aState.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test403aState? = when (stateId) {
        "fail" -> Test403aState.Fail
        "pass" -> Test403aState.Pass
        "s0" -> Test403aState.S0
        "s01" -> Test403aState.S01
        "s02" -> Test403aState.S02
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test403aState): String = when (state) {
        is Test403aState.Fail -> "fail"
        is Test403aState.Pass -> "pass"
        is Test403aState.S0 -> "s0"
        is Test403aState.S01 -> "s01"
        is Test403aState.S02 -> "s02"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test403aState): Int = when (state) {
        is Test403aState.Fail -> 4
        is Test403aState.Pass -> 3
        is Test403aState.S0 -> 0
        is Test403aState.S01 -> 1
        is Test403aState.S02 -> 2
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test403aEvent? = when (name) {
        "error.execution" -> Test403aEvent.Error.Execution
        "event1" -> Test403aEvent.Event1
        "event2" -> Test403aEvent.Event2
        "timeout" -> Test403aEvent.Timeout
        "*" -> Test403aEvent.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test403aEvent): String? = when (event) {
        is Test403aEvent.Error.Execution -> "error.execution"
        is Test403aEvent.Event1 -> "event1"
        is Test403aEvent.Event2 -> "event2"
        is Test403aEvent.Timeout -> "timeout"
        is Test403aEvent.Wildcard -> "*"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test403aState, event: Test403aEvent?): EnabledTransition<Test403aState, HistoryId>? = when (state) {
        is Test403aState.S0 -> firstEnabledTransitionOfS0(state, event)
        is Test403aState.S01 -> firstEnabledTransitionOfS01(state, event)
        is Test403aState.S02 -> firstEnabledTransitionOfS02(state, event)
        else -> null
    }


    // s0's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS0(state: Test403aState, event: Test403aEvent?): EnabledTransition<Test403aState, HistoryId>? = when {
        event is Test403aEvent.Timeout -> TransitionsPart0.transitionS0At0
        event is Test403aEvent.Event1 -> TransitionsPart0.transitionS0At1
        event is Test403aEvent.Event2 -> TransitionsPart0.transitionS0At2
        else -> null
    }


    // s01's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01(state: Test403aState, event: Test403aEvent?): EnabledTransition<Test403aState, HistoryId>? = when {
        event is Test403aEvent.Event1 -> TransitionsPart0.transitionS01At0
        event != null -> TransitionsPart0.transitionS01At1
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test403aState, event: Test403aEvent?): EnabledTransition<Test403aState, HistoryId>? = when {
        event is Test403aEvent.Event1 -> TransitionsPart0.transitionS02At0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test403a.scxml:11 :: _machine
    override fun onEntry(state: Test403aState, isDefaultEntry: Boolean) {
        when (state) {
            is Test403aState.Fail -> onEntryOfFail()
            is Test403aState.Pass -> onEntryOfPass()
            is Test403aState.S0 -> onEntryOfS0()
            is Test403aState.S01 -> onEntryOfS01()
            is Test403aState.S02 -> onEntryOfS02()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test403a.scxml:46 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test403a.scxml:45 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test403a.scxml:14 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 1000L, Test403aEvent.Timeout, EventMetadata.sentBy(sessionId, sendId = "__send_0", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfS01() {
                // SCE-MAP: test403a.scxml:23 :: s01 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(Test403aEvent.Event1)
                }
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test403a.scxml:33 :: s02 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            raiseInternal(Test403aEvent.Event2)
                }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test403a.scxml:11 :: _machine
    override fun onExit(state: Test403aState) {
        when (state) {
            is Test403aState.Fail -> onExitOfFail()
            is Test403aState.Pass -> onExitOfPass()
            is Test403aState.S0 -> onExitOfS0()
            is Test403aState.S01 -> onExitOfS01()
            is Test403aState.S02 -> onExitOfS02()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test403a.scxml:46 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test403a.scxml:45 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test403a.scxml:14 :: s0 :: _state_body
    }

    private fun onExitOfS01() {
                // SCE-MAP: test403a.scxml:23 :: s01 :: _state_body
    }

    private fun onExitOfS02() {
                // SCE-MAP: test403a.scxml:33 :: s02 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test403a.scxml:11 :: _machine
    override fun executeTransitionContent(source: Test403aState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
