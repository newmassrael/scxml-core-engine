// SCE-GENERATED — DO NOT EDIT
// source-hash: 7c010da1526dce3962148a99023f795b5efd3dc066529da8bc2dc12378934900
// template-hash: bd73e9c4959650ef155c95407520cc3ba824fd0d9a11219706cab720c4701123
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/invoke_precedes_external_dequeue/invoke_precedes_external_dequeue__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: invoke_precedes_external_dequeue__sce_synth_invoke__inv_watch.scxml:3

package com.sce.integration.invoke_precedes_external_dequeue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState : State {
    data object Missed : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Ordered : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Saw : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
    data object Waiting : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent : Event {
    sealed interface Error : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent {
        data object Execution : Error
    }
    data object Kick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object Probe : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object Ready : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object SawKick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    data object SawNoKick : InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class InvokePrecedesExternalDequeueSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState, InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent>() {

    override val initialState: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState = InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed
        "ordered" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered
        "saw" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw
        "waiting" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): String = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> "missed"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> "ordered"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> "saw"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState): Int = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> 2
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> 3
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> 1
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent? = when (name) {
        "error.execution" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Execution
        "kick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick
        "probe" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe
        "ready" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Ready
        "sawKick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawKick
        "sawNoKick" -> InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawNoKick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent): String? = when (event) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick -> "kick"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Ready -> "ready"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawKick -> "sawKick"
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.SawNoKick -> "sawNoKick"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState,
        event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState> = when (state) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> processSaw(event)
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processSaw(
        event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState> = when {
        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered, InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent
    ): TransitionResult<InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState> = when {
        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Kick -> TransitionResult.External(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw, InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting)

        event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed, InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: invoke_precedes_external_dequeue__sce_synth_invoke__inv_watch.scxml:3
    override fun onEntry(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("missed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("ordered")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("saw")) return
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: invoke_precedes_external_dequeue__sce_synth_invoke__inv_watch.scxml:3
    override fun onExit(state: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState) {
        when (state) {
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Missed -> {
                activeStateIds.remove("missed")
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Ordered -> {
                activeStateIds.remove("ordered")
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> {
                activeStateIds.remove("saw")
            }
            is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> {
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: invoke_precedes_external_dequeue__sce_synth_invoke__inv_watch.scxml:3
    override fun executeTransitionActions(
        source: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState,
        event: InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent?
    ) {
        when (source) {
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Saw -> when {
            event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawKick", "")
            }
            else -> {}
        }
        is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchState.Waiting -> when {
            event is InvokePrecedesExternalDequeueSceSynthInvokeInvWatchEvent.Probe -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawNoKick", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
