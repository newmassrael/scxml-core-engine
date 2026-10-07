// SCE-GENERATED — DO NOT EDIT
// source-hash: dd128732f0d0cc6bb147aae13f97e39edf9438b451af2c8f42fc5cd000534f68

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_real32.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_real32.scxml:17 :: _machine

package com.sce.integration.static_record_real32

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordReal32State : State {
    data object Idle : StaticRecordReal32State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordReal32Event : Event {
    data object Bump : StaticRecordReal32Event
    sealed interface Error : StaticRecordReal32Event {
        data object Execution : Error
    }
    sealed interface Reading32 : StaticRecordReal32Event {
        data object Taken : Reading32
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordReal32Payload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordReal32Reading32TakenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `reading32.taken`. Consumers inject it via the `raiseReading32Taken` seam
// on the machine — they never name this class directly.
data class StaticRecordReal32Reading32TakenPayload(val sensor: UByte, val value: Float)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Reading32` datamodel value. */
data class StaticRecordReal32Reading32Record(val sensor: UByte, val value: Float) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sensor" to SavedValues.of(sensor), "value" to SavedValues.of(value))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordReal32Reading32Record = StaticRecordReal32Reading32Record(sensor = SavedValues.uint8(SavedValues.field(value, what, "sensor"), "$what.sensor"), value = SavedValues.float32(SavedValues.field(value, what, "value"), "$what.value"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordReal32StateMachine(
) : StateMachineEngine<StaticRecordReal32State, StaticRecordReal32Event>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: StaticRecordReal32Reading32Record = StaticRecordReal32Reading32Record(sensor = 1.toUByte(), value = 0.5f)
        private set
    /** W3C SCXML 5.2: the `sum` datamodel variable, published (`sce:direction="out"`). */
    var sum: Float = 0.0f
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
        var sum: Float? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.sum?.let { sum = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticRecordReal32Reading32Record,
        val sum: Float,
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
        val configuration: Set<StaticRecordReal32State>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = last,
        sum = sum,
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
    val savedShape: String = "c893e233c9dc158798b2e4ba26c92528fbf675fb7b763ec6857c744284638233"

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
        val saved1 = StaticRecordReal32Reading32Record.fromSaved(saved.variable("last"), "last")
        val saved2 = SavedValues.float32(saved.variable("sum"), "sum")
        val saved3 = SavedValues.uint32(saved.variable("errors"), "errors")
        last = saved1
        sum = saved2
        errors = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingReading32TakenPayload: StaticRecordReal32Reading32TakenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordReal32Event, metadata: EventMetadata) {
        pendingReading32TakenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordReal32Reading32TakenPayload -> pendingReading32TakenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordReal32Event.Reading32.Taken) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingReading32TakenPayload = StaticRecordReal32Reading32TakenPayload(fields.uint8("sensor"), fields.float32("value"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `reading32.taken` — binds the event name and the payload field values in one call.
    fun raiseReading32Taken(sensor: UByte, value: Float) {
        send(
            StaticRecordReal32Event.Reading32.Taken,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordReal32Reading32TakenPayload(sensor, value),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "value" to value))
            )
        )
    }


    override val initialState: StaticRecordReal32State = StaticRecordReal32State.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordReal32State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordReal32State, HistoryId>> =
            listOf(StateTarget(StaticRecordReal32State.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticRecordReal32State, HistoryId>(
            StaticRecordReal32State.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticRecordReal32State, HistoryId>(
            StaticRecordReal32State.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticRecordReal32State, HistoryId>(
            StaticRecordReal32State.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordReal32State? = when (stateId) {
        "idle" -> StaticRecordReal32State.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordReal32State): String = when (state) {
        is StaticRecordReal32State.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordReal32State): Int = when (state) {
        is StaticRecordReal32State.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordReal32Event? = when (name) {
        "bump" -> StaticRecordReal32Event.Bump
        "error.execution" -> StaticRecordReal32Event.Error.Execution
        "reading32.taken" -> StaticRecordReal32Event.Reading32.Taken
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordReal32Event): String? = when (event) {
        is StaticRecordReal32Event.Bump -> "bump"
        is StaticRecordReal32Event.Error.Execution -> "error.execution"
        is StaticRecordReal32Event.Reading32.Taken -> "reading32.taken"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordReal32State,
        event: StaticRecordReal32Event?
    ): EnabledTransition<StaticRecordReal32State, HistoryId>? = when (state) {
        is StaticRecordReal32State.Idle -> when {
            event is StaticRecordReal32Event.Bump -> transitionIdleAt0
            event is StaticRecordReal32Event.Reading32.Taken -> transitionIdleAt1
            event is StaticRecordReal32Event.Error.Execution -> transitionIdleAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_real32.scxml:17 :: _machine
    override fun onEntry(state: StaticRecordReal32State, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordReal32State.Idle -> {
                // SCE-MAP: static_record_real32.scxml:28 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_real32.scxml:17 :: _machine
    override fun onExit(state: StaticRecordReal32State) {
        when (state) {
            is StaticRecordReal32State.Idle -> {
                // SCE-MAP: static_record_real32.scxml:28 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_real32.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticRecordReal32State, transitionIndex: Int) {
        when (source) {
        is StaticRecordReal32State.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_real32.scxml:29 :: idle :: _transition_0

            last = last.copy(value = last.value * 2.0f + 1.5f)

            sum = sum + last.value
            }
            1 -> {
                // SCE-MAP: static_record_real32.scxml:33 :: idle :: _transition_1
                if (pendingReading32TakenPayload == null) {
                    return
                }

            last = last.copy(value = pendingReading32TakenPayload!!.value)

            last = last.copy(sensor = pendingReading32TakenPayload!!.sensor)
            }
            2 -> {
                // SCE-MAP: static_record_real32.scxml:44 :: idle :: _transition_2

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordReal32Event.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
