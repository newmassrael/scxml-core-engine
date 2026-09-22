// SCE-GENERATED — DO NOT EDIT
// source-hash: ca5f07e498f08e9c44fe0c543fc369f243205362af8b0e260e1a44e3bfa1bd0d
// template-hash: 583c0d21905ba9b04748a3d6c74f6cc9796a6884a4c31fd1817c64ee900ca1d2
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/event_schema/statechart_minimal.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_minimal.scxml:8 :: _machine

package com.sce.integration.statechart_minimal

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartMinimalState : State {
    data object Done : StatechartMinimalState
    data object Waiting : StatechartMinimalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartMinimalEvent : Event {
    sealed interface Job : StatechartMinimalEvent {
        data object Completed : Job
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartMinimalPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartMinimalJobCompletedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `job.completed`. Consumers inject it via the `raiseJobCompleted` seam
// on the machine — they never name this class directly.
data class StatechartMinimalJobCompletedPayload(val elapsed_ms: UInt)


// --- State Machine (W3C SCXML) ---

class StatechartMinimalStateMachine(
) : StateMachineEngine<StatechartMinimalState, StatechartMinimalEvent>() {

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingJobCompletedPayload: StatechartMinimalJobCompletedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartMinimalEvent, metadata: EventMetadata) {
        pendingJobCompletedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartMinimalJobCompletedPayload -> pendingJobCompletedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartMinimalEvent.Job.Completed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingJobCompletedPayload = StatechartMinimalJobCompletedPayload(fields.uint32("elapsed_ms"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `job.completed` — binds the event name and the payload field values in one call.
    fun raiseJobCompleted(elapsed_ms: UInt) {
        send(
            StatechartMinimalEvent.Job.Completed,
            EventMetadata(
                type = "external",
                typedPayload = StatechartMinimalJobCompletedPayload(elapsed_ms),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("elapsed_ms" to elapsed_ms))
            )
        )
    }


    override val initialState: StatechartMinimalState = StatechartMinimalState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartMinimalState? = when (stateId) {
        "done" -> StatechartMinimalState.Done
        "waiting" -> StatechartMinimalState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartMinimalState): String = when (state) {
        is StatechartMinimalState.Done -> "done"
        is StatechartMinimalState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartMinimalState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StatechartMinimalState): Int = when (state) {
        is StatechartMinimalState.Done -> 1
        is StatechartMinimalState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartMinimalEvent? = when (name) {
        "job.completed" -> StatechartMinimalEvent.Job.Completed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartMinimalEvent): String? = when (event) {
        is StatechartMinimalEvent.Job.Completed -> "job.completed"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StatechartMinimalState,
        event: StatechartMinimalEvent
    ): TransitionResult<StatechartMinimalState> = when (state) {
        is StatechartMinimalState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWaiting(
        event: StatechartMinimalEvent
    ): TransitionResult<StatechartMinimalState> = when {
        event is StatechartMinimalEvent.Job.Completed && pendingJobCompletedPayload != null && (pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt()) -> TransitionResult.External(StatechartMinimalState.Done, StatechartMinimalState.Waiting, 0)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun onEntry(state: StatechartMinimalState, pathChild: StatechartMinimalState?) {
        when (state) {
            is StatechartMinimalState.Done -> {
                // SCE-MAP: statechart_minimal.scxml:18 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
            }
            is StatechartMinimalState.Waiting -> {
                // SCE-MAP: statechart_minimal.scxml:15 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun onExit(state: StatechartMinimalState) {
        when (state) {
            is StatechartMinimalState.Done -> {
                // SCE-MAP: statechart_minimal.scxml:18 :: done :: _state_body
                activeStateIds.remove("done")
            }
            is StatechartMinimalState.Waiting -> {
                // SCE-MAP: statechart_minimal.scxml:15 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun executeTransitionActions(
        source: StatechartMinimalState,
        event: StatechartMinimalEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        else -> {}
        }
    }
}
