// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 419df244c5f8e83941772fe0e162c3decc43983c72d904462cbbb6425fb07338
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test224__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test224

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test224SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test224SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test224SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test224SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test224SceSynthInvokeInvoke0State, Test224SceSynthInvokeInvoke0Event>() {

    override val initialState: Test224SceSynthInvokeInvoke0State = Test224SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test224SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test224SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test224SceSynthInvokeInvoke0State): String = when (state) {
        is Test224SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test224SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test224SceSynthInvokeInvoke0State): Int = when (state) {
        is Test224SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test224SceSynthInvokeInvoke0State,
        event: Test224SceSynthInvokeInvoke0Event
    ): TransitionResult<Test224SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test224__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test224SceSynthInvokeInvoke0State) {
        when (state) {
            is Test224SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test224__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test224SceSynthInvokeInvoke0State) {
        when (state) {
            is Test224SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test224__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test224SceSynthInvokeInvoke0State,
        event: Test224SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
