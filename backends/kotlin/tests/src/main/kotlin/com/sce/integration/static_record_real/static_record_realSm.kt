// SCE-GENERATED — DO NOT EDIT
// source-hash: 0665514219a2bd1920179bb95b88947aea9f3cb1e16b583d8593e5f3d24c015b

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_real.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_real.scxml:15 :: _machine

package com.sce.integration.static_record_real

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordRealState : State {
    data object Idle : StaticRecordRealState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordRealEvent : Event {
    data object Bump : StaticRecordRealEvent
    sealed interface Reading : StaticRecordRealEvent {
        data object Taken : Reading
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordRealPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordRealReadingTakenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `reading.taken`. Consumers inject it via the `raiseReadingTaken` seam
// on the machine — they never name this class directly.
data class StaticRecordRealReadingTakenPayload(val sensor: UByte, val value: Double)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Reading` datamodel value. */
data class StaticRecordRealReadingRecord(val sensor: UByte, val value: Double) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sensor" to SavedValues.of(sensor), "value" to SavedValues.of(value))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordRealReadingRecord = StaticRecordRealReadingRecord(sensor = SavedValues.uint8(SavedValues.field(value, what, "sensor"), "$what.sensor"), value = SavedValues.float64(SavedValues.field(value, what, "value"), "$what.value"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordRealStateMachine(
) : StateMachineEngine<StaticRecordRealState, StaticRecordRealEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: StaticRecordRealReadingRecord = StaticRecordRealReadingRecord(sensor = 1.toUByte(), value = 0.5)
        private set
    /** W3C SCXML 5.2: the `sum` datamodel variable, published (`sce:direction="out"`). */
    var sum: Double = 0.0
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var sum: Double? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.sum?.let { sum = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticRecordRealReadingRecord,
        val sum: Double,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRecordRealState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = last,
        sum = sum,
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

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "0b5a0c00317d882447142f0a5b7a92abd2451fbaa917cdadf8c12dbd38a54544"

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     * Each delayed `<send>` still waiting is written as the moment it comes due
     * on the wall clock whose reading now is [wallNowMs], in milliseconds since
     * the Unix epoch.
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "last" to last.toSaved(),
            "sum" to SavedValues.of(sum),
        ),
        wallNowMs,
    )

    /** [save] at the host's wall clock now. */
    fun save(): SavedState = save(SavedState.wallClockMs())

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * The delayed sends [saved] holds are armed against this machine's `clock`,
     * which is installed before a restore as before [initialize]; [wallNowMs]
     * is what time it is on the wall clock the saved `due`s were written
     * against. A send comes due when its saved moment does, and one already due
     * comes due now.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState, wallNowMs: Long) {
        beginRestore(saved, savedShape)
        val saved1 = StaticRecordRealReadingRecord.fromSaved(saved.variable("last"), "last")
        val saved2 = SavedValues.float64(saved.variable("sum"), "sum")
        last = saved1
        sum = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingReadingTakenPayload: StaticRecordRealReadingTakenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordRealEvent, metadata: EventMetadata) {
        pendingReadingTakenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordRealReadingTakenPayload -> pendingReadingTakenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordRealEvent.Reading.Taken) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingReadingTakenPayload = StaticRecordRealReadingTakenPayload(fields.uint8("sensor"), fields.float64("value"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `reading.taken` — binds the event name and the payload field values in one call.
    fun raiseReadingTaken(sensor: UByte, value: Double) {
        send(
            StaticRecordRealEvent.Reading.Taken,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordRealReadingTakenPayload(sensor, value),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "value" to value))
            )
        )
    }


    override val initialState: StaticRecordRealState = StaticRecordRealState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordRealState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordRealState, HistoryId>> =
            listOf(StateTarget(StaticRecordRealState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticRecordRealState, HistoryId>(
            StaticRecordRealState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticRecordRealState, HistoryId>(
            StaticRecordRealState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordRealState? = when (stateId) {
        "idle" -> StaticRecordRealState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordRealState): String = when (state) {
        is StaticRecordRealState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordRealState): Int = when (state) {
        is StaticRecordRealState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordRealEvent? = when (name) {
        "bump" -> StaticRecordRealEvent.Bump
        "reading.taken" -> StaticRecordRealEvent.Reading.Taken
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordRealEvent): String? = when (event) {
        is StaticRecordRealEvent.Bump -> "bump"
        is StaticRecordRealEvent.Reading.Taken -> "reading.taken"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordRealState,
        event: StaticRecordRealEvent?
    ): EnabledTransition<StaticRecordRealState, HistoryId>? = when (state) {
        is StaticRecordRealState.Idle -> when {
            event is StaticRecordRealEvent.Bump -> transitionIdleAt0
            event is StaticRecordRealEvent.Reading.Taken -> transitionIdleAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_real.scxml:15 :: _machine
    override fun onEntry(state: StaticRecordRealState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordRealState.Idle -> {
                // SCE-MAP: static_record_real.scxml:25 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_real.scxml:15 :: _machine
    override fun onExit(state: StaticRecordRealState) {
        when (state) {
            is StaticRecordRealState.Idle -> {
                // SCE-MAP: static_record_real.scxml:25 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_real.scxml:15 :: _machine
    override fun executeTransitionContent(source: StaticRecordRealState, transitionIndex: Int) {
        when (source) {
        is StaticRecordRealState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_real.scxml:26 :: idle :: _transition_0

            last = last.copy(value = last.value * 2.0 + 1.5)

            sum = sum + last.value
            }
            1 -> {
                // SCE-MAP: static_record_real.scxml:30 :: idle :: _transition_1
                if (pendingReadingTakenPayload == null) {
                    return
                }

            last = last.copy(sensor = pendingReadingTakenPayload!!.sensor)

            last = last.copy(value = pendingReadingTakenPayload!!.value)
            }
            else -> {}
        }
        }
    }
}
