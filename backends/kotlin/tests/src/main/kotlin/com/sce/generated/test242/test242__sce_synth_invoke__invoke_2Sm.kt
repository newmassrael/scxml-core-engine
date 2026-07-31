// SCE-GENERATED — DO NOT EDIT
// source-hash: 50977319f11c1ff3aac5be1771f46084e92b202125e3d418050cec95e667f58c
// template-hash: 140c4d555915ab51dfdb5b562572972b7994e3bced0046a696f7c65e279b5a12
// generated-at: 1785462747

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242SceSynthInvokeInvoke2State : State {
    data object SubFinal2 : Test242SceSynthInvokeInvoke2State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242SceSynthInvokeInvoke2Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test242SceSynthInvokeInvoke2StateMachine(
) : StateMachineEngine<Test242SceSynthInvokeInvoke2State, Test242SceSynthInvokeInvoke2Event>() {

    override val initialState: Test242SceSynthInvokeInvoke2State = Test242SceSynthInvokeInvoke2State.SubFinal2



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242SceSynthInvokeInvoke2State? = when (stateId) {
        "subFinal2" -> Test242SceSynthInvokeInvoke2State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242SceSynthInvokeInvoke2State): String = when (state) {
        is Test242SceSynthInvokeInvoke2State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test242SceSynthInvokeInvoke2State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test242SceSynthInvokeInvoke2State): Int = when (state) {
        is Test242SceSynthInvokeInvoke2State.SubFinal2 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242SceSynthInvokeInvoke2State,
        event: Test242SceSynthInvokeInvoke2Event
    ): TransitionResult<Test242SceSynthInvokeInvoke2State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3
    override fun onEntry(state: Test242SceSynthInvokeInvoke2State) {
        when (state) {
            is Test242SceSynthInvokeInvoke2State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3
    override fun onExit(state: Test242SceSynthInvokeInvoke2State) {
        when (state) {
            is Test242SceSynthInvokeInvoke2State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3
    override fun executeTransitionActions(
        source: Test242SceSynthInvokeInvoke2State,
        event: Test242SceSynthInvokeInvoke2Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
