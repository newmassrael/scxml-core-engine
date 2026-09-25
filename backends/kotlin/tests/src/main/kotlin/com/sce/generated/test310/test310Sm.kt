// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/310/test310.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test310.scxml:5 :: _machine

package com.sce.generated.test310

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test310State : State {
    data object Fail : Test310State
    data object P : Test310State
    data object Pass : Test310State
    data object S0 : Test310State
    data object S1 : Test310State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test310Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test310StateMachine(
) : StateMachineEngine<Test310State, Test310Event>() {

    override val initialState: Test310State = Test310State.S0

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
    override fun parentOf(state: Test310State): Test310State? = when (state) {
        is Test310State.S0 -> Test310State.P
        is Test310State.S1 -> Test310State.P
        else -> null
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test310State): Boolean = when (state) {
        is Test310State.P -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test310State): Boolean = when (state) {
        is Test310State.Fail, is Test310State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test310State): List<Test310State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test310State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test310State, List<Test310State>> = mapOf(
            Test310State.P to listOf(Test310State.S0, Test310State.S1),
        )

        val documentInitialTargetList: List<EntryTarget<Test310State, HistoryId>> =
            listOf(StateTarget(Test310State.P))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test310State, HistoryId>(
            Test310State.S0,
            listOf(StateTarget(Test310State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test310State, HistoryId>(
            Test310State.S0,
            listOf(StateTarget(Test310State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test310State? = when (stateId) {
        "fail" -> Test310State.Fail
        "p" -> Test310State.P
        "pass" -> Test310State.Pass
        "s0" -> Test310State.S0
        "s1" -> Test310State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test310State): String = when (state) {
        is Test310State.Fail -> "fail"
        is Test310State.P -> "p"
        is Test310State.Pass -> "pass"
        is Test310State.S0 -> "s0"
        is Test310State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test310State): Int = when (state) {
        is Test310State.Fail -> 4
        is Test310State.P -> 0
        is Test310State.Pass -> 3
        is Test310State.S0 -> 1
        is Test310State.S1 -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test310State,
        event: Test310Event?
    ): EnabledTransition<Test310State, HistoryId>? = when (state) {
        is Test310State.S0 -> when {
            event == null && isStateActive("s1") -> transitionS0At0
            event == null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test310.scxml:5 :: _machine
    override fun onEntry(state: Test310State, isDefaultEntry: Boolean) {
        when (state) {
            is Test310State.Fail -> {
                // SCE-MAP: test310.scxml:19 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test310State.P -> {
                // SCE-MAP: test310.scxml:8 :: p :: _state_body
            }
            is Test310State.Pass -> {
                // SCE-MAP: test310.scxml:18 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test310State.S0 -> {
                // SCE-MAP: test310.scxml:10 :: s0 :: _state_body
            }
            is Test310State.S1 -> {
                // SCE-MAP: test310.scxml:15 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test310.scxml:5 :: _machine
    override fun onExit(state: Test310State) {
        when (state) {
            is Test310State.Fail -> {
                // SCE-MAP: test310.scxml:19 :: fail :: _state_body
            }
            is Test310State.P -> {
                // SCE-MAP: test310.scxml:8 :: p :: _state_body
            }
            is Test310State.Pass -> {
                // SCE-MAP: test310.scxml:18 :: pass :: _state_body
            }
            is Test310State.S0 -> {
                // SCE-MAP: test310.scxml:10 :: s0 :: _state_body
            }
            is Test310State.S1 -> {
                // SCE-MAP: test310.scxml:15 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test310.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test310State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
