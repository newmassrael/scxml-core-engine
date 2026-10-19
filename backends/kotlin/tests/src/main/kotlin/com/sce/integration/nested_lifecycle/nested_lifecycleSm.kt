// SCE-GENERATED — DO NOT EDIT
// source-hash: 239a1140d1db8dfa52f222a8f17ddb51efb6fc1d0cddb120dc5b4b0ee2173ab0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/nested_lifecycle/nested_lifecycle.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: nested_lifecycle.scxml:27 :: _machine

package com.sce.integration.nested_lifecycle

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface NestedLifecycleState : State {
    data object Active : NestedLifecycleState
    data object Closed : NestedLifecycleState
    data object Held : NestedLifecycleState
    data object Loading : NestedLifecycleState
    data object Ready : NestedLifecycleState
    data object Retrying : NestedLifecycleState
    data object Session : NestedLifecycleState
    data object Setup : NestedLifecycleState
    data object Working : NestedLifecycleState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface NestedLifecycleEvent : Event {
    data object Wildcard : NestedLifecycleEvent
    data object Abort : NestedLifecycleEvent
    sealed interface Data : NestedLifecycleEvent {
        data object Failed : Data
        data object Ready : Data
    }
    data object Done : NestedLifecycleEvent
    data object Go : NestedLifecycleEvent
    data object Pause : NestedLifecycleEvent
    data object Reset : NestedLifecycleEvent
    data object Resume : NestedLifecycleEvent
}
// --- State Machine (W3C SCXML) ---

class NestedLifecycleStateMachine(
) : StateMachineEngine<NestedLifecycleState, NestedLifecycleEvent>() {

    override val initialState: NestedLifecycleState = NestedLifecycleState.Loading

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: NestedLifecycleState): NestedLifecycleState? = when (state) {
        is NestedLifecycleState.Active -> NestedLifecycleState.Session
        is NestedLifecycleState.Held -> NestedLifecycleState.Active
        is NestedLifecycleState.Loading -> NestedLifecycleState.Setup
        is NestedLifecycleState.Ready -> NestedLifecycleState.Setup
        is NestedLifecycleState.Retrying -> NestedLifecycleState.Setup
        is NestedLifecycleState.Setup -> NestedLifecycleState.Session
        is NestedLifecycleState.Working -> NestedLifecycleState.Active
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: NestedLifecycleState): Boolean = when (state) {
        is NestedLifecycleState.Active, is NestedLifecycleState.Session, is NestedLifecycleState.Setup -> true
        else -> false
    }

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: NestedLifecycleState): Boolean = when (state) {
        is NestedLifecycleState.Closed -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: NestedLifecycleState): List<NestedLifecycleState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: NestedLifecycleState): List<EntryTarget<NestedLifecycleState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<NestedLifecycleState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val childStates: Map<NestedLifecycleState, List<NestedLifecycleState>> = mapOf(
            NestedLifecycleState.Active to listOf(NestedLifecycleState.Working, NestedLifecycleState.Held),
            NestedLifecycleState.Session to listOf(NestedLifecycleState.Active, NestedLifecycleState.Setup),
            NestedLifecycleState.Setup to listOf(NestedLifecycleState.Retrying, NestedLifecycleState.Loading, NestedLifecycleState.Ready),
        )

        val initialTargets: Map<NestedLifecycleState, List<EntryTarget<NestedLifecycleState, HistoryId>>> = mapOf(
            NestedLifecycleState.Active to listOf(StateTarget(NestedLifecycleState.Working)),
            NestedLifecycleState.Session to listOf(StateTarget(NestedLifecycleState.Setup)),
            NestedLifecycleState.Setup to listOf(StateTarget(NestedLifecycleState.Loading)),
        )

        val documentInitialTargetList: List<EntryTarget<NestedLifecycleState, HistoryId>> =
            listOf(StateTarget(NestedLifecycleState.Session))

        // W3C SCXML 3.13: active's transition 0, as the microstep reads it.
        val transitionActiveAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Active,
            listOf(StateTarget(NestedLifecycleState.Held)),
            0,
            hasActions = false,
            isInternal = true,
        )

        // W3C SCXML 3.13: active's transition 1, as the microstep reads it.
        val transitionActiveAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Active,
            listOf(StateTarget(NestedLifecycleState.Active)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: held's transition 0, as the microstep reads it.
        val transitionHeldAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Held,
            listOf(StateTarget(NestedLifecycleState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: held's transition 1, as the microstep reads it.
        val transitionHeldAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Held,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: loading's transition 0, as the microstep reads it.
        val transitionLoadingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Loading,
            listOf(StateTarget(NestedLifecycleState.Ready)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: loading's transition 1, as the microstep reads it.
        val transitionLoadingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Loading,
            listOf(StateTarget(NestedLifecycleState.Retrying)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: ready's transition 0, as the microstep reads it.
        val transitionReadyAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Ready,
            listOf(StateTarget(NestedLifecycleState.Active)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: retrying's transition 0, as the microstep reads it.
        val transitionRetryingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Retrying,
            listOf(StateTarget(NestedLifecycleState.Ready)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: retrying's transition 1, as the microstep reads it.
        val transitionRetryingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Retrying,
            listOf(StateTarget(NestedLifecycleState.Loading)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: session's transition 0, as the microstep reads it.
        val transitionSessionAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Session,
            listOf(StateTarget(NestedLifecycleState.Closed)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Working,
            listOf(StateTarget(NestedLifecycleState.Closed)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<NestedLifecycleState, HistoryId>(
            NestedLifecycleState.Working,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): NestedLifecycleState? = when (stateId) {
        "active" -> NestedLifecycleState.Active
        "closed" -> NestedLifecycleState.Closed
        "held" -> NestedLifecycleState.Held
        "loading" -> NestedLifecycleState.Loading
        "ready" -> NestedLifecycleState.Ready
        "retrying" -> NestedLifecycleState.Retrying
        "session" -> NestedLifecycleState.Session
        "setup" -> NestedLifecycleState.Setup
        "working" -> NestedLifecycleState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: NestedLifecycleState): String = when (state) {
        is NestedLifecycleState.Active -> "active"
        is NestedLifecycleState.Closed -> "closed"
        is NestedLifecycleState.Held -> "held"
        is NestedLifecycleState.Loading -> "loading"
        is NestedLifecycleState.Ready -> "ready"
        is NestedLifecycleState.Retrying -> "retrying"
        is NestedLifecycleState.Session -> "session"
        is NestedLifecycleState.Setup -> "setup"
        is NestedLifecycleState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: NestedLifecycleState): Int = when (state) {
        is NestedLifecycleState.Active -> 1
        is NestedLifecycleState.Closed -> 8
        is NestedLifecycleState.Held -> 3
        is NestedLifecycleState.Loading -> 6
        is NestedLifecycleState.Ready -> 7
        is NestedLifecycleState.Retrying -> 5
        is NestedLifecycleState.Session -> 0
        is NestedLifecycleState.Setup -> 4
        is NestedLifecycleState.Working -> 2
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): NestedLifecycleEvent? = when (name) {
        "abort" -> NestedLifecycleEvent.Abort
        "data.failed" -> NestedLifecycleEvent.Data.Failed
        "data.ready" -> NestedLifecycleEvent.Data.Ready
        "done" -> NestedLifecycleEvent.Done
        "go" -> NestedLifecycleEvent.Go
        "pause" -> NestedLifecycleEvent.Pause
        "reset" -> NestedLifecycleEvent.Reset
        "resume" -> NestedLifecycleEvent.Resume
        "*" -> NestedLifecycleEvent.Wildcard
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: NestedLifecycleEvent): String? = when (event) {
        is NestedLifecycleEvent.Abort -> "abort"
        is NestedLifecycleEvent.Data.Failed -> "data.failed"
        is NestedLifecycleEvent.Data.Ready -> "data.ready"
        is NestedLifecycleEvent.Done -> "done"
        is NestedLifecycleEvent.Go -> "go"
        is NestedLifecycleEvent.Pause -> "pause"
        is NestedLifecycleEvent.Reset -> "reset"
        is NestedLifecycleEvent.Resume -> "resume"
        is NestedLifecycleEvent.Wildcard -> "*"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: NestedLifecycleState,
        event: NestedLifecycleEvent?
    ): EnabledTransition<NestedLifecycleState, HistoryId>? = when (state) {
        is NestedLifecycleState.Active -> when {
            event is NestedLifecycleEvent.Pause -> transitionActiveAt0
            event is NestedLifecycleEvent.Reset -> transitionActiveAt1
            else -> null
        }
        is NestedLifecycleState.Held -> when {
            event is NestedLifecycleEvent.Resume -> transitionHeldAt0
            event != null -> transitionHeldAt1
            else -> null
        }
        is NestedLifecycleState.Loading -> when {
            event is NestedLifecycleEvent.Data.Ready -> transitionLoadingAt0
            event is NestedLifecycleEvent.Data.Failed -> transitionLoadingAt1
            else -> null
        }
        is NestedLifecycleState.Ready -> when {
            event is NestedLifecycleEvent.Go -> transitionReadyAt0
            else -> null
        }
        is NestedLifecycleState.Retrying -> when {
            event is NestedLifecycleEvent.Data.Ready -> transitionRetryingAt0
            (event is NestedLifecycleEvent.Data.Failed || event is NestedLifecycleEvent.Data.Ready) -> transitionRetryingAt1
            else -> null
        }
        is NestedLifecycleState.Session -> when {
            event is NestedLifecycleEvent.Abort -> transitionSessionAt0
            else -> null
        }
        is NestedLifecycleState.Working -> when {
            event is NestedLifecycleEvent.Done -> transitionWorkingAt0
            event is NestedLifecycleEvent.Resume -> transitionWorkingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun onEntry(state: NestedLifecycleState, isDefaultEntry: Boolean) {
        when (state) {
            is NestedLifecycleState.Active -> {
                // SCE-MAP: nested_lifecycle.scxml:36 :: active :: _state_body
            }
            is NestedLifecycleState.Closed -> {
                // SCE-MAP: nested_lifecycle.scxml:63 :: closed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is NestedLifecycleState.Held -> {
                // SCE-MAP: nested_lifecycle.scxml:43 :: held :: _state_body
            }
            is NestedLifecycleState.Loading -> {
                // SCE-MAP: nested_lifecycle.scxml:54 :: loading :: _state_body
            }
            is NestedLifecycleState.Ready -> {
                // SCE-MAP: nested_lifecycle.scxml:58 :: ready :: _state_body
            }
            is NestedLifecycleState.Retrying -> {
                // SCE-MAP: nested_lifecycle.scxml:50 :: retrying :: _state_body
            }
            is NestedLifecycleState.Session -> {
                // SCE-MAP: nested_lifecycle.scxml:34 :: session :: _state_body
            }
            is NestedLifecycleState.Setup -> {
                // SCE-MAP: nested_lifecycle.scxml:48 :: setup :: _state_body
            }
            is NestedLifecycleState.Working -> {
                // SCE-MAP: nested_lifecycle.scxml:39 :: working :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun onExit(state: NestedLifecycleState) {
        when (state) {
            is NestedLifecycleState.Active -> {
                // SCE-MAP: nested_lifecycle.scxml:36 :: active :: _state_body
            }
            is NestedLifecycleState.Closed -> {
                // SCE-MAP: nested_lifecycle.scxml:63 :: closed :: _state_body
            }
            is NestedLifecycleState.Held -> {
                // SCE-MAP: nested_lifecycle.scxml:43 :: held :: _state_body
            }
            is NestedLifecycleState.Loading -> {
                // SCE-MAP: nested_lifecycle.scxml:54 :: loading :: _state_body
            }
            is NestedLifecycleState.Ready -> {
                // SCE-MAP: nested_lifecycle.scxml:58 :: ready :: _state_body
            }
            is NestedLifecycleState.Retrying -> {
                // SCE-MAP: nested_lifecycle.scxml:50 :: retrying :: _state_body
            }
            is NestedLifecycleState.Session -> {
                // SCE-MAP: nested_lifecycle.scxml:34 :: session :: _state_body
            }
            is NestedLifecycleState.Setup -> {
                // SCE-MAP: nested_lifecycle.scxml:48 :: setup :: _state_body
            }
            is NestedLifecycleState.Working -> {
                // SCE-MAP: nested_lifecycle.scxml:39 :: working :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: nested_lifecycle.scxml:27 :: _machine
    override fun executeTransitionContent(source: NestedLifecycleState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
