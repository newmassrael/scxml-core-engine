// GENERATED CODE — DO NOT EDIT
// Source: resources/229/test229_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test229

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test229Child0State : State {
    data object Sub0 : Test229Child0State
    data object SubFinal : Test229Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test229Child0Event : Event {
    data object ChildToParent : Test229Child0Event
    sealed interface Error : Test229Child0Event {
        data object Execution : Error
    }
    data object EventReceived : Test229Child0Event
    data object Timeout : Test229Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test229Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test229Child0State, Test229Child0Event>(scriptEngine) {

    override val initialState: Test229Child0State = Test229Child0State.Sub0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test229Child0Event? = when (name) {
        "childToParent" -> Test229Child0Event.ChildToParent
        "error.execution" -> Test229Child0Event.Error.Execution
        "eventReceived" -> Test229Child0Event.EventReceived
        "timeout" -> Test229Child0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test229Child0Event): String? = when (event) {
        is Test229Child0Event.ChildToParent -> "childToParent"
        is Test229Child0Event.Error.Execution -> "error.execution"
        is Test229Child0Event.EventReceived -> "eventReceived"
        is Test229Child0Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test229Child0State,
        event: Test229Child0Event
    ): TransitionResult<Test229Child0State> = when (state) {
        is Test229Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test229Child0Event
    ): TransitionResult<Test229Child0State> = when {
        event is Test229Child0Event.ChildToParent -> TransitionResult.External(Test229Child0State.SubFinal, Test229Child0State.Sub0)

        // W3C SCXML 3.12.1: Wildcard transition
        else -> TransitionResult.External(Test229Child0State.SubFinal)
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test229Child0State) {
        when (state) {
            is Test229Child0State.Sub0 -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            scheduleSend("__send_2", 3000L, Test229Child0Event.Timeout)
            }
            is Test229Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test229Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test229Child0State,
        event: Test229Child0Event?
    ) {
        when (source) {
        is Test229Child0State.Sub0 -> when {
            event is Test229Child0Event.ChildToParent -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("eventReceived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
