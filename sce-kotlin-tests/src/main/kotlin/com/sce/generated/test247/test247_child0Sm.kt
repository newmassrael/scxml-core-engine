// GENERATED CODE — DO NOT EDIT
// Source: resources/247/test247_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test247

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test247Child0State : State {
    data object SubFinal : Test247Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test247Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test247Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test247Child0State, Test247Child0Event>(scriptEngine) {

    override val initialState: Test247Child0State = Test247Child0State.SubFinal






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test247Child0State,
        event: Test247Child0Event
    ): TransitionResult<Test247Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test247Child0State) {
        when (state) {
            is Test247Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test247Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test247Child0State,
        event: Test247Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
