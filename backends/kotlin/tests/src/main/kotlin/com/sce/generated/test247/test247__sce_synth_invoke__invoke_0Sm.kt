// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b9b6d5a256b534ee1bf3d5ad94af0afa9df9e54bf19008d6dd27d12f1bc9a55e
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test247

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test247SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test247SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test247SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test247SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test247SceSynthInvokeInvoke0State, Test247SceSynthInvokeInvoke0Event>() {

    override val initialState: Test247SceSynthInvokeInvoke0State = Test247SceSynthInvokeInvoke0State.SubFinal

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test247SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test247SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test247SceSynthInvokeInvoke0State): String = when (state) {
        is Test247SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test247SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test247SceSynthInvokeInvoke0State): Int = when (state) {
        is Test247SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test247SceSynthInvokeInvoke0State,
        event: Test247SceSynthInvokeInvoke0Event
    ): TransitionResult<Test247SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test247SceSynthInvokeInvoke0State, pathChild: Test247SceSynthInvokeInvoke0State?) {
        when (state) {
            is Test247SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test247SceSynthInvokeInvoke0State) {
        when (state) {
            is Test247SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test247__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test247SceSynthInvokeInvoke0State,
        event: Test247SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
