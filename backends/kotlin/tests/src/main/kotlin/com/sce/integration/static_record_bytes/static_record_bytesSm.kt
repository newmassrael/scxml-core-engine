// SCE-GENERATED — DO NOT EDIT
// source-hash: cb41954d893211ff980559eb7566d5cfca26a8d312a941a8b0426661b4fb8dcd

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_bytes.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_bytes.scxml:26 :: _machine

package com.sce.integration.static_record_bytes

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordBytesState : State {
    data object Idle : StaticRecordBytesState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordBytesEvent : Event {
    data object Check : StaticRecordBytesEvent
    sealed interface Error : StaticRecordBytesEvent {
        data object Execution : Error
    }
    data object Fill : StaticRecordBytesEvent
    data object Forget : StaticRecordBytesEvent
    data object FromSpare : StaticRecordBytesEvent
    data object Keep : StaticRecordBytesEvent
    data object Lead : StaticRecordBytesEvent
    data object LongSpare : StaticRecordBytesEvent
    data object Measure : StaticRecordBytesEvent
    data object Other : StaticRecordBytesEvent
    data object Peek : StaticRecordBytesEvent
    data object Reset : StaticRecordBytesEvent
    data object Tally : StaticRecordBytesEvent
    data object ToSpare : StaticRecordBytesEvent
    data object Toowide : StaticRecordBytesEvent
    data object Width : StaticRecordBytesEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Framed` datamodel value. */
data class StaticRecordBytesFramedRecord(val sensor: UByte, val frame: ByteArray) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sensor" to SavedValues.of(sensor), "frame" to SavedValues.of(frame))

    override fun equals(other: Any?): Boolean =
        other is StaticRecordBytesFramedRecord && sensor == other.sensor && frame.contentEquals(other.frame)

    override fun hashCode(): Int = 31 * (31 * (0) + sensor.hashCode()) + frame.contentHashCode()

    /** This value with a copy of each byte string, which a host may write into. */
    fun detached(): StaticRecordBytesFramedRecord = copy(frame = frame.copyOf())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordBytesFramedRecord = StaticRecordBytesFramedRecord(sensor = SavedValues.uint8(SavedValues.field(value, what, "sensor"), "$what.sensor"), frame = SavedValues.bytes(SavedValues.field(value, what, "frame"), "$what.frame", 8))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordBytesStateMachine(
) : StateMachineEngine<StaticRecordBytesState, StaticRecordBytesEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    var last: StaticRecordBytesFramedRecord = StaticRecordBytesFramedRecord(sensor = 1.toUByte(), frame = "ab".toByteArray())
        // A record is handed out with a copy of each byte string it holds, for the
        // same reason.
        get() = field.detached()
        private set
    /** W3C SCXML 5.2: the `spare` datamodel variable, published (`sce:direction="out"`). */
    var spare: ByteArray = "hello".toByteArray()
        // A byte string is handed out as a copy: the array is the machine's own,
        // and a host that wrote into it would change the variable behind its bound.
        get() = field.copyOf()
        private set
    /** W3C SCXML 5.2: the `frames` datamodel variable, published (`sce:direction="out"`). */
    var frames: List<StaticRecordBytesFramedRecord> = emptyList()
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
    /** W3C SCXML 5.2: the `leads` datamodel variable, published (`sce:direction="out"`). */
    var leads: UInt = 0.toUInt()
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
        var leads: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.size?.let { size = it }
        params.matches?.let { matches = it }
        params.misses?.let { misses = it }
        params.leads?.let { leads = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticRecordBytesFramedRecord,
        val spare: ByteArray,
        val frames: List<StaticRecordBytesFramedRecord>,
        val size: UInt,
        val matches: UInt,
        val misses: UInt,
        val leads: UInt,
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
        val configuration: Set<StaticRecordBytesState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = last,
        spare = spare,
        frames = frames,
        size = size,
        matches = matches,
        misses = misses,
        leads = leads,
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
    val savedShape: String = "0f24f88f0f95e9cba72cf2c8194a035f79adde795a2ee38234a238dd2ed59cb8"

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
            "spare" to SavedValues.of(spare),
            "frames" to SavedValues.list(frames) { it.toSaved() },
            "size" to SavedValues.of(size),
            "matches" to SavedValues.of(matches),
            "misses" to SavedValues.of(misses),
            "leads" to SavedValues.of(leads),
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
        val saved1 = StaticRecordBytesFramedRecord.fromSaved(saved.variable("last"), "last")
        val saved2 = SavedValues.bytes(saved.variable("spare"), "spare", 16)
        val saved3 = SavedValues.list(saved.variable("frames"), "frames", 3) { e, w -> StaticRecordBytesFramedRecord.fromSaved(e, w) }
        val saved4 = SavedValues.uint32(saved.variable("size"), "size")
        val saved5 = SavedValues.uint32(saved.variable("matches"), "matches")
        val saved6 = SavedValues.uint32(saved.variable("misses"), "misses")
        val saved7 = SavedValues.uint32(saved.variable("leads"), "leads")
        val saved8 = SavedValues.uint32(saved.variable("errors"), "errors")
        last = saved1
        spare = saved2
        frames = saved3
        size = saved4
        matches = saved5
        misses = saved6
        leads = saved7
        errors = saved8
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticRecordBytesState = StaticRecordBytesState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordBytesState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordBytesState, HistoryId>> =
            listOf(StateTarget(StaticRecordBytesState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 8, as the microstep reads it.
        val transitionIdleAt8 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 9, as the microstep reads it.
        val transitionIdleAt9 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 10, as the microstep reads it.
        val transitionIdleAt10 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 11, as the microstep reads it.
        val transitionIdleAt11 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            11,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 12, as the microstep reads it.
        val transitionIdleAt12 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            12,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 13, as the microstep reads it.
        val transitionIdleAt13 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            13,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 14, as the microstep reads it.
        val transitionIdleAt14 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            14,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 15, as the microstep reads it.
        val transitionIdleAt15 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            15,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 16, as the microstep reads it.
        val transitionIdleAt16 = EnabledTransition<StaticRecordBytesState, HistoryId>(
            StaticRecordBytesState.Idle,
            emptyList(),
            16,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordBytesState? = when (stateId) {
        "idle" -> StaticRecordBytesState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordBytesState): String = when (state) {
        is StaticRecordBytesState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordBytesState): Int = when (state) {
        is StaticRecordBytesState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordBytesEvent? = when (name) {
        "check" -> StaticRecordBytesEvent.Check
        "error.execution" -> StaticRecordBytesEvent.Error.Execution
        "fill" -> StaticRecordBytesEvent.Fill
        "forget" -> StaticRecordBytesEvent.Forget
        "from_spare" -> StaticRecordBytesEvent.FromSpare
        "keep" -> StaticRecordBytesEvent.Keep
        "lead" -> StaticRecordBytesEvent.Lead
        "long_spare" -> StaticRecordBytesEvent.LongSpare
        "measure" -> StaticRecordBytesEvent.Measure
        "other" -> StaticRecordBytesEvent.Other
        "peek" -> StaticRecordBytesEvent.Peek
        "reset" -> StaticRecordBytesEvent.Reset
        "tally" -> StaticRecordBytesEvent.Tally
        "to_spare" -> StaticRecordBytesEvent.ToSpare
        "toowide" -> StaticRecordBytesEvent.Toowide
        "width" -> StaticRecordBytesEvent.Width
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordBytesEvent): String? = when (event) {
        is StaticRecordBytesEvent.Check -> "check"
        is StaticRecordBytesEvent.Error.Execution -> "error.execution"
        is StaticRecordBytesEvent.Fill -> "fill"
        is StaticRecordBytesEvent.Forget -> "forget"
        is StaticRecordBytesEvent.FromSpare -> "from_spare"
        is StaticRecordBytesEvent.Keep -> "keep"
        is StaticRecordBytesEvent.Lead -> "lead"
        is StaticRecordBytesEvent.LongSpare -> "long_spare"
        is StaticRecordBytesEvent.Measure -> "measure"
        is StaticRecordBytesEvent.Other -> "other"
        is StaticRecordBytesEvent.Peek -> "peek"
        is StaticRecordBytesEvent.Reset -> "reset"
        is StaticRecordBytesEvent.Tally -> "tally"
        is StaticRecordBytesEvent.ToSpare -> "to_spare"
        is StaticRecordBytesEvent.Toowide -> "toowide"
        is StaticRecordBytesEvent.Width -> "width"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordBytesState,
        event: StaticRecordBytesEvent?
    ): EnabledTransition<StaticRecordBytesState, HistoryId>? = when (state) {
        is StaticRecordBytesState.Idle -> when {
            event is StaticRecordBytesEvent.Fill -> transitionIdleAt0
            event is StaticRecordBytesEvent.Toowide -> transitionIdleAt1
            event is StaticRecordBytesEvent.Reset -> transitionIdleAt2
            event is StaticRecordBytesEvent.Other -> transitionIdleAt3
            event is StaticRecordBytesEvent.LongSpare -> transitionIdleAt4
            event is StaticRecordBytesEvent.FromSpare -> transitionIdleAt5
            event is StaticRecordBytesEvent.ToSpare -> transitionIdleAt6
            event is StaticRecordBytesEvent.Check && last.frame.contentEquals("ab".toByteArray()) -> transitionIdleAt7
            event is StaticRecordBytesEvent.Check && !last.frame.contentEquals("ab".toByteArray()) -> transitionIdleAt8
            event is StaticRecordBytesEvent.Measure -> transitionIdleAt9
            event is StaticRecordBytesEvent.Keep -> transitionIdleAt10
            event is StaticRecordBytesEvent.Tally -> transitionIdleAt11
            event is StaticRecordBytesEvent.Peek -> transitionIdleAt12
            event is StaticRecordBytesEvent.Width -> transitionIdleAt13
            event is StaticRecordBytesEvent.Lead && (try { com.sce.forge.runtime.SceChecked.at(frames, (0).toLong()).frame.contentEquals("ab".toByteArray()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<transition cond='frames[0].frame === 'ab''>: an integer operation overflowed or failed"); false }) -> transitionIdleAt14
            event is StaticRecordBytesEvent.Forget -> transitionIdleAt15
            event is StaticRecordBytesEvent.Error.Execution -> transitionIdleAt16
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_bytes.scxml:26 :: _machine
    override fun onEntry(state: StaticRecordBytesState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordBytesState.Idle -> {
                // SCE-MAP: static_record_bytes.scxml:42 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_bytes.scxml:26 :: _machine
    override fun onExit(state: StaticRecordBytesState) {
        when (state) {
            is StaticRecordBytesState.Idle -> {
                // SCE-MAP: static_record_bytes.scxml:42 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_bytes.scxml:26 :: _machine
    override fun executeTransitionContent(source: StaticRecordBytesState, transitionIndex: Int) {
        when (source) {
        is StaticRecordBytesState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_bytes.scxml:43 :: idle :: _transition_0

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded("abcdefgh".toByteArray(), 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_record_bytes.scxml:46 :: idle :: _transition_1

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded("abcdefghi".toByteArray(), 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_record_bytes.scxml:49 :: idle :: _transition_2

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded("ab".toByteArray(), 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_record_bytes.scxml:52 :: idle :: _transition_3

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded("ba".toByteArray(), 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_record_bytes.scxml:55 :: idle :: _transition_4

            if (try { spare = com.sce.forge.runtime.SceChecked.bounded("abcdefghijklmnop".toByteArray(), 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='spare'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_record_bytes.scxml:58 :: idle :: _transition_5

            if (try { last = last.copy(frame = com.sce.forge.runtime.SceChecked.bounded(spare, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='last.frame'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_record_bytes.scxml:61 :: idle :: _transition_6

            if (try { spare = com.sce.forge.runtime.SceChecked.bounded(last.frame, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='spare'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_record_bytes.scxml:64 :: idle :: _transition_7

            if (try { matches = com.sce.forge.runtime.SceChecked.add(matches, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='matches'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            8 -> {
                // SCE-MAP: static_record_bytes.scxml:67 :: idle :: _transition_8

            if (try { misses = com.sce.forge.runtime.SceChecked.add(misses, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='misses'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            9 -> {
                // SCE-MAP: static_record_bytes.scxml:70 :: idle :: _transition_9

            size = (last.frame).size.toUInt()
            }
            10 -> {
                // SCE-MAP: static_record_bytes.scxml:76 :: idle :: _transition_10

            if (if (frames.size < 3) { frames = frames + (last); false } else { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<sce:append target='frames'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            11 -> {
                // SCE-MAP: static_record_bytes.scxml:79 :: idle :: _transition_11


            for (f in frames) {

            if (try { spare = com.sce.forge.runtime.SceChecked.bounded(f.frame, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='spare'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            12 -> {
                // SCE-MAP: static_record_bytes.scxml:89 :: idle :: _transition_12

            if (try { spare = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceChecked.at(frames, (0).toLong()).frame, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='spare'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            13 -> {
                // SCE-MAP: static_record_bytes.scxml:92 :: idle :: _transition_13

            if (try { size = (com.sce.forge.runtime.SceChecked.at(frames, (com.sce.forge.runtime.SceChecked.sub((frames).size.toUInt(), 1.toUInt())).toLong()).frame).size.toUInt(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='size'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            14 -> {
                // SCE-MAP: static_record_bytes.scxml:95 :: idle :: _transition_14

            if (try { leads = com.sce.forge.runtime.SceChecked.add(leads, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='leads'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            15 -> {
                // SCE-MAP: static_record_bytes.scxml:98 :: idle :: _transition_15

            frames = emptyList()
            }
            16 -> {
                // SCE-MAP: static_record_bytes.scxml:101 :: idle :: _transition_16

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordBytesEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
