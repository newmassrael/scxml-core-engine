// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: aa3f7478a78abf9bf22f51a549ae822f834be956298adbc33316f195f470808d
// generated-at: 1781099318

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
