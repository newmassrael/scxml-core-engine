// SCE-GENERATED — DO NOT EDIT
// source-hash: f6c78d9a40e778435f5ba721a7a12bf6721453dde3c80246e5018de3fc670010

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_internal_queue/autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3 :: _machine

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
    override fun isFinalState(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState): Boolean = when (state) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean, is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardInternalQueueSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardInternalQueueSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch))

        // W3C SCXML 3.13: watch's transition 0, as the microstep reads it.
        val transitionWatchAt0 = EnabledTransition<AutoforwardInternalQueueSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: watch's transition 1, as the microstep reads it.
        val transitionWatchAt1 = EnabledTransition<AutoforwardInternalQueueSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardInternalQueueSceSynthInvokeInvWatchState,
        event: AutoforwardInternalQueueSceSynthInvokeInvWatchEvent?
    ): EnabledTransition<AutoforwardInternalQueueSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> when {
            event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Error.Execution -> transitionWatchAt0
            event is AutoforwardInternalQueueSceSynthInvokeInvWatchEvent.Probe -> transitionWatchAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:17 :: clean :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:16 :: leaked :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onExit(state: AutoforwardInternalQueueSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Clean -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:17 :: clean :: _state_body
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Leaked -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:16 :: leaked :: _state_body
            }
            is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun executeTransitionContent(source: AutoforwardInternalQueueSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
        is AutoforwardInternalQueueSceSynthInvokeInvWatchState.Watch -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:9 :: watch :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawInternal", "")
            }
            1 -> {
                // SCE-MAP: autoforward_internal_queue__sce_synth_invoke__inv_watch.scxml:12 :: watch :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawProbeOnly", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
