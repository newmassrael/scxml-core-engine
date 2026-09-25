// SCE-GENERATED — DO NOT EDIT
// source-hash: ca5f07e498f08e9c44fe0c543fc369f243205362af8b0e260e1a44e3bfa1bd0d

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/event_schema/statechart_bytes.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_bytes.scxml:13 :: _machine

package com.sce.integration.statechart_bytes

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartBytesState : State {
    data object Done : StatechartBytesState
    data object Waiting : StatechartBytesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartBytesEvent : Event {
    sealed interface Signal : StatechartBytesEvent {
        data object Received : Signal
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartBytesPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartBytesSignalReceivedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `signal.received`. Consumers inject it via the `raiseSignalReceived` seam
// on the machine — they never name this class directly.
data class StatechartBytesSignalReceivedPayload(val raw: ByteArray)


// --- State Machine (W3C SCXML) ---

class StatechartBytesStateMachine(
) : StateMachineEngine<StatechartBytesState, StatechartBytesEvent>() {

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingSignalReceivedPayload: StatechartBytesSignalReceivedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartBytesEvent, metadata: EventMetadata) {
        pendingSignalReceivedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartBytesSignalReceivedPayload -> pendingSignalReceivedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartBytesEvent.Signal.Received) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSignalReceivedPayload = StatechartBytesSignalReceivedPayload(fields.bytes("raw"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `signal.received` — binds the event name and the payload field values in one call.
    fun raiseSignalReceived(raw: ByteArray) {
        send(
            StatechartBytesEvent.Signal.Received,
            EventMetadata(
                type = "external",
                typedPayload = StatechartBytesSignalReceivedPayload(raw),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("raw" to raw))
            )
        )
    }


    override val initialState: StatechartBytesState = StatechartBytesState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StatechartBytesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartBytesState, HistoryId>> =
            listOf(StateTarget(StatechartBytesState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StatechartBytesState, HistoryId>(
            StatechartBytesState.Waiting,
            listOf(StateTarget(StatechartBytesState.Done)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartBytesState? = when (stateId) {
        "done" -> StatechartBytesState.Done
        "waiting" -> StatechartBytesState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartBytesState): String = when (state) {
        is StatechartBytesState.Done -> "done"
        is StatechartBytesState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartBytesState): Int = when (state) {
        is StatechartBytesState.Done -> 1
        is StatechartBytesState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartBytesEvent? = when (name) {
        "signal.received" -> StatechartBytesEvent.Signal.Received
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartBytesEvent): String? = when (event) {
        is StatechartBytesEvent.Signal.Received -> "signal.received"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartBytesState,
        event: StatechartBytesEvent?
    ): EnabledTransition<StatechartBytesState, HistoryId>? = when (state) {
        is StatechartBytesState.Waiting -> when {
            event is StatechartBytesEvent.Signal.Received && pendingSignalReceivedPayload != null && (pendingSignalReceivedPayload!!.raw.contentEquals("ack".toByteArray())) -> transitionWaitingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_bytes.scxml:13 :: _machine
    override fun onEntry(state: StatechartBytesState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartBytesState.Done -> {
                // SCE-MAP: statechart_bytes.scxml:23 :: done :: _state_body
            }
            is StatechartBytesState.Waiting -> {
                // SCE-MAP: statechart_bytes.scxml:20 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_bytes.scxml:13 :: _machine
    override fun onExit(state: StatechartBytesState) {
        when (state) {
            is StatechartBytesState.Done -> {
                // SCE-MAP: statechart_bytes.scxml:23 :: done :: _state_body
            }
            is StatechartBytesState.Waiting -> {
                // SCE-MAP: statechart_bytes.scxml:20 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_bytes.scxml:13 :: _machine
    override fun executeTransitionContent(source: StatechartBytesState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
