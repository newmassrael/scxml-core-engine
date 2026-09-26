// SCE-GENERATED — DO NOT EDIT
// source-hash: f0b11c032ed1e2694bb49d516788c50005f68b4fc73d25543193ba694c46c41b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/cancelling_an_invoke_raises_nothing/cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:3 :: _machine

package com.sce.integration.cancelling_an_invoke_raises_nothing

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface CancellingAnInvokeRaisesNothingSceSynthInvokeChildState : State {
    data object Idle : CancellingAnInvokeRaisesNothingSceSynthInvokeChildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class CancellingAnInvokeRaisesNothingSceSynthInvokeChildStateMachine(
) : StateMachineEngine<CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent>() {

    override val initialState: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState = CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, HistoryId>> =
            listOf(StateTarget(CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): CancellingAnInvokeRaisesNothingSceSynthInvokeChildState? = when (stateId) {
        "idle" -> CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState): String = when (state) {
        is CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState): Int = when (state) {
        is CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent? = when (name) {
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent): String? = null





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState,
        event: CancellingAnInvokeRaisesNothingSceSynthInvokeChildEvent?
    ): EnabledTransition<CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:3 :: _machine
    override fun onEntry(state: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, isDefaultEntry: Boolean) {
        when (state) {
            is CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:4 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:3 :: _machine
    override fun onExit(state: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState) {
        when (state) {
            is CancellingAnInvokeRaisesNothingSceSynthInvokeChildState.Idle -> {
                // SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:4 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: cancelling_an_invoke_raises_nothing__sce_synth_invoke__child.scxml:3 :: _machine
    override fun executeTransitionContent(source: CancellingAnInvokeRaisesNothingSceSynthInvokeChildState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
