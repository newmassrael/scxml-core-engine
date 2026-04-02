// GENERATED CODE — DO NOT EDIT
// Source: resources/333/test333.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test333

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test333State : State {
    data object Fail : Test333State
    data object Pass : Test333State
    data object S0 : Test333State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test333Event : Event {
    sealed interface Error : Test333Event {
        data object Execution : Error
    }
    data object Foo : Test333Event
}
// --- State Machine (W3C SCXML) ---

class Test333StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test333State, Test333Event>(scriptEngine) {

    override val initialState: Test333State = Test333State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test333State,
        event: Test333Event
    ): TransitionResult<Test333State> = when (state) {
        is Test333State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test333Event
    ): TransitionResult<Test333State> = when {
        event is Test333Event.Foo -> TransitionResult.External(Test333State.Pass, Test333State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test333State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test333State) {
        when (state) {
            is Test333State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test333State.S0 -> {
            send(Test333Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test333State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test333State,
        event: Test333Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
