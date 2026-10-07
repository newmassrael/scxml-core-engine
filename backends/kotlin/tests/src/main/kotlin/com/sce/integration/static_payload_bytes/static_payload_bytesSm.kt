// SCE-GENERATED — DO NOT EDIT
// source-hash: dd128732f0d0cc6bb147aae13f97e39edf9438b451af2c8f42fc5cd000534f68

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_payload_bytes.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_payload_bytes.scxml:23 :: _machine

package com.sce.integration.static_payload_bytes

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticPayloadBytesState : State {
    data object Idle : StaticPayloadBytesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticPayloadBytesEvent : Event {
    sealed interface Error : StaticPayloadBytesEvent {
        data object Execution : Error
    }
    sealed interface Framed : StaticPayloadBytesEvent {
        data object Taken : Framed
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticPayloadBytesPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticPayloadBytesFramedTakenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `framed.taken`. Consumers inject it via the `raiseFramedTaken` seam
// on the machine — they never name this class directly.
data class StaticPayloadBytesFramedTakenPayload(val sensor: UByte, val frame: ByteArray)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Framed` datamodel value. */
data class StaticPayloadBytesFramedRecord(val sensor: UByte, val frame: ByteArray) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sensor" to SavedValues.of(sensor), "frame" to SavedValues.of(frame))

    override fun equals(other: Any?): Boolean =
        other is StaticPayloadBytesFramedRecord && sensor == other.sensor && frame.contentEquals(other.frame)

    override fun hashCode(): Int = 31 * (31 * (0) + sensor.hashCode()) + frame.contentHashCode()

    /** This value with a copy of each byte string, which a host may write into. */
    fun detached(): StaticPayloadBytesFramedRecord = copy(frame = frame.copyOf())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadBytesFramedRecord = StaticPayloadBytesFramedRecord(sensor = SavedValues.uint8(SavedValues.field(value, what, "sensor"), "$what.sensor"), frame = SavedValues.bytes(SavedValues.field(value, what, "frame"), "$what.frame", 8))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticPayloadBytesStateMachine(
) : StateMachineEngine<StaticPayloadBytesState, StaticPayloadBytesEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: StaticPayloadBytesFramedRecord = StaticPayloadBytesFramedRecord(sensor = 1.toUByte(), frame = "ab".toByteArray())
        // A record is handed out with a copy of each byte string it holds, for the
        // same reason.
        get() = field.detached()
        private set
    /** W3C SCXML 5.2: the `held` datamodel variable, published (`sce:direction="out"`). */
    var held: ByteArray = "z".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `frames` datamodel variable, published (`sce:direction="out"`). */
    var frames: List<StaticPayloadBytesFramedRecord> = emptyList()
        // ... and so is each record of a list.
        get() = field.map { it.detached() }
        private set
    /** W3C SCXML 5.2: the `size` datamodel variable, published (`sce:direction="out"`). */
    var size: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `matches` datamodel variable, published (`sce:direction="out"`). */
    var matches: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `misses` datamodel variable, published (`sce:direction="out"`). */
    var misses: UInt = 0.toUInt()
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
        var size: UInt? = null
        var matches: UInt? = null
        var misses: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.size?.let { size = it }
        params.matches?.let { matches = it }
        params.misses?.let { misses = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticPayloadBytesFramedRecord,
        val held: ByteArray,
        val frames: List<StaticPayloadBytesFramedRecord>,
        val size: UInt,
        val matches: UInt,
        val misses: UInt,
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
        val configuration: Set<StaticPayloadBytesState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = last,
        held = held,
        frames = frames,
        size = size,
        matches = matches,
        misses = misses,
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
    val savedShape: String = "1d99c0231d4ab8a7c44c92f7adf331ab1bfc3c6129d0ab0293cbbc7394d4f46a"

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
            "held" to SavedValues.of(held),
            "frames" to SavedValues.list(frames) { it.toSaved() },
            "size" to SavedValues.of(size),
            "matches" to SavedValues.of(matches),
            "misses" to SavedValues.of(misses),
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
        val saved1 = StaticPayloadBytesFramedRecord.fromSaved(saved.variable("last"), "last")
        val saved2 = SavedValues.bytes(saved.variable("held"), "held", 4)
        val saved3 = SavedValues.list(saved.variable("frames"), "frames", 3) { e, w -> StaticPayloadBytesFramedRecord.fromSaved(e, w) }
        val saved4 = SavedValues.uint32(saved.variable("size"), "size")
        val saved5 = SavedValues.uint32(saved.variable("matches"), "matches")
        val saved6 = SavedValues.uint32(saved.variable("misses"), "misses")
        val saved7 = SavedValues.uint32(saved.variable("errors"), "errors")
        last = saved1
        held = saved2
        frames = saved3
        size = saved4
        matches = saved5
        misses = saved6
        errors = saved7
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingFramedTakenPayload: StaticPayloadBytesFramedTakenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticPayloadBytesEvent, metadata: EventMetadata) {
        pendingFramedTakenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticPayloadBytesFramedTakenPayload -> pendingFramedTakenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticPayloadBytesEvent.Framed.Taken) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingFramedTakenPayload = StaticPayloadBytesFramedTakenPayload(fields.uint8("sensor"), fields.bytes("frame"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `framed.taken` — binds the event name and the payload field values in one call.
    fun raiseFramedTaken(sensor: UByte, frame: ByteArray) {
        send(
            StaticPayloadBytesEvent.Framed.Taken,
            EventMetadata(
                type = "external",
                typedPayload = StaticPayloadBytesFramedTakenPayload(sensor, frame),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "frame" to frame))
            )
        )
    }


    override val initialState: StaticPayloadBytesState = StaticPayloadBytesState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticPayloadBytesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticPayloadBytesState, HistoryId>> =
            listOf(StateTarget(StaticPayloadBytesState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticPayloadBytesState, HistoryId>(
            StaticPayloadBytesState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticPayloadBytesState? = when (stateId) {
        "idle" -> StaticPayloadBytesState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticPayloadBytesState): String = when (state) {
        is StaticPayloadBytesState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticPayloadBytesState): Int = when (state) {
        is StaticPayloadBytesState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticPayloadBytesEvent? = when (name) {
        "error.execution" -> StaticPayloadBytesEvent.Error.Execution
        "framed.taken" -> StaticPayloadBytesEvent.Framed.Taken
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticPayloadBytesEvent): String? = when (event) {
        is StaticPayloadBytesEvent.Error.Execution -> "error.execution"
        is StaticPayloadBytesEvent.Framed.Taken -> "framed.taken"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticPayloadBytesState,
        event: StaticPayloadBytesEvent?
    ): EnabledTransition<StaticPayloadBytesState, HistoryId>? = when (state) {
        is StaticPayloadBytesState.Idle -> when {
            event is StaticPayloadBytesEvent.Framed.Taken && pendingFramedTakenPayload != null && (pendingFramedTakenPayload!!.sensor == 0.toUByte() && pendingFramedTakenPayload!!.frame.contentEquals("ab".toByteArray())) -> transitionIdleAt0
            event is StaticPayloadBytesEvent.Framed.Taken && pendingFramedTakenPayload != null && (pendingFramedTakenPayload!!.sensor == 0.toUByte()) -> transitionIdleAt1
            event is StaticPayloadBytesEvent.Framed.Taken && pendingFramedTakenPayload != null && (pendingFramedTakenPayload!!.sensor >= 200.toUByte()) -> transitionIdleAt2
            event is StaticPayloadBytesEvent.Framed.Taken && pendingFramedTakenPayload != null && (pendingFramedTakenPayload!!.sensor >= 100.toUByte()) -> transitionIdleAt3
            event is StaticPayloadBytesEvent.Framed.Taken -> transitionIdleAt4
            event is StaticPayloadBytesEvent.Error.Execution -> transitionIdleAt5
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_payload_bytes.scxml:23 :: _machine
    override fun onEntry(state: StaticPayloadBytesState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticPayloadBytesState.Idle -> {
                // SCE-MAP: static_payload_bytes.scxml:38 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_payload_bytes.scxml:23 :: _machine
    override fun onExit(state: StaticPayloadBytesState) {
        when (state) {
            is StaticPayloadBytesState.Idle -> {
                // SCE-MAP: static_payload_bytes.scxml:38 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_payload_bytes.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticPayloadBytesState, transitionIndex: Int) {
        when (source) {
        is StaticPayloadBytesState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_payload_bytes.scxml:39 :: idle :: _transition_0

            if (try { matches = com.sce.forge.runtime.SceChecked.add(matches, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='matches'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_payload_bytes.scxml:42 :: idle :: _transition_1

            if (try { misses = com.sce.forge.runtime.SceChecked.add(misses, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='misses'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_payload_bytes.scxml:45 :: idle :: _transition_2
                if (pendingFramedTakenPayload == null) {
                    return
                }

            if (try { last = StaticPayloadBytesFramedRecord(sensor = pendingFramedTakenPayload!!.sensor, frame = com.sce.forge.runtime.SceChecked.bounded(pendingFramedTakenPayload!!.frame, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='last'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { if (frames.size < 3) { frames = frames + (StaticPayloadBytesFramedRecord(sensor = pendingFramedTakenPayload!!.sensor, frame = com.sce.forge.runtime.SceChecked.bounded(pendingFramedTakenPayload!!.frame, 8))); false } else { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<sce:append target='frames'>: the list already holds its capacity of 3"); true } } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<sce:append target='frames'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_payload_bytes.scxml:49 :: idle :: _transition_3
                if (pendingFramedTakenPayload == null) {
                    return
                }

            if (try { held = com.sce.forge.runtime.SceChecked.bounded(pendingFramedTakenPayload!!.frame, 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='held'>: an integer operation overflowed or failed"); true }) {
                return
            }

            size = (pendingFramedTakenPayload!!.frame).size.toUInt()
            }
            4 -> {
                // SCE-MAP: static_payload_bytes.scxml:53 :: idle :: _transition_4
                if (pendingFramedTakenPayload == null) {
                    return
                }

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded(pendingFramedTakenPayload!!.frame, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }

            last = last.copy(sensor = pendingFramedTakenPayload!!.sensor)
            }
            5 -> {
                // SCE-MAP: static_payload_bytes.scxml:57 :: idle :: _transition_5

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadBytesEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
