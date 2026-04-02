// GENERATED CODE — DO NOT EDIT
// Source: resources/236/test236.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test236

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test236State : State {
    data object Fail : Test236State
    data object Pass : Test236State
    data object S0 : Test236State
    data object S1 : Test236State
    data object S2 : Test236State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test236Event : Event {
    sealed interface Cancel : Test236Event {
        data object Invoke : Cancel
    }
    data object ChildToParent : Test236Event
    sealed interface Done : Test236Event {
        data object Invoke : Done
    }
    sealed interface Error : Test236Event {
        data object Execution : Error
    }
    data object Timeout : Test236Event
}
// --- State Machine (W3C SCXML) ---

class Test236StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test236State, Test236Event>(scriptEngine) {

    override val initialState: Test236State = Test236State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test236Event? = when (name) {
        "cancel.invoke" -> Test236Event.Cancel.Invoke
        "childToParent" -> Test236Event.ChildToParent
        "done.invoke" -> Test236Event.Done.Invoke
        "error.execution" -> Test236Event.Error.Execution
        "timeout" -> Test236Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test236Event): String? = when (event) {
        is Test236Event.Cancel.Invoke -> "cancel.invoke"
        is Test236Event.ChildToParent -> "childToParent"
        is Test236Event.Done.Invoke -> "done.invoke"
        is Test236Event.Error.Execution -> "error.execution"
        is Test236Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test236State,
        event: Test236Event
    ): TransitionResult<Test236State> = when (state) {
        is Test236State.S0 -> processS0(event)
        is Test236State.S1 -> processS1(event)
        is Test236State.S2 -> processS2(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.ChildToParent -> TransitionResult.External(Test236State.S1)
        event is Test236Event.Done.Invoke -> TransitionResult.External(Test236State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS1(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.Done.Invoke -> TransitionResult.External(Test236State.S2)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test236State.Fail)
    }

    private fun processS2(
        event: Test236Event
    ): TransitionResult<Test236State> = when {
        event is Test236Event.Timeout -> TransitionResult.External(Test236State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test236State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test236State) {
        when (state) {
            is Test236State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test236State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test236State.S0 -> {
            scheduleSend("__send_0", 2000L, Test236Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test236Child0StateMachine(scriptEngine), false, Test236Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test236State) {
        when (state) {
            is Test236State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test236State,
        event: Test236Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
