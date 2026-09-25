// SCE-GENERATED — DO NOT EDIT
// source-hash: 34a3aa3a202a7ed359ee0ab4d1bade13a9630715ad1b32496ec4d19b29db3f4f

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_candidate_selects_the_child/chosen.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: chosen.scxml:5 :: _machine

package com.sce.integration.invoke_candidate_selects_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ChosenState : State {
    data object Done : ChosenState
    data object Speak : ChosenState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ChosenEvent : Event {
    sealed interface Error : ChosenEvent {
        data object Execution : Error
    }
    sealed interface From : ChosenEvent {
        data object Chosen : From
    }
}
// --- State Machine (W3C SCXML) ---

class ChosenStateMachine(
) : StateMachineEngine<ChosenState, ChosenEvent>() {

    override val initialState: ChosenState = ChosenState.Speak

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
    override fun isFinalState(state: ChosenState): Boolean = when (state) {
        is ChosenState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ChosenState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ChosenState, HistoryId>> =
            listOf(StateTarget(ChosenState.Speak))

        // W3C SCXML 3.13: speak's transition 0, as the microstep reads it.
        val transitionSpeakAt0 = EnabledTransition<ChosenState, HistoryId>(
            ChosenState.Speak,
            listOf(StateTarget(ChosenState.Done)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ChosenState? = when (stateId) {
        "done" -> ChosenState.Done
        "speak" -> ChosenState.Speak
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ChosenState): String = when (state) {
        is ChosenState.Done -> "done"
        is ChosenState.Speak -> "speak"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ChosenState): Int = when (state) {
        is ChosenState.Done -> 1
        is ChosenState.Speak -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ChosenEvent? = when (name) {
        "error.execution" -> ChosenEvent.Error.Execution
        "from.chosen" -> ChosenEvent.From.Chosen
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ChosenEvent): String? = when (event) {
        is ChosenEvent.Error.Execution -> "error.execution"
        is ChosenEvent.From.Chosen -> "from.chosen"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ChosenState,
        event: ChosenEvent?
    ): EnabledTransition<ChosenState, HistoryId>? = when (state) {
        is ChosenState.Speak -> when {
            event == null -> transitionSpeakAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun onEntry(state: ChosenState, isDefaultEntry: Boolean) {
        when (state) {
            is ChosenState.Done -> {
                // SCE-MAP: chosen.scxml:13 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is ChosenState.Speak -> {
                // SCE-MAP: chosen.scxml:7 :: speak :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("from.chosen", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun onExit(state: ChosenState) {
        when (state) {
            is ChosenState.Done -> {
                // SCE-MAP: chosen.scxml:13 :: done :: _state_body
            }
            is ChosenState.Speak -> {
                // SCE-MAP: chosen.scxml:7 :: speak :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: chosen.scxml:5 :: _machine
    override fun executeTransitionContent(source: ChosenState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
