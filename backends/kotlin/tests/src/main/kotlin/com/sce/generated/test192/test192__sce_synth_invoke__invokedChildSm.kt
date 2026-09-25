// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: 
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3 :: _machine

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

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test192SceSynthInvokeInvokedChildState): Boolean = when (state) {
        is Test192SceSynthInvokeInvokedChildState.SubFinal -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test192SceSynthInvokeInvokedChildState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test192SceSynthInvokeInvokedChildState, HistoryId>> =
            listOf(StateTarget(Test192SceSynthInvokeInvokedChildState.Sub0))

        // W3C SCXML 3.13: sub0's transition 0, as the microstep reads it.
        val transitionSub0At0 = EnabledTransition<Test192SceSynthInvokeInvokedChildState, HistoryId>(
            Test192SceSynthInvokeInvokedChildState.Sub0,
            listOf(StateTarget(Test192SceSynthInvokeInvokedChildState.SubFinal)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: sub0's transition 1, as the microstep reads it.
        val transitionSub0At1 = EnabledTransition<Test192SceSynthInvokeInvokedChildState, HistoryId>(
            Test192SceSynthInvokeInvokedChildState.Sub0,
            listOf(StateTarget(Test192SceSynthInvokeInvokedChildState.SubFinal)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test192SceSynthInvokeInvokedChildState,
        event: Test192SceSynthInvokeInvokedChildEvent?
    ): EnabledTransition<Test192SceSynthInvokeInvokedChildState, HistoryId>? = when (state) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> when {
            event is Test192SceSynthInvokeInvokedChildEvent.ParentToChild -> transitionSub0At0
            event is Test192SceSynthInvokeInvokedChildEvent.Timeout -> transitionSub0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3 :: _machine
    override fun onEntry(state: Test192SceSynthInvokeInvokedChildState, isDefaultEntry: Boolean) {
        when (state) {
            is Test192SceSynthInvokeInvokedChildState.Sub0 -> {
                // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:5 :: sub0 :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("childToParent", "")


            scheduleSend("__send_2", 3000L, Test192SceSynthInvokeInvokedChildEvent.Timeout)
            }
            is Test192SceSynthInvokeInvokedChildState.SubFinal -> {
                // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:18 :: subFinal :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3 :: _machine
    override fun onExit(state: Test192SceSynthInvokeInvokedChildState) {
        when (state) {
            is Test192SceSynthInvokeInvokedChildState.Sub0 -> {
                // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:5 :: sub0 :: _state_body
            }
            is Test192SceSynthInvokeInvokedChildState.SubFinal -> {
                // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:18 :: subFinal :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:3 :: _machine
    override fun executeTransitionContent(source: Test192SceSynthInvokeInvokedChildState, transitionIndex: Int) {
        when (source) {
        is Test192SceSynthInvokeInvokedChildState.Sub0 -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: test192__sce_synth_invoke__invokedChild.scxml:11 :: sub0 :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("eventReceived", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
