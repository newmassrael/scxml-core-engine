// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3 :: _machine

package com.sce.integration.invoke_precedes_dequeue_midrun

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState : State {
    data object Missed : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Ordered : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Saw : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
    data object Waiting : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent : Event {
    sealed interface Error : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent {
        data object Execution : Error
    }
    data object Kick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object Probe : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object Ready : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object SawKick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    data object SawNoKick : InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent>() {

    override val initialState: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState = InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting

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
    override fun isFinalState(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): Boolean = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed, is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting))

        // W3C SCXML 3.13: saw's transition 0, as the microstep reads it.
        val transitionSawAt0 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>(
            InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting,
            listOf(StateTarget(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed
        "ordered" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered
        "saw" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw
        "waiting" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): String = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> "missed"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> "ordered"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> "saw"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): Int = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> 2
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> 3
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> 1
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent? = when (name) {
        "error.execution" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Execution
        "kick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick
        "probe" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe
        "ready" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Ready
        "sawKick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawKick
        "sawNoKick" -> InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawNoKick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent): String? = when (event) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick -> "kick"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Ready -> "ready"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawKick -> "sawKick"
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.SawNoKick -> "sawNoKick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState,
        event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent?
    ): EnabledTransition<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> when {
            event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> transitionSawAt0
            else -> null
        }
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> when {
            event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick -> transitionWaitingAt0
            event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> transitionWaitingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onEntry(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:19 :: missed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:20 :: ordered :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:14 :: saw :: _state_body
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:5 :: waiting :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onExit(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:19 :: missed :: _state_body
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:20 :: ordered :: _state_body
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:14 :: saw :: _state_body
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:5 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun executeTransitionContent(source: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:15 :: saw :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawKick", "")
            }
            else -> {}
        }
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> when (transitionIndex) {
            1 -> {
                // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:10 :: waiting :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawNoKick", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
