// GENERATED CODE — DO NOT EDIT
// Source: resources/377/test377.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test377

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test377State : State {
    data object Fail : Test377State
    data object Pass : Test377State
    data object S0 : Test377State
    data object S1 : Test377State
    data object S2 : Test377State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test377Event : Event {
    data object Event1 : Test377Event
    data object Event2 : Test377Event
}
// --- State Machine (W3C SCXML) ---

class Test377StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test377State, Test377Event>(scriptEngine) {

    override val initialState: Test377State = Test377State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test377State,
        event: Test377Event
    ): TransitionResult<Test377State> = when (state) {
        is Test377State.S1 -> processS1(event)
        is Test377State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test377State
    ): TransitionResult<Test377State> = when (state) {
        is Test377State.S0 -> processNullS0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS0(
    ): TransitionResult<Test377State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test377State.S1)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test377Event
    ): TransitionResult<Test377State> = when {
        event is Test377Event.Event1 -> TransitionResult.External(Test377State.S2, Test377State.S1)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test377State.Fail)
    }

    private fun processS2(
        event: Test377Event
    ): TransitionResult<Test377State> = when {
        event is Test377Event.Event2 -> TransitionResult.External(Test377State.Pass, Test377State.S2)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test377State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test377State) {
        when (state) {
            is Test377State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test377State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test377State) {
        when (state) {
            is Test377State.S0 -> {
            raiseInternal(Test377Event.Event1)
            raiseInternal(Test377Event.Event2)
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test377State,
        event: Test377Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
