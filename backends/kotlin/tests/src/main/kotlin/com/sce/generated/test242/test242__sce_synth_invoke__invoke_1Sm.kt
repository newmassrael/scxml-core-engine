// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: b987ea47cf7b98cc29f6a07fbb829bd85b24bd9991a16621d5e7458fb0482788
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:3 :: _machine

package com.sce.generated.test242

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test242SceSynthInvokeInvoke1State : State {
    data object SubFinal1 : Test242SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test242SceSynthInvokeInvoke1Event : Event {

}
// --- State Machine (W3C SCXML) ---

class Test242SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test242SceSynthInvokeInvoke1State, Test242SceSynthInvokeInvoke1Event>() {

    override val initialState: Test242SceSynthInvokeInvoke1State = Test242SceSynthInvokeInvoke1State.SubFinal1



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test242SceSynthInvokeInvoke1State? = when (stateId) {
        "subFinal1" -> Test242SceSynthInvokeInvoke1State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test242SceSynthInvokeInvoke1State): String = when (state) {
        is Test242SceSynthInvokeInvoke1State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test242SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test242SceSynthInvokeInvoke1State): Int = when (state) {
        is Test242SceSynthInvokeInvoke1State.SubFinal1 -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test242SceSynthInvokeInvoke1State,
        event: Test242SceSynthInvokeInvoke1Event
    ): TransitionResult<Test242SceSynthInvokeInvoke1State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onEntry(state: Test242SceSynthInvokeInvoke1State, pathChild: Test242SceSynthInvokeInvoke1State?) {
        when (state) {
            is Test242SceSynthInvokeInvoke1State.SubFinal1 -> {
                // SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:4 :: subFinal1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun onExit(state: Test242SceSynthInvokeInvoke1State) {
        when (state) {
            is Test242SceSynthInvokeInvoke1State.SubFinal1 -> {
                // SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:4 :: subFinal1 :: _state_body
                activeStateIds.remove("subFinal1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test242__sce_synth_invoke__invoke_1.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: Test242SceSynthInvokeInvoke1State,
        event: Test242SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
