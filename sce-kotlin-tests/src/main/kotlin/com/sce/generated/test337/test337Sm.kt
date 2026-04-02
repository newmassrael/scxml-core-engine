// GENERATED CODE — DO NOT EDIT
// Source: resources/337/test337.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test337

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test337State : State {
    data object Fail : Test337State
    data object Pass : Test337State
    data object S0 : Test337State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test337Event : Event {
    data object Foo : Test337Event
}
// --- State Machine (W3C SCXML) ---

class Test337StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test337State, Test337Event>(scriptEngine) {

    override val initialState: Test337State = Test337State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test337State,
        event: Test337Event
    ): TransitionResult<Test337State> = when (state) {
        is Test337State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test337Event
    ): TransitionResult<Test337State> = when {
        event is Test337Event.Foo -> TransitionResult.External(Test337State.Pass, Test337State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test337State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test337State) {
        when (state) {
            is Test337State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test337State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test337State.S0 -> {
            raiseInternal(Test337Event.Foo)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test337State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test337State,
        event: Test337Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
