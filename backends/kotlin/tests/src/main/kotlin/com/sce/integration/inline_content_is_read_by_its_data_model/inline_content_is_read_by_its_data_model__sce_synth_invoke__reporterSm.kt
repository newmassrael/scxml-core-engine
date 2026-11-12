// SCE-GENERATED — DO NOT EDIT
// source-hash: 5d102d83be3372e77ae8dde1c74b18f608301e96c6945cff9b996a5dc929e6ec

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/inline_content_is_read_by_its_data_model/inline_content_is_read_by_its_data_model.scxml#reporter
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: inline_content_is_read_by_its_data_model.scxml:86 :: _machine

package com.sce.integration.inline_content_is_read_by_its_data_model

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InlineContentIsReadByItsDataModelSceSynthInvokeReporterState : State {
    data object Reported : InlineContentIsReadByItsDataModelSceSynthInvokeReporterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent : Event {
    sealed interface Error : InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class InlineContentIsReadByItsDataModelSceSynthInvokeReporterStateMachine(
) : StateMachineEngine<InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent>() {

    override val initialState: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState = InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState): Boolean = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported -> true
    }


    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, HistoryId>> =
            listOf(StateTarget(InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported))
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InlineContentIsReadByItsDataModelSceSynthInvokeReporterState? = when (stateId) {
        "reported" -> InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState): String = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported -> "reported"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState): Int = when (state) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent? = when (name) {
        "error.execution" -> InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent): String? = when (event) {
        is InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, event: InlineContentIsReadByItsDataModelSceSynthInvokeReporterEvent?): EnabledTransition<InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, HistoryId>? = when (state) {
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:86 :: _machine
    override fun onEntry(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, isDefaultEntry: Boolean) {
        when (state) {
            is InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported -> onEntryOfReported()
        }
    }


    private fun onEntryOfReported() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:88 :: reported :: _state_body
                // W3C SCXML 5.5: Evaluate donedata for final state
                run {
                    var doneEventData = ""
                    // W3C SCXML 5.5: inline text is the content value, finished at build time.
                    // It lands in `contentData`, the sink the expression-site census
                    // adjudicates as verbatim (`INERT_DESTINATIONS`).
                    val contentData = "<books xmlns=\"\"><book title=\"x\"></book></books>"
                    doneEventData = contentData
                    // W3C SCXML 5.5 + 6.3.1: stash onto the engine so the invoking parent's
                    // startInvoke completion callback can lift the payload onto
                    // done.invoke.<id>._event.data. Mirrors C++ AOT stashDonedataAtFinal.
                    stashDonedataAtFinal(doneEventData)
                }
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:86 :: _machine
    override fun onExit(state: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState) {
        when (state) {
            is InlineContentIsReadByItsDataModelSceSynthInvokeReporterState.Reported -> onExitOfReported()
        }
    }


    private fun onExitOfReported() {
                // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:88 :: reported :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: inline_content_is_read_by_its_data_model.scxml:86 :: _machine
    override fun executeTransitionContent(source: InlineContentIsReadByItsDataModelSceSynthInvokeReporterState, transitionIndex: Int) {
        when (source) {
            else -> {}
        }
    }
}
