// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/436/test436.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test436.scxml:3 :: _machine

package com.sce.generated.test436

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test436State : State {
    data object Fail : Test436State
    data object P : Test436State
    data object Pass : Test436State
    data object Ps0 : Test436State
    data object Ps1 : Test436State
    data object S1 : Test436State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test436Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test436StateMachine(
) : StateMachineEngine<Test436State, Test436Event>() {

    override val initialState: Test436State = Test436State.Ps0

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
    override fun parentOf(state: Test436State): Test436State? = when (state) {
        is Test436State.Ps0 -> Test436State.P
        is Test436State.Ps1 -> Test436State.P
        else -> null
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: Test436State): Boolean = when (state) {
        is Test436State.P -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test436State): Boolean = when (state) {
        is Test436State.Fail, is Test436State.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: Test436State): List<Test436State> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test436State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<Test436State, List<Test436State>> = mapOf(
            Test436State.P to listOf(Test436State.Ps0, Test436State.Ps1),
        )

        val documentInitialTargetList: List<EntryTarget<Test436State, HistoryId>> =
            listOf(StateTarget(Test436State.P))

        // W3C SCXML 3.13: ps0's transition 0, as the microstep reads it.
        val transitionPs0At0 = EnabledTransition<Test436State, HistoryId>(
            Test436State.Ps0,
            listOf(StateTarget(Test436State.Fail)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: ps0's transition 1, as the microstep reads it.
        val transitionPs0At1 = EnabledTransition<Test436State, HistoryId>(
            Test436State.Ps0,
            listOf(StateTarget(Test436State.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: ps0's transition 2, as the microstep reads it.
        val transitionPs0At2 = EnabledTransition<Test436State, HistoryId>(
            Test436State.Ps0,
            listOf(StateTarget(Test436State.Fail)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test436State? = when (stateId) {
        "fail" -> Test436State.Fail
        "p" -> Test436State.P
        "pass" -> Test436State.Pass
        "ps0" -> Test436State.Ps0
        "ps1" -> Test436State.Ps1
        "s1" -> Test436State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test436State): String = when (state) {
        is Test436State.Fail -> "fail"
        is Test436State.P -> "p"
        is Test436State.Pass -> "pass"
        is Test436State.Ps0 -> "ps0"
        is Test436State.Ps1 -> "ps1"
        is Test436State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test436State): Int = when (state) {
        is Test436State.Fail -> 5
        is Test436State.P -> 0
        is Test436State.Pass -> 4
        is Test436State.Ps0 -> 1
        is Test436State.Ps1 -> 2
        is Test436State.S1 -> 3
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test436State,
        event: Test436Event?
    ): EnabledTransition<Test436State, HistoryId>? = when (state) {
        is Test436State.Ps0 -> when {
            event == null && isStateActive("s1") -> transitionPs0At0
            event == null && isStateActive("ps1") -> transitionPs0At1
            event == null -> transitionPs0At2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test436.scxml:3 :: _machine
    override fun onEntry(state: Test436State, isDefaultEntry: Boolean) {
        when (state) {
            is Test436State.Fail -> {
                // SCE-MAP: test436.scxml:20 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test436State.P -> {
                // SCE-MAP: test436.scxml:6 :: p :: _state_body
            }
            is Test436State.Pass -> {
                // SCE-MAP: test436.scxml:19 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test436State.Ps0 -> {
                // SCE-MAP: test436.scxml:8 :: ps0 :: _state_body
            }
            is Test436State.Ps1 -> {
                // SCE-MAP: test436.scxml:14 :: ps1 :: _state_body
            }
            is Test436State.S1 -> {
                // SCE-MAP: test436.scxml:17 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test436.scxml:3 :: _machine
    override fun onExit(state: Test436State) {
        when (state) {
            is Test436State.Fail -> {
                // SCE-MAP: test436.scxml:20 :: fail :: _state_body
            }
            is Test436State.P -> {
                // SCE-MAP: test436.scxml:6 :: p :: _state_body
            }
            is Test436State.Pass -> {
                // SCE-MAP: test436.scxml:19 :: pass :: _state_body
            }
            is Test436State.Ps0 -> {
                // SCE-MAP: test436.scxml:8 :: ps0 :: _state_body
            }
            is Test436State.Ps1 -> {
                // SCE-MAP: test436.scxml:14 :: ps1 :: _state_body
            }
            is Test436State.S1 -> {
                // SCE-MAP: test436.scxml:17 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test436.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test436State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
