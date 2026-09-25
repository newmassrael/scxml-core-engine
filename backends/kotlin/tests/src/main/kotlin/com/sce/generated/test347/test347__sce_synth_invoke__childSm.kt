// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test347__sce_synth_invoke__child.scxml:3 :: _machine

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

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test347SceSynthInvokeChildState): Boolean = when (state) {
        is Test347SceSynthInvokeChildState.SubFinal -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test347SceSynthInvokeChildState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test347SceSynthInvokeChildState, HistoryId>> =
            listOf(StateTarget(Test347SceSynthInvokeChildState.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test347SceSynthInvokeChildState, HistoryId>(
            Test347SceSynthInvokeChildState.Sub0,
            listOf(StateTarget(Test347SceSynthInvokeChildState.SubFinal)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test347SceSynthInvokeChildState,
        event: Test347SceSynthInvokeChildEvent?
    ): EnabledTransition<Test347SceSynthInvokeChildState, HistoryId>? = when (state) {
        is Test347SceSynthInvokeChildState.Sub0 -> when {
            event is Test347SceSynthInvokeChildEvent.ParentToChild -> transitionSub0At0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3 :: _machine
    override fun onEntry(state: Test347SceSynthInvokeChildState, isDefaultEntry: Boolean) {
        when (state) {
            is Test347SceSynthInvokeChildState.Sub0 -> {
                // SCE-MAP: test347__sce_synth_invoke__child.scxml:4 :: sub0 :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")
            }
            is Test347SceSynthInvokeChildState.SubFinal -> {
                // SCE-MAP: test347__sce_synth_invoke__child.scxml:10 :: subFinal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3 :: _machine
    override fun onExit(state: Test347SceSynthInvokeChildState) {
        when (state) {
            is Test347SceSynthInvokeChildState.Sub0 -> {
                // SCE-MAP: test347__sce_synth_invoke__child.scxml:4 :: sub0 :: _state_body
            }
            is Test347SceSynthInvokeChildState.SubFinal -> {
                // SCE-MAP: test347__sce_synth_invoke__child.scxml:10 :: subFinal :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test347__sce_synth_invoke__child.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test347SceSynthInvokeChildState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
