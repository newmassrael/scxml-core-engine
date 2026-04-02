// GENERATED CODE — DO NOT EDIT
// Source: resources/243/test243.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test243

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test243State : State {
    data object Fail : Test243State
    data object Pass : Test243State
    data object S0 : Test243State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test243Event : Event {
    sealed interface Cancel : Test243Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test243Event {
        data object Invoke : Done
    }
    sealed interface Error : Test243Event {
        data object Execution : Error
    }
    data object Failure : Test243Event
    data object Success : Test243Event
    data object Timeout : Test243Event
}
// --- State Machine (W3C SCXML) ---

class Test243StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test243State, Test243Event>(scriptEngine) {

    override val initialState: Test243State = Test243State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test243Event? = when (name) {
        "cancel.invoke" -> Test243Event.Cancel.Invoke
        "done.invoke" -> Test243Event.Done.Invoke
        "error.execution" -> Test243Event.Error.Execution
        "failure" -> Test243Event.Failure
        "success" -> Test243Event.Success
        "timeout" -> Test243Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test243Event): String? = when (event) {
        is Test243Event.Cancel.Invoke -> "cancel.invoke"
        is Test243Event.Done.Invoke -> "done.invoke"
        is Test243Event.Error.Execution -> "error.execution"
        is Test243Event.Failure -> "failure"
        is Test243Event.Success -> "success"
        is Test243Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test243State,
        event: Test243Event
    ): TransitionResult<Test243State> = when (state) {
        is Test243State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test243Event
    ): TransitionResult<Test243State> = when {
        event is Test243Event.Success -> TransitionResult.External(Test243State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test243State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test243State) {
        when (state) {
            is Test243State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test243State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test243State.S0 -> {
            scheduleSend("__send_0", 2000L, Test243Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test243Child0StateMachine(scriptEngine), false, Test243Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test243State) {
        when (state) {
            is Test243State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test243State,
        event: Test243Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
