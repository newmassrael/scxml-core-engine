// SCE-GENERATED — DO NOT EDIT
// source-hash: 280975c88158c1a2612c8726a71e4ae581a1e42f8ef6d030924e99800aff8d10

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

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

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ParallelCompletionRaisesDoneStateState): Boolean = when (state) {
        is ParallelCompletionRaisesDoneStateState.A, is ParallelCompletionRaisesDoneStateState.B -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelCompletionRaisesDoneStateState): Boolean = when (state) {
        is ParallelCompletionRaisesDoneStateState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ParallelCompletionRaisesDoneStateState): Boolean = when (state) {
        is ParallelCompletionRaisesDoneStateState.A2, is ParallelCompletionRaisesDoneStateState.B2, is ParallelCompletionRaisesDoneStateState.Stopped -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ParallelCompletionRaisesDoneStateState): List<ParallelCompletionRaisesDoneStateState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ParallelCompletionRaisesDoneStateState): List<EntryTarget<ParallelCompletionRaisesDoneStateState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ParallelCompletionRaisesDoneStateState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ParallelCompletionRaisesDoneStateState, List<ParallelCompletionRaisesDoneStateState>> = mapOf(
            ParallelCompletionRaisesDoneStateState.A to listOf(ParallelCompletionRaisesDoneStateState.A1, ParallelCompletionRaisesDoneStateState.A2),
            ParallelCompletionRaisesDoneStateState.B to listOf(ParallelCompletionRaisesDoneStateState.B1, ParallelCompletionRaisesDoneStateState.B2),
            ParallelCompletionRaisesDoneStateState.Run to listOf(ParallelCompletionRaisesDoneStateState.A, ParallelCompletionRaisesDoneStateState.B),
        )

        val initialTargets: Map<ParallelCompletionRaisesDoneStateState, List<EntryTarget<ParallelCompletionRaisesDoneStateState, HistoryId>>> = mapOf(
            ParallelCompletionRaisesDoneStateState.A to listOf(StateTarget(ParallelCompletionRaisesDoneStateState.A1)),
            ParallelCompletionRaisesDoneStateState.B to listOf(StateTarget(ParallelCompletionRaisesDoneStateState.B1)),
        )

        val documentInitialTargetList: List<EntryTarget<ParallelCompletionRaisesDoneStateState, HistoryId>> =
            listOf(StateTarget(ParallelCompletionRaisesDoneStateState.Run))

        // W3C SCXML 3.13: a1's transition 0, as the microstep reads it.
        val transitionA1At0 = EnabledTransition<ParallelCompletionRaisesDoneStateState, HistoryId>(
            ParallelCompletionRaisesDoneStateState.A1,
            listOf(StateTarget(ParallelCompletionRaisesDoneStateState.A2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: b1's transition 0, as the microstep reads it.
        val transitionB1At0 = EnabledTransition<ParallelCompletionRaisesDoneStateState, HistoryId>(
            ParallelCompletionRaisesDoneStateState.B1,
            listOf(StateTarget(ParallelCompletionRaisesDoneStateState.B2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ParallelCompletionRaisesDoneStateState, HistoryId>(
            ParallelCompletionRaisesDoneStateState.Run,
            listOf(StateTarget(ParallelCompletionRaisesDoneStateState.Stopped)),
            0,
            hasActions = false,
            isInternal = false,
        )
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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ParallelCompletionRaisesDoneStateState,
        event: ParallelCompletionRaisesDoneStateEvent?
    ): EnabledTransition<ParallelCompletionRaisesDoneStateState, HistoryId>? = when (state) {
        is ParallelCompletionRaisesDoneStateState.A1 -> when {
            event is ParallelCompletionRaisesDoneStateEvent.Go -> transitionA1At0
            else -> null
        }
        is ParallelCompletionRaisesDoneStateState.B1 -> when {
            event is ParallelCompletionRaisesDoneStateEvent.Go -> transitionB1At0
            else -> null
        }
        is ParallelCompletionRaisesDoneStateState.Run -> when {
            event is ParallelCompletionRaisesDoneStateEvent.Bail -> transitionRunAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine
    override fun onEntry(state: ParallelCompletionRaisesDoneStateState, isDefaultEntry: Boolean) {
        when (state) {
            is ParallelCompletionRaisesDoneStateState.A -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:26 :: a :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.A1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:27 :: a1 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.A2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:30 :: a2 :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.A, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ParallelCompletionRaisesDoneStateState.Run)) {
                    raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.Run)
                }
            }
            is ParallelCompletionRaisesDoneStateState.B -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:33 :: b :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.B1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:34 :: b1 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.B2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:37 :: b2 :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.B, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ParallelCompletionRaisesDoneStateState.Run)) {
                    raiseInternal(ParallelCompletionRaisesDoneStateEvent.Done.State.Run)
                }
            }
            is ParallelCompletionRaisesDoneStateState.Run -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:24 :: run :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.Stopped -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:54 :: stopped :: _state_body
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
            }
            is ParallelCompletionRaisesDoneStateState.A1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:27 :: a1 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.A2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:30 :: a2 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.B -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:33 :: b :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.B1 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:34 :: b1 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.B2 -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:37 :: b2 :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.Run -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:24 :: run :: _state_body
            }
            is ParallelCompletionRaisesDoneStateState.Stopped -> {
                // SCE-MAP: parallel_completion_raises_done_state.scxml:54 :: stopped :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: parallel_completion_raises_done_state.scxml:21 :: _machine
    override fun executeTransitionContent(source: ParallelCompletionRaisesDoneStateState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
