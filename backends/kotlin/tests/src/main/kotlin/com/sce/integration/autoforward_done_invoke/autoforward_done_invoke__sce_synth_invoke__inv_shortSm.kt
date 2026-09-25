// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_done_invoke/autoforward_done_invoke__sce_synth_invoke__inv_short.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3 :: _machine

package com.sce.integration.autoforward_done_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvShortState : State {
    data object Over : AutoforwardDoneInvokeSceSynthInvokeInvShortState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvShortEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class AutoforwardDoneInvokeSceSynthInvokeInvShortStateMachine(
) : StateMachineEngine<AutoforwardDoneInvokeSceSynthInvokeInvShortState, AutoforwardDoneInvokeSceSynthInvokeInvShortEvent>() {

    override val initialState: AutoforwardDoneInvokeSceSynthInvokeInvShortState = AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over

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
    override fun isFinalState(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState): Boolean = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> true
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvShortState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvShortState, HistoryId>> =
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDoneInvokeSceSynthInvokeInvShortState? = when (stateId) {
        "over" -> AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState): String = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> "over"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState): Int = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDoneInvokeSceSynthInvokeInvShortEvent? = when (name) {
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `AutoforwardDoneInvokeSceSynthInvokeInvShortEvent` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: AutoforwardDoneInvokeSceSynthInvokeInvShortEvent): String? = null





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardDoneInvokeSceSynthInvokeInvShortState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvShortEvent?
    ): EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvShortState, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:5 :: over :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3 :: _machine
    override fun onExit(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:5 :: over :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3 :: _machine
    override fun executeTransitionContent(source: AutoforwardDoneInvokeSceSynthInvokeInvShortState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
