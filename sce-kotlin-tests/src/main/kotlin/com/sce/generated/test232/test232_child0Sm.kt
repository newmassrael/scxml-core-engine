// GENERATED CODE — DO NOT EDIT
// Source: resources/232/test232_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test232

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test232Child0State : State {
    data object SubFinal : Test232Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232Child0Event : Event {
    data object ChildToParent1 : Test232Child0Event
    data object ChildToParent2 : Test232Child0Event
    sealed interface Error : Test232Child0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test232Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test232Child0State, Test232Child0Event>(scriptEngine) {

    override val initialState: Test232Child0State = Test232Child0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test232Child0State? = when (stateId) {
        "subFinal" -> Test232Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test232Child0State): String = when (state) {
        is Test232Child0State.SubFinal -> "subFinal"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test232Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test232Child0State): Int = when (state) {
        is Test232Child0State.SubFinal -> 0
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232Child0Event? = when (name) {
        "childToParent1" -> Test232Child0Event.ChildToParent1
        "childToParent2" -> Test232Child0Event.ChildToParent2
        "error.execution" -> Test232Child0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232Child0Event): String? = when (event) {
        is Test232Child0Event.ChildToParent1 -> "childToParent1"
        is Test232Child0Event.ChildToParent2 -> "childToParent2"
        is Test232Child0Event.Error.Execution -> "error.execution"
        else -> null
    }


    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test232Child0State,
        event: Test232Child0Event
    ): TransitionResult<Test232Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test232Child0State) {
        when (state) {
            is Test232Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent1", "")
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent2", "")
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test232Child0State) {
        when (state) {
            is Test232Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test232Child0State,
        event: Test232Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
