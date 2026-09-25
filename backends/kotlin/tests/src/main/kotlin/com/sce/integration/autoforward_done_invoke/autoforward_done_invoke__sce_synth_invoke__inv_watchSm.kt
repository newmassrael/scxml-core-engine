// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/autoforward_done_invoke/autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine

package com.sce.integration.autoforward_done_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvWatchState : State {
    data object Missed : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
    data object Saw : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
    data object Watch : AutoforwardDoneInvokeSceSynthInvokeInvWatchState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent : Event {
    sealed interface Done : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent {
        sealed interface Invoke : Done {
            data object InvShort : Invoke
        }
    }
    sealed interface Error : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent {
        data object Execution : Error
    }
    data object Probe : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    data object SawPlatform : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    data object SawProbeOnly : AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
}
// --- State Machine (W3C SCXML) ---

class AutoforwardDoneInvokeSceSynthInvokeInvWatchStateMachine(
) : StateMachineEngine<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent>() {

    override val initialState: AutoforwardDoneInvokeSceSynthInvokeInvWatchState = AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch

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
    override fun isFinalState(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): Boolean = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed, is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>> =
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch))

        // W3C SCXML 3.13: watch's transition 0, as the microstep reads it.
        val transitionWatchAt0 = EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: watch's transition 1, as the microstep reads it.
        val transitionWatchAt1 = EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>(
            AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch,
            listOf(StateTarget(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed)),
            1,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): AutoforwardDoneInvokeSceSynthInvokeInvWatchState? = when (stateId) {
        "missed" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed
        "saw" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw
        "watch" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): String = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> "missed"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> "saw"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> "watch"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): Int = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> 2
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> 1
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent? = when (name) {
        "done.invoke.inv_short" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort
        "error.execution" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Execution
        "probe" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe
        "sawPlatform" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawPlatform
        "sawProbeOnly" -> AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawProbeOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent): String? = when (event) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> "done.invoke.inv_short"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Error.Execution -> "error.execution"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> "probe"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawPlatform -> "sawPlatform"
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.SawProbeOnly -> "sawProbeOnly"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent?
    ): EnabledTransition<AutoforwardDoneInvokeSceSynthInvokeInvWatchState, HistoryId>? = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> when {
            event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> transitionWatchAt0
            event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> transitionWatchAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, isDefaultEntry: Boolean) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:14 :: missed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:13 :: saw :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onExit(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:14 :: missed :: _state_body
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:13 :: saw :: _state_body
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun executeTransitionContent(source: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, transitionIndex: Int) {
        when (source) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:6 :: watch :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawPlatform", "")
            }
            1 -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:9 :: watch :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawProbeOnly", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
