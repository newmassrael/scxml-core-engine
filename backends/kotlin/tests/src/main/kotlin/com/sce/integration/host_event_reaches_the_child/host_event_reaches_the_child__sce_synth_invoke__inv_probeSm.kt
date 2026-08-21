// SCE-GENERATED — DO NOT EDIT
// source-hash: ceb5ba77c107690ed8824e3a95913c8f850f275ca17535023aec22eab166125d
// template-hash: f7291ab6d7896ee95dd448a8f7fc2759f6a0259c69bcc8f54f868651f4b8fe72
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine

package com.sce.integration.host_event_reaches_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface HostEventReachesTheChildSceSynthInvokeInvProbeState : State {
    data object Forwarded : HostEventReachesTheChildSceSynthInvokeInvProbeState
    data object Unforwarded : HostEventReachesTheChildSceSynthInvokeInvProbeState
    data object Watch : HostEventReachesTheChildSceSynthInvokeInvProbeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface HostEventReachesTheChildSceSynthInvokeInvProbeEvent : Event {
    sealed interface Error : HostEventReachesTheChildSceSynthInvokeInvProbeEvent {
        data object Execution : Error
    }
    data object HostPing : HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    data object Marker : HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    data object Ready : HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    data object SawHostPing : HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    data object SawMarkerOnly : HostEventReachesTheChildSceSynthInvokeInvProbeEvent
}
// --- State Machine (W3C SCXML) ---

class HostEventReachesTheChildSceSynthInvokeInvProbeStateMachine(
) : StateMachineEngine<HostEventReachesTheChildSceSynthInvokeInvProbeState, HostEventReachesTheChildSceSynthInvokeInvProbeEvent>() {

    override val initialState: HostEventReachesTheChildSceSynthInvokeInvProbeState = HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): HostEventReachesTheChildSceSynthInvokeInvProbeState? = when (stateId) {
        "forwarded" -> HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded
        "unforwarded" -> HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded
        "watch" -> HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: HostEventReachesTheChildSceSynthInvokeInvProbeState): String = when (state) {
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded -> "forwarded"
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded -> "unforwarded"
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> "watch"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: HostEventReachesTheChildSceSynthInvokeInvProbeState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: HostEventReachesTheChildSceSynthInvokeInvProbeState): Int = when (state) {
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded -> 1
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded -> 2
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): HostEventReachesTheChildSceSynthInvokeInvProbeEvent? = when (name) {
        "error.execution" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Error.Execution
        "hostPing" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.HostPing
        "marker" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Marker
        "ready" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Ready
        "sawHostPing" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.SawHostPing
        "sawMarkerOnly" -> HostEventReachesTheChildSceSynthInvokeInvProbeEvent.SawMarkerOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: HostEventReachesTheChildSceSynthInvokeInvProbeEvent): String? = when (event) {
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Error.Execution -> "error.execution"
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.HostPing -> "hostPing"
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Marker -> "marker"
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Ready -> "ready"
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.SawHostPing -> "sawHostPing"
        is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.SawMarkerOnly -> "sawMarkerOnly"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: HostEventReachesTheChildSceSynthInvokeInvProbeState,
        event: HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    ): TransitionResult<HostEventReachesTheChildSceSynthInvokeInvProbeState> = when (state) {
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> processWatch(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWatch(
        event: HostEventReachesTheChildSceSynthInvokeInvProbeEvent
    ): TransitionResult<HostEventReachesTheChildSceSynthInvokeInvProbeState> = when {
        event is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.HostPing -> TransitionResult.External(HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded, HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch)

        event is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Marker -> TransitionResult.External(HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded, HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onEntry(state: HostEventReachesTheChildSceSynthInvokeInvProbeState, pathChild: HostEventReachesTheChildSceSynthInvokeInvProbeState?) {
        when (state) {
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:16 :: forwarded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("forwarded")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:17 :: unforwarded :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("unforwarded")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:5 :: watch :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("ready", "")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun onExit(state: HostEventReachesTheChildSceSynthInvokeInvProbeState) {
        when (state) {
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Forwarded -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:16 :: forwarded :: _state_body
                activeStateIds.remove("forwarded")
            }
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Unforwarded -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:17 :: unforwarded :: _state_body
                activeStateIds.remove("unforwarded")
            }
            is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:5 :: watch :: _state_body
                activeStateIds.remove("watch")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:3 :: _machine
    override fun executeTransitionActions(
        source: HostEventReachesTheChildSceSynthInvokeInvProbeState,
        event: HostEventReachesTheChildSceSynthInvokeInvProbeEvent?
    ) {
        when (source) {
        is HostEventReachesTheChildSceSynthInvokeInvProbeState.Watch -> when {
            event is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.HostPing -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:9 :: watch :: _transition_0


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawHostPing", "")
            }
            event is HostEventReachesTheChildSceSynthInvokeInvProbeEvent.Marker -> {
                // SCE-MAP: host_event_reaches_the_child__sce_synth_invoke__inv_probe.scxml:12 :: watch :: _transition_1


            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback
            onSendToParent?.invoke("sawMarkerOnly", "")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
