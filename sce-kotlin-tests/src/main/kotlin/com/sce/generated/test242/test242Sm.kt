// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test242

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test242State : State {
    data object Fail : Test242State
    data object Pass : Test242State
    data object S0 : Test242State
    data object S02 : Test242State
    data object S03 : Test242State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242Event : Event {
    sealed interface Cancel : Test242Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test242Event {
        data object Invoke : Done
    }
    sealed interface Error : Test242Event {
        data object Execution : Error
    }
    data object Timeout : Test242Event
    data object Timeout1 : Test242Event
    data object Timeout2 : Test242Event
    data object Timeout3 : Test242Event
}
// --- State Machine (W3C SCXML) ---

class Test242StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test242State, Test242Event>(scriptEngine) {

    override val initialState: Test242State = Test242State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test242Event? = when (name) {
        "cancel.invoke" -> Test242Event.Cancel.Invoke
        "done.invoke" -> Test242Event.Done.Invoke
        "error.execution" -> Test242Event.Error.Execution
        "timeout" -> Test242Event.Timeout
        "timeout1" -> Test242Event.Timeout1
        "timeout2" -> Test242Event.Timeout2
        "timeout3" -> Test242Event.Timeout3
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test242Event): String? = when (event) {
        is Test242Event.Cancel.Invoke -> "cancel.invoke"
        is Test242Event.Done.Invoke -> "done.invoke"
        is Test242Event.Error.Execution -> "error.execution"
        is Test242Event.Timeout -> "timeout"
        is Test242Event.Timeout1 -> "timeout1"
        is Test242Event.Timeout2 -> "timeout2"
        is Test242Event.Timeout3 -> "timeout3"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242State,
        event: Test242Event
    ): TransitionResult<Test242State> = when (state) {
        is Test242State.S0 -> processS0(event)
        is Test242State.S02 -> processS02(event)
        is Test242State.S03 -> processS03(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Timeout -> TransitionResult.External(Test242State.Fail)
        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.S02)
        event is Test242Event.Timeout1 -> TransitionResult.External(Test242State.S03)
        else -> TransitionResult.Ignored
    }

    private fun processS02(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.Pass)
        event is Test242Event.Timeout2 -> TransitionResult.External(Test242State.Fail)
        else -> TransitionResult.Ignored
    }

    private fun processS03(
        event: Test242Event
    ): TransitionResult<Test242State> = when {
        event is Test242Event.Timeout3 -> TransitionResult.External(Test242State.Pass)
        event is Test242Event.Done.Invoke -> TransitionResult.External(Test242State.Fail)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test242State) {
        when (state) {
            is Test242State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test242State.S0 -> {
            scheduleSend("__send_0", 1000L, Test242Event.Timeout1)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test242sub1StateMachine(scriptEngine), false, Test242Event.Done.Invoke)
            }
            is Test242State.S02 -> {
            scheduleSend("__send_1", 1000L, Test242Event.Timeout2)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_1", Test242Child0StateMachine(scriptEngine), false, Test242Event.Done.Invoke)
            }
            is Test242State.S03 -> {
            scheduleSend("__send_2", 1000L, Test242Event.Timeout3)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_2", Test242Child1StateMachine(scriptEngine), false, Test242Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test242State) {
        when (state) {
            is Test242State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            is Test242State.S02 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_1")
            }
            is Test242State.S03 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_2")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test242State,
        event: Test242Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
