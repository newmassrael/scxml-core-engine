// GENERATED CODE — DO NOT EDIT
// Source: resources/239/test239sub1.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test239

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test239sub1State : State {
    data object Final : Test239sub1State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test239sub1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test239sub1StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test239sub1State, Test239sub1Event>(scriptEngine) {

    override val initialState: Test239sub1State = Test239sub1State.Final






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test239sub1State,
        event: Test239sub1Event
    ): TransitionResult<Test239sub1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test239sub1State) {
        when (state) {
            is Test239sub1State.Final -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test239sub1State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test239sub1State,
        event: Test239sub1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
