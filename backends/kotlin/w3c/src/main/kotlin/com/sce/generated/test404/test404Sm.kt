// SCE-GENERATED — DO NOT EDIT
// source-hash: 7f418d70ffbf9e34303ec5b694ace0e8ad8138948007a395fa2c2a2380179efc

// GENERATED CODE — DO NOT EDIT
// Source: resources/404/test404.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test404.scxml:7 :: _machine

package com.sce.generated.test404

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test404State : State {
    data object Fail : Test404State
    data object Pass : Test404State
    data object S0 : Test404State
    data object S01p : Test404State
    data object S01p1 : Test404State
    data object S01p2 : Test404State
    data object S02 : Test404State
    data object S03 : Test404State
    data object S04 : Test404State
    data object S05 : Test404State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test404Event : Event {
    data object Wildcard : Test404Event
    data object Event1 : Test404Event
    data object Event2 : Test404Event
    data object Event3 : Test404Event
    data object Event4 : Test404Event
}
// --- State Machine (W3C SCXML) ---

class Test404StateMachine(
) : StateMachineEngine<Test404State, Test404Event>() {

    override val initialState: Test404State = Test404State.S01p1

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
    override fun parentOf(state: Test404State): Test404State? = when (state) {
        is Test404State.S01p -> Test404State.S0
        is Test404State.S01p1 -> Test404State.S01p
        is Test404State.S01p2 -> Test404State.S01p
        is Test404State.S02 -> Test404State.S0
        is Test404State.S03 -> Test404State.S0
        is Test404State.S04 -> Test404State.S0
        is Test404State.S05 -> Test404State.S0
        else -> null
    }


    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test404State): Boolean = when (state) {
        is Test404State.S0 -> true
        else -> false
    }


    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test404State): Boolean = when (state) {
        is Test404State.S01p -> true
        else -> false
    }


    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test404State): Boolean = when (state) {
        is Test404State.Fail -> true
        is Test404State.Pass -> true
        else -> false
    }


    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test404State): List<Test404State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test404State): List<EntryTarget<Test404State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test404State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test404State, List<Test404State>> = LinkedHashMap<Test404State, List<Test404State>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<Test404State, List<Test404State>>) {
            table[Test404State.S0] = listOf(Test404State.S01p, Test404State.S02, Test404State.S03, Test404State.S04, Test404State.S05)
            table[Test404State.S01p] = listOf(Test404State.S01p1, Test404State.S01p2)
        }


        val initialTargets: Map<Test404State, List<EntryTarget<Test404State, HistoryId>>> = LinkedHashMap<Test404State, List<EntryTarget<Test404State, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<Test404State, List<EntryTarget<Test404State, HistoryId>>>) {
            table[Test404State.S0] = listOf(StateTarget(Test404State.S01p))
        }


        val documentInitialTargetList: List<EntryTarget<Test404State, HistoryId>> =
            listOf(StateTarget(Test404State.S0))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: s01p's transition 0, as the microstep reads it.
        val transitionS01pAt0 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S01p,
            listOf(StateTarget(Test404State.S02)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S02,
            listOf(StateTarget(Test404State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S02,
            listOf(StateTarget(Test404State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S03,
            listOf(StateTarget(Test404State.S04)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S03,
            listOf(StateTarget(Test404State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 0, as the microstep reads it.
        val transitionS04At0 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S04,
            listOf(StateTarget(Test404State.S05)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 1, as the microstep reads it.
        val transitionS04At1 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S04,
            listOf(StateTarget(Test404State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s05's transition 0, as the microstep reads it.
        val transitionS05At0 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S05,
            listOf(StateTarget(Test404State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s05's transition 1, as the microstep reads it.
        val transitionS05At1 = EnabledTransition<Test404State, HistoryId>(
            Test404State.S05,
            listOf(StateTarget(Test404State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test404State? = when (stateId) {
        "fail" -> Test404State.Fail
        "pass" -> Test404State.Pass
        "s0" -> Test404State.S0
        "s01p" -> Test404State.S01p
        "s01p1" -> Test404State.S01p1
        "s01p2" -> Test404State.S01p2
        "s02" -> Test404State.S02
        "s03" -> Test404State.S03
        "s04" -> Test404State.S04
        "s05" -> Test404State.S05
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test404State): String = when (state) {
        is Test404State.Fail -> "fail"
        is Test404State.Pass -> "pass"
        is Test404State.S0 -> "s0"
        is Test404State.S01p -> "s01p"
        is Test404State.S01p1 -> "s01p1"
        is Test404State.S01p2 -> "s01p2"
        is Test404State.S02 -> "s02"
        is Test404State.S03 -> "s03"
        is Test404State.S04 -> "s04"
        is Test404State.S05 -> "s05"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test404State): Int = when (state) {
        is Test404State.Fail -> 9
        is Test404State.Pass -> 8
        is Test404State.S0 -> 0
        is Test404State.S01p -> 1
        is Test404State.S01p1 -> 2
        is Test404State.S01p2 -> 3
        is Test404State.S02 -> 4
        is Test404State.S03 -> 5
        is Test404State.S04 -> 6
        is Test404State.S05 -> 7
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test404Event? = when (name) {
        "event1" -> Test404Event.Event1
        "event2" -> Test404Event.Event2
        "event3" -> Test404Event.Event3
        "event4" -> Test404Event.Event4
        "*" -> Test404Event.Wildcard
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test404Event): String? = when (event) {
        is Test404Event.Event1 -> "event1"
        is Test404Event.Event2 -> "event2"
        is Test404Event.Event3 -> "event3"
        is Test404Event.Event4 -> "event4"
        is Test404Event.Wildcard -> "*"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when (state) {
        is Test404State.S01p -> firstEnabledTransitionOfS01p(state, event)
        is Test404State.S02 -> firstEnabledTransitionOfS02(state, event)
        is Test404State.S03 -> firstEnabledTransitionOfS03(state, event)
        is Test404State.S04 -> firstEnabledTransitionOfS04(state, event)
        is Test404State.S05 -> firstEnabledTransitionOfS05(state, event)
        else -> null
    }


    // s01p's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS01p(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionS01pAt0
        else -> null
    }


    // s02's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS02(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when {
        event is Test404Event.Event1 -> TransitionsPart0.transitionS02At0
        event != null -> TransitionsPart0.transitionS02At1
        else -> null
    }


    // s03's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS03(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when {
        event is Test404Event.Event2 -> TransitionsPart0.transitionS03At0
        event != null -> TransitionsPart0.transitionS03At1
        else -> null
    }


    // s04's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS04(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when {
        event is Test404Event.Event3 -> TransitionsPart0.transitionS04At0
        event != null -> TransitionsPart0.transitionS04At1
        else -> null
    }


    // s05's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfS05(state: Test404State, event: Test404Event?): EnabledTransition<Test404State, HistoryId>? = when {
        event is Test404Event.Event4 -> TransitionsPart0.transitionS05At0
        event != null -> TransitionsPart0.transitionS05At1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun onEntry(state: Test404State, isDefaultEntry: Boolean) {
        when (state) {
            is Test404State.Fail -> onEntryOfFail()
            is Test404State.Pass -> onEntryOfPass()
            is Test404State.S0 -> onEntryOfS0()
            is Test404State.S01p -> onEntryOfS01p()
            is Test404State.S01p1 -> onEntryOfS01p1()
            is Test404State.S01p2 -> onEntryOfS01p2()
            is Test404State.S02 -> onEntryOfS02()
            is Test404State.S03 -> onEntryOfS03()
            is Test404State.S04 -> onEntryOfS04()
            is Test404State.S05 -> onEntryOfS05()
        }
    }


    private fun onEntryOfFail() {
                // SCE-MAP: test404.scxml:63 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfPass() {
                // SCE-MAP: test404.scxml:62 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    private fun onEntryOfS0() {
                // SCE-MAP: test404.scxml:10 :: s0 :: _state_body
    }

    private fun onEntryOfS01p() {
                // SCE-MAP: test404.scxml:14 :: s01p :: _state_body
    }

    private fun onEntryOfS01p1() {
                // SCE-MAP: test404.scxml:24 :: s01p1 :: _state_body
    }

    private fun onEntryOfS01p2() {
                // SCE-MAP: test404.scxml:31 :: s01p2 :: _state_body
    }

    private fun onEntryOfS02() {
                // SCE-MAP: test404.scxml:39 :: s02 :: _state_body
    }

    private fun onEntryOfS03() {
                // SCE-MAP: test404.scxml:44 :: s03 :: _state_body
    }

    private fun onEntryOfS04() {
                // SCE-MAP: test404.scxml:49 :: s04 :: _state_body
    }

    private fun onEntryOfS05() {
                // SCE-MAP: test404.scxml:54 :: s05 :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun onExit(state: Test404State) {
        when (state) {
            is Test404State.Fail -> onExitOfFail()
            is Test404State.Pass -> onExitOfPass()
            is Test404State.S0 -> onExitOfS0()
            is Test404State.S01p -> onExitOfS01p()
            is Test404State.S01p1 -> onExitOfS01p1()
            is Test404State.S01p2 -> onExitOfS01p2()
            is Test404State.S02 -> onExitOfS02()
            is Test404State.S03 -> onExitOfS03()
            is Test404State.S04 -> onExitOfS04()
            is Test404State.S05 -> onExitOfS05()
        }
    }


    private fun onExitOfFail() {
                // SCE-MAP: test404.scxml:63 :: fail :: _state_body
    }

    private fun onExitOfPass() {
                // SCE-MAP: test404.scxml:62 :: pass :: _state_body
    }

    private fun onExitOfS0() {
                // SCE-MAP: test404.scxml:10 :: s0 :: _state_body
    }

    private fun onExitOfS01p() {
                // SCE-MAP: test404.scxml:14 :: s01p :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            raiseInternal(Test404Event.Event3)
                }
    }

    private fun onExitOfS01p1() {
                // SCE-MAP: test404.scxml:24 :: s01p1 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            raiseInternal(Test404Event.Event2)
                }
    }

    private fun onExitOfS01p2() {
                // SCE-MAP: test404.scxml:31 :: s01p2 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/1
                run {

            raiseInternal(Test404Event.Event1)
                }
    }

    private fun onExitOfS02() {
                // SCE-MAP: test404.scxml:39 :: s02 :: _state_body
    }

    private fun onExitOfS03() {
                // SCE-MAP: test404.scxml:44 :: s03 :: _state_body
    }

    private fun onExitOfS04() {
                // SCE-MAP: test404.scxml:49 :: s04 :: _state_body
    }

    private fun onExitOfS05() {
                // SCE-MAP: test404.scxml:54 :: s05 :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test404State, transitionIndex: Int) {
        when (source) {
            is Test404State.S01p -> transitionContentOfS01p(transitionIndex)
            else -> {}
        }
    }


    // The content of `s01p`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfS01p(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfS01pAt0()
            else -> {}
        }
    }


    private fun transitionContentOfS01pAt0() {
                // SCE-MAP: test404.scxml:19 :: s01p :: _transition_0

            raiseInternal(Test404Event.Event4)
    }
}
