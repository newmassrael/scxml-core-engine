// GENERATED CODE — DO NOT EDIT
// Source: resources/226/test226.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test226

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test226State : State {
    data object Fail : Test226State
    data object Pass : Test226State
    data object S0 : Test226State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test226Event : Event {
    sealed interface Cancel : Test226Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test226Event {
        data object Invoke : Done
    }
    sealed interface Error : Test226Event {
        data object Execution : Error
    }
    data object Timeout : Test226Event
    data object VarBound : Test226Event
}
// --- State Machine (W3C SCXML) ---

class Test226StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test226State, Test226Event>(scriptEngine) {

    override val initialState: Test226State = Test226State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test226Event? = when (name) {
        "cancel.invoke" -> Test226Event.Cancel.Invoke
        "done.invoke" -> Test226Event.Done.Invoke
        "error.execution" -> Test226Event.Error.Execution
        "timeout" -> Test226Event.Timeout
        "varBound" -> Test226Event.VarBound
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test226Event): String? = when (event) {
        is Test226Event.Cancel.Invoke -> "cancel.invoke"
        is Test226Event.Done.Invoke -> "done.invoke"
        is Test226Event.Error.Execution -> "error.execution"
        is Test226Event.Timeout -> "timeout"
        is Test226Event.VarBound -> "varBound"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test226State,
        event: Test226Event
    ): TransitionResult<Test226State> = when (state) {
        is Test226State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test226Event
    ): TransitionResult<Test226State> = when {
        event is Test226Event.VarBound -> TransitionResult.External(Test226State.Pass)
        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test226State.Fail)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test226State) {
        when (state) {
            is Test226State.Fail -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test226State.Pass -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test226State.S0 -> {
            scheduleSend("__send_0", 3000L, Test226Event.Timeout)
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test226sub1StateMachine(scriptEngine), false, Test226Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test226State) {
        when (state) {
            is Test226State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test226State,
        event: Test226Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
