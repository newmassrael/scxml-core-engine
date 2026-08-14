// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: c2d2b5edda626e72c9c29ff30cd9779ac6a5e5bd714354f582d478b8c23cb559
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test235__sce_synth_invoke__foo.scxml:3 :: _machine

package com.sce.generated.test235

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test235SceSynthInvokeFooState : State {
    data object SubFinal : Test235SceSynthInvokeFooState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test235SceSynthInvokeFooEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class Test235SceSynthInvokeFooStateMachine(
) : StateMachineEngine<Test235SceSynthInvokeFooState, Test235SceSynthInvokeFooEvent>() {

    override val initialState: Test235SceSynthInvokeFooState = Test235SceSynthInvokeFooState.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test235SceSynthInvokeFooState? = when (stateId) {
        "subFinal" -> Test235SceSynthInvokeFooState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test235SceSynthInvokeFooState): String = when (state) {
        is Test235SceSynthInvokeFooState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test235SceSynthInvokeFooState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test235SceSynthInvokeFooState): Int = when (state) {
        is Test235SceSynthInvokeFooState.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test235SceSynthInvokeFooState,
        event: Test235SceSynthInvokeFooEvent
    ): TransitionResult<Test235SceSynthInvokeFooState> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test235__sce_synth_invoke__foo.scxml:3 :: _machine
    override fun onEntry(state: Test235SceSynthInvokeFooState) {
        when (state) {
            is Test235SceSynthInvokeFooState.SubFinal -> {
                // SCE-MAP: test235__sce_synth_invoke__foo.scxml:4 :: subFinal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test235__sce_synth_invoke__foo.scxml:3 :: _machine
    override fun onExit(state: Test235SceSynthInvokeFooState) {
        when (state) {
            is Test235SceSynthInvokeFooState.SubFinal -> {
                // SCE-MAP: test235__sce_synth_invoke__foo.scxml:4 :: subFinal :: _state_body
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test235__sce_synth_invoke__foo.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test235SceSynthInvokeFooState,
        event: Test235SceSynthInvokeFooEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
