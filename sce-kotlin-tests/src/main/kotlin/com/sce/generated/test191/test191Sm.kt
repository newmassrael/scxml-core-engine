// GENERATED CODE — DO NOT EDIT
// Source: resources/191/test191.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test191

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test191State : State {
    data object Fail : Test191State
    data object Pass : Test191State
    data object S0 : Test191State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test191Event : Event {
    sealed interface Cancel : Test191Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test191Event
    sealed interface Done : Test191Event {
        data object Invoke : Done
    }
    sealed interface Error : Test191Event {
        data object Execution : Error
    }
    data object Timeout : Test191Event
}
// --- State Machine (W3C SCXML) ---

class Test191StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test191State, Test191Event>(scriptEngine) {

    override val initialState: Test191State = Test191State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test191Event? = when (name) {
        "cancel.invoke" -> Test191Event.Cancel.Invoke
        "childToParent" -> Test191Event.ChildToParent
        "done.invoke" -> Test191Event.Done.Invoke
        "error.execution" -> Test191Event.Error.Execution
        "timeout" -> Test191Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test191Event): String? = when (event) {
        is Test191Event.Cancel.Invoke -> "cancel.invoke"
        is Test191Event.ChildToParent -> "childToParent"
        is Test191Event.Done.Invoke -> "done.invoke"
        is Test191Event.Error.Execution -> "error.execution"
        is Test191Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test191State,
        event: Test191Event
    ): TransitionResult<Test191State> = when (state) {
        is Test191State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test191Event
    ): TransitionResult<Test191State> = when {
        event is Test191Event.ChildToParent -> TransitionResult.External(Test191State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test191State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test191State) {
        when (state) {
            is Test191State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test191State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test191State.S0 -> {
            scheduleSend("__send_0", 5000L, Test191Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test191Child0StateMachine(scriptEngine), false, Test191Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test191State) {
        when (state) {
            is Test191State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test191State,
        event: Test191Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
