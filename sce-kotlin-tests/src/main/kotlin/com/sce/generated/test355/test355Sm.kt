// GENERATED CODE — DO NOT EDIT
// Source: resources/355/test355.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test355

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test355State : State {
    data object Fail : Test355State
    data object Pass : Test355State
    data object S0 : Test355State
    data object S1 : Test355State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test355Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test355StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test355State, Test355Event>(scriptEngine) {

    override val initialState: Test355State = Test355State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test355State,
        event: Test355Event
    ): TransitionResult<Test355State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test355State
    ): TransitionResult<Test355State> = when (state) {
        is Test355State.S0 -> processNullS0()
        is Test355State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test355State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test355State.Pass)
    }

    private fun processNullS1(
    ): TransitionResult<Test355State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test355State.Fail)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test355State) {
        when (state) {
            is Test355State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test355State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test355State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test355State,
        event: Test355Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
