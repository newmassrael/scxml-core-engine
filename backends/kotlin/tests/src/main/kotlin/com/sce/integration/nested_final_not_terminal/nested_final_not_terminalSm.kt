// SCE-GENERATED — DO NOT EDIT
// source-hash: 1b92577399a02f25bad414acd653ef70d1b84a060adcb161fcce7266e21da4f7

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_final_not_terminal.scxml:41 :: _machine

package com.sce.integration.nested_final_not_terminal

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface NestedFinalNotTerminalState : State {
    data object Pass : NestedFinalNotTerminalState
    data object Phase : NestedFinalNotTerminalState
    data object PhaseDone : NestedFinalNotTerminalState
    data object Running : NestedFinalNotTerminalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface NestedFinalNotTerminalEvent : Event {
    sealed interface Done : NestedFinalNotTerminalEvent {
        sealed interface State : Done {
            data object Phase : State
        }
    }
    data object Resume : NestedFinalNotTerminalEvent
}
// --- State Machine (W3C SCXML) ---

class NestedFinalNotTerminalStateMachine(
) : StateMachineEngine<NestedFinalNotTerminalState, NestedFinalNotTerminalEvent>() {

    override val initialState: NestedFinalNotTerminalState = NestedFinalNotTerminalState.Running

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
    override fun parentOf(state: NestedFinalNotTerminalState): NestedFinalNotTerminalState? = when (state) {
        is NestedFinalNotTerminalState.PhaseDone -> NestedFinalNotTerminalState.Phase
        is NestedFinalNotTerminalState.Running -> NestedFinalNotTerminalState.Phase
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: NestedFinalNotTerminalState): Boolean = when (state) {
        is NestedFinalNotTerminalState.Phase -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: NestedFinalNotTerminalState): Boolean = when (state) {
        is NestedFinalNotTerminalState.Pass, is NestedFinalNotTerminalState.PhaseDone -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: NestedFinalNotTerminalState): List<NestedFinalNotTerminalState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: NestedFinalNotTerminalState): List<EntryTarget<NestedFinalNotTerminalState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<NestedFinalNotTerminalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<NestedFinalNotTerminalState, List<NestedFinalNotTerminalState>> = mapOf(
            NestedFinalNotTerminalState.Phase to listOf(NestedFinalNotTerminalState.Running, NestedFinalNotTerminalState.PhaseDone),
        )

        val initialTargets: Map<NestedFinalNotTerminalState, List<EntryTarget<NestedFinalNotTerminalState, HistoryId>>> = mapOf(
            NestedFinalNotTerminalState.Phase to listOf(StateTarget(NestedFinalNotTerminalState.Running)),
        )

        val documentInitialTargetList: List<EntryTarget<NestedFinalNotTerminalState, HistoryId>> =
            listOf(StateTarget(NestedFinalNotTerminalState.Phase))

        // W3C SCXML 3.13: phase's transition 0, as the microstep reads it.
        val transitionPhaseAt0 = EnabledTransition<NestedFinalNotTerminalState, HistoryId>(
            NestedFinalNotTerminalState.Phase,
            listOf(StateTarget(NestedFinalNotTerminalState.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 0, as the microstep reads it.
        val transitionRunningAt0 = EnabledTransition<NestedFinalNotTerminalState, HistoryId>(
            NestedFinalNotTerminalState.Running,
            listOf(StateTarget(NestedFinalNotTerminalState.PhaseDone)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): NestedFinalNotTerminalState? = when (stateId) {
        "pass" -> NestedFinalNotTerminalState.Pass
        "phase" -> NestedFinalNotTerminalState.Phase
        "phaseDone" -> NestedFinalNotTerminalState.PhaseDone
        "running" -> NestedFinalNotTerminalState.Running
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: NestedFinalNotTerminalState): String = when (state) {
        is NestedFinalNotTerminalState.Pass -> "pass"
        is NestedFinalNotTerminalState.Phase -> "phase"
        is NestedFinalNotTerminalState.PhaseDone -> "phaseDone"
        is NestedFinalNotTerminalState.Running -> "running"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: NestedFinalNotTerminalState): Int = when (state) {
        is NestedFinalNotTerminalState.Pass -> 3
        is NestedFinalNotTerminalState.Phase -> 0
        is NestedFinalNotTerminalState.PhaseDone -> 2
        is NestedFinalNotTerminalState.Running -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: NestedFinalNotTerminalState,
        event: NestedFinalNotTerminalEvent?
    ): EnabledTransition<NestedFinalNotTerminalState, HistoryId>? = when (state) {
        is NestedFinalNotTerminalState.Phase -> when {
            event is NestedFinalNotTerminalEvent.Resume -> transitionPhaseAt0
            else -> null
        }
        is NestedFinalNotTerminalState.Running -> when {
            event == null -> transitionRunningAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: nested_final_not_terminal.scxml:41 :: _machine
    override fun onEntry(state: NestedFinalNotTerminalState, isDefaultEntry: Boolean) {
        when (state) {
            is NestedFinalNotTerminalState.Pass -> {
                // SCE-MAP: nested_final_not_terminal.scxml:51 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is NestedFinalNotTerminalState.Phase -> {
                // SCE-MAP: nested_final_not_terminal.scxml:44 :: phase :: _state_body
            }
            is NestedFinalNotTerminalState.PhaseDone -> {
                // SCE-MAP: nested_final_not_terminal.scxml:48 :: phaseDone :: _state_body
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(NestedFinalNotTerminalEvent.Done.State.Phase, EventMetadata.platform())
            }
            is NestedFinalNotTerminalState.Running -> {
                // SCE-MAP: nested_final_not_terminal.scxml:45 :: running :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: nested_final_not_terminal.scxml:41 :: _machine
    override fun onExit(state: NestedFinalNotTerminalState) {
        when (state) {
            is NestedFinalNotTerminalState.Pass -> {
                // SCE-MAP: nested_final_not_terminal.scxml:51 :: pass :: _state_body
            }
            is NestedFinalNotTerminalState.Phase -> {
                // SCE-MAP: nested_final_not_terminal.scxml:44 :: phase :: _state_body
            }
            is NestedFinalNotTerminalState.PhaseDone -> {
                // SCE-MAP: nested_final_not_terminal.scxml:48 :: phaseDone :: _state_body
            }
            is NestedFinalNotTerminalState.Running -> {
                // SCE-MAP: nested_final_not_terminal.scxml:45 :: running :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: nested_final_not_terminal.scxml:41 :: _machine
    override fun executeTransitionContent(source: NestedFinalNotTerminalState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
