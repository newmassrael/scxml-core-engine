// GENERATED CODE — DO NOT EDIT
// Source: resources/228/test228_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test228

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test228Child0State : State {
    data object SubFinal : Test228Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test228Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test228Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test228Child0State, Test228Child0Event>(scriptEngine) {

    override val initialState: Test228Child0State = Test228Child0State.SubFinal






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test228Child0State,
        event: Test228Child0Event
    ): TransitionResult<Test228Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test228Child0State) {
        when (state) {
            is Test228Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test228Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test228Child0State,
        event: Test228Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
