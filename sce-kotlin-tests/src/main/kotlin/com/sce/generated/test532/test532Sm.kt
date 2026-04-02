// GENERATED CODE — DO NOT EDIT
// Source: resources/532/test532.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test532

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test532State : State {
    data object Fail : Test532State
    data object Pass : Test532State
    data object S0 : Test532State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test532Event : Event {
    data object Empty : Test532Event
    sealed interface HTTP : Test532Event {
        data object POST : HTTP
    }
    sealed interface Error : Test532Event {
        data object Execution : Error
    }
    data object Timeout : Test532Event
}
// --- State Machine (W3C SCXML) ---

class Test532StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test532State, Test532Event>(scriptEngine) {

    override val initialState: Test532State = Test532State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test532State,
        event: Test532Event
    ): TransitionResult<Test532State> = when (state) {
        is Test532State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test532Event
    ): TransitionResult<Test532State> = when {
        event is Test532Event.HTTP.POST -> TransitionResult.External(Test532State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test532State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test532State) {
        when (state) {
            is Test532State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test532State.S0 -> {
            scheduleSend("__send_0", 3000L, Test532Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test532State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test532State,
        event: Test532Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
