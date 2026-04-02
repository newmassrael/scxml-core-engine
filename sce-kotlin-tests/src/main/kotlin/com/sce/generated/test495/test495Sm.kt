// GENERATED CODE — DO NOT EDIT
// Source: resources/495/test495.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test495

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test495State : State {
    data object Fail : Test495State
    data object Pass : Test495State
    data object S0 : Test495State
    data object S1 : Test495State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test495Event : Event {
    sealed interface Error : Test495Event {
        data object Execution : Error
    }
    data object Event1 : Test495Event
    data object Event2 : Test495Event
}
// --- State Machine (W3C SCXML) ---

class Test495StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test495State, Test495Event>(scriptEngine) {

    override val initialState: Test495State = Test495State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test495State,
        event: Test495Event
    ): TransitionResult<Test495State> = when (state) {
        is Test495State.S0 -> processS0(event)
        is Test495State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test495Event
    ): TransitionResult<Test495State> = when {
        event is Test495Event.Event1 -> TransitionResult.External(Test495State.Fail)
        event is Test495Event.Event2 -> TransitionResult.External(Test495State.S1)
        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test495Event
    ): TransitionResult<Test495State> = when {
        event is Test495Event.Event1 -> TransitionResult.External(Test495State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test495State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test495State) {
        when (state) {
            is Test495State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test495State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test495State.S0 -> {
            send(Test495Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            raiseInternal(Test495Event.Event2)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test495State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test495State,
        event: Test495Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
