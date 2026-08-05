// SCE-GENERATED — DO NOT EDIT
// source-hash: 8703a490654d6980486f0b9dbfaf924b4fcfbd6505e2242f771b46a183bf9e7a
// template-hash: 7afc591fda192b42ad8a433570c001416f9be57edde17b6193960abf579021c2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_dequeue_midrun/invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState,
        event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState> = when (state) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> processSaw(event)
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSaw(
        event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState> = when {
        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered, InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState> = when {
        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Kick -> TransitionResult.External(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw, InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting)

        event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed, InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3
    override fun onEntry(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("missed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ordered")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("saw")) return
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3
    override fun onExit(state: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Missed -> {
                activeStateIds.remove("missed")
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Ordered -> {
                activeStateIds.remove("ordered")
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> {
                activeStateIds.remove("saw")
            }
            is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> {
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_dequeue_midrun__sce_synth_invoke__inv_watch.scxml:3
    override fun executeTransitionActions(
        source: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState,
        event: InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent?
    ) {
        when (source) {
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Saw -> when {
            event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawKick", "")
            }
            else -> {}
        }
        is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchState.Waiting -> when {
            event is InvokePrecedesDequeueMidrunSceSynthInvokeInvWatchEvent.Probe -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawNoKick", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
