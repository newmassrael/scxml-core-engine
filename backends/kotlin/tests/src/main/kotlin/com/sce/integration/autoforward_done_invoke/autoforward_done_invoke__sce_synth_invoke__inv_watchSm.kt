// SCE-GENERATED — DO NOT EDIT
// source-hash: 54fa213afae337fd55d5bdcc6342253ac581ed7cc7a7519be41e894ee31b3f4b
// template-hash: b5bef7d045160440c6e2790d4f2e0be757d7c1cc42dee75b2002b23fd477161e
// generated-at: 0

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    ): TransitionResult<AutoforwardDoneInvokeSceSynthInvokeInvWatchState> = when (state) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> processWatch(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWatch(
        event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent
    ): TransitionResult<AutoforwardDoneInvokeSceSynthInvokeInvWatchState> = when {
        event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> TransitionResult.External(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw, AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch)

        event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> TransitionResult.External(AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed, AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onEntry(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState, pathChild: AutoforwardDoneInvokeSceSynthInvokeInvWatchState?) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:14 :: missed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("missed")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:13 :: saw :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("saw")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun onExit(state: AutoforwardDoneInvokeSceSynthInvokeInvWatchState) {
        when (state) {
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Missed -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:14 :: missed :: _state_body
                activeStateIds.remove("missed")
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Saw -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:13 :: saw :: _state_body
                activeStateIds.remove("saw")
            }
            is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:5 :: watch :: _state_body
                activeStateIds.remove("watch")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: AutoforwardDoneInvokeSceSynthInvokeInvWatchState,
        event: AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent?
    ) {
        when (source) {
        is AutoforwardDoneInvokeSceSynthInvokeInvWatchState.Watch -> when {
            event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Done.Invoke.InvShort -> {
                // SCE-MAP: autoforward_done_invoke__sce_synth_invoke__inv_watch.scxml:6 :: watch :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawPlatform", "")
            }
            event is AutoforwardDoneInvokeSceSynthInvokeInvWatchEvent.Probe -> {
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
