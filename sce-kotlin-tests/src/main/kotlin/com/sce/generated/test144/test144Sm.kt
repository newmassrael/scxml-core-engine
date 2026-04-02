// GENERATED CODE — DO NOT EDIT
// Source: resources/144/test144.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test144

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test144State : State {
    data object Fail : Test144State
    data object Pass : Test144State
    data object S0 : Test144State
    data object S1 : Test144State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test144Event : Event {
    data object Bar : Test144Event
    data object Foo : Test144Event
}
// --- State Machine (W3C SCXML) ---

class Test144StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test144State, Test144Event>(scriptEngine) {

    override val initialState: Test144State = Test144State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test144State,
        event: Test144Event
    ): TransitionResult<Test144State> = when (state) {
        is Test144State.S0 -> processS0(event)
        is Test144State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test144Event
    ): TransitionResult<Test144State> = when {
        event is Test144Event.Foo -> TransitionResult.External(Test144State.S1)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test144State.Fail)
    }

    private fun processS1(
        event: Test144Event
    ): TransitionResult<Test144State> = when {
        event is Test144Event.Bar -> TransitionResult.External(Test144State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test144State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test144State) {
        when (state) {
            is Test144State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test144State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test144State.S0 -> {
            raiseInternal(Test144Event.Foo)
            raiseInternal(Test144Event.Bar)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test144State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test144State,
        event: Test144Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
