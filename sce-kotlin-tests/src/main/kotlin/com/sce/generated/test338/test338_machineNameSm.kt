// GENERATED CODE — DO NOT EDIT
// Source: resources/338/test338_machineName.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test338

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test338MachineNameState : State {
    data object Sub0 : Test338MachineNameState
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test338MachineNameEvent : Event {
    sealed interface Error : Test338MachineNameEvent {
        data object Execution : Error
    }
    data object Event1 : Test338MachineNameEvent
}
// --- State Machine (W3C SCXML) ---

class Test338MachineNameStateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test338MachineNameState, Test338MachineNameEvent>(scriptEngine) {

    override val initialState: Test338MachineNameState = Test338MachineNameState.Sub0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test338MachineNameEvent? = when (name) {
        "error.execution" -> Test338MachineNameEvent.Error.Execution
        "event1" -> Test338MachineNameEvent.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test338MachineNameEvent): String? = when (event) {
        is Test338MachineNameEvent.Error.Execution -> "error.execution"
        is Test338MachineNameEvent.Event1 -> "event1"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test338MachineNameState,
        event: Test338MachineNameEvent
    ): TransitionResult<Test338MachineNameState> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test338MachineNameState) {
        when (state) {
            is Test338MachineNameState.Sub0 -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("event1", "")
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test338MachineNameState) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test338MachineNameState,
        event: Test338MachineNameEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
