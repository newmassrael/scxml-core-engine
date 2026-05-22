// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: e798da33d5279236b681cdea18a53a3971a9b769ae5a0bc652a7f8fc89ca7b27
// generated-at: 1779450894

// GENERATED CODE — DO NOT EDIT
// Source: resources/347/test347__sce_synth_invoke__child.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test347__sce_synth_invoke__child.scxml:3

package com.sce.generated.test347

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test347SceSynthInvokeChildState : State {
    data object Sub0 : Test347SceSynthInvokeChildState
    data object SubFinal : Test347SceSynthInvokeChildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test347SceSynthInvokeChildEvent : Event {
    data object ChildToParent : Test347SceSynthInvokeChildEvent
    sealed interface Error : Test347SceSynthInvokeChildEvent {
        data object Execution : Error
    }
    data object ParentToChild : Test347SceSynthInvokeChildEvent
}
// --- State Machine (W3C SCXML) ---

class Test347SceSynthInvokeChildStateMachine(
) : StateMachineEngine<Test347SceSynthInvokeChildState, Test347SceSynthInvokeChildEvent>() {

    override val initialState: Test347SceSynthInvokeChildState = Test347SceSynthInvokeChildState.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test347SceSynthInvokeChildState? = when (stateId) {
        "sub0" -> Test347SceSynthInvokeChildState.Sub0
        "subFinal" -> Test347SceSynthInvokeChildState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test347SceSynthInvokeChildState): String = when (state) {
        is Test347SceSynthInvokeChildState.Sub0 -> "sub0"
        is Test347SceSynthInvokeChildState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test347SceSynthInvokeChildState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test347SceSynthInvokeChildState): Int = when (state) {
        is Test347SceSynthInvokeChildState.Sub0 -> 0
        is Test347SceSynthInvokeChildState.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test347SceSynthInvokeChildEvent? = when (name) {
        "childToParent" -> Test347SceSynthInvokeChildEvent.ChildToParent
        "error.execution" -> Test347SceSynthInvokeChildEvent.Error.Execution
        "parentToChild" -> Test347SceSynthInvokeChildEvent.ParentToChild
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test347SceSynthInvokeChildEvent): String? = when (event) {
        is Test347SceSynthInvokeChildEvent.ChildToParent -> "childToParent"
        is Test347SceSynthInvokeChildEvent.Error.Execution -> "error.execution"
        is Test347SceSynthInvokeChildEvent.ParentToChild -> "parentToChild"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test347SceSynthInvokeChildState,
        event: Test347SceSynthInvokeChildEvent
    ): TransitionResult<Test347SceSynthInvokeChildState> = when (state) {
        is Test347SceSynthInvokeChildState.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test347SceSynthInvokeChildEvent
    ): TransitionResult<Test347SceSynthInvokeChildState> = when {
        event is Test347SceSynthInvokeChildEvent.ParentToChild -> TransitionResult.External(Test347SceSynthInvokeChildState.SubFinal, Test347SceSynthInvokeChildState.Sub0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3
    override fun onEntry(state: Test347SceSynthInvokeChildState) {
        when (state) {
            is Test347SceSynthInvokeChildState.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test347SceSynthInvokeChildState.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3
    override fun onExit(state: Test347SceSynthInvokeChildState) {
        when (state) {
            is Test347SceSynthInvokeChildState.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test347SceSynthInvokeChildState.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3
    override fun executeTransitionActions(
        source: Test347SceSynthInvokeChildState,
        event: Test347SceSynthInvokeChildEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
