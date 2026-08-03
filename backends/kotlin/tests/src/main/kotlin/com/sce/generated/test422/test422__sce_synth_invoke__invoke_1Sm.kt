// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 32616a8c15423facd5b04f320c1acfc24557f2b58b0b9ca0229cb783903eb112
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test422__sce_synth_invoke__invoke_1.scxml:3

package com.sce.generated.test422

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test422SceSynthInvokeInvoke1State : State {
    data object Sub1 : Test422SceSynthInvokeInvoke1State
    data object SubFinal1 : Test422SceSynthInvokeInvoke1State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test422SceSynthInvokeInvoke1Event : Event {
    sealed interface Error : Test422SceSynthInvokeInvoke1Event {
        data object Execution : Error
    }
    data object InvokeS11 : Test422SceSynthInvokeInvoke1Event
}
// --- State Machine (W3C SCXML) ---

class Test422SceSynthInvokeInvoke1StateMachine(
) : StateMachineEngine<Test422SceSynthInvokeInvoke1State, Test422SceSynthInvokeInvoke1Event>() {

    override val initialState: Test422SceSynthInvokeInvoke1State = Test422SceSynthInvokeInvoke1State.Sub1



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test422SceSynthInvokeInvoke1State? = when (stateId) {
        "sub1" -> Test422SceSynthInvokeInvoke1State.Sub1
        "subFinal1" -> Test422SceSynthInvokeInvoke1State.SubFinal1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test422SceSynthInvokeInvoke1State): String = when (state) {
        is Test422SceSynthInvokeInvoke1State.Sub1 -> "sub1"
        is Test422SceSynthInvokeInvoke1State.SubFinal1 -> "subFinal1"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test422SceSynthInvokeInvoke1State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test422SceSynthInvokeInvoke1State): Int = when (state) {
        is Test422SceSynthInvokeInvoke1State.Sub1 -> 0
        is Test422SceSynthInvokeInvoke1State.SubFinal1 -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test422SceSynthInvokeInvoke1Event? = when (name) {
        "error.execution" -> Test422SceSynthInvokeInvoke1Event.Error.Execution
        "invokeS11" -> Test422SceSynthInvokeInvoke1Event.InvokeS11
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test422SceSynthInvokeInvoke1Event): String? = when (event) {
        is Test422SceSynthInvokeInvoke1Event.Error.Execution -> "error.execution"
        is Test422SceSynthInvokeInvoke1Event.InvokeS11 -> "invokeS11"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test422SceSynthInvokeInvoke1State,
        event: Test422SceSynthInvokeInvoke1Event
    ): TransitionResult<Test422SceSynthInvokeInvoke1State> = when (state) {
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: Test422SceSynthInvokeInvoke1State
    ): TransitionResult<Test422SceSynthInvokeInvoke1State> = when (state) {
        is Test422SceSynthInvokeInvoke1State.Sub1 -> processNullSub1()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullSub1(
    ): TransitionResult<Test422SceSynthInvokeInvoke1State> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(Test422SceSynthInvokeInvoke1State.SubFinal1, Test422SceSynthInvokeInvoke1State.Sub1)
    }

    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test422__sce_synth_invoke__invoke_1.scxml:3
    override fun onEntry(state: Test422SceSynthInvokeInvoke1State) {
        when (state) {
            is Test422SceSynthInvokeInvoke1State.Sub1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub1")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("invokeS11", "")
            }
            is Test422SceSynthInvokeInvoke1State.SubFinal1 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal1")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test422__sce_synth_invoke__invoke_1.scxml:3
    override fun onExit(state: Test422SceSynthInvokeInvoke1State) {
        when (state) {
            is Test422SceSynthInvokeInvoke1State.Sub1 -> {
                activeStateIds.remove("sub1")
            }
            is Test422SceSynthInvokeInvoke1State.SubFinal1 -> {
                activeStateIds.remove("subFinal1")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test422__sce_synth_invoke__invoke_1.scxml:3
    override fun executeTransitionActions(
        source: Test422SceSynthInvokeInvoke1State,
        event: Test422SceSynthInvokeInvoke1Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
