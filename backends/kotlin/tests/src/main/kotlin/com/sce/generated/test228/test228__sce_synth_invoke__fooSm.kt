// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: ab200b8eb821f02e246ff33a9f9da5a6f5493996f3df460e1a87cc5891e5b49d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test228__sce_synth_invoke__foo.scxml:3

package com.sce.generated.test228

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test228SceSynthInvokeFooState : State {
    data object SubFinal : Test228SceSynthInvokeFooState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test228SceSynthInvokeFooEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class Test228SceSynthInvokeFooStateMachine(
) : StateMachineEngine<Test228SceSynthInvokeFooState, Test228SceSynthInvokeFooEvent>() {

    override val initialState: Test228SceSynthInvokeFooState = Test228SceSynthInvokeFooState.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test228SceSynthInvokeFooState? = when (stateId) {
        "subFinal" -> Test228SceSynthInvokeFooState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test228SceSynthInvokeFooState): String = when (state) {
        is Test228SceSynthInvokeFooState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test228SceSynthInvokeFooState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test228SceSynthInvokeFooState): Int = when (state) {
        is Test228SceSynthInvokeFooState.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test228SceSynthInvokeFooState,
        event: Test228SceSynthInvokeFooEvent
    ): TransitionResult<Test228SceSynthInvokeFooState> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test228__sce_synth_invoke__foo.scxml:3
    override fun onEntry(state: Test228SceSynthInvokeFooState) {
        when (state) {
            is Test228SceSynthInvokeFooState.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test228__sce_synth_invoke__foo.scxml:3
    override fun onExit(state: Test228SceSynthInvokeFooState) {
        when (state) {
            is Test228SceSynthInvokeFooState.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test228__sce_synth_invoke__foo.scxml:3
    override fun executeTransitionActions(
        source: Test228SceSynthInvokeFooState,
        event: Test228SceSynthInvokeFooEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
