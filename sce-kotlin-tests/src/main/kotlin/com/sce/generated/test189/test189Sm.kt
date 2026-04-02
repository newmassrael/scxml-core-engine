// GENERATED CODE — DO NOT EDIT
// Source: resources/189/test189.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test189

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test189State : State {
    data object Fail : Test189State
    data object Pass : Test189State
    data object S0 : Test189State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test189Event : Event {
    sealed interface Error : Test189Event {
        data object Execution : Error
    }
    data object Event1 : Test189Event
    data object Event2 : Test189Event
}
// --- State Machine (W3C SCXML) ---

class Test189StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test189State, Test189Event>(scriptEngine) {

    override val initialState: Test189State = Test189State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test189State,
        event: Test189Event
    ): TransitionResult<Test189State> = when (state) {
        is Test189State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test189Event
    ): TransitionResult<Test189State> = when {
        event is Test189Event.Event1 -> TransitionResult.External(Test189State.Pass)
        event is Test189Event.Event2 -> TransitionResult.External(Test189State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test189State) {
        when (state) {
            is Test189State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.S0 -> {
            send(Test189Event.Event2, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            raiseInternal(Test189Event.Event1)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test189State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test189State,
        event: Test189Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
