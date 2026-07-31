// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: f160b18d725f2c0387242c0463da6808a5b8be392d0dc888f0d564e42c83db17
// generated-at: 1785486331

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test215__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test215

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test215SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test215SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test215SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test215SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test215SceSynthInvokeInvoke0State, Test215SceSynthInvokeInvoke0Event>() {

    override val initialState: Test215SceSynthInvokeInvoke0State = Test215SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test215SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test215SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test215SceSynthInvokeInvoke0State): String = when (state) {
        is Test215SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test215SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test215SceSynthInvokeInvoke0State): Int = when (state) {
        is Test215SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test215SceSynthInvokeInvoke0State,
        event: Test215SceSynthInvokeInvoke0Event
    ): TransitionResult<Test215SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test215__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test215SceSynthInvokeInvoke0State) {
        when (state) {
            is Test215SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test215__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test215SceSynthInvokeInvoke0State) {
        when (state) {
            is Test215SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test215__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test215SceSynthInvokeInvoke0State,
        event: Test215SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
