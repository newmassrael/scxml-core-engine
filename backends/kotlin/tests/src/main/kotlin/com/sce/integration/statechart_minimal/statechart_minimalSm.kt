// SCE-GENERATED — DO NOT EDIT
// source-hash: ca5f07e498f08e9c44fe0c543fc369f243205362af8b0e260e1a44e3bfa1bd0d

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartMinimalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartMinimalState, HistoryId>> =
            listOf(StateTarget(StatechartMinimalState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StatechartMinimalState, HistoryId>(
            StatechartMinimalState.Waiting,
            listOf(StateTarget(StatechartMinimalState.Done)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
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





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartMinimalState,
        event: StatechartMinimalEvent?
    ): EnabledTransition<StatechartMinimalState, HistoryId>? = when (state) {
        is StatechartMinimalState.Waiting -> when {
            event is StatechartMinimalEvent.Job.Completed && pendingJobCompletedPayload != null && (pendingJobCompletedPayload!!.elapsed_ms == 0.toUInt()) -> transitionWaitingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun onEntry(state: StatechartMinimalState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartMinimalState.Done -> {
                // SCE-MAP: statechart_minimal.scxml:18 :: done :: _state_body
            }
            is StatechartMinimalState.Waiting -> {
                // SCE-MAP: statechart_minimal.scxml:15 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun onExit(state: StatechartMinimalState) {
        when (state) {
            is StatechartMinimalState.Done -> {
                // SCE-MAP: statechart_minimal.scxml:18 :: done :: _state_body
            }
            is StatechartMinimalState.Waiting -> {
                // SCE-MAP: statechart_minimal.scxml:15 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_minimal.scxml:8 :: _machine
    override fun executeTransitionContent(source: StatechartMinimalState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
