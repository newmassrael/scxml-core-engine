// GENERATED CODE — DO NOT EDIT
// Source: resources/339/test339.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test339

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test339State : State {
    data object Fail : Test339State
    data object Pass : Test339State
    data object S0 : Test339State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test339Event : Event {
    data object Foo : Test339Event
}
// --- State Machine (W3C SCXML) ---

class Test339StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test339State, Test339Event>(scriptEngine) {

    override val initialState: Test339State = Test339State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test339State,
        event: Test339Event
    ): TransitionResult<Test339State> = when (state) {
        is Test339State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test339Event
    ): TransitionResult<Test339State> = when {
        event is Test339Event.Foo -> TransitionResult.External(Test339State.Pass, Test339State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test339State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test339State) {
        when (state) {
            is Test339State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test339State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test339State.S0 -> {
            raiseInternal(Test339Event.Foo)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test339State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test339State,
        event: Test339Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
