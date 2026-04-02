// GENERATED CODE — DO NOT EDIT
// Source: resources/200/test200.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test200

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test200State : State {
    data object Fail : Test200State
    data object Pass : Test200State
    data object S0 : Test200State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test200Event : Event {
    sealed interface Error : Test200Event {
        data object Execution : Error
    }
    data object Event1 : Test200Event
    data object Timeout : Test200Event
}
// --- State Machine (W3C SCXML) ---

class Test200StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test200State, Test200Event>(scriptEngine) {

    override val initialState: Test200State = Test200State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test200State,
        event: Test200Event
    ): TransitionResult<Test200State> = when (state) {
        is Test200State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test200Event
    ): TransitionResult<Test200State> = when {
        event is Test200Event.Event1 -> TransitionResult.External(Test200State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test200State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test200State) {
        when (state) {
            is Test200State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test200State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test200State.S0 -> {
            send(Test200Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            send(Test200Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test200State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test200State,
        event: Test200Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
