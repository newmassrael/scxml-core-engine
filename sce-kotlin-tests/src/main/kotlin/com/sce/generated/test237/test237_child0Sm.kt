// GENERATED CODE — DO NOT EDIT
// Source: resources/237/test237_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test237

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test237Child0State : State {
    data object Sub0 : Test237Child0State
    data object SubFinal : Test237Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test237Child0Event : Event {
    sealed interface Error : Test237Child0Event {
        data object Execution : Error
    }
    data object Timeout : Test237Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test237Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test237Child0State, Test237Child0Event>(scriptEngine) {

    override val initialState: Test237Child0State = Test237Child0State.Sub0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test237Child0State,
        event: Test237Child0Event
    ): TransitionResult<Test237Child0State> = when (state) {
        is Test237Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test237Child0Event
    ): TransitionResult<Test237Child0State> = when {
        event is Test237Child0Event.Timeout -> TransitionResult.External(Test237Child0State.SubFinal, Test237Child0State.Sub0)

        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test237Child0State) {
        when (state) {
            is Test237Child0State.Sub0 -> {
            scheduleSend("__send_0", 2000L, Test237Child0Event.Timeout)
            }
            is Test237Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test237Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test237Child0State,
        event: Test237Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
