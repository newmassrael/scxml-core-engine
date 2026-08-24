// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 4cbf0ce468f2db0011b4fa010e6c117357964548e492f95e76a21755c70778e3
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:3 :: _machine

package com.sce.generated.test220

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test220SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test220SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test220SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test220SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test220SceSynthInvokeInvoke0State, Test220SceSynthInvokeInvoke0Event>() {

    override val initialState: Test220SceSynthInvokeInvoke0State = Test220SceSynthInvokeInvoke0State.SubFinal

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test220SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test220SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test220SceSynthInvokeInvoke0State): String = when (state) {
        is Test220SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test220SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test220SceSynthInvokeInvoke0State): Int = when (state) {
        is Test220SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test220SceSynthInvokeInvoke0State,
        event: Test220SceSynthInvokeInvoke0Event
    ): TransitionResult<Test220SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onEntry(state: Test220SceSynthInvokeInvoke0State, pathChild: Test220SceSynthInvokeInvoke0State?) {
        when (state) {
            is Test220SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun onExit(state: Test220SceSynthInvokeInvoke0State) {
        when (state) {
            is Test220SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:4 :: subFinal :: _state_body
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test220__sce_synth_invoke__invoke_0.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test220SceSynthInvokeInvoke0State,
        event: Test220SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
