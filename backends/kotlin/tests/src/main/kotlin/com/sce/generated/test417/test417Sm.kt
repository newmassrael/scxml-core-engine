// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/417/test417.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test417.scxml:7 :: _machine

package com.sce.generated.test417

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test417State : State {
    data object Fail : Test417State
    data object Pass : Test417State
    data object S1 : Test417State
    data object S1p1 : Test417State
    data object S1p11 : Test417State
    data object S1p111 : Test417State
    data object S1p11final : Test417State
    data object S1p12 : Test417State
    data object S1p121 : Test417State
    data object S1p12final : Test417State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test417Event : Event {
    sealed interface Done : Test417Event {
        sealed interface State : Done {
            data object S1p1 : State
            data object S1p11 : State
            data object S1p12 : State
        }
    }
    sealed interface Error : Test417Event {
        data object Execution : Error
    }
    data object Timeout : Test417Event
}
// --- State Machine (W3C SCXML) ---

class Test417StateMachine(
) : StateMachineEngine<Test417State, Test417Event>() {

    override val initialState: Test417State = Test417State.S1p111

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
    override fun parentOf(state: Test417State): Test417State? = when (state) {
        is Test417State.S1p1 -> Test417State.S1
        is Test417State.S1p11 -> Test417State.S1p1
        is Test417State.S1p111 -> Test417State.S1p11
        is Test417State.S1p11final -> Test417State.S1p11
        is Test417State.S1p12 -> Test417State.S1p1
        is Test417State.S1p121 -> Test417State.S1p12
        is Test417State.S1p12final -> Test417State.S1p12
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: Test417State): Boolean = when (state) {
        is Test417State.S1, is Test417State.S1p11, is Test417State.S1p12 -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test417State): Boolean = when (state) {
        is Test417State.S1p1 -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test417State): Boolean = when (state) {
        is Test417State.Fail, is Test417State.Pass, is Test417State.S1p11final, is Test417State.S1p12final -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test417State): List<Test417State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: Test417State): List<EntryTarget<Test417State, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test417State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test417State, List<Test417State>> = mapOf(
            Test417State.S1 to listOf(Test417State.S1p1),
            Test417State.S1p1 to listOf(Test417State.S1p11, Test417State.S1p12),
            Test417State.S1p11 to listOf(Test417State.S1p111, Test417State.S1p11final),
            Test417State.S1p12 to listOf(Test417State.S1p121, Test417State.S1p12final),
        )

        val initialTargets: Map<Test417State, List<EntryTarget<Test417State, HistoryId>>> = mapOf(
            Test417State.S1 to listOf(StateTarget(Test417State.S1p1)),
            Test417State.S1p11 to listOf(StateTarget(Test417State.S1p111)),
            Test417State.S1p12 to listOf(StateTarget(Test417State.S1p121)),
        )

        val documentInitialTargetList: List<EntryTarget<Test417State, HistoryId>> =
            listOf(StateTarget(Test417State.S1))

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test417State, HistoryId>(
            Test417State.S1,
            listOf(StateTarget(Test417State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1p1's transition 0, as the microstep reads it.
        val transitionS1p1At0 = EnabledTransition<Test417State, HistoryId>(
            Test417State.S1p1,
            listOf(StateTarget(Test417State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1p111's transition 0, as the microstep reads it.
        val transitionS1p111At0 = EnabledTransition<Test417State, HistoryId>(
            Test417State.S1p111,
            listOf(StateTarget(Test417State.S1p11final)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1p121's transition 0, as the microstep reads it.
        val transitionS1p121At0 = EnabledTransition<Test417State, HistoryId>(
            Test417State.S1p121,
            listOf(StateTarget(Test417State.S1p12final)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test417State? = when (stateId) {
        "fail" -> Test417State.Fail
        "pass" -> Test417State.Pass
        "s1" -> Test417State.S1
        "s1p1" -> Test417State.S1p1
        "s1p11" -> Test417State.S1p11
        "s1p111" -> Test417State.S1p111
        "s1p11final" -> Test417State.S1p11final
        "s1p12" -> Test417State.S1p12
        "s1p121" -> Test417State.S1p121
        "s1p12final" -> Test417State.S1p12final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test417State): String = when (state) {
        is Test417State.Fail -> "fail"
        is Test417State.Pass -> "pass"
        is Test417State.S1 -> "s1"
        is Test417State.S1p1 -> "s1p1"
        is Test417State.S1p11 -> "s1p11"
        is Test417State.S1p111 -> "s1p111"
        is Test417State.S1p11final -> "s1p11final"
        is Test417State.S1p12 -> "s1p12"
        is Test417State.S1p121 -> "s1p121"
        is Test417State.S1p12final -> "s1p12final"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test417State): Int = when (state) {
        is Test417State.Fail -> 9
        is Test417State.Pass -> 8
        is Test417State.S1 -> 0
        is Test417State.S1p1 -> 1
        is Test417State.S1p11 -> 2
        is Test417State.S1p111 -> 3
        is Test417State.S1p11final -> 4
        is Test417State.S1p12 -> 5
        is Test417State.S1p121 -> 6
        is Test417State.S1p12final -> 7
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test417State,
        event: Test417Event?
    ): EnabledTransition<Test417State, HistoryId>? = when (state) {
        is Test417State.S1 -> when {
            event is Test417Event.Timeout -> transitionS1At0
            else -> null
        }
        is Test417State.S1p1 -> when {
            event is Test417Event.Done.State.S1p1 -> transitionS1p1At0
            else -> null
        }
        is Test417State.S1p111 -> when {
            event == null -> transitionS1p111At0
            else -> null
        }
        is Test417State.S1p121 -> when {
            event == null -> transitionS1p121At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test417.scxml:7 :: _machine
    override fun onEntry(state: Test417State, isDefaultEntry: Boolean) {
        when (state) {
            is Test417State.Fail -> {
                // SCE-MAP: test417.scxml:37 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test417State.Pass -> {
                // SCE-MAP: test417.scxml:36 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test417State.S1 -> {
                // SCE-MAP: test417.scxml:9 :: s1 :: _state_body


            scheduleSend("__send_0", 1000L, Test417Event.Timeout)
            }
            is Test417State.S1p1 -> {
                // SCE-MAP: test417.scxml:15 :: s1p1 :: _state_body
            }
            is Test417State.S1p11 -> {
                // SCE-MAP: test417.scxml:18 :: s1p11 :: _state_body
            }
            is Test417State.S1p111 -> {
                // SCE-MAP: test417.scxml:19 :: s1p111 :: _state_body
            }
            is Test417State.S1p11final -> {
                // SCE-MAP: test417.scxml:22 :: s1p11final :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test417Event.Done.State.S1p11, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(Test417State.S1p1)) {
                    raiseInternal(Test417Event.Done.State.S1p1)
                }
            }
            is Test417State.S1p12 -> {
                // SCE-MAP: test417.scxml:25 :: s1p12 :: _state_body
            }
            is Test417State.S1p121 -> {
                // SCE-MAP: test417.scxml:26 :: s1p121 :: _state_body
            }
            is Test417State.S1p12final -> {
                // SCE-MAP: test417.scxml:29 :: s1p12final :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(Test417Event.Done.State.S1p12, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(Test417State.S1p1)) {
                    raiseInternal(Test417Event.Done.State.S1p1)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test417.scxml:7 :: _machine
    override fun onExit(state: Test417State) {
        when (state) {
            is Test417State.Fail -> {
                // SCE-MAP: test417.scxml:37 :: fail :: _state_body
            }
            is Test417State.Pass -> {
                // SCE-MAP: test417.scxml:36 :: pass :: _state_body
            }
            is Test417State.S1 -> {
                // SCE-MAP: test417.scxml:9 :: s1 :: _state_body
            }
            is Test417State.S1p1 -> {
                // SCE-MAP: test417.scxml:15 :: s1p1 :: _state_body
            }
            is Test417State.S1p11 -> {
                // SCE-MAP: test417.scxml:18 :: s1p11 :: _state_body
            }
            is Test417State.S1p111 -> {
                // SCE-MAP: test417.scxml:19 :: s1p111 :: _state_body
            }
            is Test417State.S1p11final -> {
                // SCE-MAP: test417.scxml:22 :: s1p11final :: _state_body
            }
            is Test417State.S1p12 -> {
                // SCE-MAP: test417.scxml:25 :: s1p12 :: _state_body
            }
            is Test417State.S1p121 -> {
                // SCE-MAP: test417.scxml:26 :: s1p121 :: _state_body
            }
            is Test417State.S1p12final -> {
                // SCE-MAP: test417.scxml:29 :: s1p12final :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test417.scxml:7 :: _machine
    override fun executeTransitionContent(source: Test417State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
