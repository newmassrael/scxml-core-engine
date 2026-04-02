// GENERATED CODE — DO NOT EDIT
// Source: resources/239/test239_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test239

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test239Child0State : State {
    data object Final : Test239Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test239Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test239Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test239Child0State, Test239Child0Event>(scriptEngine) {

    override val initialState: Test239Child0State = Test239Child0State.Final



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test239Child0State? = when (stateId) {
        "final" -> Test239Child0State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test239Child0State): String = when (state) {
        is Test239Child0State.Final -> "final"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test239Child0State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test239Child0State): Int = when (state) {
        is Test239Child0State.Final -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test239Child0State,
        event: Test239Child0Event
    ): TransitionResult<Test239Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test239Child0State) {
        when (state) {
            is Test239Child0State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test239Child0State) {
        when (state) {
            is Test239Child0State.Final -> {
                activeStateIds.remove("final")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test239Child0State,
        event: Test239Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
