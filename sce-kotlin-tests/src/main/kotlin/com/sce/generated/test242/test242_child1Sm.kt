
// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242_child1.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242Child1State : State {
    data object SubFinal2 : Test242Child1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242Child1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test242Child1StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test242Child1State, Test242Child1Event>(scriptEngine) {

    override val initialState: Test242Child1State = Test242Child1State.SubFinal2



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242Child1State? = when (stateId) {
        "subFinal2" -> Test242Child1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242Child1State): String = when (state) {
        is Test242Child1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test242Child1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test242Child1State): Int = when (state) {
        is Test242Child1State.SubFinal2 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242Child1State,
        event: Test242Child1Event
    ): TransitionResult<Test242Child1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---


    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test242Child1State) {
        when (state) {
            is Test242Child1State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test242Child1State) {
        when (state) {
            is Test242Child1State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }

    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test242Child1State,
        event: Test242Child1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
