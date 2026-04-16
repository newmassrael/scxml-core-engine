
// GENERATED CODE — DO NOT EDIT
// Source: resources/228/test228_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test228

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test228Child0State : State {
    data object SubFinal : Test228Child0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test228Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test228Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test228Child0State, Test228Child0Event>(scriptEngine) {

    override val initialState: Test228Child0State = Test228Child0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test228Child0State? = when (stateId) {
        "subFinal" -> Test228Child0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test228Child0State): String = when (state) {
        is Test228Child0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test228Child0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test228Child0State): Int = when (state) {
        is Test228Child0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test228Child0State,
        event: Test228Child0Event
    ): TransitionResult<Test228Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test228Child0State) {
        when (state) {
            is Test228Child0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test228Child0State) {
        when (state) {
            is Test228Child0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test228Child0State,
        event: Test228Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
