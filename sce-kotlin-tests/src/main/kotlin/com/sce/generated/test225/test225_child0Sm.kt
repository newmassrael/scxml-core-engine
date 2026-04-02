// GENERATED CODE — DO NOT EDIT
// Source: resources/225/test225_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test225

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test225Child0State : State {
    data object SubFinal1 : Test225Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test225Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test225Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test225Child0State, Test225Child0Event>(scriptEngine) {

    override val initialState: Test225Child0State = Test225Child0State.SubFinal1






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test225Child0State,
        event: Test225Child0Event
    ): TransitionResult<Test225Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test225Child0State) {
        when (state) {
            is Test225Child0State.SubFinal1 -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test225Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test225Child0State,
        event: Test225Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
