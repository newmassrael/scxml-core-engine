// GENERATED CODE — DO NOT EDIT
// Source: resources/330/test330.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test330

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test330State : State {
    data object Fail : Test330State
    data object Pass : Test330State
    data object S0 : Test330State
    data object S1 : Test330State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test330Event : Event {
    sealed interface Error : Test330Event {
        data object Execution : Error
    }
    data object Foo : Test330Event
}
// --- State Machine (W3C SCXML) ---

class Test330StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test330State, Test330Event>(scriptEngine) {

    override val initialState: Test330State = Test330State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test330State,
        event: Test330Event
    ): TransitionResult<Test330State> = when (state) {
        is Test330State.S0 -> processS0(event)
        is Test330State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test330Event
    ): TransitionResult<Test330State> = when {
        event is Test330Event.Foo -> TransitionResult.External(Test330State.S1, Test330State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test330State.Fail)
    }

    private fun processS1(
        event: Test330Event
    ): TransitionResult<Test330State> = when {
        event is Test330Event.Foo -> TransitionResult.External(Test330State.Pass, Test330State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test330State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test330State) {
        when (state) {
            is Test330State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.S0 -> {
            raiseInternal(Test330Event.Foo)
            }
            is Test330State.S1 -> {
            send(Test330Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test330State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test330State,
        event: Test330Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
