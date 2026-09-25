// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/406/test406.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test406.scxml:6 :: _machine

package com.sce.generated.test406

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test406State : State {
    data object Fail : Test406State
    data object Pass : Test406State
    data object S0 : Test406State
    data object S01 : Test406State
    data object S01p21 : Test406State
    data object S01p22 : Test406State
    data object S03 : Test406State
    data object S04 : Test406State
    data object S05 : Test406State
    data object S0p2 : Test406State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test406Event : Event {
    sealed interface Error : Test406Event {
        data object Execution : Error
    }
    data object Event1 : Test406Event
    data object Event2 : Test406Event
    data object Event3 : Test406Event
    data object Event4 : Test406Event
    data object Timeout : Test406Event
}
// --- State Machine (W3C SCXML) ---

class Test406StateMachine(
) : StateMachineEngine<Test406State, Test406Event>() {

    override val initialState: Test406State = Test406State.S01

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
    override fun parentOf(state: Test406State): Test406State? = when (state) {
        is Test406State.S01 -> Test406State.S0
        is Test406State.S01p21 -> Test406State.S0p2
        is Test406State.S01p22 -> Test406State.S0p2
        is Test406State.S03 -> Test406State.S0
        is Test406State.S04 -> Test406State.S0
        is Test406State.S05 -> Test406State.S0
        is Test406State.S0p2 -> Test406State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test406State): Boolean = when (state) {
        is Test406State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test406State): Boolean = when (state) {
        is Test406State.S0p2 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test406State): Boolean = when (state) {
        is Test406State.Fail, is Test406State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test406State): List<Test406State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test406State): List<EntryTarget<Test406State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test406State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test406State, List<Test406State>> = mapOf(
            Test406State.S0 to listOf(Test406State.S01, Test406State.S0p2, Test406State.S03, Test406State.S04, Test406State.S05),
            Test406State.S0p2 to listOf(Test406State.S01p21, Test406State.S01p22),
        )

        val initialTargets: Map<Test406State, List<EntryTarget<Test406State, HistoryId>>> = mapOf(
            Test406State.S0 to listOf(StateTarget(Test406State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test406State, HistoryId>> =
            listOf(StateTarget(Test406State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S0,
            listOf(StateTarget(Test406State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S01,
            listOf(StateTarget(Test406State.S0p2)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S03,
            listOf(StateTarget(Test406State.S04)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 1, as the microstep reads it.
        val transitionS03At1 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S03,
            listOf(StateTarget(Test406State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 0, as the microstep reads it.
        val transitionS04At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S04,
            listOf(StateTarget(Test406State.S05)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 1, as the microstep reads it.
        val transitionS04At1 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S04,
            listOf(StateTarget(Test406State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s05's transition 0, as the microstep reads it.
        val transitionS05At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S05,
            listOf(StateTarget(Test406State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s05's transition 1, as the microstep reads it.
        val transitionS05At1 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S05,
            listOf(StateTarget(Test406State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0p2's transition 0, as the microstep reads it.
        val transitionS0p2At0 = EnabledTransition<Test406State, HistoryId>(
            Test406State.S0p2,
            listOf(StateTarget(Test406State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test406State? = when (stateId) {
        "fail" -> Test406State.Fail
        "pass" -> Test406State.Pass
        "s0" -> Test406State.S0
        "s01" -> Test406State.S01
        "s01p21" -> Test406State.S01p21
        "s01p22" -> Test406State.S01p22
        "s03" -> Test406State.S03
        "s04" -> Test406State.S04
        "s05" -> Test406State.S05
        "s0p2" -> Test406State.S0p2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test406State): String = when (state) {
        is Test406State.Fail -> "fail"
        is Test406State.Pass -> "pass"
        is Test406State.S0 -> "s0"
        is Test406State.S01 -> "s01"
        is Test406State.S01p21 -> "s01p21"
        is Test406State.S01p22 -> "s01p22"
        is Test406State.S03 -> "s03"
        is Test406State.S04 -> "s04"
        is Test406State.S05 -> "s05"
        is Test406State.S0p2 -> "s0p2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test406State): Int = when (state) {
        is Test406State.Fail -> 9
        is Test406State.Pass -> 8
        is Test406State.S0 -> 0
        is Test406State.S01 -> 1
        is Test406State.S01p21 -> 3
        is Test406State.S01p22 -> 4
        is Test406State.S03 -> 5
        is Test406State.S04 -> 6
        is Test406State.S05 -> 7
        is Test406State.S0p2 -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test406State,
        event: Test406Event?
    ): EnabledTransition<Test406State, HistoryId>? = when (state) {
        is Test406State.S0 -> when {
            event is Test406Event.Timeout -> transitionS0At0
            else -> null
        }
        is Test406State.S01 -> when {
            event == null -> transitionS01At0
            else -> null
        }
        is Test406State.S03 -> when {
            event is Test406Event.Event2 -> transitionS03At0
            event != null -> transitionS03At1
            else -> null
        }
        is Test406State.S04 -> when {
            event is Test406Event.Event3 -> transitionS04At0
            event != null -> transitionS04At1
            else -> null
        }
        is Test406State.S05 -> when {
            event is Test406Event.Event4 -> transitionS05At0
            event != null -> transitionS05At1
            else -> null
        }
        is Test406State.S0p2 -> when {
            event is Test406Event.Event1 -> transitionS0p2At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test406.scxml:6 :: _machine
    override fun onEntry(state: Test406State, isDefaultEntry: Boolean) {
        when (state) {
            is Test406State.Fail -> {
                // SCE-MAP: test406.scxml:66 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test406State.Pass -> {
                // SCE-MAP: test406.scxml:65 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test406State.S0 -> {
                // SCE-MAP: test406.scxml:8 :: s0 :: _state_body


            scheduleSend("__send_0", 1000L, Test406Event.Timeout)
            }
            is Test406State.S01 -> {
                // SCE-MAP: test406.scxml:14 :: s01 :: _state_body
            }
            is Test406State.S01p21 -> {
                // SCE-MAP: test406.scxml:25 :: s01p21 :: _state_body

            raiseInternal(Test406Event.Event3)
            }
            is Test406State.S01p22 -> {
                // SCE-MAP: test406.scxml:32 :: s01p22 :: _state_body

            raiseInternal(Test406Event.Event4)
            }
            is Test406State.S03 -> {
                // SCE-MAP: test406.scxml:46 :: s03 :: _state_body
            }
            is Test406State.S04 -> {
                // SCE-MAP: test406.scxml:51 :: s04 :: _state_body
            }
            is Test406State.S05 -> {
                // SCE-MAP: test406.scxml:57 :: s05 :: _state_body
            }
            is Test406State.S0p2 -> {
                // SCE-MAP: test406.scxml:21 :: s0p2 :: _state_body

            raiseInternal(Test406Event.Event2)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test406.scxml:6 :: _machine
    override fun onExit(state: Test406State) {
        when (state) {
            is Test406State.Fail -> {
                // SCE-MAP: test406.scxml:66 :: fail :: _state_body
            }
            is Test406State.Pass -> {
                // SCE-MAP: test406.scxml:65 :: pass :: _state_body
            }
            is Test406State.S0 -> {
                // SCE-MAP: test406.scxml:8 :: s0 :: _state_body
            }
            is Test406State.S01 -> {
                // SCE-MAP: test406.scxml:14 :: s01 :: _state_body
            }
            is Test406State.S01p21 -> {
                // SCE-MAP: test406.scxml:25 :: s01p21 :: _state_body
            }
            is Test406State.S01p22 -> {
                // SCE-MAP: test406.scxml:32 :: s01p22 :: _state_body
            }
            is Test406State.S03 -> {
                // SCE-MAP: test406.scxml:46 :: s03 :: _state_body
            }
            is Test406State.S04 -> {
                // SCE-MAP: test406.scxml:51 :: s04 :: _state_body
            }
            is Test406State.S05 -> {
                // SCE-MAP: test406.scxml:57 :: s05 :: _state_body
            }
            is Test406State.S0p2 -> {
                // SCE-MAP: test406.scxml:21 :: s0p2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test406.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test406State, transitionIndex: Int) {
        when (source) {
        is Test406State.S01 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: test406.scxml:15 :: s01 :: _transition_0

            raiseInternal(Test406Event.Event1)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
