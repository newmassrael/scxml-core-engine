// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c9e08658681ef21dd3bd5428d9da1979a690ea0bbf7340f9b10920cbe666e5c5
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: resources/415/test415.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test415.scxml:8

package com.sce.generated.test415

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test415State : State {
    data object Final : Test415State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test415Event : Event {
    data object Event1 : Test415Event
}
// --- State Machine (W3C SCXML) ---

class Test415StateMachine(
) : StateMachineEngine<Test415State, Test415Event>() {

    override val initialState: Test415State = Test415State.Final



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test415State? = when (stateId) {
        "final" -> Test415State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test415State): String = when (state) {
        is Test415State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test415State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test415State): Int = when (state) {
        is Test415State.Final -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test415State,
        event: Test415Event
    ): TransitionResult<Test415State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test415.scxml:8
    override fun onEntry(state: Test415State) {
        when (state) {
            is Test415State.Final -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return

            raiseInternal(Test415Event.Event1)
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test415.scxml:8
    override fun onExit(state: Test415State) {
        when (state) {
            is Test415State.Final -> {
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test415.scxml:8
    override fun executeTransitionActions(
        source: Test415State,
        event: Test415Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
