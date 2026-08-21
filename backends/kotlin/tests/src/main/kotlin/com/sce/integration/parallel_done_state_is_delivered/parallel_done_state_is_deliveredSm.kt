// SCE-GENERATED — DO NOT EDIT
// source-hash: 4f209294ba851e9f433a2fd839fc088f718569422204e93318892b83dc408fac
// template-hash: 2531476627eb1f2b85917395efe91d1b55da71c6abf9c48b9fabdfd63b215bfa
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/parallel_done_state_is_delivered/parallel_done_state_is_delivered.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine

package com.sce.integration.parallel_done_state_is_delivered

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParallelDoneStateIsDeliveredState : State {
    data object A : ParallelDoneStateIsDeliveredState
    data object A1 : ParallelDoneStateIsDeliveredState
    data object A2 : ParallelDoneStateIsDeliveredState
    data object B : ParallelDoneStateIsDeliveredState
    data object B1 : ParallelDoneStateIsDeliveredState
    data object B2 : ParallelDoneStateIsDeliveredState
    data object Run : ParallelDoneStateIsDeliveredState
    data object Settled : ParallelDoneStateIsDeliveredState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParallelDoneStateIsDeliveredEvent : Event {
    sealed interface Done : ParallelDoneStateIsDeliveredEvent {
        sealed interface State : Done {
            data object A : State
            data object B : State
            data object Run : State
        }
    }
    data object Go : ParallelDoneStateIsDeliveredEvent
}
// --- State Machine (W3C SCXML) ---

class ParallelDoneStateIsDeliveredStateMachine(
) : StateMachineEngine<ParallelDoneStateIsDeliveredState, ParallelDoneStateIsDeliveredEvent>() {

    override val initialState: ParallelDoneStateIsDeliveredState = ParallelDoneStateIsDeliveredState.A1

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: ParallelDoneStateIsDeliveredState): ParallelDoneStateIsDeliveredState? = when (state) {
        is ParallelDoneStateIsDeliveredState.A -> ParallelDoneStateIsDeliveredState.Run
        is ParallelDoneStateIsDeliveredState.A1 -> ParallelDoneStateIsDeliveredState.A
        is ParallelDoneStateIsDeliveredState.A2 -> ParallelDoneStateIsDeliveredState.A
        is ParallelDoneStateIsDeliveredState.B -> ParallelDoneStateIsDeliveredState.Run
        is ParallelDoneStateIsDeliveredState.B1 -> ParallelDoneStateIsDeliveredState.B
        is ParallelDoneStateIsDeliveredState.B2 -> ParallelDoneStateIsDeliveredState.B
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: ParallelDoneStateIsDeliveredState): ParallelDoneStateIsDeliveredState = when (state) {
        is ParallelDoneStateIsDeliveredState.A -> ParallelDoneStateIsDeliveredState.A1
        is ParallelDoneStateIsDeliveredState.B -> ParallelDoneStateIsDeliveredState.B1
        is ParallelDoneStateIsDeliveredState.Run -> ParallelDoneStateIsDeliveredState.A1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParallelDoneStateIsDeliveredState? = when (stateId) {
        "a" -> ParallelDoneStateIsDeliveredState.A
        "a1" -> ParallelDoneStateIsDeliveredState.A1
        "a2" -> ParallelDoneStateIsDeliveredState.A2
        "b" -> ParallelDoneStateIsDeliveredState.B
        "b1" -> ParallelDoneStateIsDeliveredState.B1
        "b2" -> ParallelDoneStateIsDeliveredState.B2
        "run" -> ParallelDoneStateIsDeliveredState.Run
        "settled" -> ParallelDoneStateIsDeliveredState.Settled
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParallelDoneStateIsDeliveredState): String = when (state) {
        is ParallelDoneStateIsDeliveredState.A -> "a"
        is ParallelDoneStateIsDeliveredState.A1 -> "a1"
        is ParallelDoneStateIsDeliveredState.A2 -> "a2"
        is ParallelDoneStateIsDeliveredState.B -> "b"
        is ParallelDoneStateIsDeliveredState.B1 -> "b1"
        is ParallelDoneStateIsDeliveredState.B2 -> "b2"
        is ParallelDoneStateIsDeliveredState.Run -> "run"
        is ParallelDoneStateIsDeliveredState.Settled -> "settled"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ParallelDoneStateIsDeliveredState): Boolean = when (state) {
        is ParallelDoneStateIsDeliveredState.A -> false
        is ParallelDoneStateIsDeliveredState.B -> false
        is ParallelDoneStateIsDeliveredState.Run -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelDoneStateIsDeliveredState): Boolean = when (state) {
        is ParallelDoneStateIsDeliveredState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: ParallelDoneStateIsDeliveredState): List<ParallelDoneStateIsDeliveredState> = when (state) {
        is ParallelDoneStateIsDeliveredState.Run -> listOf(ParallelDoneStateIsDeliveredState.A, ParallelDoneStateIsDeliveredState.B)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: ParallelDoneStateIsDeliveredState): Int = when (state) {
        is ParallelDoneStateIsDeliveredState.A -> 1
        is ParallelDoneStateIsDeliveredState.A1 -> 2
        is ParallelDoneStateIsDeliveredState.A2 -> 3
        is ParallelDoneStateIsDeliveredState.B -> 4
        is ParallelDoneStateIsDeliveredState.B1 -> 5
        is ParallelDoneStateIsDeliveredState.B2 -> 6
        is ParallelDoneStateIsDeliveredState.Run -> 0
        is ParallelDoneStateIsDeliveredState.Settled -> 7
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: ParallelDoneStateIsDeliveredState,
        event: ParallelDoneStateIsDeliveredEvent
    ): TransitionResult<ParallelDoneStateIsDeliveredState> = when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (a has no own event transitions)
        is ParallelDoneStateIsDeliveredState.A -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is ParallelDoneStateIsDeliveredState.A1 -> {
            val result = processA1(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (a2 has no own event transitions)
        is ParallelDoneStateIsDeliveredState.A2 -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (b has no own event transitions)
        is ParallelDoneStateIsDeliveredState.B -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is ParallelDoneStateIsDeliveredState.B1 -> {
            val result = processB1(event)
            // W3C SCXML 3.13: Ancestor transition routing
            if (result !is TransitionResult.Ignored) result
            else {
                val anc1 = processRun(event)
                if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
            }
        }
        // W3C SCXML 3.13: Ancestor-only routing (b2 has no own event transitions)
        is ParallelDoneStateIsDeliveredState.B2 -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processA1(
        event: ParallelDoneStateIsDeliveredEvent
    ): TransitionResult<ParallelDoneStateIsDeliveredState> = when {
        event is ParallelDoneStateIsDeliveredEvent.Go -> TransitionResult.External(ParallelDoneStateIsDeliveredState.A2, ParallelDoneStateIsDeliveredState.A1)

        else -> TransitionResult.Ignored
    }

    private fun processB1(
        event: ParallelDoneStateIsDeliveredEvent
    ): TransitionResult<ParallelDoneStateIsDeliveredState> = when {
        event is ParallelDoneStateIsDeliveredEvent.Go -> TransitionResult.External(ParallelDoneStateIsDeliveredState.B2, ParallelDoneStateIsDeliveredState.B1)

        else -> TransitionResult.Ignored
    }

    private fun processRun(
        event: ParallelDoneStateIsDeliveredEvent
    ): TransitionResult<ParallelDoneStateIsDeliveredState> = when {
        event is ParallelDoneStateIsDeliveredEvent.Done.State.Run -> TransitionResult.External(ParallelDoneStateIsDeliveredState.Settled, ParallelDoneStateIsDeliveredState.Run)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine
    override fun onEntry(state: ParallelDoneStateIsDeliveredState, pathChild: ParallelDoneStateIsDeliveredState?) {
        when (state) {
            is ParallelDoneStateIsDeliveredState.A -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:37 :: a :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelDoneStateIsDeliveredState.A1)
                }
            }
            is ParallelDoneStateIsDeliveredState.A1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:38 :: a1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a1")) return
            }
            is ParallelDoneStateIsDeliveredState.A2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:41 :: a2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a2")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.A, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("a2")) && (activeStateIds.contains("b2"))) {
                    raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.Run)
                }
            }
            is ParallelDoneStateIsDeliveredState.B -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:44 :: b :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelDoneStateIsDeliveredState.B1)
                }
            }
            is ParallelDoneStateIsDeliveredState.B1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:45 :: b1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b1")) return
            }
            is ParallelDoneStateIsDeliveredState.B2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:48 :: b2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b2")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.B, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("a2")) && (activeStateIds.contains("b2"))) {
                    raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.Run)
                }
            }
            is ParallelDoneStateIsDeliveredState.Run -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:35 :: run :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != ParallelDoneStateIsDeliveredState.A) {
                    onEntry(ParallelDoneStateIsDeliveredState.A)
                }
                if (pathChild != ParallelDoneStateIsDeliveredState.B) {
                    onEntry(ParallelDoneStateIsDeliveredState.B)
                }
            }
            is ParallelDoneStateIsDeliveredState.Settled -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:64 :: settled :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("settled")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine
    override fun onExit(state: ParallelDoneStateIsDeliveredState) {
        when (state) {
            is ParallelDoneStateIsDeliveredState.A -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:37 :: a :: _state_body
                activeStateIds.remove("a")
            }
            is ParallelDoneStateIsDeliveredState.A1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:38 :: a1 :: _state_body
                activeStateIds.remove("a1")
            }
            is ParallelDoneStateIsDeliveredState.A2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:41 :: a2 :: _state_body
                activeStateIds.remove("a2")
            }
            is ParallelDoneStateIsDeliveredState.B -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:44 :: b :: _state_body
                activeStateIds.remove("b")
            }
            is ParallelDoneStateIsDeliveredState.B1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:45 :: b1 :: _state_body
                activeStateIds.remove("b1")
            }
            is ParallelDoneStateIsDeliveredState.B2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:48 :: b2 :: _state_body
                activeStateIds.remove("b2")
            }
            is ParallelDoneStateIsDeliveredState.Run -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:35 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<ParallelDoneStateIsDeliveredState, Int>>()
                if (activeStateIds.contains("a")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.A to 1)
                }
                if (activeStateIds.contains("a1")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.A1 to 2)
                }
                if (activeStateIds.contains("a2")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.A2 to 3)
                }
                if (activeStateIds.contains("b")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.B to 4)
                }
                if (activeStateIds.contains("b1")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.B1 to 5)
                }
                if (activeStateIds.contains("b2")) {
                    toExit.add(ParallelDoneStateIsDeliveredState.B2 to 6)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is ParallelDoneStateIsDeliveredState.Settled -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:64 :: settled :: _state_body
                activeStateIds.remove("settled")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine
    override fun executeTransitionActions(
        source: ParallelDoneStateIsDeliveredState,
        event: ParallelDoneStateIsDeliveredEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
