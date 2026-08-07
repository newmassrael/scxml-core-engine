// SCE-GENERATED — DO NOT EDIT
// source-hash: 0c53513bedc7a89c1f25c346bee5d167d30d4c794497283b17bfc7211b2b267d
// template-hash: c56c70704f5f2bb35214b9afedb17072586ef770e8f707a00c3cc931286b4b25
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/event_schema/statechart_bytes.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_bytes.scxml:13

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

    // NL→IR Item C1 Path A: lift the dequeued event's type-erased typed payload
    // into the matching nullable field (a non-typed carrier resets all to null,
    // so every typed guard fails). Twin of the Go policy's PopulateEventMetadata
    // type-switch / the C11 pop loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(metadata: EventMetadata) {
        pendingSignalReceivedPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartBytesSignalReceivedPayload -> pendingSignalReceivedPayload = tp
            else -> {}
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `signal.received` — binds the event name and the payload field values in one call.
    fun raiseSignalReceived(raw: ByteArray) {
        send(
            StatechartBytesEvent.Signal.Received,
            EventMetadata(type = "external", typedPayload = StatechartBytesSignalReceivedPayload(raw))
        )
    }


    override val initialState: StatechartBytesState = StatechartBytesState.Waiting



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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartBytesState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StatechartBytesState): Int = when (state) {
        is StatechartBytesState.Done -> 1
        is StatechartBytesState.Waiting -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StatechartBytesState,
        event: StatechartBytesEvent
    ): TransitionResult<StatechartBytesState> = when (state) {
        is StatechartBytesState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processWaiting(
        event: StatechartBytesEvent
    ): TransitionResult<StatechartBytesState> = when {
        event is StatechartBytesEvent.Signal.Received && pendingSignalReceivedPayload != null && (pendingSignalReceivedPayload!!.raw.contentEquals("ack".toByteArray())) -> TransitionResult.External(StatechartBytesState.Done, StatechartBytesState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_bytes.scxml:13
    override fun onEntry(state: StatechartBytesState) {
        when (state) {
            is StatechartBytesState.Done -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
            }
            is StatechartBytesState.Waiting -> {
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_bytes.scxml:13
    override fun onExit(state: StatechartBytesState) {
        when (state) {
            is StatechartBytesState.Done -> {
                activeStateIds.remove("done")
            }
            is StatechartBytesState.Waiting -> {
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_bytes.scxml:13
    override fun executeTransitionActions(
        source: StatechartBytesState,
        event: StatechartBytesEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
