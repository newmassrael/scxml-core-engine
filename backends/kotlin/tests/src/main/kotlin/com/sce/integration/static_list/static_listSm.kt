// SCE-GENERATED — DO NOT EDIT
// source-hash: 650ce2f72e4fe5a4cf966ce54a0964c73ad4617ce446385f874078c64955fa2f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_list.scxml:12 :: _machine

package com.sce.integration.static_list

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticListState : State {
    data object Collecting : StaticListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticListEvent : Event {
    sealed interface Day : StaticListEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticListEvent {
        data object Execution : Error
    }
    data object Reset : StaticListEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticListPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticListDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticListDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// --- State Machine (W3C SCXML) ---

class StaticListStateMachine(
) : StateMachineEngine<StaticListState, StaticListEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `picked` datamodel variable, published (`sce:direction="out"`). */
    var picked: List<UByte> = emptyList()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: List<UByte>,
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
        val configuration: Set<StaticListState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        picked = picked,
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
    private var pendingDayPickedPayload: StaticListDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticListEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticListDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticListEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticListDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticListEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticListDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticListState = StaticListState.Collecting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticListState? = when (stateId) {
        "collecting" -> StaticListState.Collecting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticListState): String = when (state) {
        is StaticListState.Collecting -> "collecting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StaticListState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StaticListState): Int = when (state) {
        is StaticListState.Collecting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticListEvent? = when (name) {
        "day.picked" -> StaticListEvent.Day.Picked
        "error.execution" -> StaticListEvent.Error.Execution
        "reset" -> StaticListEvent.Reset
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticListEvent): String? = when (event) {
        is StaticListEvent.Day.Picked -> "day.picked"
        is StaticListEvent.Error.Execution -> "error.execution"
        is StaticListEvent.Reset -> "reset"
    }




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StaticListState,
        event: StaticListEvent
    ): TransitionResult<StaticListState> = when (state) {
        is StaticListState.Collecting -> processCollecting(event)
    }


    // --- Per-State Event Handlers ---

    private fun processCollecting(
        event: StaticListEvent
    ): TransitionResult<StaticListState> = when {
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StaticListEvent.Day.Picked -> TransitionResult.Internal(0)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StaticListEvent.Reset -> TransitionResult.Internal(1)
        // W3C SCXML 3.13: Targetless transition (actions only)
        event is StaticListEvent.Error.Execution -> TransitionResult.Internal(2)
        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_list.scxml:12 :: _machine
    override fun onEntry(state: StaticListState, pathChild: StaticListState?) {
        when (state) {
            is StaticListState.Collecting -> {
                // SCE-MAP: static_list.scxml:19 :: collecting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("collecting")) return
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_list.scxml:12 :: _machine
    override fun onExit(state: StaticListState) {
        when (state) {
            is StaticListState.Collecting -> {
                // SCE-MAP: static_list.scxml:19 :: collecting :: _state_body
                activeStateIds.remove("collecting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: static_list.scxml:12 :: _machine
    override fun executeTransitionActions(
        source: StaticListState,
        event: StaticListEvent?,
        transitionIndex: Int
    ) {
        when (source) {
        is StaticListState.Collecting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_list.scxml:20 :: collecting :: _transition_0
                if (pendingDayPickedPayload == null) {
                    raisePlatformError(StaticListEvent.Error.Execution, "the content of a transition on 'day.picked' needs its typed payload, which this delivery did not carry")
                    return
                }

            if (picked.size < 3) { picked = picked + (pendingDayPickedPayload!!.dayOfMonth) } else { raisePlatformError(StaticListEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 3") }
            }
            1 -> {
                // SCE-MAP: static_list.scxml:23 :: collecting :: _transition_1

            picked = emptyList()
            }
            2 -> {
                // SCE-MAP: static_list.scxml:26 :: collecting :: _transition_2


            refusals = refusals + 1.toUInt()
            }
            else -> {}
        }
        }
    }
}
