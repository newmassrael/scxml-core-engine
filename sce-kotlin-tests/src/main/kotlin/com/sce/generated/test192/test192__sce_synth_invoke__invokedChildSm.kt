// SCE-GENERATED — DO NOT EDIT
// source-hash: f30ff39ee453ff9c2724b237e7ecc70c10c604254c7a79c1bda4dff30c4daac9
// template-hash: 00f78dbe00f429352a6571b71d3b75d9ea5e69ddb859956bf6433b48017951ce
// generated-at: 1780031382

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3

package com.sce.generated.test192

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test192SceSynthInvokeInvokedChildState : State {
    data object Sub0 : Test192SceSynthInvokeInvokedChildState
    data object SubFinal : Test192SceSynthInvokeInvokedChildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test192SceSynthInvokeInvokedChildEvent : Event {
    data object ChildToParent : Test192SceSynthInvokeInvokedChildEvent
    sealed interface Error : Test192SceSynthInvokeInvokedChildEvent {
        data object Execution : Error
    }
    data object EventReceived : Test192SceSynthInvokeInvokedChildEvent
    data object ParentToChild : Test192SceSynthInvokeInvokedChildEvent
    data object Timeout : Test192SceSynthInvokeInvokedChildEvent
}
// --- State Machine (W3C SCXML) ---

class Test192SceSynthInvokeInvokedChildStateMachine(
) : StateMachineEngine<Test192SceSynthInvokeInvokedChildState, Test192SceSynthInvokeInvokedChildEvent>() {

    override val initialState: Test192SceSynthInvokeInvokedChildState = Test192SceSynthInvokeInvokedChildState.Sub0



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test192SceSynthInvokeInvokedChildState? = when (stateId) {
        "sub0" -> Test192SceSynthInvokeInvokedChildState.Sub0
        "subFinal" -> Test192SceSynthInvokeInvokedChildState.SubFinal
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test192SceSynthInvokeInvokedChildState): String = when (state) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> "sub0"
        is Test192SceSynthInvokeInvokedChildState.SubFinal -> "subFinal"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: Test192SceSynthInvokeInvokedChildState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: Test192SceSynthInvokeInvokedChildState): Int = when (state) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> 0
        is Test192SceSynthInvokeInvokedChildState.SubFinal -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): Test192SceSynthInvokeInvokedChildEvent? = when (name) {
        "childToParent" -> Test192SceSynthInvokeInvokedChildEvent.ChildToParent
        "error.execution" -> Test192SceSynthInvokeInvokedChildEvent.Error.Execution
        "eventReceived" -> Test192SceSynthInvokeInvokedChildEvent.EventReceived
        "parentToChild" -> Test192SceSynthInvokeInvokedChildEvent.ParentToChild
        "timeout" -> Test192SceSynthInvokeInvokedChildEvent.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: Test192SceSynthInvokeInvokedChildEvent): String? = when (event) {
        is Test192SceSynthInvokeInvokedChildEvent.ChildToParent -> "childToParent"
        is Test192SceSynthInvokeInvokedChildEvent.Error.Execution -> "error.execution"
        is Test192SceSynthInvokeInvokedChildEvent.EventReceived -> "eventReceived"
        is Test192SceSynthInvokeInvokedChildEvent.ParentToChild -> "parentToChild"
        is Test192SceSynthInvokeInvokedChildEvent.Timeout -> "timeout"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: Test192SceSynthInvokeInvokedChildState,
        event: Test192SceSynthInvokeInvokedChildEvent
    ): TransitionResult<Test192SceSynthInvokeInvokedChildState> = when (state) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> processSub0(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSub0(
        event: Test192SceSynthInvokeInvokedChildEvent
    ): TransitionResult<Test192SceSynthInvokeInvokedChildState> = when {
        event is Test192SceSynthInvokeInvokedChildEvent.ParentToChild -> TransitionResult.External(Test192SceSynthInvokeInvokedChildState.SubFinal, Test192SceSynthInvokeInvokedChildState.Sub0)

        event is Test192SceSynthInvokeInvokedChildEvent.Timeout -> TransitionResult.External(Test192SceSynthInvokeInvokedChildState.SubFinal, Test192SceSynthInvokeInvokedChildState.Sub0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3
    override fun onEntry(state: Test192SceSynthInvokeInvokedChildState) {
        when (state) {
            is Test192SceSynthInvokeInvokedChildState.Sub0 -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("sub0")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")


            scheduleSend("__send_2", 3000L, Test192SceSynthInvokeInvokedChildEvent.Timeout)
            }
            is Test192SceSynthInvokeInvokedChildState.SubFinal -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("subFinal")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3
    override fun onExit(state: Test192SceSynthInvokeInvokedChildState) {
        when (state) {
            is Test192SceSynthInvokeInvokedChildState.Sub0 -> {
                activeStateIds.remove("sub0")
            }
            is Test192SceSynthInvokeInvokedChildState.SubFinal -> {
                activeStateIds.remove("subFinal")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3
    override fun executeTransitionActions(
        source: Test192SceSynthInvokeInvokedChildState,
        event: Test192SceSynthInvokeInvokedChildEvent?
    ) {
        when (source) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> when {
            event is Test192SceSynthInvokeInvokedChildEvent.ParentToChild -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("eventReceived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
