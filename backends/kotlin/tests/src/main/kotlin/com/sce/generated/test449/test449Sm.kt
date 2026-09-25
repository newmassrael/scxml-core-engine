// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/449/test449.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test449.scxml:5 :: _machine

package com.sce.generated.test449

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test449State : State {
    data object Fail : Test449State
    data object Pass : Test449State
    data object S0 : Test449State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test449Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test449StateMachine(
) : StateMachineEngine<Test449State, Test449Event>() {

    override val initialState: Test449State = Test449State.S0

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
    override fun isFinalState(state: Test449State): Boolean = when (state) {
        is Test449State.Fail, is Test449State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test449State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test449State, HistoryId>> =
            listOf(StateTarget(Test449State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test449State, HistoryId>(
            Test449State.S0,
            listOf(StateTarget(Test449State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test449State, HistoryId>(
            Test449State.S0,
            listOf(StateTarget(Test449State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test449State? = when (stateId) {
        "fail" -> Test449State.Fail
        "pass" -> Test449State.Pass
        "s0" -> Test449State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test449State): String = when (state) {
        is Test449State.Fail -> "fail"
        is Test449State.Pass -> "pass"
        is Test449State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test449State): Int = when (state) {
        is Test449State.Fail -> 2
        is Test449State.Pass -> 1
        is Test449State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test449State,
        event: Test449Event?
    ): EnabledTransition<Test449State, HistoryId>? = when (state) {
        is Test449State.S0 -> when {
            event == null -> transitionS0At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun onEntry(state: Test449State, isDefaultEntry: Boolean) {
        when (state) {
            is Test449State.Fail -> {
                // SCE-MAP: test449.scxml:14 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test449State.Pass -> {
                // SCE-MAP: test449.scxml:13 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test449State.S0 -> {
                // SCE-MAP: test449.scxml:8 :: s0 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun onExit(state: Test449State) {
        when (state) {
            is Test449State.Fail -> {
                // SCE-MAP: test449.scxml:14 :: fail :: _state_body
            }
            is Test449State.Pass -> {
                // SCE-MAP: test449.scxml:13 :: pass :: _state_body
            }
            is Test449State.S0 -> {
                // SCE-MAP: test449.scxml:8 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test449.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test449State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
