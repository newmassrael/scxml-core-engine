// SCE-GENERATED — DO NOT EDIT
// source-hash: 9682ba42436be01ddeb48458c1e76a0d9251bd3f706e09da9977af19a7844382

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_list.scxml:13 :: _machine

package com.sce.integration.static_list

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticListState : State {
    data object Collecting : StaticListState
    data object Done : StaticListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticListEvent : Event {
    sealed interface Day : StaticListEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticListEvent {
        data object Execution : Error
    }
    data object Full : StaticListEvent
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
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    var count: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: List<UByte>,
        val refusals: UInt,
        val count: UInt,
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
        count = count,
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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticListState): Boolean = when (state) {
        is StaticListState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticListState, HistoryId>> =
            listOf(StateTarget(StaticListState.Collecting))

        // W3C SCXML 3.13: collecting's transition 0, as the microstep reads it.
        val transitionCollectingAt0 = EnabledTransition<StaticListState, HistoryId>(
            StaticListState.Collecting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 1, as the microstep reads it.
        val transitionCollectingAt1 = EnabledTransition<StaticListState, HistoryId>(
            StaticListState.Collecting,
            listOf(StateTarget(StaticListState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: collecting's transition 2, as the microstep reads it.
        val transitionCollectingAt2 = EnabledTransition<StaticListState, HistoryId>(
            StaticListState.Collecting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 3, as the microstep reads it.
        val transitionCollectingAt3 = EnabledTransition<StaticListState, HistoryId>(
            StaticListState.Collecting,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticListState? = when (stateId) {
        "collecting" -> StaticListState.Collecting
        "done" -> StaticListState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticListState): String = when (state) {
        is StaticListState.Collecting -> "collecting"
        is StaticListState.Done -> "done"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticListState): Int = when (state) {
        is StaticListState.Collecting -> 0
        is StaticListState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticListEvent? = when (name) {
        "day.picked" -> StaticListEvent.Day.Picked
        "error.execution" -> StaticListEvent.Error.Execution
        "full" -> StaticListEvent.Full
        "reset" -> StaticListEvent.Reset
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticListEvent): String? = when (event) {
        is StaticListEvent.Day.Picked -> "day.picked"
        is StaticListEvent.Error.Execution -> "error.execution"
        is StaticListEvent.Full -> "full"
        is StaticListEvent.Reset -> "reset"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticListState,
        event: StaticListEvent?
    ): EnabledTransition<StaticListState, HistoryId>? = when (state) {
        is StaticListState.Collecting -> when {
            event is StaticListEvent.Day.Picked -> transitionCollectingAt0
            event is StaticListEvent.Full && (picked).size == 3 -> transitionCollectingAt1
            event is StaticListEvent.Reset -> transitionCollectingAt2
            event is StaticListEvent.Error.Execution -> transitionCollectingAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_list.scxml:13 :: _machine
    override fun onEntry(state: StaticListState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticListState.Collecting -> {
                // SCE-MAP: static_list.scxml:21 :: collecting :: _state_body
            }
            is StaticListState.Done -> {
                // SCE-MAP: static_list.scxml:34 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_list.scxml:13 :: _machine
    override fun onExit(state: StaticListState) {
        when (state) {
            is StaticListState.Collecting -> {
                // SCE-MAP: static_list.scxml:21 :: collecting :: _state_body
            }
            is StaticListState.Done -> {
                // SCE-MAP: static_list.scxml:34 :: done :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_list.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticListState, transitionIndex: Int) {
        when (source) {
        is StaticListState.Collecting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_list.scxml:22 :: collecting :: _transition_0
                if (pendingDayPickedPayload == null) {
                    return
                }

            if (picked.size < 3) { picked = picked + (pendingDayPickedPayload!!.dayOfMonth) } else { raisePlatformError(StaticListEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 3") }


            count = (picked).size.toUInt()
            }
            2 -> {
                // SCE-MAP: static_list.scxml:27 :: collecting :: _transition_2

            picked = emptyList()
            }
            3 -> {
                // SCE-MAP: static_list.scxml:30 :: collecting :: _transition_3


            refusals = refusals + 1.toUInt()
            }
            else -> {}
        }
        else -> {}
        }
    }
}
