// GENERATED CODE — DO NOT EDIT
// Source: resources/250/test250.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test250

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test250State : State {
    data object Final : Test250State
    data object S0 : Test250State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test250Event : Event {
    sealed interface Cancel : Test250Event {
        data object Invoke : Cancel
    }
    sealed interface Done : Test250Event {
        data object Invoke : Done
    }
    sealed interface Error : Test250Event {
        data object Execution : Error
    }
    data object Foo : Test250Event
}
// --- State Machine (W3C SCXML) ---

class Test250StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test250State, Test250Event>(scriptEngine) {

    override val initialState: Test250State = Test250State.S0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test250Event? = when (name) {
        "cancel.invoke" -> Test250Event.Cancel.Invoke
        "done.invoke" -> Test250Event.Done.Invoke
        "error.execution" -> Test250Event.Error.Execution
        "foo" -> Test250Event.Foo
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test250Event): String? = when (event) {
        is Test250Event.Cancel.Invoke -> "cancel.invoke"
        is Test250Event.Done.Invoke -> "done.invoke"
        is Test250Event.Error.Execution -> "error.execution"
        is Test250Event.Foo -> "foo"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test250State,
        event: Test250Event
    ): TransitionResult<Test250State> = when (state) {
        is Test250State.S0 -> processS0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processS0(
        event: Test250Event
    ): TransitionResult<Test250State> = when {
        event is Test250Event.Foo -> TransitionResult.External(Test250State.Final)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test250State) {
        when (state) {
            is Test250State.Final -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test250State.S0 -> {
            send(Test250Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
                // W3C SCXML 6.4: Start invoked child state machine
                startInvoke("_invoke_0", Test250Child0StateMachine(scriptEngine), false, Test250Event.Done.Invoke)
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test250State) {
        when (state) {
            is Test250State.S0 -> {
                // W3C SCXML 6.4: Cancel invoked child on state exit
                cancelInvoke("_invoke_0")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test250State,
        event: Test250Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
