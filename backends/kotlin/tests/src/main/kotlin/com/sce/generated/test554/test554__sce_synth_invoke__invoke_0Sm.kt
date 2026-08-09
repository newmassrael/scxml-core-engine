// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: f5fde488bb26d050ed6ca4285c6964cc031a9d1311486db8d9c07efbb803316f
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test554

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test554SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test554SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test554SceSynthInvokeInvoke0Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test554SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test554SceSynthInvokeInvoke0State, Test554SceSynthInvokeInvoke0Event>() {

    override val initialState: Test554SceSynthInvokeInvoke0State = Test554SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test554SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test554SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test554SceSynthInvokeInvoke0State): String = when (state) {
        is Test554SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test554SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test554SceSynthInvokeInvoke0State): Int = when (state) {
        is Test554SceSynthInvokeInvoke0State.SubFinal -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test554SceSynthInvokeInvoke0State,
        event: Test554SceSynthInvokeInvoke0Event
    ): TransitionResult<Test554SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test554SceSynthInvokeInvoke0State) {
        when (state) {
            is Test554SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:4
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test554SceSynthInvokeInvoke0State) {
        when (state) {
            is Test554SceSynthInvokeInvoke0State.SubFinal -> {
                // SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:4
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test554__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test554SceSynthInvokeInvoke0State,
        event: Test554SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
