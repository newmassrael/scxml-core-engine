// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 07a1057b89512b0ade7260ce662ea4e6ef3c2abde2d5bd32fb4fe82bd263d4bc
// generated-at: 1780802714

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test225

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test225SceSynthInvokeInvoke0State : State {
    data object SubFinal1 : Test225SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test225SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test225SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test225SceSynthInvokeInvoke0State, Test225SceSynthInvokeInvoke0Event>() {

    override val initialState: Test225SceSynthInvokeInvoke0State = Test225SceSynthInvokeInvoke0State.SubFinal1



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test225SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal1" -> Test225SceSynthInvokeInvoke0State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test225SceSynthInvokeInvoke0State): String = when (state) {
        is Test225SceSynthInvokeInvoke0State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test225SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test225SceSynthInvokeInvoke0State): Int = when (state) {
        is Test225SceSynthInvokeInvoke0State.SubFinal1 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test225SceSynthInvokeInvoke0State,
        event: Test225SceSynthInvokeInvoke0Event
    ): TransitionResult<Test225SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test225SceSynthInvokeInvoke0State) {
        when (state) {
            is Test225SceSynthInvokeInvoke0State.SubFinal1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test225SceSynthInvokeInvoke0State) {
        when (state) {
            is Test225SceSynthInvokeInvoke0State.SubFinal1 -> {
                activeStateIds.remove("subFinal1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test225__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test225SceSynthInvokeInvoke0State,
        event: Test225SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
