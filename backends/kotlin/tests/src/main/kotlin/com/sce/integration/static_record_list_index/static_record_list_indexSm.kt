// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ad55f268a9fbf4c094293a60d20a17c7e9e6598a9a6fe5787c0e9f2898a7e28

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_list_index.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_list_index.scxml:27 :: _machine

package com.sce.integration.static_record_list_index

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordListIndexState : State {
    data object Reading : StaticRecordListIndexState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordListIndexEvent : Event {
    sealed interface Day : StaticRecordListIndexEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticRecordListIndexEvent {
        data object Execution : Error
    }
    data object Last : StaticRecordListIndexEvent
    data object Next : StaticRecordListIndexEvent
    data object Order : StaticRecordListIndexEvent
    data object PickCursor : StaticRecordListIndexEvent
    data object PickFirst : StaticRecordListIndexEvent
    data object PickLast : StaticRecordListIndexEvent
    data object Total : StaticRecordListIndexEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordListIndexPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordListIndexDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticRecordListIndexDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Day` datamodel value. */
data class StaticRecordListIndexDayRecord(val year: UShort, val month: UByte, val dayOfMonth: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("year" to SavedValues.of(year), "month" to SavedValues.of(month), "dayOfMonth" to SavedValues.of(dayOfMonth))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordListIndexDayRecord = StaticRecordListIndexDayRecord(year = SavedValues.uint16(SavedValues.field(value, what, "year"), "$what.year"), month = SavedValues.uint8(SavedValues.field(value, what, "month"), "$what.month"), dayOfMonth = SavedValues.uint8(SavedValues.field(value, what, "dayOfMonth"), "$what.dayOfMonth"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordListIndexStateMachine(
) : StateMachineEngine<StaticRecordListIndexState, StaticRecordListIndexEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `draft` datamodel variable, the machine's own. */
    private var draft: StaticRecordListIndexDayRecord = StaticRecordListIndexDayRecord(year = 2026.toUShort(), month = 1.toUByte(), dayOfMonth = 1.toUByte())
    /** W3C SCXML 5.2: the `picked` datamodel variable, published (`sce:direction="out"`). */
    var picked: StaticRecordListIndexDayRecord = StaticRecordListIndexDayRecord(year = 2000.toUShort(), month = 1.toUByte(), dayOfMonth = 1.toUByte())
        private set
    /** W3C SCXML 5.2: the `days` datamodel variable, the machine's own. */
    private var days: List<StaticRecordListIndexDayRecord> = emptyList()
    /** W3C SCXML 5.2: the `cursor` datamodel variable, published (`sce:direction="out"`). */
    var cursor: Int = 0
        private set
    /** W3C SCXML 5.2: the `day` datamodel variable, published (`sce:direction="out"`). */
    var day: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `year` datamodel variable, published (`sce:direction="out"`). */
    var year: UShort = 0.toUShort()
        private set
    /** W3C SCXML 5.2: the `sum` datamodel variable, published (`sce:direction="out"`). */
    var sum: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `ordered` datamodel variable, published (`sce:direction="out"`). */
    var ordered: Boolean = false
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var cursor: Int? = null
        var day: UByte? = null
        var year: UShort? = null
        var sum: UInt? = null
        var ordered: Boolean? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.cursor?.let { cursor = it }
        params.day?.let { day = it }
        params.year?.let { year = it }
        params.sum?.let { sum = it }
        params.ordered?.let { ordered = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: StaticRecordListIndexDayRecord,
        val cursor: Int,
        val day: UByte,
        val year: UShort,
        val sum: UInt,
        val ordered: Boolean,
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
        val configuration: Set<StaticRecordListIndexState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        picked = picked,
        cursor = cursor,
        day = day,
        year = year,
        sum = sum,
        ordered = ordered,
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

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "61da1e53bacb07a726bb31390dd73254464024a8045abcda447f2ae9c0a2c9d8"

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
            "draft" to draft.toSaved(),
            "picked" to picked.toSaved(),
            "days" to SavedValues.list(days) { it.toSaved() },
            "cursor" to SavedValues.of(cursor),
            "day" to SavedValues.of(day),
            "year" to SavedValues.of(year),
            "sum" to SavedValues.of(sum),
            "ordered" to SavedValues.of(ordered),
            "refusals" to SavedValues.of(refusals),
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
        val saved1 = StaticRecordListIndexDayRecord.fromSaved(saved.variable("draft"), "draft")
        val saved2 = StaticRecordListIndexDayRecord.fromSaved(saved.variable("picked"), "picked")
        val saved3 = SavedValues.list(saved.variable("days"), "days", 3) { e, w -> StaticRecordListIndexDayRecord.fromSaved(e, w) }
        val saved4 = SavedValues.int32(saved.variable("cursor"), "cursor")
        val saved5 = SavedValues.uint8(saved.variable("day"), "day")
        val saved6 = SavedValues.uint16(saved.variable("year"), "year")
        val saved7 = SavedValues.uint32(saved.variable("sum"), "sum")
        val saved8 = SavedValues.bool(saved.variable("ordered"), "ordered")
        val saved9 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        draft = saved1
        picked = saved2
        days = saved3
        cursor = saved4
        day = saved5
        year = saved6
        sum = saved7
        ordered = saved8
        refusals = saved9
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticRecordListIndexDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordListIndexEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordListIndexDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordListIndexEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticRecordListIndexDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticRecordListIndexEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordListIndexDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticRecordListIndexState = StaticRecordListIndexState.Reading

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordListIndexState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordListIndexState, HistoryId>> =
            listOf(StateTarget(StaticRecordListIndexState.Reading))

        // W3C SCXML 3.13: reading's transition 0, as the microstep reads it.
        val transitionReadingAt0 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 1, as the microstep reads it.
        val transitionReadingAt1 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 2, as the microstep reads it.
        val transitionReadingAt2 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 3, as the microstep reads it.
        val transitionReadingAt3 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 4, as the microstep reads it.
        val transitionReadingAt4 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 5, as the microstep reads it.
        val transitionReadingAt5 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 6, as the microstep reads it.
        val transitionReadingAt6 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 7, as the microstep reads it.
        val transitionReadingAt7 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 8, as the microstep reads it.
        val transitionReadingAt8 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 9, as the microstep reads it.
        val transitionReadingAt9 = EnabledTransition<StaticRecordListIndexState, HistoryId>(
            StaticRecordListIndexState.Reading,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordListIndexState? = when (stateId) {
        "reading" -> StaticRecordListIndexState.Reading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordListIndexState): String = when (state) {
        is StaticRecordListIndexState.Reading -> "reading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordListIndexState): Int = when (state) {
        is StaticRecordListIndexState.Reading -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordListIndexEvent? = when (name) {
        "day.picked" -> StaticRecordListIndexEvent.Day.Picked
        "error.execution" -> StaticRecordListIndexEvent.Error.Execution
        "last" -> StaticRecordListIndexEvent.Last
        "next" -> StaticRecordListIndexEvent.Next
        "order" -> StaticRecordListIndexEvent.Order
        "pick_cursor" -> StaticRecordListIndexEvent.PickCursor
        "pick_first" -> StaticRecordListIndexEvent.PickFirst
        "pick_last" -> StaticRecordListIndexEvent.PickLast
        "total" -> StaticRecordListIndexEvent.Total
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordListIndexEvent): String? = when (event) {
        is StaticRecordListIndexEvent.Day.Picked -> "day.picked"
        is StaticRecordListIndexEvent.Error.Execution -> "error.execution"
        is StaticRecordListIndexEvent.Last -> "last"
        is StaticRecordListIndexEvent.Next -> "next"
        is StaticRecordListIndexEvent.Order -> "order"
        is StaticRecordListIndexEvent.PickCursor -> "pick_cursor"
        is StaticRecordListIndexEvent.PickFirst -> "pick_first"
        is StaticRecordListIndexEvent.PickLast -> "pick_last"
        is StaticRecordListIndexEvent.Total -> "total"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordListIndexState,
        event: StaticRecordListIndexEvent?
    ): EnabledTransition<StaticRecordListIndexState, HistoryId>? = when (state) {
        is StaticRecordListIndexState.Reading -> when {
            event is StaticRecordListIndexEvent.Day.Picked -> transitionReadingAt0
            event is StaticRecordListIndexEvent.Next -> transitionReadingAt1
            event is StaticRecordListIndexEvent.Last -> transitionReadingAt2
            event is StaticRecordListIndexEvent.PickFirst -> transitionReadingAt3
            event is StaticRecordListIndexEvent.PickLast -> transitionReadingAt4
            event is StaticRecordListIndexEvent.PickCursor -> transitionReadingAt5
            event is StaticRecordListIndexEvent.Total -> transitionReadingAt6
            event is StaticRecordListIndexEvent.Order && (try { com.sce.forge.runtime.SceChecked.at(days, (0).toLong()).dayOfMonth < com.sce.forge.runtime.SceChecked.at(days, (1).toLong()).dayOfMonth } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<transition cond='days[0].dayOfMonth < days[1].dayOfMonth'>: an integer operation overflowed or failed"); false }) -> transitionReadingAt7
            event is StaticRecordListIndexEvent.Order -> transitionReadingAt8
            event is StaticRecordListIndexEvent.Error.Execution -> transitionReadingAt9
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_list_index.scxml:27 :: _machine
    override fun onEntry(state: StaticRecordListIndexState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordListIndexState.Reading -> {
                // SCE-MAP: static_record_list_index.scxml:49 :: reading :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_list_index.scxml:27 :: _machine
    override fun onExit(state: StaticRecordListIndexState) {
        when (state) {
            is StaticRecordListIndexState.Reading -> {
                // SCE-MAP: static_record_list_index.scxml:49 :: reading :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_list_index.scxml:27 :: _machine
    override fun executeTransitionContent(source: StaticRecordListIndexState, transitionIndex: Int) {
        when (source) {
        is StaticRecordListIndexState.Reading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_list_index.scxml:50 :: reading :: _transition_0
                if (pendingDayPickedPayload == null) {
                    return
                }

            draft = draft.copy(year = pendingDayPickedPayload!!.year)

            draft = draft.copy(month = pendingDayPickedPayload!!.month)

            draft = draft.copy(dayOfMonth = pendingDayPickedPayload!!.dayOfMonth)

            if (if (days.size < 3) { days = days + (draft); false } else { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<sce:append target='days'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_record_list_index.scxml:56 :: reading :: _transition_1

            if (try { day = com.sce.forge.runtime.SceChecked.at(days, (cursor).toLong()).dayOfMonth; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='day'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { year = com.sce.forge.runtime.SceChecked.at(days, (cursor).toLong()).year; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='year'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { cursor = com.sce.forge.runtime.SceChecked.add(cursor, 1); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='cursor'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_record_list_index.scxml:61 :: reading :: _transition_2

            if (try { day = com.sce.forge.runtime.SceChecked.at(days, (com.sce.forge.runtime.SceChecked.sub((days).size.toUInt(), 1.toUInt())).toLong()).dayOfMonth; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='day'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_record_list_index.scxml:64 :: reading :: _transition_3

            if ((try { picked = picked.copy(year = com.sce.forge.runtime.SceChecked.at(days, (0).toLong()).year); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.year'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(month = com.sce.forge.runtime.SceChecked.at(days, (0).toLong()).month); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.month'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(dayOfMonth = com.sce.forge.runtime.SceChecked.at(days, (0).toLong()).dayOfMonth); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.dayOfMonth'>: an integer operation overflowed or failed"); true })) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_record_list_index.scxml:67 :: reading :: _transition_4

            if ((try { picked = picked.copy(year = com.sce.forge.runtime.SceChecked.at(days, (com.sce.forge.runtime.SceChecked.sub((days).size.toUInt(), 1.toUInt())).toLong()).year); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.year'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(month = com.sce.forge.runtime.SceChecked.at(days, (com.sce.forge.runtime.SceChecked.sub((days).size.toUInt(), 1.toUInt())).toLong()).month); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.month'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(dayOfMonth = com.sce.forge.runtime.SceChecked.at(days, (com.sce.forge.runtime.SceChecked.sub((days).size.toUInt(), 1.toUInt())).toLong()).dayOfMonth); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.dayOfMonth'>: an integer operation overflowed or failed"); true })) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_record_list_index.scxml:70 :: reading :: _transition_5

            if ((try { picked = picked.copy(year = com.sce.forge.runtime.SceChecked.at(days, (cursor).toLong()).year); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.year'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(month = com.sce.forge.runtime.SceChecked.at(days, (cursor).toLong()).month); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.month'>: an integer operation overflowed or failed"); true }) || (try { picked = picked.copy(dayOfMonth = com.sce.forge.runtime.SceChecked.at(days, (cursor).toLong()).dayOfMonth); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='picked.dayOfMonth'>: an integer operation overflowed or failed"); true })) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_record_list_index.scxml:73 :: reading :: _transition_6

            if (try { sum = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.at(days, (0).toLong()).dayOfMonth, com.sce.forge.runtime.SceChecked.at(days, (1).toLong()).dayOfMonth).toUInt(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_record_list_index.scxml:76 :: reading :: _transition_7

            ordered = true
            }
            8 -> {
                // SCE-MAP: static_record_list_index.scxml:79 :: reading :: _transition_8

            ordered = false
            }
            9 -> {
                // SCE-MAP: static_record_list_index.scxml:82 :: reading :: _transition_9

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListIndexEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
