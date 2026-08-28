// SCE-GENERATED — DO NOT EDIT
// source-hash: c3811de69809fcaca1ab0508e94a631fbb8f5a9a57e1924edd0d75fdf7afaa52
// template-hash: 26e5b2b0aec9ad85a8375690dfa8db213377e6dd6bcde53d334d893cb6b448b2
// generated-at: 0

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

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: ParallelRegionRootExternalDomainState): ParallelRegionRootExternalDomainState = when (state) {
        is ParallelRegionRootExternalDomainState.Drive -> ParallelRegionRootExternalDomainState.Working
        is ParallelRegionRootExternalDomainState.Run -> ParallelRegionRootExternalDomainState.Working
        is ParallelRegionRootExternalDomainState.Watch -> ParallelRegionRootExternalDomainState.Alive
        else -> state
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ParallelRegionRootExternalDomainState): Boolean = when (state) {
        is ParallelRegionRootExternalDomainState.Drive -> false
        is ParallelRegionRootExternalDomainState.Run -> false
        is ParallelRegionRootExternalDomainState.Watch -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelRegionRootExternalDomainState): Boolean = when (state) {
        is ParallelRegionRootExternalDomainState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: ParallelRegionRootExternalDomainState): List<ParallelRegionRootExternalDomainState> = when (state) {
        is ParallelRegionRootExternalDomainState.Run -> listOf(ParallelRegionRootExternalDomainState.Drive, ParallelRegionRootExternalDomainState.Watch)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
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





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: ParallelRegionRootExternalDomainState,
        event: ParallelRegionRootExternalDomainEvent
    ): TransitionResult<ParallelRegionRootExternalDomainState> = when (state) {
        is ParallelRegionRootExternalDomainState.Alive -> processAlive(event)
        is ParallelRegionRootExternalDomainState.Drive -> processDrive(event)
        // W3C SCXML 3.13: Ancestor-only routing (paused has no own event transitions)
        is ParallelRegionRootExternalDomainState.Paused -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (restarting has no own event transitions)
        is ParallelRegionRootExternalDomainState.Restarting -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (working has no own event transitions)
        is ParallelRegionRootExternalDomainState.Working -> {
            val anc1 = processDrive(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processAlive(
        event: ParallelRegionRootExternalDomainEvent
    ): TransitionResult<ParallelRegionRootExternalDomainState> = when {
        event is ParallelRegionRootExternalDomainEvent.Restart -> TransitionResult.External(ParallelRegionRootExternalDomainState.Rebuilding, ParallelRegionRootExternalDomainState.Alive)

        event is ParallelRegionRootExternalDomainEvent.Hold -> TransitionResult.External(ParallelRegionRootExternalDomainState.Rebuilding, ParallelRegionRootExternalDomainState.Alive)

        else -> TransitionResult.Ignored
    }

    private fun processDrive(
        event: ParallelRegionRootExternalDomainEvent
    ): TransitionResult<ParallelRegionRootExternalDomainState> = when {
        event is ParallelRegionRootExternalDomainEvent.Restart -> TransitionResult.External(ParallelRegionRootExternalDomainState.Restarting, ParallelRegionRootExternalDomainState.Drive)

        event is ParallelRegionRootExternalDomainEvent.Hold -> TransitionResult.InternalToTarget(ParallelRegionRootExternalDomainState.Paused, ParallelRegionRootExternalDomainState.Drive)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun onEntry(state: ParallelRegionRootExternalDomainState, pathChild: ParallelRegionRootExternalDomainState?) {
        when (state) {
            is ParallelRegionRootExternalDomainState.Alive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:65 :: alive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("alive")) return
            }
            is ParallelRegionRootExternalDomainState.Drive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:45 :: drive :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("drive")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelRegionRootExternalDomainState.Working)
                }
            }
            is ParallelRegionRootExternalDomainState.Paused -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:55 :: paused :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("paused")) return
            }
            is ParallelRegionRootExternalDomainState.Rebuilding -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:69 :: rebuilding :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("rebuilding")) return
            }
            is ParallelRegionRootExternalDomainState.Restarting -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:54 :: restarting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("restarting")) return
            }
            is ParallelRegionRootExternalDomainState.Run -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:37 :: run :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != ParallelRegionRootExternalDomainState.Drive) {
                    onEntry(ParallelRegionRootExternalDomainState.Drive)
                }
                if (pathChild != ParallelRegionRootExternalDomainState.Watch) {
                    onEntry(ParallelRegionRootExternalDomainState.Watch)
                }
            }
            is ParallelRegionRootExternalDomainState.Watch -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:64 :: watch :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("watch")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelRegionRootExternalDomainState.Alive)
                }
            }
            is ParallelRegionRootExternalDomainState.Working -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:53 :: working :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("working")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun onExit(state: ParallelRegionRootExternalDomainState) {
        when (state) {
            is ParallelRegionRootExternalDomainState.Alive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:65 :: alive :: _state_body
                activeStateIds.remove("alive")
            }
            is ParallelRegionRootExternalDomainState.Drive -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:45 :: drive :: _state_body
                activeStateIds.remove("drive")
            }
            is ParallelRegionRootExternalDomainState.Paused -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:55 :: paused :: _state_body
                activeStateIds.remove("paused")
            }
            is ParallelRegionRootExternalDomainState.Rebuilding -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:69 :: rebuilding :: _state_body
                activeStateIds.remove("rebuilding")
            }
            is ParallelRegionRootExternalDomainState.Restarting -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:54 :: restarting :: _state_body
                activeStateIds.remove("restarting")
            }
            is ParallelRegionRootExternalDomainState.Run -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:37 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<ParallelRegionRootExternalDomainState, Int>>()
                if (activeStateIds.contains("drive")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Drive to 1)
                }
                if (activeStateIds.contains("paused")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Paused to 4)
                }
                if (activeStateIds.contains("restarting")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Restarting to 3)
                }
                if (activeStateIds.contains("working")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Working to 2)
                }
                if (activeStateIds.contains("watch")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Watch to 5)
                }
                if (activeStateIds.contains("alive")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Alive to 6)
                }
                if (activeStateIds.contains("rebuilding")) {
                    toExit.add(ParallelRegionRootExternalDomainState.Rebuilding to 7)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is ParallelRegionRootExternalDomainState.Watch -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:64 :: watch :: _state_body
                activeStateIds.remove("watch")
            }
            is ParallelRegionRootExternalDomainState.Working -> {
                // SCE-MAP: parallel_region_root_external_domain.scxml:53 :: working :: _state_body
                activeStateIds.remove("working")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: parallel_region_root_external_domain.scxml:34 :: _machine
    override fun executeTransitionActions(
        source: ParallelRegionRootExternalDomainState,
        event: ParallelRegionRootExternalDomainEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
