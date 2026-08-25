// SCE-GENERATED — DO NOT EDIT
// source-hash: 280975c88158c1a2612c8726a71e4ae581a1e42f8ef6d030924e99800aff8d10
// template-hash: c11ce025286de32d15ba70522b50fb24cf722356167a9d021470bd1434f2dd9a
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/parallel_completion_raises_done_state/parallel_completion_raises_done_state.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine

package com.sce.integration.parallel_completion_raises_done_state

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface ParallelCompletionRaisesDoneStateState : State {
    data object A : ParallelCompletionRaisesDoneStateState
    data object A1 : ParallelCompletionRaisesDoneStateState
    data object A2 : ParallelCompletionRaisesDoneStateState
    data object B : ParallelCompletionRaisesDoneStateState
    data object B1 : ParallelCompletionRaisesDoneStateState
    data object B2 : ParallelCompletionRaisesDoneStateState
    data object Run : ParallelCompletionRaisesDoneStateState
    data object Stopped : ParallelCompletionRaisesDoneStateState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface ParallelCompletionRaisesDoneStateEvent : Event {
    data object Bail : ParallelCompletionRaisesDoneStateEvent
    sealed interface Done : ParallelCompletionRaisesDoneStateEvent {
        sealed interface State : Done {
            data object A : State
            data object B : State
            data object Run : State
        }
    }
    data object Go : ParallelCompletionRaisesDoneStateEvent
}
// --- State Machine (W3C SCXML) ---

class ParallelCompletionRaisesDoneStateStateMachine(
) : StateMachineEngine<ParallelCompletionRaisesDoneStateState, ParallelCompletionRaisesDoneStateEvent>() {

    override val initialState: ParallelCompletionRaisesDoneStateState = ParallelCompletionRaisesDoneStateState.A1

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: ParallelCompletionRaisesDoneStateState): ParallelCompletionRaisesDoneStateState? = when (state) {
        is ParallelCompletionRaisesDoneStateState.A -> ParallelCompletionRaisesDoneStateState.Run
        is ParallelCompletionRaisesDoneStateState.A1 -> ParallelCompletionRaisesDoneStateState.A
        is ParallelCompletionRaisesDoneStateState.A2 -> ParallelCompletionRaisesDoneStateState.A
        is ParallelCompletionRaisesDoneStateState.B -> ParallelCompletionRaisesDoneStateState.Run
        is ParallelCompletionRaisesDoneStateState.B1 -> ParallelCompletionRaisesDoneStateState.B
        is ParallelCompletionRaisesDoneStateState.B2 -> ParallelCompletionRaisesDoneStateState.B
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: ParallelCompletionRaisesDoneStateState): ParallelCompletionRaisesDoneStateState = when (state) {
        is ParallelCompletionRaisesDoneStateState.A -> ParallelCompletionRaisesDoneStateState.A1
        is ParallelCompletionRaisesDoneStateState.B -> ParallelCompletionRaisesDoneStateState.B1
        is ParallelCompletionRaisesDoneStateState.Run -> ParallelCompletionRaisesDoneStateState.A1
        else -> state
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): ParallelCompletionRaisesDoneStateState? = when (stateId) {
        "a" -> ParallelCompletionRaisesDoneStateState.A
        "a1" -> ParallelCompletionRaisesDoneStateState.A1
        "a2" -> ParallelCompletionRaisesDoneStateState.A2
        "b" -> ParallelCompletionRaisesDoneStateState.B
        "b1" -> ParallelCompletionRaisesDoneStateState.B1
        "b2" -> ParallelCompletionRaisesDoneStateState.B2
        "run" -> ParallelCompletionRaisesDoneStateState.Run
        "stopped" -> ParallelCompletionRaisesDoneStateState.Stopped
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: ParallelCompletionRaisesDoneStateState): String = when (state) {
        is ParallelCompletionRaisesDoneStateState.A -> "a"
        is ParallelCompletionRaisesDoneStateState.A1 -> "a1"
        is ParallelCompletionRaisesDoneStateState.A2 -> "a2"
        is ParallelCompletionRaisesDoneStateState.B -> "b"
        is ParallelCompletionRaisesDoneStateState.B1 -> "b1"
        is ParallelCompletionRaisesDoneStateState.B2 -> "b2"
        is ParallelCompletionRaisesDoneStateState.Run -> "run"
        is ParallelCompletionRaisesDoneStateState.Stopped -> "stopped"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: ParallelCompletionRaisesDoneStateState): Boolean = when (state) {
        is ParallelCompletionRaisesDoneStateState.A -> false
        is ParallelCompletionRaisesDoneStateState.B -> false
        is ParallelCompletionRaisesDoneStateState.Run -> false
        else -> true
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelCompletionRaisesDoneStateState): Boolean = when (state) {
        is ParallelCompletionRaisesDoneStateState.Run -> true
        else -> false
    }

    // W3C SCXML 3.4: Get child regions of a parallel state (C++ getParallelRegions pattern)
    override fun getParallelRegions(state: ParallelCompletionRaisesDoneStateState): List<ParallelCompletionRaisesDoneStateState> = when (state) {
        is ParallelCompletionRaisesDoneStateState.Run -> listOf(ParallelCompletionRaisesDoneStateState.A, ParallelCompletionRaisesDoneStateState.B)
        else -> emptyList()
    }

    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: ParallelCompletionRaisesDoneStateState): Int = when (state) {
        is ParallelCompletionRaisesDoneStateState.A -> 1
        is ParallelCompletionRaisesDoneStateState.A1 -> 2
        is ParallelCompletionRaisesDoneStateState.A2 -> 3
        is ParallelCompletionRaisesDoneStateState.B -> 4
        is ParallelCompletionRaisesDoneStateState.B1 -> 5
        is ParallelCompletionRaisesDoneStateState.B2 -> 6
        is ParallelCompletionRaisesDoneStateState.Run -> 0
        is ParallelCompletionRaisesDoneStateState.Stopped -> 7
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: ParallelCompletionRaisesDoneStateState,
        event: ParallelCompletionRaisesDoneStateEvent
    ): TransitionResult<ParallelCompletionRaisesDoneStateState> = when (state) {
        // W3C SCXML 3.13: Ancestor-only routing (a has no own event transitions)
        is ParallelCompletionRaisesDoneStateState.A -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is ParallelCompletionRaisesDoneStateState.A1 -> {
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
        is ParallelCompletionRaisesDoneStateState.A2 -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (b has no own event transitions)
        is ParallelCompletionRaisesDoneStateState.B -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        is ParallelCompletionRaisesDoneStateState.B1 -> {
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
        is ParallelCompletionRaisesDoneStateState.B2 -> {
            val anc1 = processRun(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processA1(
        event: ParallelCompletionRaisesDoneStateEvent
    ): TransitionResult<ParallelCompletionRaisesDoneStateState> = when {
        event is ParallelCompletionRaisesDoneStateEvent.Go -> TransitionResult.External(ParallelCompletionRaisesDoneStateState.A2, ParallelCompletionRaisesDoneStateState.A1)

        else -> TransitionResult.Ignored
    }

    private fun processB1(
        event: ParallelCompletionRaisesDoneStateEvent
    ): TransitionResult<ParallelCompletionRaisesDoneStateState> = when {
        event is ParallelCompletionRaisesDoneStateEvent.Go -> TransitionResult.External(ParallelCompletionRaisesDoneStateState.B2, ParallelCompletionRaisesDoneStateState.B1)

        else -> TransitionResult.Ignored
    }

    private fun processRun(
        event: ParallelCompletionRaisesDoneStateEvent
    ): TransitionResult<ParallelCompletionRaisesDoneStateState> = when {
        event is ParallelCompletionRaisesDoneStateEvent.Bail -> TransitionResult.External(ParallelCompletionRaisesDoneStateState.Stopped, ParallelCompletionRaisesDoneStateState.Run)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine
    override fun onEntry(state: ParallelCompletionRaisesDoneStateState, pathChild: ParallelCompletionRaisesDoneStateState?) {
        when (state) {
            is ParallelCompletionRaisesDoneStateState.A -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:26 :: a :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelCompletionRaisesDoneStateState.A1)
                }
            }
            is ParallelCompletionRaisesDoneStateState.A1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:27 :: a1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a1")) return
            }
            is ParallelCompletionRaisesDoneStateState.A2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:30 :: a2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("a2")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.A, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("a2")) && (activeStateIds.contains("b2"))) {
                    raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.Run)
                }
            }
            is ParallelCompletionRaisesDoneStateState.B -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:33 :: b :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b")) return
                if (pathChild == null) {
                    // W3C SCXML 3.3: Enter initial child (C++ executeEntryActions pattern)
                    onEntry(ParallelCompletionRaisesDoneStateState.B1)
                }
            }
            is ParallelCompletionRaisesDoneStateState.B1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:34 :: b1 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b1")) return
            }
            is ParallelCompletionRaisesDoneStateState.B2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:37 :: b2 :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("b2")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.B, EventMetadata.platform())
                // W3C SCXML 3.7.1: Check if all regions of parallel grandparent are complete
                if ((activeStateIds.contains("a2")) && (activeStateIds.contains("b2"))) {
                    raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.Run)
                }
            }
            is ParallelCompletionRaisesDoneStateState.Run -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:24 :: run :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("run")) return
                // W3C SCXML 3.4 + §scxml-D-addDescendantStatesToEnter: a
                // `<parallel>` hands out defaults even when it is only an
                // ancestor — Appendix D's one exception to the ancestor rule.
                // The exception has its own exception: not the region the entry
                // set is already descending into, which `pathChild` names and
                // which the caller enters with the target's own path.
                if (pathChild != ParallelCompletionRaisesDoneStateState.A) {
                    onEntry(ParallelCompletionRaisesDoneStateState.A)
                }
                if (pathChild != ParallelCompletionRaisesDoneStateState.B) {
                    onEntry(ParallelCompletionRaisesDoneStateState.B)
                }
            }
            is ParallelCompletionRaisesDoneStateState.Stopped -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:54 :: stopped :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("stopped")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine
    override fun onExit(state: ParallelCompletionRaisesDoneStateState) {
        when (state) {
            is ParallelCompletionRaisesDoneStateState.A -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:26 :: a :: _state_body
                activeStateIds.remove("a")
            }
            is ParallelCompletionRaisesDoneStateState.A1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:27 :: a1 :: _state_body
                activeStateIds.remove("a1")
            }
            is ParallelCompletionRaisesDoneStateState.A2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:30 :: a2 :: _state_body
                activeStateIds.remove("a2")
            }
            is ParallelCompletionRaisesDoneStateState.B -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:33 :: b :: _state_body
                activeStateIds.remove("b")
            }
            is ParallelCompletionRaisesDoneStateState.B1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:34 :: b1 :: _state_body
                activeStateIds.remove("b1")
            }
            is ParallelCompletionRaisesDoneStateState.B2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:37 :: b2 :: _state_body
                activeStateIds.remove("b2")
            }
            is ParallelCompletionRaisesDoneStateState.Run -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:24 :: run :: _state_body
                // W3C SCXML 3.4/3.13: Exit active descendants of parallel state
                // in reverse document order (deepest states exit first).
                // Defensive: when called from exitHierarchy, descendants are already
                // exited and removed from activeStateIds — the contains() checks below
                // prevent double-exit. This code is needed for direct onExit() calls.
                val toExit = mutableListOf<Pair<ParallelCompletionRaisesDoneStateState, Int>>()
                if (activeStateIds.contains("a")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.A to 1)
                }
                if (activeStateIds.contains("a1")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.A1 to 2)
                }
                if (activeStateIds.contains("a2")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.A2 to 3)
                }
                if (activeStateIds.contains("b")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.B to 4)
                }
                if (activeStateIds.contains("b1")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.B1 to 5)
                }
                if (activeStateIds.contains("b2")) {
                    toExit.add(ParallelCompletionRaisesDoneStateState.B2 to 6)
                }
                toExit.sortByDescending { it.second }
                for ((desc, _) in toExit) {
                    onExit(desc)
                }
                activeStateIds.remove("run")
            }
            is ParallelCompletionRaisesDoneStateState.Stopped -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:54 :: stopped :: _state_body
                activeStateIds.remove("stopped")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine
    override fun executeTransitionActions(
        source: ParallelCompletionRaisesDoneStateState,
        event: ParallelCompletionRaisesDoneStateEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
