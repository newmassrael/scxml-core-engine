// SCE-GENERATED — DO NOT EDIT
// source-hash: d6cb6d8e9f3a39015ea70f70076b5982d6409d91636abf5c1ae6682daf0bd246

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_child_reply_arrives_without_a_tick/a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:3 :: _machine

package com.sce.integration.a_child_reply_arrives_without_a_tick

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AChildReplyArrivesWithoutATickSceSynthInvokeKidState : State {
    data object C0 : AChildReplyArrivesWithoutATickSceSynthInvokeKidState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent : Event {
    sealed interface Error : AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent {
        data object Execution : Error
    }
    data object Hello : AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent
}
// --- State Machine (W3C SCXML) ---

class AChildReplyArrivesWithoutATickSceSynthInvokeKidStateMachine(
) : StateMachineEngine<AChildReplyArrivesWithoutATickSceSynthInvokeKidState, AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent>() {

    override val initialState: AChildReplyArrivesWithoutATickSceSynthInvokeKidState = AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0

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
    override val documentInitialTargets: List<EntryTarget<AChildReplyArrivesWithoutATickSceSynthInvokeKidState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AChildReplyArrivesWithoutATickSceSynthInvokeKidState, HistoryId>> =
            listOf(StateTarget(AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0))
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AChildReplyArrivesWithoutATickSceSynthInvokeKidState? = when (stateId) {
        "c0" -> AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AChildReplyArrivesWithoutATickSceSynthInvokeKidState): String = when (state) {
        is AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0 -> "c0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AChildReplyArrivesWithoutATickSceSynthInvokeKidState): Int = when (state) {
        is AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0 -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent? = when (name) {
        "error.execution" -> AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent.Error.Execution
        "hello" -> AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent.Hello
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent): String? = when (event) {
        is AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent.Error.Execution -> "error.execution"
        is AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent.Hello -> "hello"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AChildReplyArrivesWithoutATickSceSynthInvokeKidState,
        event: AChildReplyArrivesWithoutATickSceSynthInvokeKidEvent?
    ): EnabledTransition<AChildReplyArrivesWithoutATickSceSynthInvokeKidState, HistoryId>? = when (state) {
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun onEntry(state: AChildReplyArrivesWithoutATickSceSynthInvokeKidState, isDefaultEntry: Boolean) {
        when (state) {
            is AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0 -> {
                // SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:5 :: c0 :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("hello", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun onExit(state: AChildReplyArrivesWithoutATickSceSynthInvokeKidState) {
        when (state) {
            is AChildReplyArrivesWithoutATickSceSynthInvokeKidState.C0 -> {
                // SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:5 :: c0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_child_reply_arrives_without_a_tick__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun executeTransitionContent(source: AChildReplyArrivesWithoutATickSceSynthInvokeKidState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
