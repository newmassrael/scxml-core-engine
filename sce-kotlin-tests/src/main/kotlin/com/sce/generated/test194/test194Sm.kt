// GENERATED CODE — DO NOT EDIT
// Source: resources/194/test194.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test194

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test194State : State {
    data object Fail : Test194State
    data object Pass : Test194State
    data object S0 : Test194State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test194Event : Event {
    sealed interface Error : Test194Event {
        data object Execution : Error
    }
    data object Event2 : Test194Event
    data object Timeout : Test194Event
}
// --- State Machine (W3C SCXML) ---

class Test194StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test194State, Test194Event>(scriptEngine) {

    override val initialState: Test194State = Test194State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test194State,
        event: Test194Event
    ): TransitionResult<Test194State> = when (state) {
        is Test194State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test194Event
    ): TransitionResult<Test194State> = when {
        event is Test194Event.Error.Execution -> TransitionResult.External(Test194State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test194State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test194State) {
        when (state) {
            is Test194State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test194State.S0 -> {
            // W3C SCXML 6.2 (test194): Invalid target raises error.execution
            raiseInternal(Test194Event.Error.Execution, EventMetadata(type = "platform", sendId = "__send_0"))
            return  // W3C SCXML 5.10: Stop subsequent executable content
            send(Test194Event.Timeout, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test194State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test194State,
        event: Test194Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
