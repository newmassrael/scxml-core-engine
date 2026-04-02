// GENERATED CODE — DO NOT EDIT
// Source: resources/215/test215_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test215

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test215Child0State : State {
    data object SubFinal : Test215Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test215Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test215Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test215Child0State, Test215Child0Event>(scriptEngine) {

    override val initialState: Test215Child0State = Test215Child0State.SubFinal






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test215Child0State,
        event: Test215Child0Event
    ): TransitionResult<Test215Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test215Child0State) {
        when (state) {
            is Test215Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test215Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test215Child0State,
        event: Test215Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
