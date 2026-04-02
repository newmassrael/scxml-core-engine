// GENERATED CODE — DO NOT EDIT
// Source: resources/577/test577.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test577

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test577State : State {
    data object Fail : Test577State
    data object Pass : Test577State
    data object S0 : Test577State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test577Event : Event {
    sealed interface Error : Test577Event {
        data object Communication : Error
        data object Execution : Error
    }
    data object Event1 : Test577Event
    data object Test : Test577Event
}
// --- State Machine (W3C SCXML) ---

class Test577StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test577State, Test577Event>(scriptEngine) {

    override val initialState: Test577State = Test577State.S0






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test577State,
        event: Test577Event
    ): TransitionResult<Test577State> = when (state) {
        is Test577State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test577Event
    ): TransitionResult<Test577State> = when {
        event is Test577Event.Error.Communication -> TransitionResult.External(Test577State.Pass, Test577State.S0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test577State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test577State) {
        when (state) {
            is Test577State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test577State.S0 -> {
            send(Test577Event.Event1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            // W3C SCXML C.2 (test577): BasicHTTP requires target, missing raises error.communication
            raiseInternal(Test577Event.Error.Communication, EventMetadata.platform())
            return  // W3C SCXML 5.10: Stop subsequent executable content
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test577State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test577State,
        event: Test577Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
