// GENERATED CODE — DO NOT EDIT
// Source: resources/225/test225_child0.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test225

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test225Child0State : State {
    data object SubFinal1 : Test225Child0State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test225Child0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test225Child0StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test225Child0State, Test225Child0Event>(scriptEngine) {

    override val initialState: Test225Child0State = Test225Child0State.SubFinal1



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test225Child0State? = when (stateId) {
        "subFinal1" -> Test225Child0State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test225Child0State): String = when (state) {
        is Test225Child0State.SubFinal1 -> "subFinal1"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test225Child0State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test225Child0State): Int = when (state) {
        is Test225Child0State.SubFinal1 -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test225Child0State,
        event: Test225Child0Event
    ): TransitionResult<Test225Child0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test225Child0State) {
        when (state) {
            is Test225Child0State.SubFinal1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test225Child0State) {
        when (state) {
            is Test225Child0State.SubFinal1 -> {
                activeStateIds.remove("subFinal1")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test225Child0State,
        event: Test225Child0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
