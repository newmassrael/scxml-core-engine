// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 5b0237a7a83721c40de92b1914fb5f3ab69530a228f19b8f33cd3af4e27ebf24
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3 :: _machine

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

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



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
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun onEntry(state: Test242SceSynthInvokeInvoke2State, pathChild: Test242SceSynthInvokeInvoke2State?) {
        when (state) {
            is Test242SceSynthInvokeInvoke2State.SubFinal2 -> {
                // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:4 :: subFinal2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal2")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun onExit(state: Test242SceSynthInvokeInvoke2State) {
        when (state) {
            is Test242SceSynthInvokeInvoke2State.SubFinal2 -> {
                // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:4 :: subFinal2 :: _state_body
                activeStateIds.remove("subFinal2")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test242__sce_synth_invoke__invoke_2.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test242SceSynthInvokeInvoke2State,
        event: Test242SceSynthInvokeInvoke2Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
