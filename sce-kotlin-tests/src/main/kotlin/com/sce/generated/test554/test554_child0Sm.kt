// GENERATED CODE — DO NOT EDIT
// Source: resources/554/test554_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test554

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test554Child0State : State {
    data object SubFinal : Test554Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test554Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test554Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test554Child0State, Test554Child0Event>(scriptEngine) {

    override val initialState: Test554Child0State = Test554Child0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test554Child0State? = when (stateId) {
        "subFinal" -> Test554Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test554Child0State): String = when (state) {
        is Test554Child0State.SubFinal -> "subFinal"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test554Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test554Child0State): Int = when (state) {
        is Test554Child0State.SubFinal -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test554Child0State,
        event: Test554Child0Event
    ): TransitionResult<Test554Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test554Child0State) {
        when (state) {
            is Test554Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test554Child0State) {
        when (state) {
            is Test554Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test554Child0State,
        event: Test554Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
