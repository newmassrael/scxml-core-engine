// GENERATED CODE — DO NOT EDIT
// Source: resources/216/test216sub1.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test216

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test216sub1State : State {
    data object Final : Test216sub1State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test216sub1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test216sub1StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test216sub1State, Test216sub1Event>(scriptEngine) {

    override val initialState: Test216sub1State = Test216sub1State.Final






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test216sub1State,
        event: Test216sub1Event
    ): TransitionResult<Test216sub1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test216sub1State) {
        when (state) {
            is Test216sub1State.Final -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test216sub1State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test216sub1State,
        event: Test216sub1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
