// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 685fb4e0713193a522c8703edbc4c7f9a7c6eb1a29822dc1f9bfa6c38d3bf333
// generated-at: 1780579912

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test232__sce_synth_invoke__invoke_0.scxml:3

package com.sce.generated.test232

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test232SceSynthInvokeInvoke0State : State {
    data object SubFinal : Test232SceSynthInvokeInvoke0State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test232SceSynthInvokeInvoke0Event : Event {
    data object ChildToParent1 : Test232SceSynthInvokeInvoke0Event
    data object ChildToParent2 : Test232SceSynthInvokeInvoke0Event
    sealed interface Error : Test232SceSynthInvokeInvoke0Event {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class Test232SceSynthInvokeInvoke0StateMachine(
) : StateMachineEngine<Test232SceSynthInvokeInvoke0State, Test232SceSynthInvokeInvoke0Event>() {

    override val initialState: Test232SceSynthInvokeInvoke0State = Test232SceSynthInvokeInvoke0State.SubFinal



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test232SceSynthInvokeInvoke0State? = when (stateId) {
        "subFinal" -> Test232SceSynthInvokeInvoke0State.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test232SceSynthInvokeInvoke0State): String = when (state) {
        is Test232SceSynthInvokeInvoke0State.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test232SceSynthInvokeInvoke0State): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test232SceSynthInvokeInvoke0State): Int = when (state) {
        is Test232SceSynthInvokeInvoke0State.SubFinal -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test232SceSynthInvokeInvoke0Event? = when (name) {
        "childToParent1" -> Test232SceSynthInvokeInvoke0Event.ChildToParent1
        "childToParent2" -> Test232SceSynthInvokeInvoke0Event.ChildToParent2
        "error.execution" -> Test232SceSynthInvokeInvoke0Event.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test232SceSynthInvokeInvoke0Event): String? = when (event) {
        is Test232SceSynthInvokeInvoke0Event.ChildToParent1 -> "childToParent1"
        is Test232SceSynthInvokeInvoke0Event.ChildToParent2 -> "childToParent2"
        is Test232SceSynthInvokeInvoke0Event.Error.Execution -> "error.execution"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test232SceSynthInvokeInvoke0State,
        event: Test232SceSynthInvokeInvoke0Event
    ): TransitionResult<Test232SceSynthInvokeInvoke0State> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test232__sce_synth_invoke__invoke_0.scxml:3
    override fun onEntry(state: Test232SceSynthInvokeInvoke0State) {
        when (state) {
            is Test232SceSynthInvokeInvoke0State.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent1", "")


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent2", "")
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test232__sce_synth_invoke__invoke_0.scxml:3
    override fun onExit(state: Test232SceSynthInvokeInvoke0State) {
        when (state) {
            is Test232SceSynthInvokeInvoke0State.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test232__sce_synth_invoke__invoke_0.scxml:3
    override fun executeTransitionActions(
        source: Test232SceSynthInvokeInvoke0State,
        event: Test232SceSynthInvokeInvoke0Event?
    ) {
        when (source) {
        else -> {}
        }
    }
}
