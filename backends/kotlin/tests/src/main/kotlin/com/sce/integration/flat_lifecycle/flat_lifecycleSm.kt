// SCE-GENERATED — DO NOT EDIT
// source-hash: c39067aa383d615204755340a5be95d22b2cb7f22bae673ca1ae1c99211df076

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/flat_lifecycle/flat_lifecycle.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: flat_lifecycle.scxml:17 :: _machine

package com.sce.integration.flat_lifecycle

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface FlatLifecycleState : State {
    data object Idle : FlatLifecycleState
    data object Paused : FlatLifecycleState
    data object Running : FlatLifecycleState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface FlatLifecycleEvent : Event {
    data object Pause : FlatLifecycleEvent
    data object Ping : FlatLifecycleEvent
    data object Resume : FlatLifecycleEvent
    data object Start : FlatLifecycleEvent
    data object Stop : FlatLifecycleEvent
    data object Tick : FlatLifecycleEvent
}
// --- State Machine (W3C SCXML) ---

class FlatLifecycleStateMachine(
) : StateMachineEngine<FlatLifecycleState, FlatLifecycleEvent>() {

    override val initialState: FlatLifecycleState = FlatLifecycleState.Idle

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<FlatLifecycleState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<FlatLifecycleState, HistoryId>> =
            listOf(StateTarget(FlatLifecycleState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Idle,
            listOf(StateTarget(FlatLifecycleState.Running)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Idle,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 0, as the microstep reads it.
        val transitionPausedAt0 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Paused,
            listOf(StateTarget(FlatLifecycleState.Running)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 1, as the microstep reads it.
        val transitionPausedAt1 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Paused,
            listOf(StateTarget(FlatLifecycleState.Idle)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 0, as the microstep reads it.
        val transitionRunningAt0 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Running,
            listOf(StateTarget(FlatLifecycleState.Paused)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 1, as the microstep reads it.
        val transitionRunningAt1 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Running,
            listOf(StateTarget(FlatLifecycleState.Running)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 2, as the microstep reads it.
        val transitionRunningAt2 = EnabledTransition<FlatLifecycleState, HistoryId>(
            FlatLifecycleState.Running,
            emptyList(),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): FlatLifecycleState? = when (stateId) {
        "idle" -> FlatLifecycleState.Idle
        "paused" -> FlatLifecycleState.Paused
        "running" -> FlatLifecycleState.Running
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: FlatLifecycleState): String = when (state) {
        is FlatLifecycleState.Idle -> "idle"
        is FlatLifecycleState.Paused -> "paused"
        is FlatLifecycleState.Running -> "running"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: FlatLifecycleState): Int = when (state) {
        is FlatLifecycleState.Idle -> 0
        is FlatLifecycleState.Paused -> 2
        is FlatLifecycleState.Running -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): FlatLifecycleEvent? = when (name) {
        "pause" -> FlatLifecycleEvent.Pause
        "ping" -> FlatLifecycleEvent.Ping
        "resume" -> FlatLifecycleEvent.Resume
        "start" -> FlatLifecycleEvent.Start
        "stop" -> FlatLifecycleEvent.Stop
        "tick" -> FlatLifecycleEvent.Tick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: FlatLifecycleEvent): String? = when (event) {
        is FlatLifecycleEvent.Pause -> "pause"
        is FlatLifecycleEvent.Ping -> "ping"
        is FlatLifecycleEvent.Resume -> "resume"
        is FlatLifecycleEvent.Start -> "start"
        is FlatLifecycleEvent.Stop -> "stop"
        is FlatLifecycleEvent.Tick -> "tick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: FlatLifecycleState,
        event: FlatLifecycleEvent?
    ): EnabledTransition<FlatLifecycleState, HistoryId>? = when (state) {
        is FlatLifecycleState.Idle -> when {
            event is FlatLifecycleEvent.Start -> transitionIdleAt0
            event is FlatLifecycleEvent.Ping -> transitionIdleAt1
            else -> null
        }
        is FlatLifecycleState.Paused -> when {
            event is FlatLifecycleEvent.Resume -> transitionPausedAt0
            event is FlatLifecycleEvent.Stop -> transitionPausedAt1
            else -> null
        }
        is FlatLifecycleState.Running -> when {
            event is FlatLifecycleEvent.Pause -> transitionRunningAt0
            event is FlatLifecycleEvent.Tick -> transitionRunningAt1
            event is FlatLifecycleEvent.Ping -> transitionRunningAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: flat_lifecycle.scxml:17 :: _machine
    override fun onEntry(state: FlatLifecycleState, isDefaultEntry: Boolean) {
        when (state) {
            is FlatLifecycleState.Idle -> {
                // SCE-MAP: flat_lifecycle.scxml:24 :: idle :: _state_body
            }
            is FlatLifecycleState.Paused -> {
                // SCE-MAP: flat_lifecycle.scxml:33 :: paused :: _state_body
            }
            is FlatLifecycleState.Running -> {
                // SCE-MAP: flat_lifecycle.scxml:28 :: running :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: flat_lifecycle.scxml:17 :: _machine
    override fun onExit(state: FlatLifecycleState) {
        when (state) {
            is FlatLifecycleState.Idle -> {
                // SCE-MAP: flat_lifecycle.scxml:24 :: idle :: _state_body
            }
            is FlatLifecycleState.Paused -> {
                // SCE-MAP: flat_lifecycle.scxml:33 :: paused :: _state_body
            }
            is FlatLifecycleState.Running -> {
                // SCE-MAP: flat_lifecycle.scxml:28 :: running :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: flat_lifecycle.scxml:17 :: _machine
    override fun executeTransitionContent(source: FlatLifecycleState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
