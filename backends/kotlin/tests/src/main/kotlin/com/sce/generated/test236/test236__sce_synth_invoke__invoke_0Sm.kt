// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a
// template-hash: 670395eefe7272d78e62bf7a7fd9181e96e4a744175a58a4c4de1240c73f57bc
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test236

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test236SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test236SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test236SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent : Test236SceSynthInvokeInvoke0Event
    sealed interface Error : Test236SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test236SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test236SceSynthInvokeInvoke0State, Test236SceSynthInvokeInvoke0Event>() {

    override val initialState: Test236SceSynthInvokeInvoke0State = Test236SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test236SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test236SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test236SceSynthInvokeInvoke0State): String = when (state) {
        is Test236SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test236SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test236SceSynthInvokeInvoke0State): Int = when (state) {
        is Test236SceSynthInvokeInvoke0State.SubFinal -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test236SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent" -> Test236SceSynthInvokeInvoke0Event.ChildToParent
        "error.execution" -> Test236SceSynthInvokeInvoke0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test236SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test236SceSynthInvokeInvoke0Event.ChildToParent -> "childToParent"
        is Test236SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test236SceSynthInvokeInvoke0State,
        event: Test236SceSynthInvokeInvoke0Event
    ): TransitionResult<Test236SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test236SceSynthInvokeInvoke0State) {
        when (state) {
            is Test236SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test236SceSynthInvokeInvoke0State) {
        when (state) {
            is Test236SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test236__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test236SceSynthInvokeInvoke0State,
        event: Test236SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
