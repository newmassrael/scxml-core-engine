// GENERATED CODE — DO NOT EDIT
// Source: resources/335/test335.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test335

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test335State : State {
    data object Fail : Test335State
    data object Pass : Test335State
    data object S0 : Test335State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test335Event : Event {
    data object Foo : Test335Event
}
// --- State Machine (W3C SCXML) ---

class Test335StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test335State, Test335Event>(scriptEngine) {

    override val initialState: Test335State = Test335State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test335State,
        event: Test335Event
    ): TransitionResult<Test335State> = when (state) {
        is Test335State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test335Event
    ): TransitionResult<Test335State> = when {
        event is Test335Event.Foo -> TransitionResult.External(Test335State.Pass, Test335State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test335State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test335State) {
        when (state) {
            is Test335State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test335State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test335State.S0 -> {
            raiseInternal(Test335Event.Foo)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test335State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test335State,
        event: Test335Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
