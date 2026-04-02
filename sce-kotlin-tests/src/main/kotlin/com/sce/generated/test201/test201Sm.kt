// GENERATED CODE — DO NOT EDIT
// Source: resources/201/test201.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test201

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test201State : State {
    data object Fail : Test201State
    data object Pass : Test201State
    data object S0 : Test201State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test201Event : Event {
    sealed interface Error : Test201Event {
        data object Execution : Error
    }
    data object Event1 : Test201Event
    data object Timeout : Test201Event
}
// --- State Machine (W3C SCXML) ---

class Test201StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test201State, Test201Event>(scriptEngine) {

    override val initialState: Test201State = Test201State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test201State,
        event: Test201Event
    ): TransitionResult<Test201State> = when (state) {
        is Test201State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test201Event
    ): TransitionResult<Test201State> = when {
        event is Test201Event.Event1 -> TransitionResult.External(Test201State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test201State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test201State) {
        when (state) {
            is Test201State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test201State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test201State.S0 -> {
            send(Test201Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            send(Test201Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test201State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test201State,
        event: Test201Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
