
// GENERATED CODE — DO NOT EDIT
// Source: resources/236/test236_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test236

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test236Child0State : State {
    data object SubFinal : Test236Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test236Child0Event : Event {
    data object ChildToParent : Test236Child0Event
    sealed interface Error : Test236Child0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test236Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test236Child0State, Test236Child0Event>(scriptEngine) {

    override val initialState: Test236Child0State = Test236Child0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test236Child0State? = when (stateId) {
        "subFinal" -> Test236Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test236Child0State): String = when (state) {
        is Test236Child0State.SubFinal -> "subFinal"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test236Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test236Child0State): Int = when (state) {
        is Test236Child0State.SubFinal -> 0
        else -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test236Child0Event? = when (name) {
        "childToParent" -> Test236Child0Event.ChildToParent
        "error.execution" -> Test236Child0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test236Child0Event): String? = when (event) {
        is Test236Child0Event.ChildToParent -> "childToParent"
        is Test236Child0Event.Error.Execution -> "error.execution"
        else -> null
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test236Child0State,
        event: Test236Child0Event
    ): TransitionResult<Test236Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test236Child0State) {
        when (state) {
            is Test236Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test236Child0State) {
        when (state) {
            is Test236Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            else -> {}
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test236Child0State,
        event: Test236Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
