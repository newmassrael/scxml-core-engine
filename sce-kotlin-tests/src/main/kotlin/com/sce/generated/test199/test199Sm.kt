// GENERATED CODE — DO NOT EDIT
// Source: resources/199/test199.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test199

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test199State : State {
    data object Fail : Test199State
    data object Pass : Test199State
    data object S0 : Test199State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test199Event : Event {
    sealed interface Error : Test199Event {
        data object Execution : Error
    }
    data object Event1 : Test199Event
    data object Timeout : Test199Event
}
// --- State Machine (W3C SCXML) ---

class Test199StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test199State, Test199Event>(scriptEngine) {

    override val initialState: Test199State = Test199State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test199State,
        event: Test199Event
    ): TransitionResult<Test199State> = when (state) {
        is Test199State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test199Event
    ): TransitionResult<Test199State> = when {
        event is Test199Event.Error.Execution -> TransitionResult.External(Test199State.Pass, Test199State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test199State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test199State) {
        when (state) {
            is Test199State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test199State.S0 -> {
            // W3C SCXML 6.2 (test199): Unsupported send type raises error.execution
            raiseInternal(Test199Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            send(Test199Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test199State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test199State,
        event: Test199Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
