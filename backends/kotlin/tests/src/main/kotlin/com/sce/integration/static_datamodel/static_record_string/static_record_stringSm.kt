// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_string.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_string.scxml:28 :: _machine

package com.sce.integration.static_datamodel.static_record_string

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordStringState : State {
    data object Idle : StaticRecordStringState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordStringEvent : Event {
    sealed interface Error : StaticRecordStringEvent {
        data object Execution : Error
    }
    data object Fill : StaticRecordStringEvent
    data object Forget : StaticRecordStringEvent
    data object FromNote : StaticRecordStringEvent
    data object Isfirst : StaticRecordStringEvent
    data object Keep : StaticRecordStringEvent
    sealed interface Labelled : StaticRecordStringEvent {
        data object Taken : Labelled
    }
    data object LongNote : StaticRecordStringEvent
    data object Peek : StaticRecordStringEvent
    data object Peeklast : StaticRecordStringEvent
    data object Reset : StaticRecordStringEvent
    data object Take : StaticRecordStringEvent
    data object Takelast : StaticRecordStringEvent
    data object Tally : StaticRecordStringEvent
    data object ToNote : StaticRecordStringEvent
    data object Toowide : StaticRecordStringEvent
    data object Wide : StaticRecordStringEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordStringPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordStringLabelledTakenPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `labelled.taken`. Consumers inject it via the `raiseLabelledTaken` seam
// on the machine — they never name this class directly.
data class StaticRecordStringLabelledTakenPayload(val sensor: UByte, val label: String)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Labelled` datamodel value. */
data class StaticRecordStringLabelledRecord(val sensor: UByte, val label: String) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sensor" to SavedValues.of(sensor), "label" to SavedValues.of(label))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordStringLabelledRecord = StaticRecordStringLabelledRecord(sensor = SavedValues.uint8(SavedValues.field(value, what, "sensor"), "$what.sensor"), label = SavedValues.string(SavedValues.field(value, what, "label"), "$what.label", 8))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticRecordStringStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticRecordStringState, StaticRecordStringEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticRecordStringLabelledRecord = StaticRecordStringLabelledRecord(sensor = 1.toUByte(), label = "a")

    /** What the published `last` holds. */
    val last: StaticRecordStringLabelledRecord get() = vLast
    /** W3C SCXML 5.2: the `note` datamodel variable, published (`sce:direction="out"`). */
    private var vNote: String = "hello"

    /** What the published `note` holds. */
    val note: String get() = vNote
    /** W3C SCXML 5.2: the `labels` datamodel variable, published (`sce:direction="out"`). */
    private var vLabels: List<StaticRecordStringLabelledRecord> = emptyList()

    /** What the published `labels` holds. */
    val labels: List<StaticRecordStringLabelledRecord> get() = vLabels
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var note: String? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.note?.let { vNote = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticRecordStringLabelledRecord,
        val note: String,
        val labels: List<StaticRecordStringLabelledRecord>,
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
        val configuration: Set<StaticRecordStringState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = vLast,
        note = vNote,
        labels = vLabels,
        errors = vErrors,
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
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "2d4abb7c5b44b37484e095adbc16d0d2cd766c64317344ff2d6da5cfe3032c86",
        revised = "99115f5e5956fca06cf060ea01da1957f57b5efdd9a0b4372efbc4580cc98e25",
        format2First = "2d4abb7c5b44b37484e095adbc16d0d2cd766c64317344ff2d6da5cfe3032c86",
        format2Revised = "99115f5e5956fca06cf060ea01da1957f57b5efdd9a0b4372efbc4580cc98e25",
        records = listOf(
            RecordLineage(
                event = "labelled.taken",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("last", false),
                    RecordHolder("labels", true),
                ),
            ),
        ),
        enums = listOf(
        ),
    )

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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "last" to vLast.toSaved()
        variables += "note" to SavedValues.of(vNote)
        variables += "labels" to SavedValues.list(vLabels) { it.toSaved() }
        variables += "errors" to SavedValues.of(vErrors)
    }

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
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val writeVariablesPart0 = restoreVariablesPart0(savedGrown)
        writeVariablesPart0()
        enterSaved(savedGrown, wallNowMs)
    }

    // Reads this part's variables from [savedGrown], refusing a value its
    // variable's type cannot hold, and answers what writes them: nothing is
    // written until every part has read.
    private fun restoreVariablesPart0(savedGrown: SavedState): () -> Unit {
        val saved1 = StaticRecordStringLabelledRecord.fromSaved(savedGrown.variable("last"), "last")
        val saved2 = SavedValues.string(savedGrown.variable("note"), "note", 16)
        val saved3 = SavedValues.list(savedGrown.variable("labels"), "labels", 3) { e, w -> StaticRecordStringLabelledRecord.fromSaved(e, w) }
        val saved4 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        return {
            vLast = saved1
            vNote = saved2
            vLabels = saved3
            vErrors = saved4
        }
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        StaticRecordStringLabelledRecord.fromSaved(savedGrown.variable("last"), "last")
        SavedValues.string(savedGrown.variable("note"), "note", 16)
        SavedValues.list(savedGrown.variable("labels"), "labels", 3) { e, w -> StaticRecordStringLabelledRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingLabelledTakenPayload: StaticRecordStringLabelledTakenPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordStringEvent, metadata: EventMetadata) {
        pendingLabelledTakenPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordStringLabelledTakenPayload -> pendingLabelledTakenPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordStringEvent.Labelled.Taken) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingLabelledTakenPayload = StaticRecordStringLabelledTakenPayload(fields.uint8("sensor"), fields.string("label", 8))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `labelled.taken` — binds the event name and the payload field values in one call.
    fun raiseLabelledTaken(sensor: UByte, label: String) {
        send(
            StaticRecordStringEvent.Labelled.Taken,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordStringLabelledTakenPayload(sensor, label),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sensor" to sensor, "label" to label))
            )
        )
    }


    override val initialState: StaticRecordStringState = StaticRecordStringState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordStringState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordStringState, HistoryId>> =
            listOf(StateTarget(StaticRecordStringState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 8, as the microstep reads it.
        val transitionIdleAt8 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 9, as the microstep reads it.
        val transitionIdleAt9 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 10, as the microstep reads it.
        val transitionIdleAt10 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 11, as the microstep reads it.
        val transitionIdleAt11 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            11,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 12, as the microstep reads it.
        val transitionIdleAt12 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            12,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 13, as the microstep reads it.
        val transitionIdleAt13 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            13,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 14, as the microstep reads it.
        val transitionIdleAt14 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            14,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 15, as the microstep reads it.
        val transitionIdleAt15 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            15,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 16, as the microstep reads it.
        val transitionIdleAt16 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            16,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 17, as the microstep reads it.
        val transitionIdleAt17 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            17,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 18, as the microstep reads it.
        val transitionIdleAt18 = EnabledTransition<StaticRecordStringState, HistoryId>(
            StaticRecordStringState.Idle,
            emptyList(),
            18,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordStringState? = when (stateId) {
        "idle" -> StaticRecordStringState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordStringState): String = when (state) {
        is StaticRecordStringState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordStringState): Int = when (state) {
        is StaticRecordStringState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordStringEvent? = when (name) {
        "error.execution" -> StaticRecordStringEvent.Error.Execution
        "fill" -> StaticRecordStringEvent.Fill
        "forget" -> StaticRecordStringEvent.Forget
        "from_note" -> StaticRecordStringEvent.FromNote
        "isfirst" -> StaticRecordStringEvent.Isfirst
        "keep" -> StaticRecordStringEvent.Keep
        "labelled.taken" -> StaticRecordStringEvent.Labelled.Taken
        "long_note" -> StaticRecordStringEvent.LongNote
        "peek" -> StaticRecordStringEvent.Peek
        "peeklast" -> StaticRecordStringEvent.Peeklast
        "reset" -> StaticRecordStringEvent.Reset
        "take" -> StaticRecordStringEvent.Take
        "takelast" -> StaticRecordStringEvent.Takelast
        "tally" -> StaticRecordStringEvent.Tally
        "to_note" -> StaticRecordStringEvent.ToNote
        "toowide" -> StaticRecordStringEvent.Toowide
        "wide" -> StaticRecordStringEvent.Wide
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordStringEvent): String? = when (event) {
        is StaticRecordStringEvent.Error.Execution -> "error.execution"
        is StaticRecordStringEvent.Fill -> "fill"
        is StaticRecordStringEvent.Forget -> "forget"
        is StaticRecordStringEvent.FromNote -> "from_note"
        is StaticRecordStringEvent.Isfirst -> "isfirst"
        is StaticRecordStringEvent.Keep -> "keep"
        is StaticRecordStringEvent.Labelled.Taken -> "labelled.taken"
        is StaticRecordStringEvent.LongNote -> "long_note"
        is StaticRecordStringEvent.Peek -> "peek"
        is StaticRecordStringEvent.Peeklast -> "peeklast"
        is StaticRecordStringEvent.Reset -> "reset"
        is StaticRecordStringEvent.Take -> "take"
        is StaticRecordStringEvent.Takelast -> "takelast"
        is StaticRecordStringEvent.Tally -> "tally"
        is StaticRecordStringEvent.ToNote -> "to_note"
        is StaticRecordStringEvent.Toowide -> "toowide"
        is StaticRecordStringEvent.Wide -> "wide"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticRecordStringState, event: StaticRecordStringEvent?): EnabledTransition<StaticRecordStringState, HistoryId>? = when (state) {
        is StaticRecordStringState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticRecordStringState, event: StaticRecordStringEvent?): EnabledTransition<StaticRecordStringState, HistoryId>? = when {
        event is StaticRecordStringEvent.Fill -> TransitionsPart0.transitionIdleAt0
        event is StaticRecordStringEvent.Wide -> TransitionsPart0.transitionIdleAt1
        event is StaticRecordStringEvent.Toowide -> TransitionsPart0.transitionIdleAt2
        event is StaticRecordStringEvent.Reset -> TransitionsPart0.transitionIdleAt3
        event is StaticRecordStringEvent.LongNote -> TransitionsPart0.transitionIdleAt4
        event is StaticRecordStringEvent.FromNote -> TransitionsPart0.transitionIdleAt5
        event is StaticRecordStringEvent.ToNote -> TransitionsPart0.transitionIdleAt6
        event is StaticRecordStringEvent.Labelled.Taken && pendingLabelledTakenPayload != null && (pendingLabelledTakenPayload!!.sensor >= 200.toUByte()) -> TransitionsPart0.transitionIdleAt7
        event is StaticRecordStringEvent.Labelled.Taken && pendingLabelledTakenPayload != null && (pendingLabelledTakenPayload!!.sensor >= 100.toUByte()) -> TransitionsPart0.transitionIdleAt8
        event is StaticRecordStringEvent.Labelled.Taken -> TransitionsPart0.transitionIdleAt9
        event is StaticRecordStringEvent.Keep -> TransitionsPart0.transitionIdleAt10
        event is StaticRecordStringEvent.Tally -> TransitionsPart0.transitionIdleAt11
        event is StaticRecordStringEvent.Peek -> TransitionsPart0.transitionIdleAt12
        event is StaticRecordStringEvent.Peeklast -> TransitionsPart0.transitionIdleAt13
        event is StaticRecordStringEvent.Isfirst && (try { com.sce.forge.runtime.SceChecked.at(vLabels, (0).toLong()).label == "whole" } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<transition cond='labels[0].label === 'whole''>: an integer operation overflowed or failed"); false }) -> TransitionsPart0.transitionIdleAt14
        event is StaticRecordStringEvent.Take -> TransitionsPart0.transitionIdleAt15
        event is StaticRecordStringEvent.Takelast -> TransitionsPart0.transitionIdleAt16
        event is StaticRecordStringEvent.Forget -> TransitionsPart0.transitionIdleAt17
        event is StaticRecordStringEvent.Error.Execution -> TransitionsPart0.transitionIdleAt18
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_string.scxml:28 :: _machine
    override fun onEntry(state: StaticRecordStringState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordStringState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_record_string.scxml:40 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_string.scxml:28 :: _machine
    override fun onExit(state: StaticRecordStringState) {
        when (state) {
            is StaticRecordStringState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_record_string.scxml:40 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_string.scxml:28 :: _machine
    override fun executeTransitionContent(source: StaticRecordStringState, transitionIndex: Int) {
        when (source) {
            is StaticRecordStringState.Idle -> transitionContentOfIdle(transitionIndex)
        }
    }


    // The content of `idle`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfIdle(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfIdleAt0()
            1 -> transitionContentOfIdleAt1()
            2 -> transitionContentOfIdleAt2()
            3 -> transitionContentOfIdleAt3()
            4 -> transitionContentOfIdleAt4()
            5 -> transitionContentOfIdleAt5()
            6 -> transitionContentOfIdleAt6()
            7 -> transitionContentOfIdleAt7()
            8 -> transitionContentOfIdleAt8()
            9 -> transitionContentOfIdleAt9()
            10 -> transitionContentOfIdleAt10()
            11 -> transitionContentOfIdleAt11()
            12 -> transitionContentOfIdleAt12()
            13 -> transitionContentOfIdleAt13()
            14 -> transitionContentOfIdleAt14()
            15 -> transitionContentOfIdleAt15()
            16 -> transitionContentOfIdleAt16()
            17 -> transitionContentOfIdleAt17()
            18 -> transitionContentOfIdleAt18()
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_record_string.scxml:41 :: idle :: _transition_0

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded("abcdefgh", 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_record_string.scxml:44 :: idle :: _transition_1

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded("é€é", 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_record_string.scxml:47 :: idle :: _transition_2

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded("é€éé", 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: static_record_string.scxml:50 :: idle :: _transition_3

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded("a", 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: static_record_string.scxml:53 :: idle :: _transition_4

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded("abcdefghijklmnop", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt5() {
                // SCE-MAP: static_record_string.scxml:56 :: idle :: _transition_5

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded(vNote, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt6() {
                // SCE-MAP: static_record_string.scxml:59 :: idle :: _transition_6

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(vLast.label, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt7() {
                // SCE-MAP: static_record_string.scxml:65 :: idle :: _transition_7
                if (pendingLabelledTakenPayload == null) {
                    return
                }

            if (try { vLast = StaticRecordStringLabelledRecord(sensor = pendingLabelledTakenPayload!!.sensor, label = com.sce.forge.runtime.SceChecked.bounded(pendingLabelledTakenPayload!!.label, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { if (vLabels.size < 3) { vLabels = vLabels + (StaticRecordStringLabelledRecord(sensor = pendingLabelledTakenPayload!!.sensor, label = com.sce.forge.runtime.SceChecked.bounded(pendingLabelledTakenPayload!!.label, 8))); false } else { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<sce:append target='labels'>: the list already holds its capacity of 3"); true } } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<sce:append target='labels'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt8() {
                // SCE-MAP: static_record_string.scxml:74 :: idle :: _transition_8
                if (pendingLabelledTakenPayload == null) {
                    return
                }

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(pendingLabelledTakenPayload!!.label, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt9() {
                // SCE-MAP: static_record_string.scxml:77 :: idle :: _transition_9
                if (pendingLabelledTakenPayload == null) {
                    return
                }

            if (try { vLast = vLast.copy(label = com.sce.forge.runtime.SceChecked.bounded(pendingLabelledTakenPayload!!.label, 8)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last.label'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLast = vLast.copy(sensor = pendingLabelledTakenPayload!!.sensor)
    }

    private fun transitionContentOfIdleAt10() {
                // SCE-MAP: static_record_string.scxml:84 :: idle :: _transition_10

            if (if (vLabels.size < 3) { vLabels = vLabels + (vLast); false } else { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<sce:append target='labels'>: the list already holds its capacity of 3"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt11() {
                // SCE-MAP: static_record_string.scxml:87 :: idle :: _transition_11


            for (l in vLabels) {

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(l.label, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfIdleAt12() {
                // SCE-MAP: static_record_string.scxml:97 :: idle :: _transition_12

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceChecked.at(vLabels, (0).toLong()).label, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt13() {
                // SCE-MAP: static_record_string.scxml:100 :: idle :: _transition_13

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceChecked.at(vLabels, (com.sce.forge.runtime.SceChecked.sub((vLabels).size.toUInt(), 1.toUInt())).toLong()).label, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt14() {
                // SCE-MAP: static_record_string.scxml:103 :: idle :: _transition_14

            if (try { vNote = com.sce.forge.runtime.SceChecked.bounded("first", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='note'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt15() {
                // SCE-MAP: static_record_string.scxml:110 :: idle :: _transition_15

            if (try { vLast = com.sce.forge.runtime.SceChecked.at(vLabels, (0).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt16() {
                // SCE-MAP: static_record_string.scxml:113 :: idle :: _transition_16

            if (try { vLast = com.sce.forge.runtime.SceChecked.at(vLabels, (com.sce.forge.runtime.SceChecked.sub((vLabels).size.toUInt(), 1.toUInt())).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='last'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfIdleAt17() {
                // SCE-MAP: static_record_string.scxml:116 :: idle :: _transition_17

            vLabels = emptyList()
    }

    private fun transitionContentOfIdleAt18() {
                // SCE-MAP: static_record_string.scxml:119 :: idle :: _transition_18

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRecordStringEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
