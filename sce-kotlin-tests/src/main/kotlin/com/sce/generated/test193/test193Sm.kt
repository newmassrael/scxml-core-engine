// GENERATED CODE — DO NOT EDIT
// Source: resources/193/test193.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test193

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test193State : State {
    data object Fail : Test193State
    data object Pass : Test193State
    data object S0 : Test193State
    data object S1 : Test193State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test193Event : Event {
    sealed interface Error : Test193Event {
        data object Execution : Error
    }
    data object Event1 : Test193Event
    data object Internal : Test193Event
    data object Timeout : Test193Event
}
// --- State Machine (W3C SCXML) ---

class Test193StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test193State, Test193Event>(scriptEngine) {

    override val initialState: Test193State = Test193State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test193State,
        event: Test193Event
    ): TransitionResult<Test193State> = when (state) {
        is Test193State.S0 -> processS0(event)
        is Test193State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test193Event
    ): TransitionResult<Test193State> = when {
        event is Test193Event.Event1 -> TransitionResult.External(Test193State.Fail)
        event is Test193Event.Internal -> TransitionResult.External(Test193State.S1)
        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test193Event
    ): TransitionResult<Test193State> = when {
        event is Test193Event.Event1 -> TransitionResult.External(Test193State.Pass)
        event is Test193Event.Timeout -> TransitionResult.External(Test193State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test193State) {
        when (state) {
            is Test193State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test193State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test193State.S0 -> {
            send(Test193Event.Internal, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            send(Test193Event.Event1, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: ""))
            scheduleSend("__send_2", 1000L, Test193Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test193State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test193State,
        event: Test193Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
