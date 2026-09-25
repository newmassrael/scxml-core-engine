// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/413/test413.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test413.scxml:7 :: _machine

package com.sce.generated.test413

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test413State : State {
    data object Fail : Test413State
    data object Pass : Test413State
    data object S1 : Test413State
    data object S2 : Test413State
    data object S2p1 : Test413State
    data object S2p11 : Test413State
    data object S2p111 : Test413State
    data object S2p112 : Test413State
    data object S2p12 : Test413State
    data object S2p121 : Test413State
    data object S2p122 : Test413State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test413Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test413StateMachine(
) : StateMachineEngine<Test413State, Test413Event>() {

    override val initialState: Test413State = Test413State.S2p112

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
    override fun parentOf(state: Test413State): Test413State? = when (state) {
        is Test413State.S2p1 -> Test413State.S2
        is Test413State.S2p11 -> Test413State.S2p1
        is Test413State.S2p111 -> Test413State.S2p11
        is Test413State.S2p112 -> Test413State.S2p11
        is Test413State.S2p12 -> Test413State.S2p1
        is Test413State.S2p121 -> Test413State.S2p12
        is Test413State.S2p122 -> Test413State.S2p12
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test413State): Boolean = when (state) {
        is Test413State.S2, is Test413State.S2p11, is Test413State.S2p12 -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test413State): Boolean = when (state) {
        is Test413State.S2p1 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test413State): Boolean = when (state) {
        is Test413State.Fail, is Test413State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test413State): List<Test413State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test413State): List<EntryTarget<Test413State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test413State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test413State, List<Test413State>> = mapOf(
            Test413State.S2 to listOf(Test413State.S2p1),
            Test413State.S2p1 to listOf(Test413State.S2p11, Test413State.S2p12),
            Test413State.S2p11 to listOf(Test413State.S2p111, Test413State.S2p112),
            Test413State.S2p12 to listOf(Test413State.S2p121, Test413State.S2p122),
        )

        val initialTargets: Map<Test413State, List<EntryTarget<Test413State, HistoryId>>> = mapOf(
            Test413State.S2 to listOf(StateTarget(Test413State.S2p1)),
            Test413State.S2p11 to listOf(StateTarget(Test413State.S2p111)),
            Test413State.S2p12 to listOf(StateTarget(Test413State.S2p121)),
        )

        val documentInitialTargetList: List<EntryTarget<Test413State, HistoryId>> =
            listOf(StateTarget(Test413State.S2p112), StateTarget(Test413State.S2p122))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S1,
            listOf(StateTarget(Test413State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2p1's transition 0, as the microstep reads it.
        val transitionS2p1At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S2p1,
            listOf(StateTarget(Test413State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2p111's transition 0, as the microstep reads it.
        val transitionS2p111At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S2p111,
            listOf(StateTarget(Test413State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2p112's transition 0, as the microstep reads it.
        val transitionS2p112At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S2p112,
            listOf(StateTarget(Test413State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2p121's transition 0, as the microstep reads it.
        val transitionS2p121At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S2p121,
            listOf(StateTarget(Test413State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2p122's transition 0, as the microstep reads it.
        val transitionS2p122At0 = EnabledTransition<Test413State, HistoryId>(
            Test413State.S2p122,
            listOf(StateTarget(Test413State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test413State? = when (stateId) {
        "fail" -> Test413State.Fail
        "pass" -> Test413State.Pass
        "s1" -> Test413State.S1
        "s2" -> Test413State.S2
        "s2p1" -> Test413State.S2p1
        "s2p11" -> Test413State.S2p11
        "s2p111" -> Test413State.S2p111
        "s2p112" -> Test413State.S2p112
        "s2p12" -> Test413State.S2p12
        "s2p121" -> Test413State.S2p121
        "s2p122" -> Test413State.S2p122
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test413State): String = when (state) {
        is Test413State.Fail -> "fail"
        is Test413State.Pass -> "pass"
        is Test413State.S1 -> "s1"
        is Test413State.S2 -> "s2"
        is Test413State.S2p1 -> "s2p1"
        is Test413State.S2p11 -> "s2p11"
        is Test413State.S2p111 -> "s2p111"
        is Test413State.S2p112 -> "s2p112"
        is Test413State.S2p12 -> "s2p12"
        is Test413State.S2p121 -> "s2p121"
        is Test413State.S2p122 -> "s2p122"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test413State): Int = when (state) {
        is Test413State.Fail -> 10
        is Test413State.Pass -> 9
        is Test413State.S1 -> 0
        is Test413State.S2 -> 1
        is Test413State.S2p1 -> 2
        is Test413State.S2p11 -> 3
        is Test413State.S2p111 -> 4
        is Test413State.S2p112 -> 5
        is Test413State.S2p12 -> 6
        is Test413State.S2p121 -> 7
        is Test413State.S2p122 -> 8
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test413State,
        event: Test413Event?
    ): EnabledTransition<Test413State, HistoryId>? = when (state) {
        is Test413State.S1 -> when {
            event == null -> transitionS1At0
            else -> null
        }
        is Test413State.S2p1 -> when {
            event == null -> transitionS2p1At0
            else -> null
        }
        is Test413State.S2p111 -> when {
            event == null -> transitionS2p111At0
            else -> null
        }
        is Test413State.S2p112 -> when {
            event == null && isStateActive("s2p122") -> transitionS2p112At0
            else -> null
        }
        is Test413State.S2p121 -> when {
            event == null -> transitionS2p121At0
            else -> null
        }
        is Test413State.S2p122 -> when {
            event == null && isStateActive("s2p112") -> transitionS2p122At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test413.scxml:7 :: _machine
    override fun onEntry(state: Test413State, isDefaultEntry: Boolean) {
        when (state) {
            is Test413State.Fail -> {
                // SCE-MAP: test413.scxml:47 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test413State.Pass -> {
                // SCE-MAP: test413.scxml:46 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test413State.S1 -> {
                // SCE-MAP: test413.scxml:9 :: s1 :: _state_body
            }
            is Test413State.S2 -> {
                // SCE-MAP: test413.scxml:13 :: s2 :: _state_body
            }
            is Test413State.S2p1 -> {
                // SCE-MAP: test413.scxml:15 :: s2p1 :: _state_body
            }
            is Test413State.S2p11 -> {
                // SCE-MAP: test413.scxml:20 :: s2p11 :: _state_body
            }
            is Test413State.S2p111 -> {
                // SCE-MAP: test413.scxml:21 :: s2p111 :: _state_body
            }
            is Test413State.S2p112 -> {
                // SCE-MAP: test413.scxml:25 :: s2p112 :: _state_body
            }
            is Test413State.S2p12 -> {
                // SCE-MAP: test413.scxml:31 :: s2p12 :: _state_body
            }
            is Test413State.S2p121 -> {
                // SCE-MAP: test413.scxml:32 :: s2p121 :: _state_body
            }
            is Test413State.S2p122 -> {
                // SCE-MAP: test413.scxml:36 :: s2p122 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test413.scxml:7 :: _machine
    override fun onExit(state: Test413State) {
        when (state) {
            is Test413State.Fail -> {
                // SCE-MAP: test413.scxml:47 :: fail :: _state_body
            }
            is Test413State.Pass -> {
                // SCE-MAP: test413.scxml:46 :: pass :: _state_body
            }
            is Test413State.S1 -> {
                // SCE-MAP: test413.scxml:9 :: s1 :: _state_body
            }
            is Test413State.S2 -> {
                // SCE-MAP: test413.scxml:13 :: s2 :: _state_body
            }
            is Test413State.S2p1 -> {
                // SCE-MAP: test413.scxml:15 :: s2p1 :: _state_body
            }
            is Test413State.S2p11 -> {
                // SCE-MAP: test413.scxml:20 :: s2p11 :: _state_body
            }
            is Test413State.S2p111 -> {
                // SCE-MAP: test413.scxml:21 :: s2p111 :: _state_body
            }
            is Test413State.S2p112 -> {
                // SCE-MAP: test413.scxml:25 :: s2p112 :: _state_body
            }
            is Test413State.S2p12 -> {
                // SCE-MAP: test413.scxml:31 :: s2p12 :: _state_body
            }
            is Test413State.S2p121 -> {
                // SCE-MAP: test413.scxml:32 :: s2p121 :: _state_body
            }
            is Test413State.S2p122 -> {
                // SCE-MAP: test413.scxml:36 :: s2p122 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test413.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test413State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
