// GENERATED CODE — DO NOT EDIT
// Source: resources/192/test192_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test192

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test192Child0State : State {
    data object Sub0 : Test192Child0State
    data object SubFinal : Test192Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test192Child0Event : Event {
    data object ChildToParent : Test192Child0Event
    sealed interface Error : Test192Child0Event {
        data object Execution : Error
    }
    data object EventReceived : Test192Child0Event
    data object ParentToChild : Test192Child0Event
    data object Timeout : Test192Child0Event
}
// --- State Machine (W3C SCXML) ---

class Test192Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test192Child0State, Test192Child0Event>(scriptEngine) {

    override val initialState: Test192Child0State = Test192Child0State.Sub0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test192Child0Event? = when (name) {
        "childToParent" -> Test192Child0Event.ChildToParent
        "error.execution" -> Test192Child0Event.Error.Execution
        "eventReceived" -> Test192Child0Event.EventReceived
        "parentToChild" -> Test192Child0Event.ParentToChild
        "timeout" -> Test192Child0Event.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test192Child0Event): String? = when (event) {
        is Test192Child0Event.ChildToParent -> "childToParent"
        is Test192Child0Event.Error.Execution -> "error.execution"
        is Test192Child0Event.EventReceived -> "eventReceived"
        is Test192Child0Event.ParentToChild -> "parentToChild"
        is Test192Child0Event.Timeout -> "timeout"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test192Child0State,
        event: Test192Child0Event
    ): TransitionResult<Test192Child0State> = when (state) {
        is Test192Child0State.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test192Child0Event
    ): TransitionResult<Test192Child0State> = when {
        event is Test192Child0Event.ParentToChild -> TransitionResult.External(Test192Child0State.SubFinal)
        event is Test192Child0Event.Timeout -> TransitionResult.External(Test192Child0State.SubFinal)
        else -> TransitionResult.Ignored
    }

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test192Child0State) {
        when (state) {
            is Test192Child0State.Sub0 -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            scheduleSend("__send_2", 3000L, Test192Child0Event.Timeout)
            }
            is Test192Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test192Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test192Child0State,
        event: Test192Child0Event?
    ) {
        when (source) {
        is Test192Child0State.Sub0 -> when {
            event is Test192Child0Event.ParentToChild -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("eventReceived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
