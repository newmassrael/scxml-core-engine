// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d102d83be3372e77ae8dde1c74b18f608301e96c6945cff9b996a5dc929e6ec

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml#emitter
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: inline_content_is_read_by_its_data_model.scxml:46 :: _machine

package com.sce.integration.inline_content_is_read_by_its_data_model

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState : State {
    data object Emit : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState
    data object Sent : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent : Event {
    sealed interface Error : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Number : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent
    data object Object : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent
    data object Text : InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent
}
// --- State Machine (W3C SCXML) ---

class InlineContentIsReadByItsDataModelSceSynthInvokeEmitterStateMachine(
) : StateMachineEngine<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent>() {

    override val initialState: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState = InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState): Boolean = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent -> true
        else -> false
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, HistoryId>> =
            listOf(StateTarget(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: emit's transition 0, as the microstep reads it.
        val transitionEmitAt0 = EnabledTransition<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, HistoryId>(
            InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit,
            listOf(StateTarget(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState? = when (stateId) {
        "emit" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit
        "sent" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState): String = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit -> "emit"
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent -> "sent"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState): Int = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit -> 0
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent -> 1
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent? = when (name) {
        "error.communication" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Communication
        "error.execution" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Execution
        "number" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Number
        "object" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Object
        "text" -> InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Text
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent): String? = when (event) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Communication -> "error.communication"
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Execution -> "error.execution"
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Number -> "number"
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Object -> "object"
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Text -> "text"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, event: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent?): EnabledTransition<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, HistoryId>? = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit -> firstEnabledTransitionOfEmit(state, event)
        else -> null
    }


    // emit's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfEmit(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, event: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent?): EnabledTransition<InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, HistoryId>? = when {
        event == null -> TransitionsPart0.transitionEmitAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:46 :: _machine
    override fun onEntry(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, isDefaultEntry: Boolean) {
        when (state) {
            is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit -> onEntryOfEmit()
            is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent -> onEntryOfSent()
        }
    }


    private fun onEntryOfEmit() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:48 :: emit :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = "21"
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("number", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = "\"two words\""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("text", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = "{\"k\":7}"
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(InlineContentIsReadByItsDataModelSceSynthInvokeEmitterEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_2")
                return@send true
            }
            toParent("object", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
    }

    private fun onEntryOfSent() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:59 :: sent :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:46 :: _machine
    override fun onExit(state: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState) {
        when (state) {
            is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Emit -> onExitOfEmit()
            is InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState.Sent -> onExitOfSent()
        }
    }


    private fun onExitOfEmit() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:48 :: emit :: _state_body
    }

    private fun onExitOfSent() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:59 :: sent :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:46 :: _machine
    override fun executeTransitionContent(source: InlineContentIsReadByItsDataModelSceSynthInvokeEmitterState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
