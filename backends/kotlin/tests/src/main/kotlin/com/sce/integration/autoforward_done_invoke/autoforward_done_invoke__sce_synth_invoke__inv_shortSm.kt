// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b
// template-hash: 025e57d78939dcd3c3bbc54b606a62c00b45f367a9a3d9faa2cdd4bf5896d8fc
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_done_invoke/autoforward_done_invoke__sce_synth_invoke__inv_short.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3

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



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDoneInvokeSceSynthInvokeInvShortState? = when (stateId) {
        "over" -> AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState): String = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> "over"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardDoneInvokeSceSynthInvokeInvShortState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvShortEvent
    ): TransitionResult<AutoforwardDoneInvokeSceSynthInvokeInvShortState> = when (state) {
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3
    override fun onEntry(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("over")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3
    override fun onExit(state: AutoforwardDoneInvokeSceSynthInvokeInvShortState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvShortState.Over -> {
                activeStateIds.remove("over")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_short.scxml:3
    override fun executeTransitionActions(
        source: AutoforwardDoneInvokeSceSynthInvokeInvShortState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvShortEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
