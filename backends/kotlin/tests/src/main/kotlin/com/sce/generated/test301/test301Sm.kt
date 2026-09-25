// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/301/test301.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test301.scxml:6 :: _machine

package com.sce.generated.test301

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test301State : State {
    data object Fail : Test301State
    data object Pass : Test301State
    data object S0 : Test301State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test301Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test301StateMachine(
) : StateMachineEngine<Test301State, Test301Event>() {

    override val initialState: Test301State = Test301State.Pass

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
    override fun isFinalState(state: Test301State): Boolean = when (state) {
        is Test301State.Fail, is Test301State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test301State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test301State, HistoryId>> =
            listOf(StateTarget(Test301State.Pass))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test301State, HistoryId>(
            Test301State.S0,
            listOf(StateTarget(Test301State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test301State? = when (stateId) {
        "fail" -> Test301State.Fail
        "pass" -> Test301State.Pass
        "s0" -> Test301State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test301State): String = when (state) {
        is Test301State.Fail -> "fail"
        is Test301State.Pass -> "pass"
        is Test301State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test301State): Int = when (state) {
        is Test301State.Fail -> 2
        is Test301State.Pass -> 1
        is Test301State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test301State,
        event: Test301Event?
    ): EnabledTransition<Test301State, HistoryId>? = when (state) {
        is Test301State.S0 -> when {
            event == null -> transitionS0At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test301.scxml:6 :: _machine
    override fun onEntry(state: Test301State, isDefaultEntry: Boolean) {
        when (state) {
            is Test301State.Fail -> {
                // SCE-MAP: test301.scxml:14 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test301State.Pass -> {
                // SCE-MAP: test301.scxml:13 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test301State.S0 -> {
                // SCE-MAP: test301.scxml:9 :: s0 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test301.scxml:6 :: _machine
    override fun onExit(state: Test301State) {
        when (state) {
            is Test301State.Fail -> {
                // SCE-MAP: test301.scxml:14 :: fail :: _state_body
            }
            is Test301State.Pass -> {
                // SCE-MAP: test301.scxml:13 :: pass :: _state_body
            }
            is Test301State.S0 -> {
                // SCE-MAP: test301.scxml:9 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test301.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test301State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
