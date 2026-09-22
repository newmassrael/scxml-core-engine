// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: a184cafe7a67a901b89b33f0188251221f7a1b0a549681a02c5ff67b5487c305
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: backends/kotlin/tests/src/main/kotlin/com/sce/generated/test216/test216_hybrid0.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test216_hybrid0.scxml:2 :: _machine

package com.sce.generated.test216

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test216Hybrid0State : State {
    data object Final : Test216Hybrid0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test216Hybrid0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test216Hybrid0StateMachine(
) : StateMachineEngine<Test216Hybrid0State, Test216Hybrid0Event>() {

    override val initialState: Test216Hybrid0State = Test216Hybrid0State.Final

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test216Hybrid0State? = when (stateId) {
        "final" -> Test216Hybrid0State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test216Hybrid0State): String = when (state) {
        is Test216Hybrid0State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test216Hybrid0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test216Hybrid0State): Int = when (state) {
        is Test216Hybrid0State.Final -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test216Hybrid0State,
        event: Test216Hybrid0Event
    ): TransitionResult<Test216Hybrid0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test216_hybrid0.scxml:2 :: _machine
    override fun onEntry(state: Test216Hybrid0State, pathChild: Test216Hybrid0State?) {
        when (state) {
            is Test216Hybrid0State.Final -> {
                // SCE-MAP: test216_hybrid0.scxml:3 :: final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test216_hybrid0.scxml:2 :: _machine
    override fun onExit(state: Test216Hybrid0State) {
        when (state) {
            is Test216Hybrid0State.Final -> {
                // SCE-MAP: test216_hybrid0.scxml:3 :: final :: _state_body
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test216_hybrid0.scxml:2 :: _machine
    override fun executeTransitionActions(
        source: Test216Hybrid0State,
        event: Test216Hybrid0Event?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
