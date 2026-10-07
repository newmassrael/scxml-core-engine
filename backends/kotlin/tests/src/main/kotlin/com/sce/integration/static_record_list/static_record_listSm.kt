// SCE-GENERATED — DO NOT EDIT
// source-hash: ae9d5cfab95c0cc5735b6cfc7f8be270a02d917fc84091175492688fbe3b3627

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_list.scxml:16 :: _machine

package com.sce.integration.static_record_list

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordListState : State {
    data object Collecting : StaticRecordListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordListEvent : Event {
    data object Clear : StaticRecordListEvent
    data object Copy : StaticRecordListEvent
    sealed interface Day : StaticRecordListEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticRecordListEvent {
        data object Execution : Error
    }
    data object Latest : StaticRecordListEvent
    data object Reuse : StaticRecordListEvent
    data object Sum : StaticRecordListEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordListPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordListDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticRecordListDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Day` datamodel value. */
data class StaticRecordListDayRecord(val year: UShort, val month: UByte, val dayOfMonth: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("year" to SavedValues.of(year), "month" to SavedValues.of(month), "dayOfMonth" to SavedValues.of(dayOfMonth))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordListDayRecord = StaticRecordListDayRecord(year = SavedValues.uint16(SavedValues.field(value, what, "year"), "$what.year"), month = SavedValues.uint8(SavedValues.field(value, what, "month"), "$what.month"), dayOfMonth = SavedValues.uint8(SavedValues.field(value, what, "dayOfMonth"), "$what.dayOfMonth"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordListStateMachine(
) : StateMachineEngine<StaticRecordListState, StaticRecordListEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `draft` datamodel variable, published (`sce:direction="out"`). */
    var draft: StaticRecordListDayRecord = StaticRecordListDayRecord(year = 2026.toUShort(), month = 1.toUByte(), dayOfMonth = 1.toUByte())
        private set
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: StaticRecordListDayRecord = StaticRecordListDayRecord(year = 2000.toUShort(), month = 1.toUByte(), dayOfMonth = 1.toUByte())
        private set
    /** W3C SCXML 5.2: the `days` datamodel variable, published (`sce:direction="out"`). */
    var days: List<StaticRecordListDayRecord> = emptyList()
        private set
    /** W3C SCXML 5.2: the `copies` datamodel variable, published (`sce:direction="out"`). */
    var copies: List<StaticRecordListDayRecord> = emptyList()
        private set
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    var total: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var total: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.total?.let { total = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val draft: StaticRecordListDayRecord,
        val last: StaticRecordListDayRecord,
        val days: List<StaticRecordListDayRecord>,
        val copies: List<StaticRecordListDayRecord>,
        val total: UInt,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRecordListState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        draft = draft,
        last = last,
        days = days,
        copies = copies,
        total = total,
        errors = errors,
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
    val savedShape: String = "e6e67decba7328a76e16c522269b667c9df51c2beab7540118fb8a22a0ce7f0b"

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
            "last" to last.toSaved(),
            "days" to SavedValues.list(days) { it.toSaved() },
            "copies" to SavedValues.list(copies) { it.toSaved() },
            "total" to SavedValues.of(total),
            "errors" to SavedValues.of(errors),
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
        val saved1 = StaticRecordListDayRecord.fromSaved(saved.variable("draft"), "draft")
        val saved2 = StaticRecordListDayRecord.fromSaved(saved.variable("last"), "last")
        val saved3 = SavedValues.list(saved.variable("days"), "days", 3) { e, w -> StaticRecordListDayRecord.fromSaved(e, w) }
        val saved4 = SavedValues.list(saved.variable("copies"), "copies", 3) { e, w -> StaticRecordListDayRecord.fromSaved(e, w) }
        val saved5 = SavedValues.uint32(saved.variable("total"), "total")
        val saved6 = SavedValues.uint32(saved.variable("errors"), "errors")
        draft = saved1
        last = saved2
        days = saved3
        copies = saved4
        total = saved5
        errors = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticRecordListDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordListEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordListDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordListEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticRecordListDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticRecordListEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordListDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticRecordListState = StaticRecordListState.Collecting

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordListState, HistoryId>> =
            listOf(StateTarget(StaticRecordListState.Collecting))

        // W3C SCXML 3.13: collecting's transition 0, as the microstep reads it.
        val transitionCollectingAt0 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 1, as the microstep reads it.
        val transitionCollectingAt1 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 2, as the microstep reads it.
        val transitionCollectingAt2 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 3, as the microstep reads it.
        val transitionCollectingAt3 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 4, as the microstep reads it.
        val transitionCollectingAt4 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 5, as the microstep reads it.
        val transitionCollectingAt5 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: collecting's transition 6, as the microstep reads it.
        val transitionCollectingAt6 = EnabledTransition<StaticRecordListState, HistoryId>(
            StaticRecordListState.Collecting,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordListState? = when (stateId) {
        "collecting" -> StaticRecordListState.Collecting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordListState): String = when (state) {
        is StaticRecordListState.Collecting -> "collecting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordListState): Int = when (state) {
        is StaticRecordListState.Collecting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordListEvent? = when (name) {
        "clear" -> StaticRecordListEvent.Clear
        "copy" -> StaticRecordListEvent.Copy
        "day.picked" -> StaticRecordListEvent.Day.Picked
        "error.execution" -> StaticRecordListEvent.Error.Execution
        "latest" -> StaticRecordListEvent.Latest
        "reuse" -> StaticRecordListEvent.Reuse
        "sum" -> StaticRecordListEvent.Sum
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordListEvent): String? = when (event) {
        is StaticRecordListEvent.Clear -> "clear"
        is StaticRecordListEvent.Copy -> "copy"
        is StaticRecordListEvent.Day.Picked -> "day.picked"
        is StaticRecordListEvent.Error.Execution -> "error.execution"
        is StaticRecordListEvent.Latest -> "latest"
        is StaticRecordListEvent.Reuse -> "reuse"
        is StaticRecordListEvent.Sum -> "sum"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordListState,
        event: StaticRecordListEvent?
    ): EnabledTransition<StaticRecordListState, HistoryId>? = when (state) {
        is StaticRecordListState.Collecting -> when {
            event is StaticRecordListEvent.Day.Picked -> transitionCollectingAt0
            event is StaticRecordListEvent.Sum -> transitionCollectingAt1
            event is StaticRecordListEvent.Copy -> transitionCollectingAt2
            event is StaticRecordListEvent.Latest -> transitionCollectingAt3
            event is StaticRecordListEvent.Reuse -> transitionCollectingAt4
            event is StaticRecordListEvent.Clear -> transitionCollectingAt5
            event is StaticRecordListEvent.Error.Execution -> transitionCollectingAt6
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_list.scxml:16 :: _machine
    override fun onEntry(state: StaticRecordListState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordListState.Collecting -> {
                // SCE-MAP: static_record_list.scxml:35 :: collecting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_list.scxml:16 :: _machine
    override fun onExit(state: StaticRecordListState) {
        when (state) {
            is StaticRecordListState.Collecting -> {
                // SCE-MAP: static_record_list.scxml:35 :: collecting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_list.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticRecordListState, transitionIndex: Int) {
        when (source) {
        is StaticRecordListState.Collecting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_list.scxml:38 :: collecting :: _transition_0
                if (pendingDayPickedPayload == null) {
                    return
                }

            draft = draft.copy(year = pendingDayPickedPayload!!.year)

            draft = draft.copy(month = pendingDayPickedPayload!!.month)

            draft = draft.copy(dayOfMonth = pendingDayPickedPayload!!.dayOfMonth)

            if (if (days.size < 3) { days = days + (draft); false } else { raisePlatformError(StaticRecordListEvent.Error.Execution, "<sce:append target='days'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_record_list.scxml:45 :: collecting :: _transition_1

            total = 0.toUInt()


            for (d in days) {

            if (try { total = com.sce.forge.runtime.SceChecked.add(total, d.dayOfMonth.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            2 -> {
                // SCE-MAP: static_record_list.scxml:52 :: collecting :: _transition_2

            copies = emptyList()


            for ((sce_position_of_i, d) in days.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()

            if (if (copies.size < 3) { copies = copies + (d); false } else { raisePlatformError(StaticRecordListEvent.Error.Execution, "<sce:append target='copies'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            }
            3 -> {
                // SCE-MAP: static_record_list.scxml:60 :: collecting :: _transition_3


            for (d in days) {

            last = d
            }
            }
            4 -> {
                // SCE-MAP: static_record_list.scxml:66 :: collecting :: _transition_4

            draft = last
            }
            5 -> {
                // SCE-MAP: static_record_list.scxml:69 :: collecting :: _transition_5

            days = emptyList()
            }
            6 -> {
                // SCE-MAP: static_record_list.scxml:72 :: collecting :: _transition_6

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordListEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
