// GENERATED CODE — DO NOT EDIT
// Source: resources/276/test276.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test276

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test276State : State {
    data object Fail : Test276State
    data object Pass : Test276State
    data object S0 : Test276State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test276Event : Event {
    sealed interface Cancel : Test276Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test276Event {
        data object Invoke : Done
    }
    sealed interface Error : Test276Event {
        data object Execution : Error
    }
    data object Event0 : Test276Event
    data object Event1 : Test276Event
}
// --- State Machine (W3C SCXML) ---

class Test276StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test276State, Test276Event>(scriptEngine) {

    override val initialState: Test276State = Test276State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test276Event? = when (name) {
        "cancel.invoke" -> Test276Event.Cancel.Invoke
        "done.invoke" -> Test276Event.Done.Invoke
        "error.execution" -> Test276Event.Error.Execution
        "event0" -> Test276Event.Event0
        "event1" -> Test276Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test276Event): String? = when (event) {
        is Test276Event.Cancel.Invoke -> "cancel.invoke"
        is Test276Event.Done.Invoke -> "done.invoke"
        is Test276Event.Error.Execution -> "error.execution"
        is Test276Event.Event0 -> "event0"
        is Test276Event.Event1 -> "event1"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test276State,
        event: Test276Event
    ): TransitionResult<Test276State> = when (state) {
        is Test276State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test276Event
    ): TransitionResult<Test276State> = when {
        event is Test276Event.Event1 -> TransitionResult.External(Test276State.Pass)
        event is Test276Event.Event0 -> TransitionResult.External(Test276State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test276State) {
        when (state) {
            is Test276State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test276State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test276State.S0 -> {
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test276sub1StateMachine(scriptEngine), false, Test276Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test276State) {
        when (state) {
            is Test276State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test276State,
        event: Test276Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
