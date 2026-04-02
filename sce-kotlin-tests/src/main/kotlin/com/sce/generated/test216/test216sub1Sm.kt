// GENERATED CODE — DO NOT EDIT
// Source: resources/216/test216sub1.scxml
// Generator: SCE Kotlin Code Generator v1.0

package com.sce.generated.test216

import com.sce.runtime.*

// --- States (W3C SCXML 3.2) ---

sealed interface Test216sub1State : State {
    data object Final : Test216sub1State
}
// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test216sub1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test216sub1StateMachine(
    scriptEngine: ScxmlScriptEngine? = null
) : StateMachineEngine<Test216sub1State, Test216sub1Event>(scriptEngine) {

    override val initialState: Test216sub1State = Test216sub1State.Final



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test216sub1State? = when (stateId) {
        "final" -> Test216sub1State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test216sub1State): String = when (state) {
        is Test216sub1State.Final -> "final"
        else -> ""
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test216sub1State): Boolean = when (state) {
        else -> true
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test216sub1State): Int = when (state) {
        is Test216sub1State.Final -> 0
        else -> 0
    }



    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test216sub1State,
        event: Test216sub1Event
    ): TransitionResult<Test216sub1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    // Entry Actions (W3C SCXML 3.8)
    override fun onEntry(state: Test216sub1State) {
        when (state) {
            is Test216sub1State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            else -> {}
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    override fun onExit(state: Test216sub1State) {
        when (state) {
            is Test216sub1State.Final -> {
                activeStateIds.remove("final")
            }
            else -> {}
        }
    }
    // Transition Actions (W3C SCXML 3.13)
    override fun executeTransitionActions(
        source: Test216sub1State,
        event: Test216sub1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
