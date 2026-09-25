// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

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
        is Test404State.Fail, is Test404State.Pass -> true
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
        val childStates: Map<Test404State, List<Test404State>> = mapOf(
            Test404State.S0 to listOf(Test404State.S01p, Test404State.S02, Test404State.S03, Test404State.S04, Test404State.S05),
            Test404State.S01p to listOf(Test404State.S01p1, Test404State.S01p2),
        )

        val initialTargets: Map<Test404State, List<EntryTarget<Test404State, HistoryId>>> = mapOf(
            Test404State.S0 to listOf(StateTarget(Test404State.S01p)),
        )

        val documentInitialTargetList: List<EntryTarget<Test404State, HistoryId>> =
            listOf(StateTarget(Test404State.S0))

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






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test404State,
        event: Test404Event?
    ): EnabledTransition<Test404State, HistoryId>? = when (state) {
        is Test404State.S01p -> when {
            event == null -> transitionS01pAt0
            else -> null
        }
        is Test404State.S02 -> when {
            event is Test404Event.Event1 -> transitionS02At0
            event != null -> transitionS02At1
            else -> null
        }
        is Test404State.S03 -> when {
            event is Test404Event.Event2 -> transitionS03At0
            event != null -> transitionS03At1
            else -> null
        }
        is Test404State.S04 -> when {
            event is Test404Event.Event3 -> transitionS04At0
            event != null -> transitionS04At1
            else -> null
        }
        is Test404State.S05 -> when {
            event is Test404Event.Event4 -> transitionS05At0
            event != null -> transitionS05At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun onEntry(state: Test404State, isDefaultEntry: Boolean) {
        when (state) {
            is Test404State.Fail -> {
                // SCE-MAP: test404.scxml:63 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test404State.Pass -> {
                // SCE-MAP: test404.scxml:62 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test404State.S0 -> {
                // SCE-MAP: test404.scxml:10 :: s0 :: _state_body
            }
            is Test404State.S01p -> {
                // SCE-MAP: test404.scxml:14 :: s01p :: _state_body
            }
            is Test404State.S01p1 -> {
                // SCE-MAP: test404.scxml:24 :: s01p1 :: _state_body
            }
            is Test404State.S01p2 -> {
                // SCE-MAP: test404.scxml:31 :: s01p2 :: _state_body
            }
            is Test404State.S02 -> {
                // SCE-MAP: test404.scxml:39 :: s02 :: _state_body
            }
            is Test404State.S03 -> {
                // SCE-MAP: test404.scxml:44 :: s03 :: _state_body
            }
            is Test404State.S04 -> {
                // SCE-MAP: test404.scxml:49 :: s04 :: _state_body
            }
            is Test404State.S05 -> {
                // SCE-MAP: test404.scxml:54 :: s05 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun onExit(state: Test404State) {
        when (state) {
            is Test404State.Fail -> {
                // SCE-MAP: test404.scxml:63 :: fail :: _state_body
            }
            is Test404State.Pass -> {
                // SCE-MAP: test404.scxml:62 :: pass :: _state_body
            }
            is Test404State.S0 -> {
                // SCE-MAP: test404.scxml:10 :: s0 :: _state_body
            }
            is Test404State.S01p -> {
                // SCE-MAP: test404.scxml:14 :: s01p :: _state_body

            raiseInternal(Test404Event.Event3)
            }
            is Test404State.S01p1 -> {
                // SCE-MAP: test404.scxml:24 :: s01p1 :: _state_body

            raiseInternal(Test404Event.Event2)
            }
            is Test404State.S01p2 -> {
                // SCE-MAP: test404.scxml:31 :: s01p2 :: _state_body

            raiseInternal(Test404Event.Event1)
            }
            is Test404State.S02 -> {
                // SCE-MAP: test404.scxml:39 :: s02 :: _state_body
            }
            is Test404State.S03 -> {
                // SCE-MAP: test404.scxml:44 :: s03 :: _state_body
            }
            is Test404State.S04 -> {
                // SCE-MAP: test404.scxml:49 :: s04 :: _state_body
            }
            is Test404State.S05 -> {
                // SCE-MAP: test404.scxml:54 :: s05 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test404.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test404State, transitionIndex: Int) {
        when (source) {
        is Test404State.S01p -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: test404.scxml:19 :: s01p :: _transition_0

            raiseInternal(Test404Event.Event4)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
