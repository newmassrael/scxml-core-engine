// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/387/test387.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test387.scxml:7 :: _machine

package com.sce.generated.test387

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test387State : State {
    data object Fail : Test387State
    data object Pass : Test387State
    data object S0 : Test387State
    data object S01 : Test387State
    data object S011 : Test387State
    data object S012 : Test387State
    data object S02 : Test387State
    data object S021 : Test387State
    data object S022 : Test387State
    data object S1 : Test387State
    data object S11 : Test387State
    data object S111 : Test387State
    data object S112 : Test387State
    data object S12 : Test387State
    data object S121 : Test387State
    data object S122 : Test387State
    data object S3 : Test387State
    data object S4 : Test387State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test387Event : Event {
    data object EnteringS011 : Test387Event
    data object EnteringS012 : Test387Event
    data object EnteringS021 : Test387Event
    data object EnteringS022 : Test387Event
    data object EnteringS111 : Test387Event
    data object EnteringS112 : Test387Event
    data object EnteringS121 : Test387Event
    data object EnteringS122 : Test387Event
    sealed interface Error : Test387Event {
        data object Execution : Error
    }
    data object Timeout : Test387Event
}
// --- State Machine (W3C SCXML) ---

class Test387StateMachine(
) : StateMachineEngine<Test387State, Test387Event>() {

    override val initialState: Test387State = Test387State.S3

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
    override fun parentOf(state: Test387State): Test387State? = when (state) {
        is Test387State.S01 -> Test387State.S0
        is Test387State.S011 -> Test387State.S01
        is Test387State.S012 -> Test387State.S01
        is Test387State.S02 -> Test387State.S0
        is Test387State.S021 -> Test387State.S02
        is Test387State.S022 -> Test387State.S02
        is Test387State.S11 -> Test387State.S1
        is Test387State.S111 -> Test387State.S11
        is Test387State.S112 -> Test387State.S11
        is Test387State.S12 -> Test387State.S1
        is Test387State.S121 -> Test387State.S12
        is Test387State.S122 -> Test387State.S12
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test387State): Boolean = when (state) {
        is Test387State.S0, is Test387State.S01, is Test387State.S02, is Test387State.S1, is Test387State.S11, is Test387State.S12 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test387State): Boolean = when (state) {
        is Test387State.Fail, is Test387State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test387State): List<Test387State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test387State): List<EntryTarget<Test387State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test387State, HistoryId>>
        get() = documentInitialTargetList

    // W3C SCXML 3.10: the state a <history> is declared in.
    override fun historyParentOf(history: HistoryId): Test387State = historyParents.getValue(history)

    // W3C SCXML 3.10.2: a <history>'s default transition target, as written.
    override fun historyDefaultTargetsOf(history: HistoryId): List<EntryTarget<Test387State, HistoryId>> =
        historyDefaultTargets.getValue(history)

    // W3C SCXML 3.10: a state's <history> children, each with whether it is
    // deep — what the runtime records as the state is exited.
    override fun historiesOf(state: Test387State): List<Pair<HistoryId, Boolean>> =
        historiesByParent[state] ?: emptyList()

    private companion object {
        /** W3C SCXML 3.10: the `s0HistDeep` <history> (deep). */
        val historyS0HistDeep = HistoryId(0)

        /** W3C SCXML 3.10: the `s0HistShallow` <history> (shallow). */
        val historyS0HistShallow = HistoryId(1)

        /** W3C SCXML 3.10: the `s1HistDeep` <history> (deep). */
        val historyS1HistDeep = HistoryId(2)

        /** W3C SCXML 3.10: the `s1HistShallow` <history> (shallow). */
        val historyS1HistShallow = HistoryId(3)

        val childStates: Map<Test387State, List<Test387State>> = mapOf(
            Test387State.S0 to listOf(Test387State.S01, Test387State.S02),
            Test387State.S01 to listOf(Test387State.S011, Test387State.S012),
            Test387State.S02 to listOf(Test387State.S021, Test387State.S022),
            Test387State.S1 to listOf(Test387State.S11, Test387State.S12),
            Test387State.S11 to listOf(Test387State.S111, Test387State.S112),
            Test387State.S12 to listOf(Test387State.S121, Test387State.S122),
        )

        val initialTargets: Map<Test387State, List<EntryTarget<Test387State, HistoryId>>> = mapOf(
            Test387State.S0 to listOf(StateTarget(Test387State.S01)),
            Test387State.S01 to listOf(StateTarget(Test387State.S011)),
            Test387State.S02 to listOf(StateTarget(Test387State.S021)),
            Test387State.S1 to listOf(StateTarget(Test387State.S11)),
            Test387State.S11 to listOf(StateTarget(Test387State.S111)),
            Test387State.S12 to listOf(StateTarget(Test387State.S121)),
        )

        val documentInitialTargetList: List<EntryTarget<Test387State, HistoryId>> =
            listOf(StateTarget(Test387State.S3))

        val historyParents: Map<HistoryId, Test387State> = mapOf(
            historyS0HistDeep to Test387State.S0,
            historyS0HistShallow to Test387State.S0,
            historyS1HistDeep to Test387State.S1,
            historyS1HistShallow to Test387State.S1,
        )

        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<Test387State, HistoryId>>> = mapOf(
            historyS0HistDeep to listOf(StateTarget(Test387State.S022)),
            historyS0HistShallow to listOf(StateTarget(Test387State.S01)),
            historyS1HistDeep to listOf(StateTarget(Test387State.S122)),
            historyS1HistShallow to listOf(StateTarget(Test387State.S11)),
        )

        val historiesByParent: Map<Test387State, List<Pair<HistoryId, Boolean>>> = mapOf(
            Test387State.S0 to listOf(historyS0HistDeep to true, historyS0HistShallow to false),
            Test387State.S1 to listOf(historyS1HistDeep to true, historyS1HistShallow to false),
        )

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S0,
            listOf(StateTarget(Test387State.S4)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S0,
            listOf(StateTarget(Test387State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S1,
            listOf(StateTarget(Test387State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S1,
            listOf(StateTarget(Test387State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3's transition 0, as the microstep reads it.
        val transitionS3At0 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S3,
            listOf(HistoryTarget(historyS0HistShallow)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s4's transition 0, as the microstep reads it.
        val transitionS4At0 = EnabledTransition<Test387State, HistoryId>(
            Test387State.S4,
            listOf(HistoryTarget(historyS1HistDeep)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test387State? = when (stateId) {
        "fail" -> Test387State.Fail
        "pass" -> Test387State.Pass
        "s0" -> Test387State.S0
        "s01" -> Test387State.S01
        "s011" -> Test387State.S011
        "s012" -> Test387State.S012
        "s02" -> Test387State.S02
        "s021" -> Test387State.S021
        "s022" -> Test387State.S022
        "s1" -> Test387State.S1
        "s11" -> Test387State.S11
        "s111" -> Test387State.S111
        "s112" -> Test387State.S112
        "s12" -> Test387State.S12
        "s121" -> Test387State.S121
        "s122" -> Test387State.S122
        "s3" -> Test387State.S3
        "s4" -> Test387State.S4
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test387State): String = when (state) {
        is Test387State.Fail -> "fail"
        is Test387State.Pass -> "pass"
        is Test387State.S0 -> "s0"
        is Test387State.S01 -> "s01"
        is Test387State.S011 -> "s011"
        is Test387State.S012 -> "s012"
        is Test387State.S02 -> "s02"
        is Test387State.S021 -> "s021"
        is Test387State.S022 -> "s022"
        is Test387State.S1 -> "s1"
        is Test387State.S11 -> "s11"
        is Test387State.S111 -> "s111"
        is Test387State.S112 -> "s112"
        is Test387State.S12 -> "s12"
        is Test387State.S121 -> "s121"
        is Test387State.S122 -> "s122"
        is Test387State.S3 -> "s3"
        is Test387State.S4 -> "s4"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test387State): Int = when (state) {
        is Test387State.Fail -> 17
        is Test387State.Pass -> 16
        is Test387State.S0 -> 0
        is Test387State.S01 -> 1
        is Test387State.S011 -> 2
        is Test387State.S012 -> 3
        is Test387State.S02 -> 4
        is Test387State.S021 -> 5
        is Test387State.S022 -> 6
        is Test387State.S1 -> 7
        is Test387State.S11 -> 8
        is Test387State.S111 -> 9
        is Test387State.S112 -> 10
        is Test387State.S12 -> 11
        is Test387State.S121 -> 12
        is Test387State.S122 -> 13
        is Test387State.S3 -> 14
        is Test387State.S4 -> 15
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test387State,
        event: Test387Event?
    ): EnabledTransition<Test387State, HistoryId>? = when (state) {
        is Test387State.S0 -> when {
            event is Test387Event.EnteringS011 -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        is Test387State.S1 -> when {
            event is Test387Event.EnteringS122 -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        is Test387State.S3 -> when {
            event == null -> transitionS3At0
            else -> null
        }
        is Test387State.S4 -> when {
            event == null -> transitionS4At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test387.scxml:7 :: _machine
    override fun onEntry(state: Test387State, isDefaultEntry: Boolean) {
        when (state) {
            is Test387State.Fail -> {
                // SCE-MAP: test387.scxml:99 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.Pass -> {
                // SCE-MAP: test387.scxml:98 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test387State.S0 -> {
                // SCE-MAP: test387.scxml:10 :: s0 :: _state_body
            }
            is Test387State.S01 -> {
                // SCE-MAP: test387.scxml:21 :: s01 :: _state_body
            }
            is Test387State.S011 -> {
                // SCE-MAP: test387.scxml:22 :: s011 :: _state_body

            raiseInternal(Test387Event.EnteringS011)
            }
            is Test387State.S012 -> {
                // SCE-MAP: test387.scxml:27 :: s012 :: _state_body

            raiseInternal(Test387Event.EnteringS012)
            }
            is Test387State.S02 -> {
                // SCE-MAP: test387.scxml:33 :: s02 :: _state_body
            }
            is Test387State.S021 -> {
                // SCE-MAP: test387.scxml:34 :: s021 :: _state_body

            raiseInternal(Test387Event.EnteringS021)
            }
            is Test387State.S022 -> {
                // SCE-MAP: test387.scxml:39 :: s022 :: _state_body

            raiseInternal(Test387Event.EnteringS022)
            }
            is Test387State.S1 -> {
                // SCE-MAP: test387.scxml:48 :: s1 :: _state_body
            }
            is Test387State.S11 -> {
                // SCE-MAP: test387.scxml:59 :: s11 :: _state_body
            }
            is Test387State.S111 -> {
                // SCE-MAP: test387.scxml:60 :: s111 :: _state_body

            raiseInternal(Test387Event.EnteringS111)
            }
            is Test387State.S112 -> {
                // SCE-MAP: test387.scxml:65 :: s112 :: _state_body

            raiseInternal(Test387Event.EnteringS112)
            }
            is Test387State.S12 -> {
                // SCE-MAP: test387.scxml:71 :: s12 :: _state_body
            }
            is Test387State.S121 -> {
                // SCE-MAP: test387.scxml:72 :: s121 :: _state_body

            raiseInternal(Test387Event.EnteringS121)
            }
            is Test387State.S122 -> {
                // SCE-MAP: test387.scxml:77 :: s122 :: _state_body

            raiseInternal(Test387Event.EnteringS122)
            }
            is Test387State.S3 -> {
                // SCE-MAP: test387.scxml:87 :: s3 :: _state_body


            scheduleSend("__send_0", 1000L, Test387Event.Timeout)
            }
            is Test387State.S4 -> {
                // SCE-MAP: test387.scxml:94 :: s4 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test387.scxml:7 :: _machine
    override fun onExit(state: Test387State) {
        when (state) {
            is Test387State.Fail -> {
                // SCE-MAP: test387.scxml:99 :: fail :: _state_body
            }
            is Test387State.Pass -> {
                // SCE-MAP: test387.scxml:98 :: pass :: _state_body
            }
            is Test387State.S0 -> {
                // SCE-MAP: test387.scxml:10 :: s0 :: _state_body
            }
            is Test387State.S01 -> {
                // SCE-MAP: test387.scxml:21 :: s01 :: _state_body
            }
            is Test387State.S011 -> {
                // SCE-MAP: test387.scxml:22 :: s011 :: _state_body
            }
            is Test387State.S012 -> {
                // SCE-MAP: test387.scxml:27 :: s012 :: _state_body
            }
            is Test387State.S02 -> {
                // SCE-MAP: test387.scxml:33 :: s02 :: _state_body
            }
            is Test387State.S021 -> {
                // SCE-MAP: test387.scxml:34 :: s021 :: _state_body
            }
            is Test387State.S022 -> {
                // SCE-MAP: test387.scxml:39 :: s022 :: _state_body
            }
            is Test387State.S1 -> {
                // SCE-MAP: test387.scxml:48 :: s1 :: _state_body
            }
            is Test387State.S11 -> {
                // SCE-MAP: test387.scxml:59 :: s11 :: _state_body
            }
            is Test387State.S111 -> {
                // SCE-MAP: test387.scxml:60 :: s111 :: _state_body
            }
            is Test387State.S112 -> {
                // SCE-MAP: test387.scxml:65 :: s112 :: _state_body
            }
            is Test387State.S12 -> {
                // SCE-MAP: test387.scxml:71 :: s12 :: _state_body
            }
            is Test387State.S121 -> {
                // SCE-MAP: test387.scxml:72 :: s121 :: _state_body
            }
            is Test387State.S122 -> {
                // SCE-MAP: test387.scxml:77 :: s122 :: _state_body
            }
            is Test387State.S3 -> {
                // SCE-MAP: test387.scxml:87 :: s3 :: _state_body
            }
            is Test387State.S4 -> {
                // SCE-MAP: test387.scxml:94 :: s4 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test387.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test387State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
