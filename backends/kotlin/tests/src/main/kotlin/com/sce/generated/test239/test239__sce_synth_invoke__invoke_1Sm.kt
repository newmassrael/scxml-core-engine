// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 01d4ae2083bec7e32e332b36f0feb0c22f9503210f70693517ba1b7aa0094003
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:3 :: _machine

package com.sce.generated.test239

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test239SceSynthInvokeInvoke1State : State {
    data object Final : Test239SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test239SceSynthInvokeInvoke1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test239SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test239SceSynthInvokeInvoke1State, Test239SceSynthInvokeInvoke1Event>() {

    override val initialState: Test239SceSynthInvokeInvoke1State = Test239SceSynthInvokeInvoke1State.Final

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test239SceSynthInvokeInvoke1State? = when (stateId) {
        "final" -> Test239SceSynthInvokeInvoke1State.Final
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test239SceSynthInvokeInvoke1State): String = when (state) {
        is Test239SceSynthInvokeInvoke1State.Final -> "final"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test239SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test239SceSynthInvokeInvoke1State): Int = when (state) {
        is Test239SceSynthInvokeInvoke1State.Final -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test239SceSynthInvokeInvoke1State,
        event: Test239SceSynthInvokeInvoke1Event
    ): TransitionResult<Test239SceSynthInvokeInvoke1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onEntry(state: Test239SceSynthInvokeInvoke1State, pathChild: Test239SceSynthInvokeInvoke1State?) {
        when (state) {
            is Test239SceSynthInvokeInvoke1State.Final -> {
                // SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:4 :: final :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("final")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onExit(state: Test239SceSynthInvokeInvoke1State) {
        when (state) {
            is Test239SceSynthInvokeInvoke1State.Final -> {
                // SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:4 :: final :: _state_body
                activeStateIds.remove("final")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test239__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test239SceSynthInvokeInvoke1State,
        event: Test239SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
