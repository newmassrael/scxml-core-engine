// GENERATED CODE — DO NOT EDIT
// Source: resources/522/test522.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test522

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test522State : State {
    data object Fail : Test522State
    data object Pass : Test522State
    data object S0 : Test522State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test522Event : Event {
    sealed interface Error : Test522Event {
        data object Self : Error
        data object Execution : Error
    }
    data object Test : Test522Event
    data object Timeout : Test522Event
}
// --- State Machine (W3C SCXML) ---

class Test522StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test522State, Test522Event>(scriptEngine) {

    override val initialState: Test522State = Test522State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test522State,
        event: Test522Event
    ): TransitionResult<Test522State> = when (state) {
        is Test522State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test522Event
    ): TransitionResult<Test522State> = when {
        event is Test522Event.Timeout -> TransitionResult.External(Test522State.Fail)
        // W3C SCXML 3.12.1: Prefix match for "error"
        (event is Test522Event.Error || event is Test522Event.Error.Execution) -> TransitionResult.External(Test522State.Fail)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test522State.Pass)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test522State) {
        when (state) {
            is Test522State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test522State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test522State.S0 -> {
            scheduleSend("__send_0", 30000L, Test522Event.Timeout)
            send(Test522Event.Test, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test522State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test522State,
        event: Test522Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
