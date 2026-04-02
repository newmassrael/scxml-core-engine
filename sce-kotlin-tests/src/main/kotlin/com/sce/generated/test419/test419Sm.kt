// GENERATED CODE — DO NOT EDIT
// Source: resources/419/test419.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test419

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test419State : State {
    data object Fail : Test419State
    data object Pass : Test419State
    data object S1 : Test419State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test419Event : Event {
    sealed interface Error : Test419Event {
        data object Execution : Error
    }
    data object ExternalEvent : Test419Event
    data object InternalEvent : Test419Event
}
// --- State Machine (W3C SCXML) ---

class Test419StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test419State, Test419Event>(scriptEngine) {

    override val initialState: Test419State = Test419State.S1






    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test419State,
        event: Test419Event
    ): TransitionResult<Test419State> = when (state) {
        is Test419State.S1 -> processS1(event)
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test419State
    ): TransitionResult<Test419State> = when (state) {
        is Test419State.S1 -> processNullS1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullS1(
    ): TransitionResult<Test419State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test419State.Pass)
    }

    // --- Per-State Event Handlers ---

    private fun processS1(
        event: Test419Event
    ): TransitionResult<Test419State> = when {
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test419State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test419State) {
        when (state) {
            is Test419State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test419State.S1 -> {
            raiseInternal(Test419Event.InternalEvent)
            send(Test419Event.ExternalEvent, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test419State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test419State,
        event: Test419Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
