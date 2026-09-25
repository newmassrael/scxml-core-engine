// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/375/test375.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test375.scxml:5 :: _machine

package com.sce.generated.test375

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test375State : State {
    data object Fail : Test375State
    data object Pass : Test375State
    data object S0 : Test375State
    data object S1 : Test375State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test375Event : Event {
    data object Event1 : Test375Event
    data object Event2 : Test375Event
}
// --- State Machine (W3C SCXML) ---

class Test375StateMachine(
) : StateMachineEngine<Test375State, Test375Event>() {

    override val initialState: Test375State = Test375State.S0

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test375State): Boolean = when (state) {
        is Test375State.Fail, is Test375State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test375State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test375State, HistoryId>> =
            listOf(StateTarget(Test375State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test375State, HistoryId>(
            Test375State.S0,
            listOf(StateTarget(Test375State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test375State, HistoryId>(
            Test375State.S0,
            listOf(StateTarget(Test375State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test375State, HistoryId>(
            Test375State.S1,
            listOf(StateTarget(Test375State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test375State, HistoryId>(
            Test375State.S1,
            listOf(StateTarget(Test375State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test375State? = when (stateId) {
        "fail" -> Test375State.Fail
        "pass" -> Test375State.Pass
        "s0" -> Test375State.S0
        "s1" -> Test375State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test375State): String = when (state) {
        is Test375State.Fail -> "fail"
        is Test375State.Pass -> "pass"
        is Test375State.S0 -> "s0"
        is Test375State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test375State): Int = when (state) {
        is Test375State.Fail -> 3
        is Test375State.Pass -> 2
        is Test375State.S0 -> 0
        is Test375State.S1 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test375State,
        event: Test375Event?
    ): EnabledTransition<Test375State, HistoryId>? = when (state) {
        is Test375State.S0 -> when {
            event is Test375Event.Event1 -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        is Test375State.S1 -> when {
            event is Test375Event.Event2 -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test375.scxml:5 :: _machine
    override fun onEntry(state: Test375State, isDefaultEntry: Boolean) {
        when (state) {
            is Test375State.Fail -> {
                // SCE-MAP: test375.scxml:29 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.Pass -> {
                // SCE-MAP: test375.scxml:28 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.S0 -> {
                // SCE-MAP: test375.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test375Event.Event1)
                }
                // W3C SCXML 3.8: Onentry block 2/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test375Event.Event2)
                }
            }
            is Test375State.S1 -> {
                // SCE-MAP: test375.scxml:22 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test375.scxml:5 :: _machine
    override fun onExit(state: Test375State) {
        when (state) {
            is Test375State.Fail -> {
                // SCE-MAP: test375.scxml:29 :: fail :: _state_body
            }
            is Test375State.Pass -> {
                // SCE-MAP: test375.scxml:28 :: pass :: _state_body
            }
            is Test375State.S0 -> {
                // SCE-MAP: test375.scxml:9 :: s0 :: _state_body
            }
            is Test375State.S1 -> {
                // SCE-MAP: test375.scxml:22 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test375.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test375State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
