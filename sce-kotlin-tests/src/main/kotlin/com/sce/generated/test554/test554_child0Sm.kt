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
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test554Child0State) {
        when (state) {
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
