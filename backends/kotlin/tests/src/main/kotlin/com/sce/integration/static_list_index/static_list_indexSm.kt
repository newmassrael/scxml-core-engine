// SCE-GENERATED — DO NOT EDIT
// source-hash: ee37533f857b01b43223ccdf57f0449c0b326ab00d940ee5b3e09fb6a557b07f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_list_index.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_list_index.scxml:27 :: _machine

package com.sce.integration.static_list_index

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticListIndexState : State {
    data object Reading : StaticListIndexState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticListIndexEvent : Event {
    data object Back : StaticListIndexEvent
    data object Compare : StaticListIndexEvent
    sealed interface Day : StaticListIndexEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticListIndexEvent {
        data object Execution : Error
    }
    data object More : StaticListIndexEvent
    data object Next : StaticListIndexEvent
    data object Read : StaticListIndexEvent
    data object Readlast : StaticListIndexEvent
    data object Reset : StaticListIndexEvent
    data object Shrink : StaticListIndexEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticListIndexPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticListIndexDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticListIndexDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// --- State Machine (W3C SCXML) ---

class StaticListIndexStateMachine(
) : StateMachineEngine<StaticListIndexState, StaticListIndexEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `picked` datamodel variable, published (`sce:direction="out"`). */
    var picked: List<UByte> = emptyList()
        private set
    /** W3C SCXML 5.2: the `cursor` datamodel variable, published (`sce:direction="out"`). */
    var cursor: Int = 0
        private set
    /** W3C SCXML 5.2: the `under` datamodel variable, published (`sce:direction="out"`). */
    var under: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `first` datamodel variable, published (`sce:direction="out"`). */
    var first: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `ordered` datamodel variable, published (`sce:direction="out"`). */
    var ordered: Boolean = false
        private set
    /** W3C SCXML 5.2: the `room` datamodel variable, published (`sce:direction="out"`). */
    var room: Boolean = false
        private set
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    var count: UInt = 0.toUInt()
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
        var under: UByte? = null
        var first: UByte? = null
        var last: UByte? = null
        var ordered: Boolean? = null
        var room: Boolean? = null
        var count: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.cursor?.let { cursor = it }
        params.under?.let { under = it }
        params.first?.let { first = it }
        params.last?.let { last = it }
        params.ordered?.let { ordered = it }
        params.room?.let { room = it }
        params.count?.let { count = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: List<UByte>,
        val cursor: Int,
        val under: UByte,
        val first: UByte,
        val last: UByte,
        val ordered: Boolean,
        val room: Boolean,
        val count: UInt,
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
        val configuration: Set<StaticListIndexState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        picked = picked,
        cursor = cursor,
        under = under,
        first = first,
        last = last,
        ordered = ordered,
        room = room,
        count = count,
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
    val savedShape: String = "fcee04d52d8557a799792a90fbe66c3f0417605379d6e8342226dec4bc4d5ffe"

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
            "picked" to SavedValues.list(picked) { SavedValues.of(it) },
            "cursor" to SavedValues.of(cursor),
            "under" to SavedValues.of(under),
            "first" to SavedValues.of(first),
            "last" to SavedValues.of(last),
            "ordered" to SavedValues.of(ordered),
            "room" to SavedValues.of(room),
            "count" to SavedValues.of(count),
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
        val saved1 = SavedValues.list(saved.variable("picked"), "picked", 3) { e, w -> SavedValues.uint8(e, w) }
        val saved2 = SavedValues.int32(saved.variable("cursor"), "cursor")
        val saved3 = SavedValues.uint8(saved.variable("under"), "under")
        val saved4 = SavedValues.uint8(saved.variable("first"), "first")
        val saved5 = SavedValues.uint8(saved.variable("last"), "last")
        val saved6 = SavedValues.bool(saved.variable("ordered"), "ordered")
        val saved7 = SavedValues.bool(saved.variable("room"), "room")
        val saved8 = SavedValues.uint32(saved.variable("count"), "count")
        val saved9 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        picked = saved1
        cursor = saved2
        under = saved3
        first = saved4
        last = saved5
        ordered = saved6
        room = saved7
        count = saved8
        refusals = saved9
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticListIndexDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticListIndexEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticListIndexDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticListIndexEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticListIndexDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticListIndexEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticListIndexDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticListIndexState = StaticListIndexState.Reading

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
    override val documentInitialTargets: List<EntryTarget<StaticListIndexState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticListIndexState, HistoryId>> =
            listOf(StateTarget(StaticListIndexState.Reading))

        // W3C SCXML 3.13: reading's transition 0, as the microstep reads it.
        val transitionReadingAt0 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 1, as the microstep reads it.
        val transitionReadingAt1 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 2, as the microstep reads it.
        val transitionReadingAt2 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 3, as the microstep reads it.
        val transitionReadingAt3 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 4, as the microstep reads it.
        val transitionReadingAt4 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 5, as the microstep reads it.
        val transitionReadingAt5 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 6, as the microstep reads it.
        val transitionReadingAt6 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 7, as the microstep reads it.
        val transitionReadingAt7 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 8, as the microstep reads it.
        val transitionReadingAt8 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 9, as the microstep reads it.
        val transitionReadingAt9 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 10, as the microstep reads it.
        val transitionReadingAt10 = EnabledTransition<StaticListIndexState, HistoryId>(
            StaticListIndexState.Reading,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticListIndexState? = when (stateId) {
        "reading" -> StaticListIndexState.Reading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticListIndexState): String = when (state) {
        is StaticListIndexState.Reading -> "reading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticListIndexState): Int = when (state) {
        is StaticListIndexState.Reading -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticListIndexEvent? = when (name) {
        "back" -> StaticListIndexEvent.Back
        "compare" -> StaticListIndexEvent.Compare
        "day.picked" -> StaticListIndexEvent.Day.Picked
        "error.execution" -> StaticListIndexEvent.Error.Execution
        "more" -> StaticListIndexEvent.More
        "next" -> StaticListIndexEvent.Next
        "read" -> StaticListIndexEvent.Read
        "readlast" -> StaticListIndexEvent.Readlast
        "reset" -> StaticListIndexEvent.Reset
        "shrink" -> StaticListIndexEvent.Shrink
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticListIndexEvent): String? = when (event) {
        is StaticListIndexEvent.Back -> "back"
        is StaticListIndexEvent.Compare -> "compare"
        is StaticListIndexEvent.Day.Picked -> "day.picked"
        is StaticListIndexEvent.Error.Execution -> "error.execution"
        is StaticListIndexEvent.More -> "more"
        is StaticListIndexEvent.Next -> "next"
        is StaticListIndexEvent.Read -> "read"
        is StaticListIndexEvent.Readlast -> "readlast"
        is StaticListIndexEvent.Reset -> "reset"
        is StaticListIndexEvent.Shrink -> "shrink"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticListIndexState,
        event: StaticListIndexEvent?
    ): EnabledTransition<StaticListIndexState, HistoryId>? = when (state) {
        is StaticListIndexState.Reading -> when {
            event is StaticListIndexEvent.Day.Picked -> transitionReadingAt0
            event is StaticListIndexEvent.Read -> transitionReadingAt1
            event is StaticListIndexEvent.Readlast -> transitionReadingAt2
            event is StaticListIndexEvent.Shrink -> transitionReadingAt3
            event is StaticListIndexEvent.More -> transitionReadingAt4
            event is StaticListIndexEvent.Next -> transitionReadingAt5
            event is StaticListIndexEvent.Back -> transitionReadingAt6
            event is StaticListIndexEvent.Compare && (try { com.sce.forge.runtime.SceChecked.at(picked, (0).toLong()) < com.sce.forge.runtime.SceChecked.at(picked, (1).toLong()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<transition cond='picked[0] < picked[1]'>: an integer operation overflowed or failed"); false }) -> transitionReadingAt7
            event is StaticListIndexEvent.Compare -> transitionReadingAt8
            event is StaticListIndexEvent.Reset -> transitionReadingAt9
            event is StaticListIndexEvent.Error.Execution -> transitionReadingAt10
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_list_index.scxml:27 :: _machine
    override fun onEntry(state: StaticListIndexState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticListIndexState.Reading -> {
                // SCE-MAP: static_list_index.scxml:41 :: reading :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_list_index.scxml:27 :: _machine
    override fun onExit(state: StaticListIndexState) {
        when (state) {
            is StaticListIndexState.Reading -> {
                // SCE-MAP: static_list_index.scxml:41 :: reading :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_list_index.scxml:27 :: _machine
    override fun executeTransitionContent(source: StaticListIndexState, transitionIndex: Int) {
        when (source) {
        is StaticListIndexState.Reading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_list_index.scxml:42 :: reading :: _transition_0
                if (pendingDayPickedPayload == null) {
                    return
                }

            if (if (picked.size < 3) { picked = picked + (pendingDayPickedPayload!!.dayOfMonth); false } else { raisePlatformError(StaticListIndexEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_list_index.scxml:45 :: reading :: _transition_1

            if (try { first = com.sce.forge.runtime.SceChecked.at(picked, (0).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='first'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_list_index.scxml:48 :: reading :: _transition_2

            if (try { last = com.sce.forge.runtime.SceChecked.at(picked, (com.sce.forge.runtime.SceChecked.sub((picked).size.toUInt(), 1.toUInt())).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='last'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_list_index.scxml:51 :: reading :: _transition_3

            if (try { count = com.sce.forge.runtime.SceChecked.sub((picked).size.toUInt(), 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='count'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_list_index.scxml:54 :: reading :: _transition_4

            room = cursor < (picked).size.toUInt().toInt()
            }
            5 -> {
                // SCE-MAP: static_list_index.scxml:57 :: reading :: _transition_5

            if (try { under = com.sce.forge.runtime.SceChecked.at(picked, (cursor).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='under'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { cursor = com.sce.forge.runtime.SceChecked.add(cursor, 1); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='cursor'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_list_index.scxml:61 :: reading :: _transition_6

            if (try { under = com.sce.forge.runtime.SceChecked.at(picked, (com.sce.forge.runtime.SceChecked.sub(cursor, 1)).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='under'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { cursor = com.sce.forge.runtime.SceChecked.sub(cursor, 1); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='cursor'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_list_index.scxml:65 :: reading :: _transition_7

            ordered = true
            }
            8 -> {
                // SCE-MAP: static_list_index.scxml:68 :: reading :: _transition_8

            ordered = false
            }
            9 -> {
                // SCE-MAP: static_list_index.scxml:71 :: reading :: _transition_9

            picked = emptyList()
            }
            10 -> {
                // SCE-MAP: static_list_index.scxml:74 :: reading :: _transition_10

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListIndexEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
