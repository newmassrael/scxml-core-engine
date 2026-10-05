// SCE-GENERATED — DO NOT EDIT
// source-hash: 8153420d7cf0af90d3fcf1a0988689d0a2d9a01bb414f5c89e80914a42bd4533

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_fields.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_fields.scxml:14 :: _machine

package com.sce.integration.static_record_fields

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordFieldsState : State {
    data object Showing : StaticRecordFieldsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordFieldsEvent : Event {
    sealed interface Day : StaticRecordFieldsEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticRecordFieldsEvent {
        data object Execution : Error
    }
    data object Next : StaticRecordFieldsEvent
    data object Rewind : StaticRecordFieldsEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordFieldsPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordFieldsDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticRecordFieldsDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Day` datamodel value. */
data class StaticRecordFieldsDayRecord(val year: UShort, val month: UByte, val dayOfMonth: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("year" to SavedValues.of(year), "month" to SavedValues.of(month), "dayOfMonth" to SavedValues.of(dayOfMonth))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordFieldsDayRecord = StaticRecordFieldsDayRecord(year = SavedValues.uint16(SavedValues.field(value, what, "year"), "$what.year"), month = SavedValues.uint8(SavedValues.field(value, what, "month"), "$what.month"), dayOfMonth = SavedValues.uint8(SavedValues.field(value, what, "dayOfMonth"), "$what.dayOfMonth"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordFieldsStateMachine(
) : StateMachineEngine<StaticRecordFieldsState, StaticRecordFieldsEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticRecordFieldsDayRecord = StaticRecordFieldsDayRecord(year = 2026.toUShort(), month = 9.toUByte(), dayOfMonth = 24.toUByte())
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
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticRecordFieldsDayRecord,
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
        val configuration: Set<StaticRecordFieldsState>,
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

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "23e94bbee0f8862f1ad51a0f614fe3df492096ccaf6676d9eb8cd371639f9fba"

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
            "shown" to shown.toSaved(),
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
        val saved1 = StaticRecordFieldsDayRecord.fromSaved(saved.variable("shown"), "shown")
        val saved2 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        shown = saved1
        refusals = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticRecordFieldsDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordFieldsEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordFieldsDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordFieldsEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticRecordFieldsDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticRecordFieldsEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordFieldsDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticRecordFieldsState = StaticRecordFieldsState.Showing

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordFieldsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordFieldsState, HistoryId>> =
            listOf(StateTarget(StaticRecordFieldsState.Showing))

        // W3C SCXML 3.13: showing's transition 0, as the microstep reads it.
        val transitionShowingAt0 = EnabledTransition<StaticRecordFieldsState, HistoryId>(
            StaticRecordFieldsState.Showing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 1, as the microstep reads it.
        val transitionShowingAt1 = EnabledTransition<StaticRecordFieldsState, HistoryId>(
            StaticRecordFieldsState.Showing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 2, as the microstep reads it.
        val transitionShowingAt2 = EnabledTransition<StaticRecordFieldsState, HistoryId>(
            StaticRecordFieldsState.Showing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 3, as the microstep reads it.
        val transitionShowingAt3 = EnabledTransition<StaticRecordFieldsState, HistoryId>(
            StaticRecordFieldsState.Showing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordFieldsState? = when (stateId) {
        "showing" -> StaticRecordFieldsState.Showing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordFieldsState): String = when (state) {
        is StaticRecordFieldsState.Showing -> "showing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordFieldsState): Int = when (state) {
        is StaticRecordFieldsState.Showing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordFieldsEvent? = when (name) {
        "day.picked" -> StaticRecordFieldsEvent.Day.Picked
        "error.execution" -> StaticRecordFieldsEvent.Error.Execution
        "next" -> StaticRecordFieldsEvent.Next
        "rewind" -> StaticRecordFieldsEvent.Rewind
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordFieldsEvent): String? = when (event) {
        is StaticRecordFieldsEvent.Day.Picked -> "day.picked"
        is StaticRecordFieldsEvent.Error.Execution -> "error.execution"
        is StaticRecordFieldsEvent.Next -> "next"
        is StaticRecordFieldsEvent.Rewind -> "rewind"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordFieldsState,
        event: StaticRecordFieldsEvent?
    ): EnabledTransition<StaticRecordFieldsState, HistoryId>? = when (state) {
        is StaticRecordFieldsState.Showing -> when {
            event is StaticRecordFieldsEvent.Next && shown.dayOfMonth < 28.toUByte() -> transitionShowingAt0
            event is StaticRecordFieldsEvent.Day.Picked -> transitionShowingAt1
            event is StaticRecordFieldsEvent.Rewind -> transitionShowingAt2
            event is StaticRecordFieldsEvent.Error.Execution -> transitionShowingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_fields.scxml:14 :: _machine
    override fun onEntry(state: StaticRecordFieldsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordFieldsState.Showing -> {
                // SCE-MAP: static_record_fields.scxml:25 :: showing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_fields.scxml:14 :: _machine
    override fun onExit(state: StaticRecordFieldsState) {
        when (state) {
            is StaticRecordFieldsState.Showing -> {
                // SCE-MAP: static_record_fields.scxml:25 :: showing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_fields.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticRecordFieldsState, transitionIndex: Int) {
        when (source) {
        is StaticRecordFieldsState.Showing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_fields.scxml:26 :: showing :: _transition_0

            if (try { shown = shown.copy(dayOfMonth = com.sce.forge.runtime.SceChecked.add(shown.dayOfMonth, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordFieldsEvent.Error.Execution, "<assign location='shown.dayOfMonth'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_record_fields.scxml:29 :: showing :: _transition_1
                if (pendingDayPickedPayload == null) {
                    return
                }

            shown = shown.copy(year = pendingDayPickedPayload!!.year)

            shown = shown.copy(month = pendingDayPickedPayload!!.month)

            shown = shown.copy(dayOfMonth = pendingDayPickedPayload!!.dayOfMonth)
            }
            2 -> {
                // SCE-MAP: static_record_fields.scxml:34 :: showing :: _transition_2

            if (try { shown = shown.copy(year = com.sce.forge.runtime.SceChecked.sub(shown.year, 5000.toUShort())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordFieldsEvent.Error.Execution, "<assign location='shown.year'>: an integer operation overflowed or failed"); true }) {
                return
            }

            shown = shown.copy(month = 1.toUByte())
            }
            3 -> {
                // SCE-MAP: static_record_fields.scxml:38 :: showing :: _transition_3

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordFieldsEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
