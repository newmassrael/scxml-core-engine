// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 68681672197f6dd374cca8d9d5846bb9f25c1cd0277222ecf9f6bf02adbac43e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/242/test242sub1.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242sub1.scxml:5

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242sub1State : State {
    data object Final : Test242sub1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242sub1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test242sub1StateMachine(
) : StateMachineEngine<Test242sub1State, Test242sub1Event>() {

    override val initialState: Test242sub1State = Test242sub1State.Final



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242sub1State? = when (stateId) {
        "final" -> Test242sub1State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242sub1State): String = when (state) {
        is Test242sub1State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test242sub1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test242sub1State): Int = when (state) {
        is Test242sub1State.Final -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242sub1State,
        event: Test242sub1Event
    ): TransitionResult<Test242sub1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242sub1.scxml:5
    override fun onEntry(state: Test242sub1State) {
        when (state) {
            is Test242sub1State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242sub1.scxml:5
    override fun onExit(state: Test242sub1State) {
        when (state) {
            is Test242sub1State.Final -> {
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test242sub1.scxml:5
    override fun executeTransitionActions(
        source: Test242sub1State,
        event: Test242sub1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
