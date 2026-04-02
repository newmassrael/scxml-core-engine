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
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test239Child0State) {
        when (state) {
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
