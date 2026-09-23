// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: backends/kotlin/tests/src/main/kotlin/com/sce/generated/test530/test530_hybrid0.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test530_hybrid0.scxml:2 :: _machine

package com.sce.generated.test530

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test530Hybrid0State : State {
    data object Final : Test530Hybrid0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test530Hybrid0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test530Hybrid0StateMachine(
) : StateMachineEngine<Test530Hybrid0State, Test530Hybrid0Event>() {

    override val initialState: Test530Hybrid0State = Test530Hybrid0State.Final

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test530Hybrid0State? = when (stateId) {
        "final" -> Test530Hybrid0State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test530Hybrid0State): String = when (state) {
        is Test530Hybrid0State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test530Hybrid0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test530Hybrid0State): Int = when (state) {
        is Test530Hybrid0State.Final -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test530Hybrid0State,
        event: Test530Hybrid0Event
    ): TransitionResult<Test530Hybrid0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test530_hybrid0.scxml:2 :: _machine
    override fun onEntry(state: Test530Hybrid0State, pathChild: Test530Hybrid0State?) {
        when (state) {
            is Test530Hybrid0State.Final -> {
                // SCE-MAP: test530_hybrid0.scxml:3 :: final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test530_hybrid0.scxml:2 :: _machine
    override fun onExit(state: Test530Hybrid0State) {
        when (state) {
            is Test530Hybrid0State.Final -> {
                // SCE-MAP: test530_hybrid0.scxml:3 :: final :: _state_body
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test530_hybrid0.scxml:2 :: _machine
    override fun executeTransitionActions(
        source: Test530Hybrid0State,
        event: Test530Hybrid0Event?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
