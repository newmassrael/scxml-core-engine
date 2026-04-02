// GENERATED CODE — DO NOT EDIT
// Source: resources/423/test423.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test423

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test423State : State {
    data object Fail : Test423State
    data object Pass : Test423State
    data object S0 : Test423State
    data object S1 : Test423State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test423Event : Event {
    sealed interface Error : Test423Event {
        data object Execution : Error
    }
    data object ExternalEvent1 : Test423Event
    data object ExternalEvent2 : Test423Event
    data object InternalEvent : Test423Event
}
// --- State Machine (W3C SCXML) ---

class Test423StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test423State, Test423Event>(scriptEngine) {

    override val initialState: Test423State = Test423State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test423State,
        event: Test423Event
    ): TransitionResult<Test423State> = when (state) {
        is Test423State.S0 -> processS0(event)
        is Test423State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test423Event
    ): TransitionResult<Test423State> = when {
        event is Test423Event.InternalEvent -> TransitionResult.External(Test423State.S1)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test423State.Fail)
    }

    private fun processS1(
        event: Test423Event
    ): TransitionResult<Test423State> = when {
        event is Test423Event.ExternalEvent2 -> TransitionResult.External(Test423State.Pass)
        event is Test423Event.InternalEvent -> TransitionResult.External(Test423State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test423State) {
        when (state) {
            is Test423State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.S0 -> {
            send(Test423Event.ExternalEvent1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            scheduleSend("__send_1", 1000L, Test423Event.ExternalEvent2)
            raiseInternal(Test423Event.InternalEvent)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test423State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test423State,
        event: Test423Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
