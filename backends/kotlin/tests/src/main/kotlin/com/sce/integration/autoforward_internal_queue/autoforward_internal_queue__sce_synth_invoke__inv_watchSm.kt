// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c78d9a40e778435f5ba721a7a12bf6721453dde3c80246e5018de3fc670010
// template-hash: 5acba0e3347282f793223e6756c0e705a2e09e70e21550d5eb5dc6ae9d6f33ae
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_internal_queue/autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3

package com.sce.integration.autoforward_internal_queue

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardInternalQueueSceSynthInvokeInvWatchState : State {
    data object Clean : AutoforwardInternalQueueSceSynthInvokeInvWatchState
    data object Leaked : AutoforwardInternalQueueSceSynthInvokeInvWatchState
    data object Watch : AutoforwardInternalQueueSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardInternalQueueSceSynthInvokeInvWatchEvent : Event {
    sealed interface Error : AutoforwardInternalQueueSceSynthInvokeInvWatchEvent {
        data object Execution : Error
    }
    data object Probe : AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
    data object Ready : AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
    data object SawInternal : AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
    data object SawProbeOnly : AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardInternalQueueSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<AutoforwardInternalQueueSceSynthInvokeInvWatchState, AutoforwardInternalQueueSceSynthInvokeInvWatchEvent>() {

    override val initialState: AutoforwardInternalQueueSceSynthInvokeInvWatchState = AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardInternalQueueSceSynthInvokeInvWatchState? = when (stateId) {
        "clean" -> AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean
        "leaked" -> AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked
        "watch" -> AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState): String = when (state) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> "clean"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> "leaked"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> "watch"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState): Int = when (state) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> 2
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> 1
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardInternalQueueSceSynthInvokeInvWatchEvent? = when (name) {
        "error.execution" -> AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Error.Execution
        "probe" -> AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Probe
        "ready" -> AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Ready
        "sawInternal" -> AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.SawInternal
        "sawProbeOnly" -> AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.SawProbeOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardInternalQueueSceSynthInvokeInvWatchEvent): String? = when (event) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Ready -> "ready"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.SawInternal -> "sawInternal"
        is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.SawProbeOnly -> "sawProbeOnly"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardInternalQueueSceSynthInvokeInvWatchState,
        event: AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
    ): TransitionResult<AutoforwardInternalQueueSceSynthInvokeInvWatchState> = when (state) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> processWatch(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWatch(
        event: AutoforwardInternalQueueSceSynthInvokeInvWatchEvent
    ): TransitionResult<AutoforwardInternalQueueSceSynthInvokeInvWatchState> = when {
        event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Error.Execution -> TransitionResult.External(AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked, AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch)

        event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean, AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3
    override fun onEntry(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("clean")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("leaked")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3
    override fun onExit(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> {
                activeStateIds.remove("clean")
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> {
                activeStateIds.remove("leaked")
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> {
                activeStateIds.remove("watch")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3
    override fun executeTransitionActions(
        source: AutoforwardInternalQueueSceSynthInvokeInvWatchState,
        event: AutoforwardInternalQueueSceSynthInvokeInvWatchEvent?
    ) {
        when (source) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> when {
            event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Error.Execution -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawInternal", "")
            }
            event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Probe -> {


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawProbeOnly", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
