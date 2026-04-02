// GENERATED CODE — DO NOT EDIT
// Source: resources/220/test220.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test220

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test220State : State {
    data object Fail : Test220State
    data object Pass : Test220State
    data object S0 : Test220State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test220Event : Event {
    sealed interface Cancel : Test220Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test220Event {
        data object Invoke : Done
    }
    sealed interface Error : Test220Event {
        data object Execution : Error
    }
    data object Timeout : Test220Event
}
// --- State Machine (W3C SCXML) ---

class Test220StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test220State, Test220Event>(scriptEngine) {

    override val initialState: Test220State = Test220State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test220Event? = when (name) {
        "cancel.invoke" -> Test220Event.Cancel.Invoke
        "done.invoke" -> Test220Event.Done.Invoke
        "error.execution" -> Test220Event.Error.Execution
        "timeout" -> Test220Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test220Event): String? = when (event) {
        is Test220Event.Cancel.Invoke -> "cancel.invoke"
        is Test220Event.Done.Invoke -> "done.invoke"
        is Test220Event.Error.Execution -> "error.execution"
        is Test220Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test220State,
        event: Test220Event
    ): TransitionResult<Test220State> = when (state) {
        is Test220State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test220Event
    ): TransitionResult<Test220State> = when {
        event is Test220Event.Done.Invoke -> TransitionResult.External(Test220State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test220State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test220State) {
        when (state) {
            is Test220State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test220State.S0 -> {
            scheduleSend("__send_0", 5000L, Test220Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test220Child0StateMachine(scriptEngine), false, Test220Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test220State) {
        when (state) {
            is Test220State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test220State,
        event: Test220Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
