// SCE-GENERATED — DO NOT EDIT
// source-hash: 4f209294ba851e9f433a2fd839fc088f718569422204e93318892b83dc408fac

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

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

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: ParallelDoneStateIsDeliveredState): Boolean = when (state) {
        is ParallelDoneStateIsDeliveredState.A, is ParallelDoneStateIsDeliveredState.B -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: ParallelDoneStateIsDeliveredState): Boolean = when (state) {
        is ParallelDoneStateIsDeliveredState.Run -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: ParallelDoneStateIsDeliveredState): Boolean = when (state) {
        is ParallelDoneStateIsDeliveredState.A2, is ParallelDoneStateIsDeliveredState.B2, is ParallelDoneStateIsDeliveredState.Settled -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: ParallelDoneStateIsDeliveredState): List<ParallelDoneStateIsDeliveredState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: ParallelDoneStateIsDeliveredState): List<EntryTarget<ParallelDoneStateIsDeliveredState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<ParallelDoneStateIsDeliveredState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<ParallelDoneStateIsDeliveredState, List<ParallelDoneStateIsDeliveredState>> = mapOf(
            ParallelDoneStateIsDeliveredState.A to listOf(ParallelDoneStateIsDeliveredState.A1, ParallelDoneStateIsDeliveredState.A2),
            ParallelDoneStateIsDeliveredState.B to listOf(ParallelDoneStateIsDeliveredState.B1, ParallelDoneStateIsDeliveredState.B2),
            ParallelDoneStateIsDeliveredState.Run to listOf(ParallelDoneStateIsDeliveredState.A, ParallelDoneStateIsDeliveredState.B),
        )

        val initialTargets: Map<ParallelDoneStateIsDeliveredState, List<EntryTarget<ParallelDoneStateIsDeliveredState, HistoryId>>> = mapOf(
            ParallelDoneStateIsDeliveredState.A to listOf(StateTarget(ParallelDoneStateIsDeliveredState.A1)),
            ParallelDoneStateIsDeliveredState.B to listOf(StateTarget(ParallelDoneStateIsDeliveredState.B1)),
        )

        val documentInitialTargetList: List<EntryTarget<ParallelDoneStateIsDeliveredState, HistoryId>> =
            listOf(StateTarget(ParallelDoneStateIsDeliveredState.Run))

        // W3C SCXML 3.13: a1's transition 0, as the microstep reads it.
        val transitionA1At0 = EnabledTransition<ParallelDoneStateIsDeliveredState, HistoryId>(
            ParallelDoneStateIsDeliveredState.A1,
            listOf(StateTarget(ParallelDoneStateIsDeliveredState.A2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: b1's transition 0, as the microstep reads it.
        val transitionB1At0 = EnabledTransition<ParallelDoneStateIsDeliveredState, HistoryId>(
            ParallelDoneStateIsDeliveredState.B1,
            listOf(StateTarget(ParallelDoneStateIsDeliveredState.B2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: run's transition 0, as the microstep reads it.
        val transitionRunAt0 = EnabledTransition<ParallelDoneStateIsDeliveredState, HistoryId>(
            ParallelDoneStateIsDeliveredState.Run,
            listOf(StateTarget(ParallelDoneStateIsDeliveredState.Settled)),
            0,
            hasActions = false,
            isInternal = false,
        )
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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: ParallelDoneStateIsDeliveredState,
        event: ParallelDoneStateIsDeliveredEvent?
    ): EnabledTransition<ParallelDoneStateIsDeliveredState, HistoryId>? = when (state) {
        is ParallelDoneStateIsDeliveredState.A1 -> when {
            event is ParallelDoneStateIsDeliveredEvent.Go -> transitionA1At0
            else -> null
        }
        is ParallelDoneStateIsDeliveredState.B1 -> when {
            event is ParallelDoneStateIsDeliveredEvent.Go -> transitionB1At0
            else -> null
        }
        is ParallelDoneStateIsDeliveredState.Run -> when {
            event is ParallelDoneStateIsDeliveredEvent.Done.State.Run -> transitionRunAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine
    override fun onEntry(state: ParallelDoneStateIsDeliveredState, isDefaultEntry: Boolean) {
        when (state) {
            is ParallelDoneStateIsDeliveredState.A -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:37 :: a :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.A1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:38 :: a1 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.A2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:41 :: a2 :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.A, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ParallelDoneStateIsDeliveredState.Run)) {
                    raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.Run)
                }
            }
            is ParallelDoneStateIsDeliveredState.B -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:44 :: b :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.B1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:45 :: b1 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.B2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:48 :: b2 :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.B, EventMetadata.platform())
                // W3C SCXML 3.7.1: this <final> may have completed the
                // <parallel> grandparent — Appendix D's isInFinalState, which
                // counts a region that is itself a <parallel> only once all
                // of ITS regions are final.
                if (isStateInFinalState(ParallelDoneStateIsDeliveredState.Run)) {
                    raiseInternal(ParallelDoneStateIsDeliveredEvent.Done.State.Run)
                }
            }
            is ParallelDoneStateIsDeliveredState.Run -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:35 :: run :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.Settled -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:64 :: settled :: _state_body
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
            }
            is ParallelDoneStateIsDeliveredState.A1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:38 :: a1 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.A2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:41 :: a2 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.B -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:44 :: b :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.B1 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:45 :: b1 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.B2 -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:48 :: b2 :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.Run -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:35 :: run :: _state_body
            }
            is ParallelDoneStateIsDeliveredState.Settled -> {
                // SCE-MAP: parallel_done_state_is_delivered.scxml:64 :: settled :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: parallel_done_state_is_delivered.scxml:32 :: _machine
    override fun executeTransitionContent(source: ParallelDoneStateIsDeliveredState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
