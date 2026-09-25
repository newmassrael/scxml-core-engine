// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/355/test355.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test355.scxml:5 :: _machine

package com.sce.generated.test355

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test355State : State {
    data object Fail : Test355State
    data object Pass : Test355State
    data object S0 : Test355State
    data object S1 : Test355State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test355Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test355StateMachine(
) : StateMachineEngine<Test355State, Test355Event>() {

    override val initialState: Test355State = Test355State.S0

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
    override fun isFinalState(state: Test355State): Boolean = when (state) {
        is Test355State.Fail, is Test355State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test355State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test355State, HistoryId>> =
            listOf(StateTarget(Test355State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test355State, HistoryId>(
            Test355State.S0,
            listOf(StateTarget(Test355State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test355State, HistoryId>(
            Test355State.S1,
            listOf(StateTarget(Test355State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test355State? = when (stateId) {
        "fail" -> Test355State.Fail
        "pass" -> Test355State.Pass
        "s0" -> Test355State.S0
        "s1" -> Test355State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test355State): String = when (state) {
        is Test355State.Fail -> "fail"
        is Test355State.Pass -> "pass"
        is Test355State.S0 -> "s0"
        is Test355State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test355State): Int = when (state) {
        is Test355State.Fail -> 3
        is Test355State.Pass -> 2
        is Test355State.S0 -> 0
        is Test355State.S1 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test355State,
        event: Test355Event?
    ): EnabledTransition<Test355State, HistoryId>? = when (state) {
        is Test355State.S0 -> when {
            event == null -> transitionS0At0
            else -> null
        }
        is Test355State.S1 -> when {
            event == null -> transitionS1At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test355.scxml:5 :: _machine
    override fun onEntry(state: Test355State, isDefaultEntry: Boolean) {
        when (state) {
            is Test355State.Fail -> {
                // SCE-MAP: test355.scxml:17 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test355State.Pass -> {
                // SCE-MAP: test355.scxml:16 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test355State.S0 -> {
                // SCE-MAP: test355.scxml:8 :: s0 :: _state_body
            }
            is Test355State.S1 -> {
                // SCE-MAP: test355.scxml:12 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test355.scxml:5 :: _machine
    override fun onExit(state: Test355State) {
        when (state) {
            is Test355State.Fail -> {
                // SCE-MAP: test355.scxml:17 :: fail :: _state_body
            }
            is Test355State.Pass -> {
                // SCE-MAP: test355.scxml:16 :: pass :: _state_body
            }
            is Test355State.S0 -> {
                // SCE-MAP: test355.scxml:8 :: s0 :: _state_body
            }
            is Test355State.S1 -> {
                // SCE-MAP: test355.scxml:12 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test355.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test355State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
