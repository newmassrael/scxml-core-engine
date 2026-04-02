// GENERATED CODE — DO NOT EDIT
// Source: resources/208/test208.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test208

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test208State : State {
    data object Fail : Test208State
    data object Pass : Test208State
    data object S0 : Test208State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test208Event : Event {
    sealed interface Error : Test208Event {
        data object Execution : Error
    }
    data object Event1 : Test208Event
    data object Event2 : Test208Event
}
// --- State Machine (W3C SCXML) ---

class Test208StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test208State, Test208Event>(scriptEngine) {

    override val initialState: Test208State = Test208State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test208State,
        event: Test208Event
    ): TransitionResult<Test208State> = when (state) {
        is Test208State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test208Event
    ): TransitionResult<Test208State> = when {
        event is Test208Event.Event2 -> TransitionResult.External(Test208State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test208State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test208State) {
        when (state) {
            is Test208State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.S0 -> {
            scheduleSend("foo", 1000L, Test208Event.Event1)
            scheduleSend("__send_0", 1500L, Test208Event.Event2)
            cancelSend("foo")
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test208State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test208State,
        event: Test208Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
