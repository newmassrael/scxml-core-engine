// GENERATED CODE — DO NOT EDIT
// Source: resources/531/test531.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test531

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test531State : State {
    data object Fail : Test531State
    data object Pass : Test531State
    data object S0 : Test531State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test531Event : Event {
    sealed interface Error : Test531Event {
        data object Execution : Error
    }
    data object Test : Test531Event
    data object Timeout : Test531Event
}
// --- State Machine (W3C SCXML) ---

class Test531StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test531State, Test531Event>(scriptEngine) {

    override val initialState: Test531State = Test531State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test531State,
        event: Test531Event
    ): TransitionResult<Test531State> = when (state) {
        is Test531State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test531Event
    ): TransitionResult<Test531State> = when {
        event is Test531Event.Test -> TransitionResult.External(Test531State.Pass, Test531State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test531State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test531State) {
        when (state) {
            is Test531State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test531State.S0 -> {
            scheduleSend("__send_0", 3000L, Test531Event.Timeout)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test531State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test531State,
        event: Test531Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
