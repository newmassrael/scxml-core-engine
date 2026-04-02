// GENERATED CODE — DO NOT EDIT
// Source: resources/185/test185.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test185

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test185State : State {
    data object Fail : Test185State
    data object Pass : Test185State
    data object S0 : Test185State
    data object S1 : Test185State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test185Event : Event {
    sealed interface Error : Test185Event {
        data object Execution : Error
    }
    data object Event1 : Test185Event
    data object Event2 : Test185Event
}
// --- State Machine (W3C SCXML) ---

class Test185StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test185State, Test185Event>(scriptEngine) {

    override val initialState: Test185State = Test185State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test185State,
        event: Test185Event
    ): TransitionResult<Test185State> = when (state) {
        is Test185State.S0 -> processS0(event)
        is Test185State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test185Event
    ): TransitionResult<Test185State> = when {
        event is Test185Event.Event1 -> TransitionResult.External(Test185State.S1, Test185State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test185State.Fail)
    }

    private fun processS1(
        event: Test185Event
    ): TransitionResult<Test185State> = when {
        event is Test185Event.Event2 -> TransitionResult.External(Test185State.Pass, Test185State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test185State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test185State) {
        when (state) {
            is Test185State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test185State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test185State.S0 -> {
            scheduleSend("__send_0", 1000L, Test185Event.Event2)
            send(Test185Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test185State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test185State,
        event: Test185Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
