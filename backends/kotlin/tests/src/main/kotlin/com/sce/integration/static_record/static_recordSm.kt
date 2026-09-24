// SCE-GENERATED — DO NOT EDIT
// source-hash: 650ce2f72e4fe5a4cf966ce54a0964c73ad4617ce446385f874078c64955fa2f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record.scxml:12 :: _machine

package com.sce.integration.static_record

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordState : State {
    data object Showing : StaticRecordState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordEvent : Event {
    sealed interface Day : StaticRecordEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticRecordEvent {
        data object Execution : Error
    }
    data object Next : StaticRecordEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticRecordDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// ── SCE Accepted Subset §2.15: sce-static record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Day` datamodel value. */
data class StaticRecordDayRecord(val year: UShort, val month: UByte, val dayOfMonth: UByte)
// --- State Machine (W3C SCXML) ---

class StaticRecordStateMachine(
) : StateMachineEngine<StaticRecordState, StaticRecordEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticRecordDayRecord = StaticRecordDayRecord(year = 2026.toUShort(), month = 9.toUByte(), dayOfMonth = 24.toUByte())
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticRecordDayRecord,
        val refusals: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRecordState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
        refusals = refusals,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), currentData(), false)
    )

    /**
     * The machine as the host sees it: one [Snapshot] per completed macrostep
     * (W3C SCXML Appendix D), never a state between two microsteps.
     *
     * Compose integration: `val s by sm.snapshot.collectAsState()`
     */
    val snapshot: kotlinx.coroutines.flow.StateFlow<Snapshot>
        get() = _snapshot

    override fun onMacrostepComplete(truncated: Boolean) {
        _snapshot.value = Snapshot(activeConfiguration, currentData(), truncated)
    }

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticRecordDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticRecordDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticRecordEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticRecordState = StaticRecordState.Showing

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordState? = when (stateId) {
        "showing" -> StaticRecordState.Showing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordState): String = when (state) {
        is StaticRecordState.Showing -> "showing"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StaticRecordState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StaticRecordState): Int = when (state) {
        is StaticRecordState.Showing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordEvent? = when (name) {
        "day.picked" -> StaticRecordEvent.Day.Picked
        "error.execution" -> StaticRecordEvent.Error.Execution
        "next" -> StaticRecordEvent.Next
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordEvent): String? = when (event) {
        is StaticRecordEvent.Day.Picked -> "day.picked"
        is StaticRecordEvent.Error.Execution -> "error.execution"
        is StaticRecordEvent.Next -> "next"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StaticRecordState,
        event: StaticRecordEvent
    ): TransitionResult<StaticRecordState> = when (state) {
        is StaticRecordState.Showing -> processShowing(event)
    }


    // --- Per-State Event Handlers ---

    private fun processShowing(
        event: StaticRecordEvent
    ): TransitionResult<StaticRecordState> = when {
        event is StaticRecordEvent.Next && shown.dayOfMonth < 28.toUByte() -> TransitionResult.Internal(0)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StaticRecordEvent.Day.Picked -> TransitionResult.Internal(1)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StaticRecordEvent.Error.Execution -> TransitionResult.Internal(2)
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record.scxml:12 :: _machine
    override fun onEntry(state: StaticRecordState, pathChild: StaticRecordState?) {
        when (state) {
            is StaticRecordState.Showing -> {
                // SCE-MAP: static_record.scxml:23 :: showing :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("showing")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record.scxml:12 :: _machine
    override fun onExit(state: StaticRecordState) {
        when (state) {
            is StaticRecordState.Showing -> {
                // SCE-MAP: static_record.scxml:23 :: showing :: _state_body
                activeStateIds.remove("showing")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: static_record.scxml:12 :: _machine
    override fun executeTransitionActions(
        source: StaticRecordState,
        event: StaticRecordEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is StaticRecordState.Showing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record.scxml:24 :: showing :: _transition_0


            shown = shown.copy(dayOfMonth = (shown.dayOfMonth.toUInt() + 1.toUInt()).toUByte())
            }
            1 -> {
                // SCE-MAP: static_record.scxml:27 :: showing :: _transition_1
                if (pendingDayPickedPayload == null) {
                    raisePlatformError(StaticRecordEvent.Error.Execution, "the content of a transition on 'day.picked' needs its typed payload, which this delivery did not carry")
                    return
                }


            shown = shown.copy(year = pendingDayPickedPayload!!.year)


            shown = shown.copy(month = pendingDayPickedPayload!!.month)


            shown = shown.copy(dayOfMonth = pendingDayPickedPayload!!.dayOfMonth)
            }
            2 -> {
                // SCE-MAP: static_record.scxml:34 :: showing :: _transition_2


            refusals = refusals + 1.toUInt()
            }
            else -> {}
        }
        }
    }
}
