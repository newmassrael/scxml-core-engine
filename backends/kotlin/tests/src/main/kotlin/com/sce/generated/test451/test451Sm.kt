// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/451/test451.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test451.scxml:5 :: _machine

package com.sce.generated.test451

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test451State : State {
    data object Fail : Test451State
    data object P : Test451State
    data object Pass : Test451State
    data object S0 : Test451State
    data object S1 : Test451State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test451Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test451StateMachine(
) : StateMachineEngine<Test451State, Test451Event>() {

    override val initialState: Test451State = Test451State.S0

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
    override fun parentOf(state: Test451State): Test451State? = when (state) {
        is Test451State.S0 -> Test451State.P
        is Test451State.S1 -> Test451State.P
        else -> null
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test451State): Boolean = when (state) {
        is Test451State.P -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test451State): Boolean = when (state) {
        is Test451State.Fail, is Test451State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test451State): List<Test451State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test451State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test451State, List<Test451State>> = mapOf(
            Test451State.P to listOf(Test451State.S0, Test451State.S1),
        )

        val documentInitialTargetList: List<EntryTarget<Test451State, HistoryId>> =
            listOf(StateTarget(Test451State.P))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test451State, HistoryId>(
            Test451State.S0,
            listOf(StateTarget(Test451State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test451State, HistoryId>(
            Test451State.S0,
            listOf(StateTarget(Test451State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test451State? = when (stateId) {
        "fail" -> Test451State.Fail
        "p" -> Test451State.P
        "pass" -> Test451State.Pass
        "s0" -> Test451State.S0
        "s1" -> Test451State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test451State): String = when (state) {
        is Test451State.Fail -> "fail"
        is Test451State.P -> "p"
        is Test451State.Pass -> "pass"
        is Test451State.S0 -> "s0"
        is Test451State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test451State): Int = when (state) {
        is Test451State.Fail -> 4
        is Test451State.P -> 0
        is Test451State.Pass -> 3
        is Test451State.S0 -> 1
        is Test451State.S1 -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test451State,
        event: Test451Event?
    ): EnabledTransition<Test451State, HistoryId>? = when (state) {
        is Test451State.S0 -> when {
            event == null && isStateActive("s1") -> transitionS0At0
            event == null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test451.scxml:5 :: _machine
    override fun onEntry(state: Test451State, isDefaultEntry: Boolean) {
        when (state) {
            is Test451State.Fail -> {
                // SCE-MAP: test451.scxml:19 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test451State.P -> {
                // SCE-MAP: test451.scxml:8 :: p :: _state_body
            }
            is Test451State.Pass -> {
                // SCE-MAP: test451.scxml:18 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test451State.S0 -> {
                // SCE-MAP: test451.scxml:10 :: s0 :: _state_body
            }
            is Test451State.S1 -> {
                // SCE-MAP: test451.scxml:15 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test451.scxml:5 :: _machine
    override fun onExit(state: Test451State) {
        when (state) {
            is Test451State.Fail -> {
                // SCE-MAP: test451.scxml:19 :: fail :: _state_body
            }
            is Test451State.P -> {
                // SCE-MAP: test451.scxml:8 :: p :: _state_body
            }
            is Test451State.Pass -> {
                // SCE-MAP: test451.scxml:18 :: pass :: _state_body
            }
            is Test451State.S0 -> {
                // SCE-MAP: test451.scxml:10 :: s0 :: _state_body
            }
            is Test451State.S1 -> {
                // SCE-MAP: test451.scxml:15 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test451.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test451State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
