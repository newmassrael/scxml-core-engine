// GENERATED CODE — DO NOT EDIT
// Source: resources/375/test375.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test375

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test375State : State {
    data object Fail : Test375State
    data object Pass : Test375State
    data object S0 : Test375State
    data object S1 : Test375State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test375Event : Event {
    data object Event1 : Test375Event
    data object Event2 : Test375Event
}
// --- State Machine (W3C SCXML) ---

class Test375StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test375State, Test375Event>(scriptEngine) {

    override val initialState: Test375State = Test375State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test375State,
        event: Test375Event
    ): TransitionResult<Test375State> = when (state) {
        is Test375State.S0 -> processS0(event)
        is Test375State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test375Event
    ): TransitionResult<Test375State> = when {
        event is Test375Event.Event1 -> TransitionResult.External(Test375State.S1, Test375State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test375State.Fail)
    }

    private fun processS1(
        event: Test375Event
    ): TransitionResult<Test375State> = when {
        event is Test375Event.Event2 -> TransitionResult.External(Test375State.Pass, Test375State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test375State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test375State) {
        when (state) {
            is Test375State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test375State.S0 -> {
            raiseInternal(Test375Event.Event1)
            raiseInternal(Test375Event.Event2)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test375State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test375State,
        event: Test375Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
