// SCE-GENERATED — DO NOT EDIT
// source-hash: 853e159a7eb79ab15c04e875f7bec1835f6503b1cce8655a99ae83b3ec39b1dc

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/a_delayed_send_reaches_what_its_target_names/a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:3 :: _machine

package com.sce.integration.a_delayed_send_reaches_what_its_target_names

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState : State {
    data object Idle : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState
    data object Over : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent : Event {
    sealed interface Error : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent {
        data object Execution : Error
    }
    data object Hello : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent
    data object Lost : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent
    data object LostReached : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent
    data object Stop : ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent
}
// --- State Machine (W3C SCXML) ---

class ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneStateMachine(
) : StateMachineEngine<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent>() {

    override val initialState: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState = ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle

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
    override fun isFinalState(state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState): Boolean = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, HistoryId>> =
            listOf(StateTarget(ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, HistoryId>(
            ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle,
            listOf(StateTarget(ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState? = when (stateId) {
        "idle" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle
        "over" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState): String = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> "idle"
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over -> "over"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState): Int = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> 0
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent? = when (name) {
        "error.execution" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Error.Execution
        "hello" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Hello
        "lost" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Lost
        "lostReached" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.LostReached
        "stop" -> ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Stop
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent): String? = when (event) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Error.Execution -> "error.execution"
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Hello -> "hello"
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Lost -> "lost"
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.LostReached -> "lostReached"
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Stop -> "stop"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState,
        event: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent?
    ): EnabledTransition<ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, HistoryId>? = when (state) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> when {
            event is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Lost -> transitionIdleAt0
            event is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneEvent.Stop -> transitionIdleAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:3 :: _machine
    override fun onEntry(state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, isDefaultEntry: Boolean) {
        when (state) {
            is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:5 :: idle :: _state_body
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
            is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:14 :: over :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:3 :: _machine
    override fun onExit(state: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState) {
        when (state) {
            is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:5 :: idle :: _state_body
            }
            is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Over -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:14 :: over :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:3 :: _machine
    override fun executeTransitionContent(source: ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState, transitionIndex: Int) {
        when (source) {
        is ADelayedSendReachesWhatItsTargetNamesSceSynthInvokeGoneState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: a_delayed_send_reaches_what_its_target_names__sce_synth_invoke__gone.scxml:9 :: idle :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("lostReached", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
