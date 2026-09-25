// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/412/test412.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test412.scxml:6 :: _machine

package com.sce.generated.test412

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test412State : State {
    data object Fail : Test412State
    data object Pass : Test412State
    data object S0 : Test412State
    data object S01 : Test412State
    data object S011 : Test412State
    data object S02 : Test412State
    data object S03 : Test412State
    data object S04 : Test412State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test412Event : Event {
    sealed interface Error : Test412Event {
        data object Execution : Error
    }
    data object Event1 : Test412Event
    data object Event2 : Test412Event
    data object Event3 : Test412Event
    data object Timeout : Test412Event
}
// --- State Machine (W3C SCXML) ---

class Test412StateMachine(
) : StateMachineEngine<Test412State, Test412Event>() {

    override val initialState: Test412State = Test412State.S011

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: Test412State): Test412State? = when (state) {
        is Test412State.S01 -> Test412State.S0
        is Test412State.S011 -> Test412State.S01
        is Test412State.S02 -> Test412State.S0
        is Test412State.S03 -> Test412State.S0
        is Test412State.S04 -> Test412State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test412State): Boolean = when (state) {
        is Test412State.S0, is Test412State.S01 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test412State): Boolean = when (state) {
        is Test412State.Fail, is Test412State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test412State): List<Test412State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test412State): List<EntryTarget<Test412State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test412State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test412State, List<Test412State>> = mapOf(
            Test412State.S0 to listOf(Test412State.S01, Test412State.S02, Test412State.S03, Test412State.S04),
            Test412State.S01 to listOf(Test412State.S011),
        )

        val initialTargets: Map<Test412State, List<EntryTarget<Test412State, HistoryId>>> = mapOf(
            Test412State.S0 to listOf(StateTarget(Test412State.S01)),
            Test412State.S01 to listOf(StateTarget(Test412State.S011)),
        )

        val documentInitialTargetList: List<EntryTarget<Test412State, HistoryId>> =
            listOf(StateTarget(Test412State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S0,
            listOf(StateTarget(Test412State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S0,
            listOf(StateTarget(Test412State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 2, as the microstep reads it.
        val transitionS0At2 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S0,
            listOf(StateTarget(Test412State.Pass)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s011's transition 0, as the microstep reads it.
        val transitionS011At0 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S011,
            listOf(StateTarget(Test412State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S02,
            listOf(StateTarget(Test412State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 1, as the microstep reads it.
        val transitionS02At1 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S02,
            listOf(StateTarget(Test412State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S03,
            listOf(StateTarget(Test412State.S04)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S03,
            listOf(StateTarget(Test412State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 0, as the microstep reads it.
        val transitionS04At0 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S04,
            listOf(StateTarget(Test412State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 1, as the microstep reads it.
        val transitionS04At1 = EnabledTransition<Test412State, HistoryId>(
            Test412State.S04,
            listOf(StateTarget(Test412State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test412State? = when (stateId) {
        "fail" -> Test412State.Fail
        "pass" -> Test412State.Pass
        "s0" -> Test412State.S0
        "s01" -> Test412State.S01
        "s011" -> Test412State.S011
        "s02" -> Test412State.S02
        "s03" -> Test412State.S03
        "s04" -> Test412State.S04
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test412State): String = when (state) {
        is Test412State.Fail -> "fail"
        is Test412State.Pass -> "pass"
        is Test412State.S0 -> "s0"
        is Test412State.S01 -> "s01"
        is Test412State.S011 -> "s011"
        is Test412State.S02 -> "s02"
        is Test412State.S03 -> "s03"
        is Test412State.S04 -> "s04"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test412State): Int = when (state) {
        is Test412State.Fail -> 7
        is Test412State.Pass -> 6
        is Test412State.S0 -> 0
        is Test412State.S01 -> 1
        is Test412State.S011 -> 2
        is Test412State.S02 -> 3
        is Test412State.S03 -> 4
        is Test412State.S04 -> 5
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test412State,
        event: Test412Event?
    ): EnabledTransition<Test412State, HistoryId>? = when (state) {
        is Test412State.S0 -> when {
            event is Test412Event.Timeout -> transitionS0At0
            event is Test412Event.Event1 -> transitionS0At1
            event is Test412Event.Event2 -> transitionS0At2
            else -> null
        }
        is Test412State.S011 -> when {
            event == null -> transitionS011At0
            else -> null
        }
        is Test412State.S02 -> when {
            event is Test412Event.Event1 -> transitionS02At0
            event != null -> transitionS02At1
            else -> null
        }
        is Test412State.S03 -> when {
            event is Test412Event.Event2 -> transitionS03At0
            event != null -> transitionS03At1
            else -> null
        }
        is Test412State.S04 -> when {
            event is Test412Event.Event3 -> transitionS04At0
            event != null -> transitionS04At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun onEntry(state: Test412State, isDefaultEntry: Boolean) {
        when (state) {
            is Test412State.Fail -> {
                // SCE-MAP: test412.scxml:54 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.Pass -> {
                // SCE-MAP: test412.scxml:53 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test412State.S0 -> {
                // SCE-MAP: test412.scxml:9 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test412Event.Timeout)
            }
            is Test412State.S01 -> {
                // SCE-MAP: test412.scxml:18 :: s01 :: _state_body

            raiseInternal(Test412Event.Event1)
                // W3C SCXML 3.3: the <initial> transition's content runs when,
                // and only when, this state's initial state is entered by
                // default — not when the state is entered only as the ancestor
                // of a deeper target.
                if (isDefaultEntry) {

            raiseInternal(Test412Event.Event2)
                }
            }
            is Test412State.S011 -> {
                // SCE-MAP: test412.scxml:28 :: s011 :: _state_body

            raiseInternal(Test412Event.Event3)
            }
            is Test412State.S02 -> {
                // SCE-MAP: test412.scxml:36 :: s02 :: _state_body
            }
            is Test412State.S03 -> {
                // SCE-MAP: test412.scxml:41 :: s03 :: _state_body
            }
            is Test412State.S04 -> {
                // SCE-MAP: test412.scxml:46 :: s04 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun onExit(state: Test412State) {
        when (state) {
            is Test412State.Fail -> {
                // SCE-MAP: test412.scxml:54 :: fail :: _state_body
            }
            is Test412State.Pass -> {
                // SCE-MAP: test412.scxml:53 :: pass :: _state_body
            }
            is Test412State.S0 -> {
                // SCE-MAP: test412.scxml:9 :: s0 :: _state_body
            }
            is Test412State.S01 -> {
                // SCE-MAP: test412.scxml:18 :: s01 :: _state_body
            }
            is Test412State.S011 -> {
                // SCE-MAP: test412.scxml:28 :: s011 :: _state_body
            }
            is Test412State.S02 -> {
                // SCE-MAP: test412.scxml:36 :: s02 :: _state_body
            }
            is Test412State.S03 -> {
                // SCE-MAP: test412.scxml:41 :: s03 :: _state_body
            }
            is Test412State.S04 -> {
                // SCE-MAP: test412.scxml:46 :: s04 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test412.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test412State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
