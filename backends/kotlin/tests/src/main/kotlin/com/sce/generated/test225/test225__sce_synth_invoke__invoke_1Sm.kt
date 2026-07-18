// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: c496f893fb4def171deba817f047a2a335356d181c631fa74825a157a7412c3e
// generated-at: 1784370263

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test225__sce_synth_invoke__invoke_1.scxml:3

package com.sce.generated.test225

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test225SceSynthInvokeInvoke1State : State {
    data object SubFinal2 : Test225SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test225SceSynthInvokeInvoke1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test225SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test225SceSynthInvokeInvoke1State, Test225SceSynthInvokeInvoke1Event>() {

    override val initialState: Test225SceSynthInvokeInvoke1State = Test225SceSynthInvokeInvoke1State.SubFinal2



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test225SceSynthInvokeInvoke1State? = when (stateId) {
        "subFinal2" -> Test225SceSynthInvokeInvoke1State.SubFinal2
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test225SceSynthInvokeInvoke1State): String = when (state) {
        is Test225SceSynthInvokeInvoke1State.SubFinal2 -> "subFinal2"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test225SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test225SceSynthInvokeInvoke1State): Int = when (state) {
        is Test225SceSynthInvokeInvoke1State.SubFinal2 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test225SceSynthInvokeInvoke1State,
        event: Test225SceSynthInvokeInvoke1Event
    ): TransitionResult<Test225SceSynthInvokeInvoke1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test225__sce_synth_invoke__invoke_1.scxml:3
    override fun onEntry(state: Test225SceSynthInvokeInvoke1State) {
        when (state) {
            is Test225SceSynthInvokeInvoke1State.SubFinal2 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test225__sce_synth_invoke__invoke_1.scxml:3
    override fun onExit(state: Test225SceSynthInvokeInvoke1State) {
        when (state) {
            is Test225SceSynthInvokeInvoke1State.SubFinal2 -> {
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test225__sce_synth_invoke__invoke_1.scxml:3
    override fun executeTransitionActions(
        source: Test225SceSynthInvokeInvoke1State,
        event: Test225SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
