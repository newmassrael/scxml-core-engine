// GENERATED CODE — DO NOT EDIT
// Source: resources/509/test509.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test509

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test509State : State {
    data object Fail : Test509State
    data object Pass : Test509State
    data object S0 : Test509State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test509Event : Event {
    sealed interface Error : Test509Event {
        data object Execution : Error
    }
    data object Test : Test509Event
    data object Timeout : Test509Event
}
// --- State Machine (W3C SCXML) ---

class Test509StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test509State, Test509Event>(scriptEngine) {

    override val initialState: Test509State = Test509State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test509State,
        event: Test509Event
    ): TransitionResult<Test509State> = when (state) {
        is Test509State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test509Event
    ): TransitionResult<Test509State> = when {
        event is Test509Event.Test -> TransitionResult.External(Test509State.Pass, Test509State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test509State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test509State) {
        when (state) {
            is Test509State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test509State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test509State.S0 -> {
            scheduleSend("__send_0", 30000L, Test509Event.Timeout)
            send(Test509Event.Test, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test509State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test509State,
        event: Test509Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
