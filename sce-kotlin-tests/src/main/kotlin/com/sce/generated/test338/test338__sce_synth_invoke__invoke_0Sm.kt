// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 168f4a554705bdfb42cc51a9fbd01e4e5fc028c49c4d6f47071af9577599e075
// generated-at: 1779449862

// GENERATED CODE — DO NOT EDIT
// Source: resources/338/test338__sce_synth_invoke__invoke_0.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test338

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test338SceSynthInvokeInvoke0State : State {
    data object Sub0 : Test338SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test338SceSynthInvokeInvoke0Event : Event {
    sealed interface Error : Test338SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
    data object Event1 : Test338SceSynthInvokeInvoke0Event
}
// --- State Machine (W3C SCXML) ---

class Test338SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test338SceSynthInvokeInvoke0State, Test338SceSynthInvokeInvoke0Event>() {

    override val initialState: Test338SceSynthInvokeInvoke0State = Test338SceSynthInvokeInvoke0State.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test338SceSynthInvokeInvoke0State? = when (stateId) {
        "sub0" -> Test338SceSynthInvokeInvoke0State.Sub0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test338SceSynthInvokeInvoke0State): String = when (state) {
        is Test338SceSynthInvokeInvoke0State.Sub0 -> "sub0"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test338SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test338SceSynthInvokeInvoke0State): Int = when (state) {
        is Test338SceSynthInvokeInvoke0State.Sub0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test338SceSynthInvokeInvoke0Event? = when (name) {
        "error.execution" -> Test338SceSynthInvokeInvoke0Event.Error.Execution
        "event1" -> Test338SceSynthInvokeInvoke0Event.Event1
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test338SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test338SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
        is Test338SceSynthInvokeInvoke0Event.Event1 -> "event1"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test338SceSynthInvokeInvoke0State,
        event: Test338SceSynthInvokeInvoke0Event
    ): TransitionResult<Test338SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test338SceSynthInvokeInvoke0State) {
        when (state) {
            is Test338SceSynthInvokeInvoke0State.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("event1", "")
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test338SceSynthInvokeInvoke0State) {
        when (state) {
            is Test338SceSynthInvokeInvoke0State.Sub0 -> {
                activeStateIds.remove("sub0")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test338__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test338SceSynthInvokeInvoke0State,
        event: Test338SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
