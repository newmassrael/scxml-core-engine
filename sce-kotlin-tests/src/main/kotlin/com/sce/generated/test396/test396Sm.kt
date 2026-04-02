// GENERATED CODE — DO NOT EDIT
// Source: resources/396/test396.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test396

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test396State : State {
    data object Fail : Test396State
    data object Pass : Test396State
    data object S0 : Test396State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test396Event : Event {
    data object Foo : Test396Event
}
// --- State Machine (W3C SCXML) ---

class Test396StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test396State, Test396Event>(scriptEngine) {

    override val initialState: Test396State = Test396State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test396State,
        event: Test396Event
    ): TransitionResult<Test396State> = when (state) {
        is Test396State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test396Event
    ): TransitionResult<Test396State> = when {
        event is Test396Event.Foo -> TransitionResult.External(Test396State.Pass)
        event is Test396Event.Foo -> TransitionResult.External(Test396State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test396State) {
        when (state) {
            is Test396State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.S0 -> {
            raiseInternal(Test396Event.Foo)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test396State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test396State,
        event: Test396Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
