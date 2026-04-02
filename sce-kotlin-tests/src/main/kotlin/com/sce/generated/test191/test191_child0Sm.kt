// GENERATED CODE — DO NOT EDIT
// Source: resources/191/test191_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test191

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test191Child0State : State {
    data object Sub0 : Test191Child0State
    data object SubFinal : Test191Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test191Child0Event : Event {
    data object ChildToParent : Test191Child0Event
    sealed interface Error : Test191Child0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test191Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test191Child0State, Test191Child0Event>(scriptEngine) {

    override val initialState: Test191Child0State = Test191Child0State.Sub0




    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test191Child0Event? = when (name) {
        "childToParent" -> Test191Child0Event.ChildToParent
        "error.execution" -> Test191Child0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test191Child0Event): String? = when (event) {
        is Test191Child0Event.ChildToParent -> "childToParent"
        is Test191Child0Event.Error.Execution -> "error.execution"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test191Child0State,
        event: Test191Child0Event
    ): TransitionResult<Test191Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test191Child0State
    ): TransitionResult<Test191Child0State> = when (state) {
        is Test191Child0State.Sub0 -> processNullSub0()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub0(
    ): TransitionResult<Test191Child0State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test191Child0State.SubFinal)
    }

    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test191Child0State) {
        when (state) {
            is Test191Child0State.Sub0 -> {
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test191Child0State.SubFinal -> {
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test191Child0State) {
        when (state) {
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test191Child0State,
        event: Test191Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
