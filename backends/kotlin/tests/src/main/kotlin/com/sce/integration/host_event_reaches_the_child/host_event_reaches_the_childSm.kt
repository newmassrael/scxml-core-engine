// SCE-GENERATED — DO NOT EDIT
// source-hash: ceb5ba77c107690ed8824e3a95913c8f850f275ca17535023aec22eab166125d
// template-hash: 6d29ccd65cc69c7036210e21d4c9d2a46b7717262dc7e045f86a45620f80383f
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/host_event_reaches_the_child/host_event_reaches_the_child.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine

package com.sce.integration.host_event_reaches_the_child

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface HostEventReachesTheChildState : State {
    data object Armed : HostEventReachesTheChildState
    data object Fail : HostEventReachesTheChildState
    data object Pass : HostEventReachesTheChildState
    data object Phase : HostEventReachesTheChildState
    data object Waiting : HostEventReachesTheChildState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface HostEventReachesTheChildEvent : Event {
    sealed interface Cancel : HostEventReachesTheChildEvent {
        data object Invoke : Cancel
    }
    sealed interface Done : HostEventReachesTheChildEvent {
        data object Invoke : Done
    }
    sealed interface Error : HostEventReachesTheChildEvent {
        data object Execution : Error
    }
    data object HostPing : HostEventReachesTheChildEvent
    data object Marker : HostEventReachesTheChildEvent
    data object Ready : HostEventReachesTheChildEvent
    data object SawHostPing : HostEventReachesTheChildEvent
    data object SawMarkerOnly : HostEventReachesTheChildEvent
}
// --- State Machine (W3C SCXML) ---

class HostEventReachesTheChildStateMachine(
) : StateMachineEngine<HostEventReachesTheChildState, HostEventReachesTheChildEvent>() {

    override val initialState: HostEventReachesTheChildState = HostEventReachesTheChildState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: HostEventReachesTheChildState): HostEventReachesTheChildState? = when (state) {
        is HostEventReachesTheChildState.Armed -> HostEventReachesTheChildState.Phase
        is HostEventReachesTheChildState.Waiting -> HostEventReachesTheChildState.Phase
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: HostEventReachesTheChildState): HostEventReachesTheChildState = when (state) {
        is HostEventReachesTheChildState.Phase -> HostEventReachesTheChildState.Waiting
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): HostEventReachesTheChildState? = when (stateId) {
        "armed" -> HostEventReachesTheChildState.Armed
        "fail" -> HostEventReachesTheChildState.Fail
        "pass" -> HostEventReachesTheChildState.Pass
        "phase" -> HostEventReachesTheChildState.Phase
        "waiting" -> HostEventReachesTheChildState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: HostEventReachesTheChildState): String = when (state) {
        is HostEventReachesTheChildState.Armed -> "armed"
        is HostEventReachesTheChildState.Fail -> "fail"
        is HostEventReachesTheChildState.Pass -> "pass"
        is HostEventReachesTheChildState.Phase -> "phase"
        is HostEventReachesTheChildState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: HostEventReachesTheChildState): Boolean = when (state) {
        is HostEventReachesTheChildState.Phase -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: HostEventReachesTheChildState): Int = when (state) {
        is HostEventReachesTheChildState.Armed -> 2
        is HostEventReachesTheChildState.Fail -> 4
        is HostEventReachesTheChildState.Pass -> 3
        is HostEventReachesTheChildState.Phase -> 0
        is HostEventReachesTheChildState.Waiting -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): HostEventReachesTheChildEvent? = when (name) {
        "cancel.invoke" -> HostEventReachesTheChildEvent.Cancel.Invoke
        "done.invoke" -> HostEventReachesTheChildEvent.Done.Invoke
        "error.execution" -> HostEventReachesTheChildEvent.Error.Execution
        "hostPing" -> HostEventReachesTheChildEvent.HostPing
        "marker" -> HostEventReachesTheChildEvent.Marker
        "ready" -> HostEventReachesTheChildEvent.Ready
        "sawHostPing" -> HostEventReachesTheChildEvent.SawHostPing
        "sawMarkerOnly" -> HostEventReachesTheChildEvent.SawMarkerOnly
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: HostEventReachesTheChildEvent): String? = when (event) {
        is HostEventReachesTheChildEvent.Cancel.Invoke -> "cancel.invoke"
        is HostEventReachesTheChildEvent.Done.Invoke -> "done.invoke"
        is HostEventReachesTheChildEvent.Error.Execution -> "error.execution"
        is HostEventReachesTheChildEvent.HostPing -> "hostPing"
        is HostEventReachesTheChildEvent.Marker -> "marker"
        is HostEventReachesTheChildEvent.Ready -> "ready"
        is HostEventReachesTheChildEvent.SawHostPing -> "sawHostPing"
        is HostEventReachesTheChildEvent.SawMarkerOnly -> "sawMarkerOnly"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: HostEventReachesTheChildState,
        event: HostEventReachesTheChildEvent
    ): TransitionResult<HostEventReachesTheChildState> = when (state) {
        is HostEventReachesTheChildState.Armed -> {
            val result = processArmed(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processPhase(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        is HostEventReachesTheChildState.Phase -> processPhase(event)
        is HostEventReachesTheChildState.Waiting -> {
            val result = processWaiting(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processPhase(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processArmed(
        event: HostEventReachesTheChildEvent
    ): TransitionResult<HostEventReachesTheChildState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is HostEventReachesTheChildEvent.HostPing -> TransitionResult.Internal
        else -> TransitionResult.Ignored
    }

    private fun processPhase(
        event: HostEventReachesTheChildEvent
    ): TransitionResult<HostEventReachesTheChildState> = when {
        event is HostEventReachesTheChildEvent.SawHostPing -> TransitionResult.External(HostEventReachesTheChildState.Pass, HostEventReachesTheChildState.Phase)

        event is HostEventReachesTheChildEvent.SawMarkerOnly -> TransitionResult.External(HostEventReachesTheChildState.Fail, HostEventReachesTheChildState.Phase)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: HostEventReachesTheChildEvent
    ): TransitionResult<HostEventReachesTheChildState> = when {
        event is HostEventReachesTheChildEvent.Ready -> TransitionResult.External(HostEventReachesTheChildState.Armed, HostEventReachesTheChildState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun onEntry(state: HostEventReachesTheChildState, pathChild: HostEventReachesTheChildState?) {
        when (state) {
            is HostEventReachesTheChildState.Armed -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:94 :: armed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("armed")) return
            }
            is HostEventReachesTheChildState.Fail -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:103 :: fail :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("fail")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildState.Pass -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:102 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildState.Phase -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:70 :: phase :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                run {
                    // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                    val generatedInvokeId = "phase.${System.identityHashCode(this)}.inv_probe"
                    deferInvoke(state, generatedInvokeId) {
                        val childSM = HostEventReachesTheChildSceSynthInvokeInvProbeStateMachine()
                        // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                        startInvoke("inv_probe", childSM, true, HostEventReachesTheChildEvent.Done.Invoke, "", generatedInvokeId)
                    }
                }
            }
            is HostEventReachesTheChildState.Waiting -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:91 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun onExit(state: HostEventReachesTheChildState) {
        when (state) {
            is HostEventReachesTheChildState.Armed -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:94 :: armed :: _state_body
                activeStateIds.remove("armed")
            }
            is HostEventReachesTheChildState.Fail -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:103 :: fail :: _state_body
                activeStateIds.remove("fail")
            }
            is HostEventReachesTheChildState.Pass -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:102 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is HostEventReachesTheChildState.Phase -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:70 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_probe")
                activeStateIds.remove("phase")
            }
            is HostEventReachesTheChildState.Waiting -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:91 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun executeTransitionActions(
        source: HostEventReachesTheChildState,
        event: HostEventReachesTheChildEvent?
    ) {
        when (source) {
        is HostEventReachesTheChildState.Armed -> when {
            event is HostEventReachesTheChildEvent.HostPing -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:95 :: armed :: _transition_0


            // W3C SCXML 6.4 (test192): Send event to invoked child
            sendToChild("inv_probe", "marker")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
