// GENERATED CODE — DO NOT EDIT
// Source: resources/348/test348.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test348

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test348State : State {
    data object Fail : Test348State
    data object Pass : Test348State
    data object S0 : Test348State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test348Event : Event {
    sealed interface Error : Test348Event {
        data object Execution : Error
    }
    data object S0Event : Test348Event
}
// --- State Machine (W3C SCXML) ---

class Test348StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test348State, Test348Event>(scriptEngine) {

    override val initialState: Test348State = Test348State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test348State,
        event: Test348Event
    ): TransitionResult<Test348State> = when (state) {
        is Test348State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test348Event
    ): TransitionResult<Test348State> = when {
        event is Test348Event.S0Event -> TransitionResult.External(Test348State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test348State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test348State) {
        when (state) {
            is Test348State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test348State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test348State.S0 -> {
            send(Test348Event.S0Event, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test348State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test348State,
        event: Test348Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
