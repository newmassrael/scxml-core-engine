// GENERATED CODE — DO NOT EDIT
// Source: resources/415/test415.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test415

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test415State : State {
    data object Final : Test415State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test415Event : Event {
    data object Event1 : Test415Event
}
// --- State Machine (W3C SCXML) ---

class Test415StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test415State, Test415Event>(scriptEngine) {

    override val initialState: Test415State = Test415State.Final






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test415State,
        event: Test415Event
    ): TransitionResult<Test415State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test415State) {
        when (state) {
            is Test415State.Final -> {
            raiseInternal(Test415Event.Event1)
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test415State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test415State,
        event: Test415Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
