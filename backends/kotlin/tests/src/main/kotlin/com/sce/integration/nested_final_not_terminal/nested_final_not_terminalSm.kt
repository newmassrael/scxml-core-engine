// SCE-GENERATED — DO NOT EDIT
// source-hash: 1b92577399a02f25bad414acd653ef70d1b84a060adcb161fcce7266e21da4f7
// template-hash: 4c8716167d13ae127559f117ceaafdd30c55d8d87332557ef62bafcb20bdd1b8
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/nested_final_not_terminal/nested_final_not_terminal.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_final_not_terminal.scxml:41

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: NestedFinalNotTerminalState): NestedFinalNotTerminalState? = when (state) {
        is NestedFinalNotTerminalState.PhaseDone -> NestedFinalNotTerminalState.Phase
        is NestedFinalNotTerminalState.Running -> NestedFinalNotTerminalState.Phase
        else -> null
    }

    // W3C SCXML 3.3/3.4: Resolve compound/parallel state to initial leaf state
    override fun resolveLeafState(state: NestedFinalNotTerminalState): NestedFinalNotTerminalState = when (state) {
        is NestedFinalNotTerminalState.Phase -> NestedFinalNotTerminalState.Running
        else -> state
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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: NestedFinalNotTerminalState): Boolean = when (state) {
        is NestedFinalNotTerminalState.Phase -> false
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: NestedFinalNotTerminalState): Int = when (state) {
        is NestedFinalNotTerminalState.Pass -> 3
        is NestedFinalNotTerminalState.Phase -> 0
        is NestedFinalNotTerminalState.PhaseDone -> 2
        is NestedFinalNotTerminalState.Running -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: NestedFinalNotTerminalState,
        event: NestedFinalNotTerminalEvent
    ): TransitionResult<NestedFinalNotTerminalState> = when (state) {
        is NestedFinalNotTerminalState.Phase -> processPhase(event)
        // W3C SCXML 3.13: Ancestor-only routing (phaseDone has no own event transitions)
        is NestedFinalNotTerminalState.PhaseDone -> {
            val anc1 = processPhase(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        // W3C SCXML 3.13: Ancestor-only routing (running has no own event transitions)
        is NestedFinalNotTerminalState.Running -> {
            val anc1 = processPhase(event)
            if (anc1 !is TransitionResult.Ignored) anc1
            else TransitionResult.Ignored
        }
        else -> TransitionResult.Ignored
    }

    // W3C SCXML Appendix D: Eventless (null) transition check
    override fun processNullEvent(
        state: NestedFinalNotTerminalState
    ): TransitionResult<NestedFinalNotTerminalState> = when (state) {
        is NestedFinalNotTerminalState.Running -> processNullRunning()
        else -> TransitionResult.Ignored
    }

    // --- Per-State Null (Eventless) Handlers ---

    private fun processNullRunning(
    ): TransitionResult<NestedFinalNotTerminalState> = when {
        // W3C SCXML 3.13: First unconditional transition wins (document order)
        else -> TransitionResult.External(NestedFinalNotTerminalState.PhaseDone, NestedFinalNotTerminalState.Running)
    }

    // --- Per-State Event Handlers ---

    private fun processPhase(
        event: NestedFinalNotTerminalEvent
    ): TransitionResult<NestedFinalNotTerminalState> = when {
        event is NestedFinalNotTerminalEvent.Resume -> TransitionResult.External(NestedFinalNotTerminalState.Pass, NestedFinalNotTerminalState.Phase)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: nested_final_not_terminal.scxml:41
    override fun onEntry(state: NestedFinalNotTerminalState) {
        when (state) {
            is NestedFinalNotTerminalState.Pass -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is NestedFinalNotTerminalState.Phase -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phase")) return
            }
            is NestedFinalNotTerminalState.PhaseDone -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("phaseDone")) return
                // W3C SCXML 3.7: Final child state reached, raise done.state for parent
                raiseInternal(NestedFinalNotTerminalEvent.Done.State.Phase, EventMetadata.platform())
            }
            is NestedFinalNotTerminalState.Running -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("running")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: nested_final_not_terminal.scxml:41
    override fun onExit(state: NestedFinalNotTerminalState) {
        when (state) {
            is NestedFinalNotTerminalState.Pass -> {
                activeStateIds.remove("pass")
            }
            is NestedFinalNotTerminalState.Phase -> {
                activeStateIds.remove("phase")
            }
            is NestedFinalNotTerminalState.PhaseDone -> {
                activeStateIds.remove("phaseDone")
            }
            is NestedFinalNotTerminalState.Running -> {
                activeStateIds.remove("running")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: nested_final_not_terminal.scxml:41
    override fun executeTransitionActions(
        source: NestedFinalNotTerminalState,
        event: NestedFinalNotTerminalEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
