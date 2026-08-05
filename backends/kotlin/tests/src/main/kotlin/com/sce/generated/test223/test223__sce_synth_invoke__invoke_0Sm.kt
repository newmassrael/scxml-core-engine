// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 9fc57a6c741c08949d8257dc6586d295bb33ea67c2f5a0208405c69dc89fe99d
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test223__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test223

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test223SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test223SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test223SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test223SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test223SceSynthInvokeInvoke0State, Test223SceSynthInvokeInvoke0Event>() {

    override val initialState: Test223SceSynthInvokeInvoke0State = Test223SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test223SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test223SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test223SceSynthInvokeInvoke0State): String = when (state) {
        is Test223SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test223SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test223SceSynthInvokeInvoke0State): Int = when (state) {
        is Test223SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test223SceSynthInvokeInvoke0State,
        event: Test223SceSynthInvokeInvoke0Event
    ): TransitionResult<Test223SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test223__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test223SceSynthInvokeInvoke0State) {
        when (state) {
            is Test223SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test223__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test223SceSynthInvokeInvoke0State) {
        when (state) {
            is Test223SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test223__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test223SceSynthInvokeInvoke0State,
        event: Test223SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
