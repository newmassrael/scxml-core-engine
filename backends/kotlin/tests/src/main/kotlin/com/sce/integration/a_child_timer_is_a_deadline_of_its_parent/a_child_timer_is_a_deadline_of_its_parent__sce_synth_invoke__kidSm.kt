// SCE-GENERATED — DO NOT EDIT
// source-hash: 928525b042b933add53b083473e027b93afffa099739addb8c269c7194b0e79d

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:3 :: _machine

package com.sce.integration.a_child_timer_is_a_deadline_of_its_parent

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState : State {
    data object First : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState
    data object Second : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState
    data object Spoke : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent : Event {
    sealed interface Error : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent {
        data object Execution : Error
    }
    data object One : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent
    data object Two : AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent
}
// --- State Machine (W3C SCXML) ---

class AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidStateMachine(
) : StateMachineEngine<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent>() {

    override val initialState: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState = AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState): Boolean = when (state) {
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, HistoryId>> =
            listOf(StateTarget(AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First))

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, HistoryId>(
            AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First,
            listOf(StateTarget(AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, HistoryId>(
            AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second,
            listOf(StateTarget(AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState? = when (stateId) {
        "first" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First
        "second" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second
        "spoke" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState): String = when (state) {
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First -> "first"
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second -> "second"
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke -> "spoke"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState): Int = when (state) {
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First -> 0
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second -> 1
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent? = when (name) {
        "error.execution" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Error.Execution
        "one" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.One
        "two" -> AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Two
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent): String? = when (event) {
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Error.Execution -> "error.execution"
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.One -> "one"
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Two -> "two"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState,
        event: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent?
    ): EnabledTransition<AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, HistoryId>? = when (state) {
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First -> when {
            event is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.One -> transitionFirstAt0
            else -> null
        }
        is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second -> when {
            event is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Two -> transitionSecondAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun onEntry(state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, isDefaultEntry: Boolean) {
        when (state) {
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:5 :: first :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 200L, AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.One, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:11 :: second :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_1", 200L, AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidEvent.Two, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:17 :: spoke :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun onExit(state: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState) {
        when (state) {
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.First -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:5 :: first :: _state_body
            }
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Second -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:11 :: second :: _state_body
            }
            is AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState.Spoke -> {
                // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:17 :: spoke :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_child_timer_is_a_deadline_of_its_parent__sce_synth_invoke__kid.scxml:3 :: _machine
    override fun executeTransitionContent(source: AChildTimerIsADeadlineOfItsParentSceSynthInvokeKidState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
