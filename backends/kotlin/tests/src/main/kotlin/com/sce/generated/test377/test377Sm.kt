// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/377/test377.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test377.scxml:5 :: _machine

package com.sce.generated.test377

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test377State : State {
    data object Fail : Test377State
    data object Pass : Test377State
    data object S0 : Test377State
    data object S1 : Test377State
    data object S2 : Test377State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test377Event : Event {
    data object Event1 : Test377Event
    data object Event2 : Test377Event
}
// --- State Machine (W3C SCXML) ---

class Test377StateMachine(
) : StateMachineEngine<Test377State, Test377Event>() {

    override val initialState: Test377State = Test377State.S0

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
    override fun isFinalState(state: Test377State): Boolean = when (state) {
        is Test377State.Fail, is Test377State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test377State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test377State, HistoryId>> =
            listOf(StateTarget(Test377State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test377State, HistoryId>(
            Test377State.S0,
            listOf(StateTarget(Test377State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test377State, HistoryId>(
            Test377State.S1,
            listOf(StateTarget(Test377State.S2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test377State, HistoryId>(
            Test377State.S1,
            listOf(StateTarget(Test377State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 0, as the microstep reads it.
        val transitionS2At0 = EnabledTransition<Test377State, HistoryId>(
            Test377State.S2,
            listOf(StateTarget(Test377State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s2's transition 1, as the microstep reads it.
        val transitionS2At1 = EnabledTransition<Test377State, HistoryId>(
            Test377State.S2,
            listOf(StateTarget(Test377State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test377State? = when (stateId) {
        "fail" -> Test377State.Fail
        "pass" -> Test377State.Pass
        "s0" -> Test377State.S0
        "s1" -> Test377State.S1
        "s2" -> Test377State.S2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test377State): String = when (state) {
        is Test377State.Fail -> "fail"
        is Test377State.Pass -> "pass"
        is Test377State.S0 -> "s0"
        is Test377State.S1 -> "s1"
        is Test377State.S2 -> "s2"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test377State): Int = when (state) {
        is Test377State.Fail -> 4
        is Test377State.Pass -> 3
        is Test377State.S0 -> 0
        is Test377State.S1 -> 1
        is Test377State.S2 -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test377State,
        event: Test377Event?
    ): EnabledTransition<Test377State, HistoryId>? = when (state) {
        is Test377State.S0 -> when {
            event == null -> transitionS0At0
            else -> null
        }
        is Test377State.S1 -> when {
            event is Test377Event.Event1 -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        is Test377State.S2 -> when {
            event is Test377Event.Event2 -> transitionS2At0
            event != null -> transitionS2At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun onEntry(state: Test377State, isDefaultEntry: Boolean) {
        when (state) {
            is Test377State.Fail -> {
                // SCE-MAP: test377.scxml:34 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test377State.Pass -> {
                // SCE-MAP: test377.scxml:33 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test377State.S0 -> {
                // SCE-MAP: test377.scxml:9 :: s0 :: _state_body
            }
            is Test377State.S1 -> {
                // SCE-MAP: test377.scxml:20 :: s1 :: _state_body
            }
            is Test377State.S2 -> {
                // SCE-MAP: test377.scxml:27 :: s2 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun onExit(state: Test377State) {
        when (state) {
            is Test377State.Fail -> {
                // SCE-MAP: test377.scxml:34 :: fail :: _state_body
            }
            is Test377State.Pass -> {
                // SCE-MAP: test377.scxml:33 :: pass :: _state_body
            }
            is Test377State.S0 -> {
                // SCE-MAP: test377.scxml:9 :: s0 :: _state_body
                // W3C SCXML 3.9: Onexit block 1/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test377Event.Event1)
                }
                // W3C SCXML 3.9: Onexit block 2/2
                // C++ EntryExitHelper pattern: each block executes independently
                // Action-level error handling (try-catch in each action) provides isolation
                run {

            raiseInternal(Test377Event.Event2)
                }
            }
            is Test377State.S1 -> {
                // SCE-MAP: test377.scxml:20 :: s1 :: _state_body
            }
            is Test377State.S2 -> {
                // SCE-MAP: test377.scxml:27 :: s2 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test377.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test377State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
