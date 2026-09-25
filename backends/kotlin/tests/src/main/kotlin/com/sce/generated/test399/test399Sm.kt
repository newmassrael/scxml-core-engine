// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/399/test399.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test399.scxml:6 :: _machine

package com.sce.generated.test399

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test399State : State {
    data object Fail : Test399State
    data object Pass : Test399State
    data object S0 : Test399State
    data object S01 : Test399State
    data object S02 : Test399State
    data object S03 : Test399State
    data object S04 : Test399State
    data object S05 : Test399State
    data object S06 : Test399State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test399Event : Event {
    data object Bar : Test399Event
    sealed interface Error : Test399Event {
        data object Execution : Error
    }
    sealed interface Foo : Test399Event {
        data object Self : Foo
        data object Zoo : Foo
    }
    data object Foos : Test399Event
    data object Timeout : Test399Event
}
// --- State Machine (W3C SCXML) ---

class Test399StateMachine(
) : StateMachineEngine<Test399State, Test399Event>() {

    override val initialState: Test399State = Test399State.S01

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
    override fun parentOf(state: Test399State): Test399State? = when (state) {
        is Test399State.S01 -> Test399State.S0
        is Test399State.S02 -> Test399State.S0
        is Test399State.S03 -> Test399State.S0
        is Test399State.S04 -> Test399State.S0
        is Test399State.S05 -> Test399State.S0
        is Test399State.S06 -> Test399State.S0
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test399State): Boolean = when (state) {
        is Test399State.S0 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test399State): Boolean = when (state) {
        is Test399State.Fail, is Test399State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test399State): List<Test399State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test399State): List<EntryTarget<Test399State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test399State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test399State, List<Test399State>> = mapOf(
            Test399State.S0 to listOf(Test399State.S01, Test399State.S02, Test399State.S03, Test399State.S04, Test399State.S05, Test399State.S06),
        )

        val initialTargets: Map<Test399State, List<EntryTarget<Test399State, HistoryId>>> = mapOf(
            Test399State.S0 to listOf(StateTarget(Test399State.S01)),
        )

        val documentInitialTargetList: List<EntryTarget<Test399State, HistoryId>> =
            listOf(StateTarget(Test399State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S0,
            listOf(StateTarget(Test399State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s01's transition 0, as the microstep reads it.
        val transitionS01At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S01,
            listOf(StateTarget(Test399State.S02)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s02's transition 0, as the microstep reads it.
        val transitionS02At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S02,
            listOf(StateTarget(Test399State.S03)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s03's transition 0, as the microstep reads it.
        val transitionS03At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S03,
            listOf(StateTarget(Test399State.S04)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 0, as the microstep reads it.
        val transitionS04At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S04,
            listOf(StateTarget(Test399State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s04's transition 1, as the microstep reads it.
        val transitionS04At1 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S04,
            listOf(StateTarget(Test399State.S05)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s05's transition 0, as the microstep reads it.
        val transitionS05At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S05,
            listOf(StateTarget(Test399State.S06)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s06's transition 0, as the microstep reads it.
        val transitionS06At0 = EnabledTransition<Test399State, HistoryId>(
            Test399State.S06,
            listOf(StateTarget(Test399State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test399State? = when (stateId) {
        "fail" -> Test399State.Fail
        "pass" -> Test399State.Pass
        "s0" -> Test399State.S0
        "s01" -> Test399State.S01
        "s02" -> Test399State.S02
        "s03" -> Test399State.S03
        "s04" -> Test399State.S04
        "s05" -> Test399State.S05
        "s06" -> Test399State.S06
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test399State): String = when (state) {
        is Test399State.Fail -> "fail"
        is Test399State.Pass -> "pass"
        is Test399State.S0 -> "s0"
        is Test399State.S01 -> "s01"
        is Test399State.S02 -> "s02"
        is Test399State.S03 -> "s03"
        is Test399State.S04 -> "s04"
        is Test399State.S05 -> "s05"
        is Test399State.S06 -> "s06"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test399State): Int = when (state) {
        is Test399State.Fail -> 8
        is Test399State.Pass -> 7
        is Test399State.S0 -> 0
        is Test399State.S01 -> 1
        is Test399State.S02 -> 2
        is Test399State.S03 -> 3
        is Test399State.S04 -> 4
        is Test399State.S05 -> 5
        is Test399State.S06 -> 6
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test399State,
        event: Test399Event?
    ): EnabledTransition<Test399State, HistoryId>? = when (state) {
        is Test399State.S0 -> when {
            event is Test399Event.Timeout -> transitionS0At0
            else -> null
        }
        is Test399State.S01 -> when {
            (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> transitionS01At0
            else -> null
        }
        is Test399State.S02 -> when {
            (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> transitionS02At0
            else -> null
        }
        is Test399State.S03 -> when {
            (event is Test399Event.Bar || event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> transitionS03At0
            else -> null
        }
        is Test399State.S04 -> when {
            (event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> transitionS04At0
            event is Test399Event.Foos -> transitionS04At1
            else -> null
        }
        is Test399State.S05 -> when {
            (event is Test399Event.Foo || event is Test399Event.Foo.Zoo) -> transitionS05At0
            else -> null
        }
        is Test399State.S06 -> when {
            event != null -> transitionS06At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun onEntry(state: Test399State, isDefaultEntry: Boolean) {
        when (state) {
            is Test399State.Fail -> {
                // SCE-MAP: test399.scxml:69 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.Pass -> {
                // SCE-MAP: test399.scxml:68 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test399State.S0 -> {
                // SCE-MAP: test399.scxml:9 :: s0 :: _state_body


            scheduleSend("__send_0", 2000L, Test399Event.Timeout)
            }
            is Test399State.S01 -> {
                // SCE-MAP: test399.scxml:17 :: s01 :: _state_body

            raiseInternal(Test399Event.Foo.Self)
            }
            is Test399State.S02 -> {
                // SCE-MAP: test399.scxml:25 :: s02 :: _state_body

            raiseInternal(Test399Event.Bar)
            }
            is Test399State.S03 -> {
                // SCE-MAP: test399.scxml:33 :: s03 :: _state_body

            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S04 -> {
                // SCE-MAP: test399.scxml:41 :: s04 :: _state_body

            raiseInternal(Test399Event.Foos)
            }
            is Test399State.S05 -> {
                // SCE-MAP: test399.scxml:50 :: s05 :: _state_body

            raiseInternal(Test399Event.Foo.Zoo)
            }
            is Test399State.S06 -> {
                // SCE-MAP: test399.scxml:58 :: s06 :: _state_body

            raiseInternal(Test399Event.Foo.Self)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun onExit(state: Test399State) {
        when (state) {
            is Test399State.Fail -> {
                // SCE-MAP: test399.scxml:69 :: fail :: _state_body
            }
            is Test399State.Pass -> {
                // SCE-MAP: test399.scxml:68 :: pass :: _state_body
            }
            is Test399State.S0 -> {
                // SCE-MAP: test399.scxml:9 :: s0 :: _state_body
            }
            is Test399State.S01 -> {
                // SCE-MAP: test399.scxml:17 :: s01 :: _state_body
            }
            is Test399State.S02 -> {
                // SCE-MAP: test399.scxml:25 :: s02 :: _state_body
            }
            is Test399State.S03 -> {
                // SCE-MAP: test399.scxml:33 :: s03 :: _state_body
            }
            is Test399State.S04 -> {
                // SCE-MAP: test399.scxml:41 :: s04 :: _state_body
            }
            is Test399State.S05 -> {
                // SCE-MAP: test399.scxml:50 :: s05 :: _state_body
            }
            is Test399State.S06 -> {
                // SCE-MAP: test399.scxml:58 :: s06 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test399.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test399State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
