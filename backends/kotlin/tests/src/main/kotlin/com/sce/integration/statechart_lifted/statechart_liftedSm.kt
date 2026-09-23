// SCE-GENERATED — DO NOT EDIT
// source-hash: ca5f07e498f08e9c44fe0c543fc369f243205362af8b0e260e1a44e3bfa1bd0d

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/event_schema/statechart_lifted.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_lifted.scxml:23 :: _machine

package com.sce.integration.statechart_lifted

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartLiftedState : State {
    data object Done : StatechartLiftedState
    data object Refused : StatechartLiftedState
    data object Waiting : StatechartLiftedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartLiftedEvent : Event {
    sealed interface Error : StatechartLiftedEvent {
        data object Execution : Error
    }
    sealed interface Job : StatechartLiftedEvent {
        data object Completed : Job
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartLiftedPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartLiftedJobCompletedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.completed`. Consumers inject it via the `raiseJobCompleted` seam
// on the machine — they never name this class directly.
data class StatechartLiftedJobCompletedPayload(val elapsed_ms: UInt)


// --- State Machine (W3C SCXML) ---

class StatechartLiftedStateMachine(
) : StateMachineEngine<StatechartLiftedState, StatechartLiftedEvent>() {

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingJobCompletedPayload: StatechartLiftedJobCompletedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartLiftedEvent, metadata: EventMetadata) {
        pendingJobCompletedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartLiftedJobCompletedPayload -> pendingJobCompletedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartLiftedEvent.Job.Completed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobCompletedPayload = StatechartLiftedJobCompletedPayload(fields.uint32("elapsed_ms"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.completed` — binds the event name and the payload field values in one call.
    fun raiseJobCompleted(elapsed_ms: UInt) {
        send(
            StatechartLiftedEvent.Job.Completed,
            EventMetadata(
                type = "external",
                typedPayload = StatechartLiftedJobCompletedPayload(elapsed_ms),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("elapsed_ms" to elapsed_ms))
            )
        )
    }


    override val initialState: StatechartLiftedState = StatechartLiftedState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartLiftedState? = when (stateId) {
        "done" -> StatechartLiftedState.Done
        "refused" -> StatechartLiftedState.Refused
        "waiting" -> StatechartLiftedState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartLiftedState): String = when (state) {
        is StatechartLiftedState.Done -> "done"
        is StatechartLiftedState.Refused -> "refused"
        is StatechartLiftedState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartLiftedState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StatechartLiftedState): Int = when (state) {
        is StatechartLiftedState.Done -> 1
        is StatechartLiftedState.Refused -> 2
        is StatechartLiftedState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartLiftedEvent? = when (name) {
        "error.execution" -> StatechartLiftedEvent.Error.Execution
        "job.completed" -> StatechartLiftedEvent.Job.Completed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartLiftedEvent): String? = when (event) {
        is StatechartLiftedEvent.Error.Execution -> "error.execution"
        is StatechartLiftedEvent.Job.Completed -> "job.completed"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StatechartLiftedState,
        event: StatechartLiftedEvent
    ): TransitionResult<StatechartLiftedState> = when (state) {
        is StatechartLiftedState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWaiting(
        event: StatechartLiftedEvent
    ): TransitionResult<StatechartLiftedState> = when {
        event is StatechartLiftedEvent.Error.Execution -> TransitionResult.External(StatechartLiftedState.Refused, StatechartLiftedState.Waiting, 0)

        event is StatechartLiftedEvent.Job.Completed && pendingJobCompletedPayload != null && (pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt()) -> TransitionResult.External(StatechartLiftedState.Done, StatechartLiftedState.Waiting, 1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_lifted.scxml:23 :: _machine
    override fun onEntry(state: StatechartLiftedState, pathChild: StatechartLiftedState?) {
        when (state) {
            is StatechartLiftedState.Done -> {
                // SCE-MAP: statechart_lifted.scxml:34 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
            }
            is StatechartLiftedState.Refused -> {
                // SCE-MAP: statechart_lifted.scxml:35 :: refused :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("refused")) return
            }
            is StatechartLiftedState.Waiting -> {
                // SCE-MAP: statechart_lifted.scxml:30 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_lifted.scxml:23 :: _machine
    override fun onExit(state: StatechartLiftedState) {
        when (state) {
            is StatechartLiftedState.Done -> {
                // SCE-MAP: statechart_lifted.scxml:34 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is StatechartLiftedState.Refused -> {
                // SCE-MAP: statechart_lifted.scxml:35 :: refused :: _state_body
                activeStateIds.remove("refused")
            }
            is StatechartLiftedState.Waiting -> {
                // SCE-MAP: statechart_lifted.scxml:30 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_lifted.scxml:23 :: _machine
    override fun executeTransitionActions(
        source: StatechartLiftedState,
        event: StatechartLiftedEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
