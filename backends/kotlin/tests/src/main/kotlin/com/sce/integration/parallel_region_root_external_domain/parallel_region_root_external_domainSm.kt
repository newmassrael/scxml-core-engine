// SCE-GENERATED — DO NOT EDIT
// source-hash: 46fc9a1e2eb5ee9cf2888d4c74504c715b0935d2521f04679c48d3203c504686

// GENERATED CODE — DO NOT EDIT
// Source: tests/integration/parallel_region_root_external_domain.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine

package com.sce.integration.parallel_region_root_external_domain

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParallelRegionRootExternalDomainState : State {
    data object Alive : ParallelRegionRootExternalDomainState
    data object Drive : ParallelRegionRootExternalDomainState
    data object Paused : ParallelRegionRootExternalDomainState
    data object Rebuilding : ParallelRegionRootExternalDomainState
    data object Restarting : ParallelRegionRootExternalDomainState
    data object Run : ParallelRegionRootExternalDomainState
    data object Watch : ParallelRegionRootExternalDomainState
    data object Working : ParallelRegionRootExternalDomainState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParallelRegionRootExternalDomainEvent : Event {
    data object Hold : ParallelRegionRootExternalDomainEvent
    data object Restart : ParallelRegionRootExternalDomainEvent
}
// --- State Machine (W3C SCXML) ---

class ParallelRegionRootExternalDomainStateMachine(
) : StateMachineEngine<ParallelRegionRootExternalDomainState, ParallelRegionRootExternalDomainEvent>() {

    override val initialState: ParallelRegionRootExternalDomainState = ParallelRegionRootExternalDomainState.Working

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: ParallelRegionRootExternalDomainState): ParallelRegionRootExternalDomainState? = when (state) {
        is ParallelRegionRootExternalDomainState.Alive -> ParallelRegionRootExternalDomainState.Watch
        is ParallelRegionRootExternalDomainState.Drive -> ParallelRegionRootExternalDomainState.Run
        is ParallelRegionRootExternalDomainState.Paused -> ParallelRegionRootExternalDomainState.Drive
        is ParallelRegionRootExternalDomainState.Rebuilding -> ParallelRegionRootExternalDomainState.Watch
        is ParallelRegionRootExternalDomainState.Restarting -> ParallelRegionRootExternalDomainState.Drive
        is ParallelRegionRootExternalDomainState.Watch -> ParallelRegionRootExternalDomainState.Run
        is ParallelRegionRootExternalDomainState.Working -> ParallelRegionRootExternalDomainState.Drive
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ParallelRegionRootExternalDomainState): Boolean = when (state) {
        is ParallelRegionRootExternalDomainState.Drive, is ParallelRegionRootExternalDomainState.Watch -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelRegionRootExternalDomainState): Boolean = when (state) {
        is ParallelRegionRootExternalDomainState.Run -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ParallelRegionRootExternalDomainState): List<ParallelRegionRootExternalDomainState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ParallelRegionRootExternalDomainState): List<EntryTarget<ParallelRegionRootExternalDomainState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ParallelRegionRootExternalDomainState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ParallelRegionRootExternalDomainState, List<ParallelRegionRootExternalDomainState>> = mapOf(
            ParallelRegionRootExternalDomainState.Drive to listOf(ParallelRegionRootExternalDomainState.Working, ParallelRegionRootExternalDomainState.Restarting, ParallelRegionRootExternalDomainState.Paused),
            ParallelRegionRootExternalDomainState.Run to listOf(ParallelRegionRootExternalDomainState.Drive, ParallelRegionRootExternalDomainState.Watch),
            ParallelRegionRootExternalDomainState.Watch to listOf(ParallelRegionRootExternalDomainState.Alive, ParallelRegionRootExternalDomainState.Rebuilding),
        )

        val initialTargets: Map<ParallelRegionRootExternalDomainState, List<EntryTarget<ParallelRegionRootExternalDomainState, HistoryId>>> = mapOf(
            ParallelRegionRootExternalDomainState.Drive to listOf(StateTarget(ParallelRegionRootExternalDomainState.Working)),
            ParallelRegionRootExternalDomainState.Watch to listOf(StateTarget(ParallelRegionRootExternalDomainState.Alive)),
        )

        val documentInitialTargetList: List<EntryTarget<ParallelRegionRootExternalDomainState, HistoryId>> =
            listOf(StateTarget(ParallelRegionRootExternalDomainState.Run))

        // W3C SCXML 3.13: alive's transition 0, as the microstep reads it.
        val transitionAliveAt0 = EnabledTransition<ParallelRegionRootExternalDomainState, HistoryId>(
            ParallelRegionRootExternalDomainState.Alive,
            listOf(StateTarget(ParallelRegionRootExternalDomainState.Rebuilding)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: alive's transition 1, as the microstep reads it.
        val transitionAliveAt1 = EnabledTransition<ParallelRegionRootExternalDomainState, HistoryId>(
            ParallelRegionRootExternalDomainState.Alive,
            listOf(StateTarget(ParallelRegionRootExternalDomainState.Rebuilding)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: drive's transition 0, as the microstep reads it.
        val transitionDriveAt0 = EnabledTransition<ParallelRegionRootExternalDomainState, HistoryId>(
            ParallelRegionRootExternalDomainState.Drive,
            listOf(StateTarget(ParallelRegionRootExternalDomainState.Restarting)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: drive's transition 1, as the microstep reads it.
        val transitionDriveAt1 = EnabledTransition<ParallelRegionRootExternalDomainState, HistoryId>(
            ParallelRegionRootExternalDomainState.Drive,
            listOf(StateTarget(ParallelRegionRootExternalDomainState.Paused)),
            1,
            hasActions = false,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParallelRegionRootExternalDomainState? = when (stateId) {
        "alive" -> ParallelRegionRootExternalDomainState.Alive
        "drive" -> ParallelRegionRootExternalDomainState.Drive
        "paused" -> ParallelRegionRootExternalDomainState.Paused
        "rebuilding" -> ParallelRegionRootExternalDomainState.Rebuilding
        "restarting" -> ParallelRegionRootExternalDomainState.Restarting
        "run" -> ParallelRegionRootExternalDomainState.Run
        "watch" -> ParallelRegionRootExternalDomainState.Watch
        "working" -> ParallelRegionRootExternalDomainState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParallelRegionRootExternalDomainState): String = when (state) {
        is ParallelRegionRootExternalDomainState.Alive -> "alive"
        is ParallelRegionRootExternalDomainState.Drive -> "drive"
        is ParallelRegionRootExternalDomainState.Paused -> "paused"
        is ParallelRegionRootExternalDomainState.Rebuilding -> "rebuilding"
        is ParallelRegionRootExternalDomainState.Restarting -> "restarting"
        is ParallelRegionRootExternalDomainState.Run -> "run"
        is ParallelRegionRootExternalDomainState.Watch -> "watch"
        is ParallelRegionRootExternalDomainState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: ParallelRegionRootExternalDomainState): Int = when (state) {
        is ParallelRegionRootExternalDomainState.Alive -> 6
        is ParallelRegionRootExternalDomainState.Drive -> 1
        is ParallelRegionRootExternalDomainState.Paused -> 4
        is ParallelRegionRootExternalDomainState.Rebuilding -> 7
        is ParallelRegionRootExternalDomainState.Restarting -> 3
        is ParallelRegionRootExternalDomainState.Run -> 0
        is ParallelRegionRootExternalDomainState.Watch -> 5
        is ParallelRegionRootExternalDomainState.Working -> 2
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ParallelRegionRootExternalDomainState,
        event: ParallelRegionRootExternalDomainEvent?
    ): EnabledTransition<ParallelRegionRootExternalDomainState, HistoryId>? = when (state) {
        is ParallelRegionRootExternalDomainState.Alive -> when {
            event is ParallelRegionRootExternalDomainEvent.Restart -> transitionAliveAt0
            event is ParallelRegionRootExternalDomainEvent.Hold -> transitionAliveAt1
            else -> null
        }
        is ParallelRegionRootExternalDomainState.Drive -> when {
            event is ParallelRegionRootExternalDomainEvent.Restart -> transitionDriveAt0
            event is ParallelRegionRootExternalDomainEvent.Hold -> transitionDriveAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun onEntry(state: ParallelRegionRootExternalDomainState, isDefaultEntry: Boolean) {
        when (state) {
            is ParallelRegionRootExternalDomainState.Alive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:65 :: alive :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Drive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:45 :: drive :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Paused -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:55 :: paused :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Rebuilding -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:69 :: rebuilding :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Restarting -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:54 :: restarting :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Run -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:37 :: run :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Watch -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:64 :: watch :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Working -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:53 :: working :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun onExit(state: ParallelRegionRootExternalDomainState) {
        when (state) {
            is ParallelRegionRootExternalDomainState.Alive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:65 :: alive :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Drive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:45 :: drive :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Paused -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:55 :: paused :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Rebuilding -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:69 :: rebuilding :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Restarting -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:54 :: restarting :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Run -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:37 :: run :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Watch -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:64 :: watch :: _state_body
            }
            is ParallelRegionRootExternalDomainState.Working -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:53 :: working :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun executeTransitionContent(source: ParallelRegionRootExternalDomainState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
