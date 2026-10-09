// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_payload.scxml:14 :: _machine

package com.sce.integration.static_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticPayloadState : State {
    data object Waiting : StaticPayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticPayloadEvent : Event {
    sealed interface Day : StaticPayloadEvent {
        data object Picked : Day
    }
    sealed interface Error : StaticPayloadEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticPayloadPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticPayloadDayPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `day.picked`. Consumers inject it via the `raiseDayPicked` seam
// on the machine — they never name this class directly.
data class StaticPayloadDayPickedPayload(val year: UShort, val month: UByte, val dayOfMonth: UByte)


// --- State Machine (W3C SCXML) ---

class StaticPayloadStateMachine(
) : StateMachineEngine<StaticPayloadState, StaticPayloadEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `day` datamodel variable, published (`sce:direction="out"`). */
    var day: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `late` datamodel variable, published (`sce:direction="out"`). */
    var late: Boolean = false
        private set
    /** W3C SCXML 5.2: the `sinceEpoch` datamodel variable, published (`sce:direction="out"`). */
    var sinceEpoch: UShort = 0.toUShort()
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
        var day: UByte? = null
        var late: Boolean? = null
        var sinceEpoch: UShort? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.day?.let { day = it }
        params.late?.let { late = it }
        params.sinceEpoch?.let { sinceEpoch = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val day: UByte,
        val late: Boolean,
        val sinceEpoch: UShort,
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
        val configuration: Set<StaticPayloadState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        day = day,
        late = late,
        sinceEpoch = sinceEpoch,
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
    val savedShape: String = "ac4f8425a964a524cc7580fefe89402b4ca0003a776cebb0521f1eb9ab3dbb27"

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
            "day" to SavedValues.of(day),
            "late" to SavedValues.of(late),
            "sinceEpoch" to SavedValues.of(sinceEpoch),
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
        val saved1 = SavedValues.uint8(saved.variable("day"), "day")
        val saved2 = SavedValues.bool(saved.variable("late"), "late")
        val saved3 = SavedValues.uint16(saved.variable("sinceEpoch"), "sinceEpoch")
        val saved4 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        day = saved1
        late = saved2
        sinceEpoch = saved3
        refusals = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDayPickedPayload: StaticPayloadDayPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticPayloadEvent, metadata: EventMetadata) {
        pendingDayPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticPayloadDayPickedPayload -> pendingDayPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticPayloadEvent.Day.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDayPickedPayload = StaticPayloadDayPickedPayload(fields.uint16("year"), fields.uint8("month"), fields.uint8("dayOfMonth"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `day.picked` — binds the event name and the payload field values in one call.
    fun raiseDayPicked(year: UShort, month: UByte, dayOfMonth: UByte) {
        send(
            StaticPayloadEvent.Day.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticPayloadDayPickedPayload(year, month, dayOfMonth),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("year" to year, "month" to month, "dayOfMonth" to dayOfMonth))
            )
        )
    }


    override val initialState: StaticPayloadState = StaticPayloadState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StaticPayloadState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticPayloadState, HistoryId>> =
            listOf(StateTarget(StaticPayloadState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticPayloadState, HistoryId>(
            StaticPayloadState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticPayloadState, HistoryId>(
            StaticPayloadState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StaticPayloadState, HistoryId>(
            StaticPayloadState.Waiting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticPayloadState? = when (stateId) {
        "waiting" -> StaticPayloadState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticPayloadState): String = when (state) {
        is StaticPayloadState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticPayloadState): Int = when (state) {
        is StaticPayloadState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticPayloadEvent? = when (name) {
        "day.picked" -> StaticPayloadEvent.Day.Picked
        "error.execution" -> StaticPayloadEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticPayloadEvent): String? = when (event) {
        is StaticPayloadEvent.Day.Picked -> "day.picked"
        is StaticPayloadEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticPayloadState,
        event: StaticPayloadEvent?
    ): EnabledTransition<StaticPayloadState, HistoryId>? = when (state) {
        is StaticPayloadState.Waiting -> when {
            event is StaticPayloadEvent.Day.Picked && pendingDayPickedPayload != null && (pendingDayPickedPayload!!.dayOfMonth > 15.toUByte()) -> transitionWaitingAt0
            event is StaticPayloadEvent.Day.Picked -> transitionWaitingAt1
            event is StaticPayloadEvent.Error.Execution -> transitionWaitingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_payload.scxml:14 :: _machine
    override fun onEntry(state: StaticPayloadState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticPayloadState.Waiting -> {
                // SCE-MAP: static_payload.scxml:23 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_payload.scxml:14 :: _machine
    override fun onExit(state: StaticPayloadState) {
        when (state) {
            is StaticPayloadState.Waiting -> {
                // SCE-MAP: static_payload.scxml:23 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_payload.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticPayloadState, transitionIndex: Int) {
        when (source) {
        is StaticPayloadState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_payload.scxml:24 :: waiting :: _transition_0
                if (pendingDayPickedPayload == null) {
                    return
                }

            late = true

            day = pendingDayPickedPayload!!.dayOfMonth

            if (try { sinceEpoch = com.sce.forge.runtime.SceChecked.sub(pendingDayPickedPayload!!.year, 2000.toUShort()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEvent.Error.Execution, "<assign location='sinceEpoch'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_payload.scxml:29 :: waiting :: _transition_1
                if (pendingDayPickedPayload == null) {
                    return
                }

            late = false

            day = pendingDayPickedPayload!!.dayOfMonth

            if (try { sinceEpoch = com.sce.forge.runtime.SceChecked.sub(pendingDayPickedPayload!!.year, 2000.toUShort()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEvent.Error.Execution, "<assign location='sinceEpoch'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_payload.scxml:34 :: waiting :: _transition_2

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
