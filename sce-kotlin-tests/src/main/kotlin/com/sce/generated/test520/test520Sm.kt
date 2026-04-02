// GENERATED CODE — DO NOT EDIT
// Source: resources/520/test520.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test520

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test520State : State {
    data object Fail : Test520State
    data object Pass : Test520State
    data object S0 : Test520State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test520Event : Event {
    data object Empty : Test520Event
    sealed interface HTTP : Test520Event {
        data object POST : HTTP
    }
    sealed interface Error : Test520Event {
        data object Execution : Error
    }
    data object Timeout : Test520Event
}
// --- State Machine (W3C SCXML) ---

class Test520StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test520State, Test520Event>(scriptEngine) {

    override val initialState: Test520State = Test520State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test520State,
        event: Test520Event
    ): TransitionResult<Test520State> = when (state) {
        is Test520State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test520Event
    ): TransitionResult<Test520State> = when {
        event is Test520Event.HTTP.POST -> TransitionResult.External(Test520State.Pass)
        event is Test520Event.HTTP.POST -> TransitionResult.External(Test520State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test520State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test520State) {
        when (state) {
            is Test520State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test520State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test520State.S0 -> {
            scheduleSend("__send_0", 30000L, Test520Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test520State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test520State,
        event: Test520Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
