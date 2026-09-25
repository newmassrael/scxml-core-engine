// SCE-GENERATED — DO NOT EDIT
// source-hash: ceb5ba77c107690ed8824e3a95913c8f850f275ca17535023aec22eab166125d

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: HostEventReachesTheChildState): HostEventReachesTheChildState? = when (state) {
        is HostEventReachesTheChildState.Armed -> HostEventReachesTheChildState.Phase
        is HostEventReachesTheChildState.Waiting -> HostEventReachesTheChildState.Phase
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: HostEventReachesTheChildState): Boolean = when (state) {
        is HostEventReachesTheChildState.Phase -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: HostEventReachesTheChildState): Boolean = when (state) {
        is HostEventReachesTheChildState.Fail, is HostEventReachesTheChildState.Pass -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: HostEventReachesTheChildState): List<HostEventReachesTheChildState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: HostEventReachesTheChildState): List<EntryTarget<HostEventReachesTheChildState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<HostEventReachesTheChildState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<HostEventReachesTheChildState, List<HostEventReachesTheChildState>> = mapOf(
            HostEventReachesTheChildState.Phase to listOf(HostEventReachesTheChildState.Waiting, HostEventReachesTheChildState.Armed),
        )

        val initialTargets: Map<HostEventReachesTheChildState, List<EntryTarget<HostEventReachesTheChildState, HistoryId>>> = mapOf(
            HostEventReachesTheChildState.Phase to listOf(StateTarget(HostEventReachesTheChildState.Waiting)),
        )

        val documentInitialTargetList: List<EntryTarget<HostEventReachesTheChildState, HistoryId>> =
            listOf(StateTarget(HostEventReachesTheChildState.Phase))

        // W3C SCXML 3.13: armed's transition 0, as the microstep reads it.
        val transitionArmedAt0 = EnabledTransition<HostEventReachesTheChildState, HistoryId>(
            HostEventReachesTheChildState.Armed,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<HostEventReachesTheChildState, HistoryId>(
            HostEventReachesTheChildState.Phase,
            listOf(StateTarget(HostEventReachesTheChildState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: phase's transition 1, as the microstep reads it.
        val transitionPhaseAt1 = EnabledTransition<HostEventReachesTheChildState, HistoryId>(
            HostEventReachesTheChildState.Phase,
            listOf(StateTarget(HostEventReachesTheChildState.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<HostEventReachesTheChildState, HistoryId>(
            HostEventReachesTheChildState.Waiting,
            listOf(StateTarget(HostEventReachesTheChildState.Armed)),
            0,
            hasActions = false,
            isInternal = false,
        )
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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: HostEventReachesTheChildState,
        event: HostEventReachesTheChildEvent?
    ): EnabledTransition<HostEventReachesTheChildState, HistoryId>? = when (state) {
        is HostEventReachesTheChildState.Armed -> when {
            event is HostEventReachesTheChildEvent.HostPing -> transitionArmedAt0
            else -> null
        }
        is HostEventReachesTheChildState.Phase -> when {
            event is HostEventReachesTheChildEvent.SawHostPing -> transitionPhaseAt0
            event is HostEventReachesTheChildEvent.SawMarkerOnly -> transitionPhaseAt1
            else -> null
        }
        is HostEventReachesTheChildState.Waiting -> when {
            event is HostEventReachesTheChildEvent.Ready -> transitionWaitingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun onEntry(state: HostEventReachesTheChildState, isDefaultEntry: Boolean) {
        when (state) {
            is HostEventReachesTheChildState.Armed -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:94 :: armed :: _state_body
            }
            is HostEventReachesTheChildState.Fail -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:103 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildState.Pass -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:102 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is HostEventReachesTheChildState.Phase -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:70 :: phase :: _state_body
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
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun onExit(state: HostEventReachesTheChildState) {
        when (state) {
            is HostEventReachesTheChildState.Armed -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:94 :: armed :: _state_body
            }
            is HostEventReachesTheChildState.Fail -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:103 :: fail :: _state_body
            }
            is HostEventReachesTheChildState.Pass -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:102 :: pass :: _state_body
            }
            is HostEventReachesTheChildState.Phase -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:70 :: phase :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("inv_probe")
            }
            is HostEventReachesTheChildState.Waiting -> {
                // SCE-MAP: host_event_reaches_the_child.scxml:91 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: host_event_reaches_the_child.scxml:67 :: _machine
    override fun executeTransitionContent(source: HostEventReachesTheChildState, transitionIndex: Int) {
        when (source) {
        is HostEventReachesTheChildState.Armed -> when (transitionIndex) {
            0 -> {
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
