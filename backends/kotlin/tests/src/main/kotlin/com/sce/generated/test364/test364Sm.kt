// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/364/test364.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test364.scxml:7 :: _machine

package com.sce.generated.test364

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test364State : State {
    data object Fail : Test364State
    data object Pass : Test364State
    data object S1 : Test364State
    data object S11 : Test364State
    data object S111 : Test364State
    data object S11p1 : Test364State
    data object S11p11 : Test364State
    data object S11p111 : Test364State
    data object S11p112 : Test364State
    data object S11p12 : Test364State
    data object S11p121 : Test364State
    data object S11p122 : Test364State
    data object S2 : Test364State
    data object S21 : Test364State
    data object S211 : Test364State
    data object S21p1 : Test364State
    data object S21p11 : Test364State
    data object S21p111 : Test364State
    data object S21p112 : Test364State
    data object S21p12 : Test364State
    data object S21p121 : Test364State
    data object S21p122 : Test364State
    data object S3 : Test364State
    data object S31 : Test364State
    data object S311 : Test364State
    data object S3111 : Test364State
    data object S3112 : Test364State
    data object S312 : Test364State
    data object S32 : Test364State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test364Event : Event {
    data object InS11p112 : Test364Event
    data object InS21p112 : Test364Event
    sealed interface Error : Test364Event {
        data object Execution : Error
    }
    data object Timeout : Test364Event
}
// --- State Machine (W3C SCXML) ---

class Test364StateMachine(
) : StateMachineEngine<Test364State, Test364Event>() {

    override val initialState: Test364State = Test364State.S1

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
    override fun parentOf(state: Test364State): Test364State? = when (state) {
        is Test364State.S11 -> Test364State.S1
        is Test364State.S111 -> Test364State.S11
        is Test364State.S11p1 -> Test364State.S11
        is Test364State.S11p11 -> Test364State.S11p1
        is Test364State.S11p111 -> Test364State.S11p11
        is Test364State.S11p112 -> Test364State.S11p11
        is Test364State.S11p12 -> Test364State.S11p1
        is Test364State.S11p121 -> Test364State.S11p12
        is Test364State.S11p122 -> Test364State.S11p12
        is Test364State.S21 -> Test364State.S2
        is Test364State.S211 -> Test364State.S21
        is Test364State.S21p1 -> Test364State.S21
        is Test364State.S21p11 -> Test364State.S21p1
        is Test364State.S21p111 -> Test364State.S21p11
        is Test364State.S21p112 -> Test364State.S21p11
        is Test364State.S21p12 -> Test364State.S21p1
        is Test364State.S21p121 -> Test364State.S21p12
        is Test364State.S21p122 -> Test364State.S21p12
        is Test364State.S31 -> Test364State.S3
        is Test364State.S311 -> Test364State.S31
        is Test364State.S3111 -> Test364State.S311
        is Test364State.S3112 -> Test364State.S311
        is Test364State.S312 -> Test364State.S311
        is Test364State.S32 -> Test364State.S311
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test364State): Boolean = when (state) {
        is Test364State.S1, is Test364State.S11, is Test364State.S11p11, is Test364State.S11p12, is Test364State.S2, is Test364State.S21, is Test364State.S21p11, is Test364State.S21p12, is Test364State.S3, is Test364State.S31, is Test364State.S311 -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test364State): Boolean = when (state) {
        is Test364State.S11p1, is Test364State.S21p1 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test364State): Boolean = when (state) {
        is Test364State.Fail, is Test364State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test364State): List<Test364State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test364State): List<EntryTarget<Test364State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test364State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test364State, List<Test364State>> = mapOf(
            Test364State.S1 to listOf(Test364State.S11),
            Test364State.S11 to listOf(Test364State.S111, Test364State.S11p1),
            Test364State.S11p1 to listOf(Test364State.S11p11, Test364State.S11p12),
            Test364State.S11p11 to listOf(Test364State.S11p111, Test364State.S11p112),
            Test364State.S11p12 to listOf(Test364State.S11p121, Test364State.S11p122),
            Test364State.S2 to listOf(Test364State.S21),
            Test364State.S21 to listOf(Test364State.S211, Test364State.S21p1),
            Test364State.S21p1 to listOf(Test364State.S21p11, Test364State.S21p12),
            Test364State.S21p11 to listOf(Test364State.S21p111, Test364State.S21p112),
            Test364State.S21p12 to listOf(Test364State.S21p121, Test364State.S21p122),
            Test364State.S3 to listOf(Test364State.S31),
            Test364State.S31 to listOf(Test364State.S311),
            Test364State.S311 to listOf(Test364State.S3111, Test364State.S3112, Test364State.S312, Test364State.S32),
        )

        val initialTargets: Map<Test364State, List<EntryTarget<Test364State, HistoryId>>> = mapOf(
            Test364State.S1 to listOf(StateTarget(Test364State.S11p112), StateTarget(Test364State.S11p122)),
            Test364State.S11 to listOf(StateTarget(Test364State.S111)),
            Test364State.S11p11 to listOf(StateTarget(Test364State.S11p111)),
            Test364State.S11p12 to listOf(StateTarget(Test364State.S11p121)),
            Test364State.S2 to listOf(StateTarget(Test364State.S21p112), StateTarget(Test364State.S21p122)),
            Test364State.S21 to listOf(StateTarget(Test364State.S211)),
            Test364State.S21p11 to listOf(StateTarget(Test364State.S21p111)),
            Test364State.S21p12 to listOf(StateTarget(Test364State.S21p121)),
            Test364State.S3 to listOf(StateTarget(Test364State.S31)),
            Test364State.S31 to listOf(StateTarget(Test364State.S311)),
            Test364State.S311 to listOf(StateTarget(Test364State.S3111)),
        )

        val documentInitialTargetList: List<EntryTarget<Test364State, HistoryId>> =
            listOf(StateTarget(Test364State.S1))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S1,
            listOf(StateTarget(Test364State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s11p122's transition 0, as the microstep reads it.
        val transitionS11p122At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S11p122,
            listOf(StateTarget(Test364State.S2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 0, as the microstep reads it.
        val transitionS2At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S2,
            listOf(StateTarget(Test364State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s21p122's transition 0, as the microstep reads it.
        val transitionS21p122At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S21p122,
            listOf(StateTarget(Test364State.S3)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3's transition 0, as the microstep reads it.
        val transitionS3At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S3,
            listOf(StateTarget(Test364State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s3111's transition 0, as the microstep reads it.
        val transitionS3111At0 = EnabledTransition<Test364State, HistoryId>(
            Test364State.S3111,
            listOf(StateTarget(Test364State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test364State? = when (stateId) {
        "fail" -> Test364State.Fail
        "pass" -> Test364State.Pass
        "s1" -> Test364State.S1
        "s11" -> Test364State.S11
        "s111" -> Test364State.S111
        "s11p1" -> Test364State.S11p1
        "s11p11" -> Test364State.S11p11
        "s11p111" -> Test364State.S11p111
        "s11p112" -> Test364State.S11p112
        "s11p12" -> Test364State.S11p12
        "s11p121" -> Test364State.S11p121
        "s11p122" -> Test364State.S11p122
        "s2" -> Test364State.S2
        "s21" -> Test364State.S21
        "s211" -> Test364State.S211
        "s21p1" -> Test364State.S21p1
        "s21p11" -> Test364State.S21p11
        "s21p111" -> Test364State.S21p111
        "s21p112" -> Test364State.S21p112
        "s21p12" -> Test364State.S21p12
        "s21p121" -> Test364State.S21p121
        "s21p122" -> Test364State.S21p122
        "s3" -> Test364State.S3
        "s31" -> Test364State.S31
        "s311" -> Test364State.S311
        "s3111" -> Test364State.S3111
        "s3112" -> Test364State.S3112
        "s312" -> Test364State.S312
        "s32" -> Test364State.S32
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test364State): String = when (state) {
        is Test364State.Fail -> "fail"
        is Test364State.Pass -> "pass"
        is Test364State.S1 -> "s1"
        is Test364State.S11 -> "s11"
        is Test364State.S111 -> "s111"
        is Test364State.S11p1 -> "s11p1"
        is Test364State.S11p11 -> "s11p11"
        is Test364State.S11p111 -> "s11p111"
        is Test364State.S11p112 -> "s11p112"
        is Test364State.S11p12 -> "s11p12"
        is Test364State.S11p121 -> "s11p121"
        is Test364State.S11p122 -> "s11p122"
        is Test364State.S2 -> "s2"
        is Test364State.S21 -> "s21"
        is Test364State.S211 -> "s211"
        is Test364State.S21p1 -> "s21p1"
        is Test364State.S21p11 -> "s21p11"
        is Test364State.S21p111 -> "s21p111"
        is Test364State.S21p112 -> "s21p112"
        is Test364State.S21p12 -> "s21p12"
        is Test364State.S21p121 -> "s21p121"
        is Test364State.S21p122 -> "s21p122"
        is Test364State.S3 -> "s3"
        is Test364State.S31 -> "s31"
        is Test364State.S311 -> "s311"
        is Test364State.S3111 -> "s3111"
        is Test364State.S3112 -> "s3112"
        is Test364State.S312 -> "s312"
        is Test364State.S32 -> "s32"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test364State): Int = when (state) {
        is Test364State.Fail -> 28
        is Test364State.Pass -> 27
        is Test364State.S1 -> 0
        is Test364State.S11 -> 1
        is Test364State.S111 -> 2
        is Test364State.S11p1 -> 3
        is Test364State.S11p11 -> 4
        is Test364State.S11p111 -> 5
        is Test364State.S11p112 -> 6
        is Test364State.S11p12 -> 7
        is Test364State.S11p121 -> 8
        is Test364State.S11p122 -> 9
        is Test364State.S2 -> 10
        is Test364State.S21 -> 11
        is Test364State.S211 -> 12
        is Test364State.S21p1 -> 13
        is Test364State.S21p11 -> 14
        is Test364State.S21p111 -> 15
        is Test364State.S21p112 -> 16
        is Test364State.S21p12 -> 17
        is Test364State.S21p121 -> 18
        is Test364State.S21p122 -> 19
        is Test364State.S3 -> 20
        is Test364State.S31 -> 21
        is Test364State.S311 -> 22
        is Test364State.S3111 -> 23
        is Test364State.S3112 -> 24
        is Test364State.S312 -> 25
        is Test364State.S32 -> 26
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test364State,
        event: Test364Event?
    ): EnabledTransition<Test364State, HistoryId>? = when (state) {
        is Test364State.S1 -> when {
            event is Test364Event.Timeout -> transitionS1At0
            else -> null
        }
        is Test364State.S11p122 -> when {
            event is Test364Event.InS11p112 -> transitionS11p122At0
            else -> null
        }
        is Test364State.S2 -> when {
            event is Test364Event.Timeout -> transitionS2At0
            else -> null
        }
        is Test364State.S21p122 -> when {
            event is Test364Event.InS21p112 -> transitionS21p122At0
            else -> null
        }
        is Test364State.S3 -> when {
            event == null -> transitionS3At0
            else -> null
        }
        is Test364State.S3111 -> when {
            event == null -> transitionS3111At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun onEntry(state: Test364State, isDefaultEntry: Boolean) {
        when (state) {
            is Test364State.Fail -> {
                // SCE-MAP: test364.scxml:76 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test364State.Pass -> {
                // SCE-MAP: test364.scxml:75 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test364State.S1 -> {
                // SCE-MAP: test364.scxml:9 :: s1 :: _state_body


            scheduleSend("__send_0", 1000L, Test364Event.Timeout)
            }
            is Test364State.S11 -> {
                // SCE-MAP: test364.scxml:14 :: s11 :: _state_body
            }
            is Test364State.S111 -> {
                // SCE-MAP: test364.scxml:15 :: s111 :: _state_body
            }
            is Test364State.S11p1 -> {
                // SCE-MAP: test364.scxml:16 :: s11p1 :: _state_body
            }
            is Test364State.S11p11 -> {
                // SCE-MAP: test364.scxml:17 :: s11p11 :: _state_body
            }
            is Test364State.S11p111 -> {
                // SCE-MAP: test364.scxml:18 :: s11p111 :: _state_body
            }
            is Test364State.S11p112 -> {
                // SCE-MAP: test364.scxml:19 :: s11p112 :: _state_body

            raiseInternal(Test364Event.InS11p112)
            }
            is Test364State.S11p12 -> {
                // SCE-MAP: test364.scxml:25 :: s11p12 :: _state_body
            }
            is Test364State.S11p121 -> {
                // SCE-MAP: test364.scxml:26 :: s11p121 :: _state_body
            }
            is Test364State.S11p122 -> {
                // SCE-MAP: test364.scxml:27 :: s11p122 :: _state_body
            }
            is Test364State.S2 -> {
                // SCE-MAP: test364.scxml:35 :: s2 :: _state_body
            }
            is Test364State.S21 -> {
                // SCE-MAP: test364.scxml:40 :: s21 :: _state_body
            }
            is Test364State.S211 -> {
                // SCE-MAP: test364.scxml:41 :: s211 :: _state_body
            }
            is Test364State.S21p1 -> {
                // SCE-MAP: test364.scxml:42 :: s21p1 :: _state_body
            }
            is Test364State.S21p11 -> {
                // SCE-MAP: test364.scxml:43 :: s21p11 :: _state_body
            }
            is Test364State.S21p111 -> {
                // SCE-MAP: test364.scxml:44 :: s21p111 :: _state_body
            }
            is Test364State.S21p112 -> {
                // SCE-MAP: test364.scxml:45 :: s21p112 :: _state_body

            raiseInternal(Test364Event.InS21p112)
            }
            is Test364State.S21p12 -> {
                // SCE-MAP: test364.scxml:51 :: s21p12 :: _state_body
            }
            is Test364State.S21p121 -> {
                // SCE-MAP: test364.scxml:52 :: s21p121 :: _state_body
            }
            is Test364State.S21p122 -> {
                // SCE-MAP: test364.scxml:53 :: s21p122 :: _state_body
            }
            is Test364State.S3 -> {
                // SCE-MAP: test364.scxml:61 :: s3 :: _state_body
            }
            is Test364State.S31 -> {
                // SCE-MAP: test364.scxml:63 :: s31 :: _state_body
            }
            is Test364State.S311 -> {
                // SCE-MAP: test364.scxml:64 :: s311 :: _state_body
            }
            is Test364State.S3111 -> {
                // SCE-MAP: test364.scxml:65 :: s3111 :: _state_body
            }
            is Test364State.S3112 -> {
                // SCE-MAP: test364.scxml:68 :: s3112 :: _state_body
            }
            is Test364State.S312 -> {
                // SCE-MAP: test364.scxml:69 :: s312 :: _state_body
            }
            is Test364State.S32 -> {
                // SCE-MAP: test364.scxml:70 :: s32 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun onExit(state: Test364State) {
        when (state) {
            is Test364State.Fail -> {
                // SCE-MAP: test364.scxml:76 :: fail :: _state_body
            }
            is Test364State.Pass -> {
                // SCE-MAP: test364.scxml:75 :: pass :: _state_body
            }
            is Test364State.S1 -> {
                // SCE-MAP: test364.scxml:9 :: s1 :: _state_body
            }
            is Test364State.S11 -> {
                // SCE-MAP: test364.scxml:14 :: s11 :: _state_body
            }
            is Test364State.S111 -> {
                // SCE-MAP: test364.scxml:15 :: s111 :: _state_body
            }
            is Test364State.S11p1 -> {
                // SCE-MAP: test364.scxml:16 :: s11p1 :: _state_body
            }
            is Test364State.S11p11 -> {
                // SCE-MAP: test364.scxml:17 :: s11p11 :: _state_body
            }
            is Test364State.S11p111 -> {
                // SCE-MAP: test364.scxml:18 :: s11p111 :: _state_body
            }
            is Test364State.S11p112 -> {
                // SCE-MAP: test364.scxml:19 :: s11p112 :: _state_body
            }
            is Test364State.S11p12 -> {
                // SCE-MAP: test364.scxml:25 :: s11p12 :: _state_body
            }
            is Test364State.S11p121 -> {
                // SCE-MAP: test364.scxml:26 :: s11p121 :: _state_body
            }
            is Test364State.S11p122 -> {
                // SCE-MAP: test364.scxml:27 :: s11p122 :: _state_body
            }
            is Test364State.S2 -> {
                // SCE-MAP: test364.scxml:35 :: s2 :: _state_body
            }
            is Test364State.S21 -> {
                // SCE-MAP: test364.scxml:40 :: s21 :: _state_body
            }
            is Test364State.S211 -> {
                // SCE-MAP: test364.scxml:41 :: s211 :: _state_body
            }
            is Test364State.S21p1 -> {
                // SCE-MAP: test364.scxml:42 :: s21p1 :: _state_body
            }
            is Test364State.S21p11 -> {
                // SCE-MAP: test364.scxml:43 :: s21p11 :: _state_body
            }
            is Test364State.S21p111 -> {
                // SCE-MAP: test364.scxml:44 :: s21p111 :: _state_body
            }
            is Test364State.S21p112 -> {
                // SCE-MAP: test364.scxml:45 :: s21p112 :: _state_body
            }
            is Test364State.S21p12 -> {
                // SCE-MAP: test364.scxml:51 :: s21p12 :: _state_body
            }
            is Test364State.S21p121 -> {
                // SCE-MAP: test364.scxml:52 :: s21p121 :: _state_body
            }
            is Test364State.S21p122 -> {
                // SCE-MAP: test364.scxml:53 :: s21p122 :: _state_body
            }
            is Test364State.S3 -> {
                // SCE-MAP: test364.scxml:61 :: s3 :: _state_body
            }
            is Test364State.S31 -> {
                // SCE-MAP: test364.scxml:63 :: s31 :: _state_body
            }
            is Test364State.S311 -> {
                // SCE-MAP: test364.scxml:64 :: s311 :: _state_body
            }
            is Test364State.S3111 -> {
                // SCE-MAP: test364.scxml:65 :: s3111 :: _state_body
            }
            is Test364State.S3112 -> {
                // SCE-MAP: test364.scxml:68 :: s3112 :: _state_body
            }
            is Test364State.S312 -> {
                // SCE-MAP: test364.scxml:69 :: s312 :: _state_body
            }
            is Test364State.S32 -> {
                // SCE-MAP: test364.scxml:70 :: s32 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test364.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test364State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
