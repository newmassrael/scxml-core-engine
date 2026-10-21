// SCE-GENERATED — DO NOT EDIT
// source-hash: 602cf8b2e076ca5de83736526fa544c7671ce4b8406bed6ee323cbe2df13499b

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_payload_enum_unimported.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_payload_enum_unimported.scxml:16 :: _machine

package com.sce.integration.static_payload_enum_unimported

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticPayloadEnumUnimportedState : State {
    data object Listening : StaticPayloadEnumUnimportedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticPayloadEnumUnimportedEvent : Event {
    sealed interface RecordList : StaticPayloadEnumUnimportedEvent {
        data object Refused : RecordList
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticPayloadEnumUnimportedPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticPayloadEnumUnimportedRecordListRefusedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.refused`. Consumers inject it via the `raiseRecordListRefused` seam
// on the machine — they never name this class directly.
data class StaticPayloadEnumUnimportedRecordListRefusedPayload(val id: UInt, val field: UShort, val kind: StaticPayloadEnumUnimportedRecordCommandEnum, val reason: StaticPayloadEnumUnimportedWhyEnum)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Why` datamodel value. */
enum class StaticPayloadEnumUnimportedWhyEnum(val declaredName: String) {
    NONE("none"),
    NOT_WRITER("notWriter"),
    NO_ROOM_FOR_VALUE("noRoomForValue"),
    NO_ROOM_FOR_HISTORY("noRoomForHistory"),
    NO_ROOM_FOR_DIGEST("noRoomForDigest"),
    TEXT_TOO_LONG("textTooLong"),
    CHANGE_TOO_LARGE("changeTooLarge"),
    REGISTER_NAMED_TWICE("registerNamedTwice"),
    CLOCK_EXHAUSTED("clockExhausted");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NONE -> 0u.toUByte()
        NOT_WRITER -> 1u.toUByte()
        NO_ROOM_FOR_VALUE -> 2u.toUByte()
        NO_ROOM_FOR_HISTORY -> 3u.toUByte()
        NO_ROOM_FOR_DIGEST -> 4u.toUByte()
        TEXT_TOO_LONG -> 5u.toUByte()
        CHANGE_TOO_LARGE -> 6u.toUByte()
        REGISTER_NAMED_TWICE -> 7u.toUByte()
        CLOCK_EXHAUSTED -> 8u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadEnumUnimportedWhyEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Why")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticPayloadEnumUnimportedWhyEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: an `enum:record_command` datamodel value. */
enum class StaticPayloadEnumUnimportedRecordCommandEnum(val declaredName: String) {
    EDIT("edit"),
    CLEAR("clear"),
    RETIRE("retire");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        EDIT -> 0u.toUByte()
        CLEAR -> 1u.toUByte()
        RETIRE -> 2u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadEnumUnimportedRecordCommandEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of record_command")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticPayloadEnumUnimportedRecordCommandEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Refused` datamodel value. */
data class StaticPayloadEnumUnimportedRefusedRecord(val id: UInt, val field: UShort, val kind: StaticPayloadEnumUnimportedRecordCommandEnum, val reason: StaticPayloadEnumUnimportedWhyEnum) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "kind" to kind.toSaved(), "reason" to reason.toSaved())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadEnumUnimportedRefusedRecord = StaticPayloadEnumUnimportedRefusedRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), kind = StaticPayloadEnumUnimportedRecordCommandEnum.fromSaved(SavedValues.field(value, what, "kind"), "$what.kind"), reason = StaticPayloadEnumUnimportedWhyEnum.fromSaved(SavedValues.field(value, what, "reason"), "$what.reason"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticPayloadEnumUnimportedStateMachine(
) : StateMachineEngine<StaticPayloadEnumUnimportedState, StaticPayloadEnumUnimportedEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticPayloadEnumUnimportedWhyEnum = StaticPayloadEnumUnimportedWhyEnum.NONE

    /** What the published `last` holds. */
    val last: StaticPayloadEnumUnimportedWhyEnum get() = vLast
    /** W3C SCXML 5.2: the `kept` datamodel variable, published (`sce:direction="out"`). */
    private var vKept: List<StaticPayloadEnumUnimportedRefusedRecord> = emptyList()

    /** What the published `kept` holds. */
    val kept: List<StaticPayloadEnumUnimportedRefusedRecord> get() = vKept
    /** W3C SCXML 5.2: the `row` datamodel variable, published (`sce:direction="out"`). */
    private var vRow: UInt = 0.toUInt()

    /** What the published `row` holds. */
    val row: UInt get() = vRow
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var row: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.row?.let { vRow = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticPayloadEnumUnimportedWhyEnum,
        val kept: List<StaticPayloadEnumUnimportedRefusedRecord>,
        val row: UInt,
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
        val configuration: Set<StaticPayloadEnumUnimportedState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = vLast,
        kept = vKept,
        row = vRow,
        refusals = vRefusals,
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
        first = "b438ab24fdbeaa8a6e5b4374868634cd1f5563b57bdda5aa1ab05ac6aa345cb4",
        revised = "49947d270ab07ed43e599cbb214b814e9848b17e4a4db30d99f64b907fb7c69d",
        format2First = "d405568787fe6df52784dfd89231491aa1d68562f4788d2515c076e8d9bac5d0",
        format2Revised = "54654acbc9d1b0f11710436fbb18e8367f3a511e51dc474789a41fe83a68fa95",
        records = listOf(
            RecordLineage(
                event = "record_list.refused",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("kept", true),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:sce:std/shared/record_command.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("kept", "kind", true),
                ),
            ),
            EnumLineage(
                key = "enum:sce:std/shared/record_refusal.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("last", null, false),
                    EnumHolder("kept", "reason", true),
                ),
            ),
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
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "last" to vLast.toSaved(),
            "kept" to SavedValues.list(vKept) { it.toSaved() },
            "row" to SavedValues.of(vRow),
            "refusals" to SavedValues.of(vRefusals),
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
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val saved1 = StaticPayloadEnumUnimportedWhyEnum.fromSaved(savedGrown.variable("last"), "last")
        val saved2 = SavedValues.list(savedGrown.variable("kept"), "kept", 2) { e, w -> StaticPayloadEnumUnimportedRefusedRecord.fromSaved(e, w) }
        val saved3 = SavedValues.uint32(savedGrown.variable("row"), "row")
        val saved4 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vLast = saved1
        vKept = saved2
        vRow = saved3
        vRefusals = saved4
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingRecordListRefusedPayload: StaticPayloadEnumUnimportedRecordListRefusedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticPayloadEnumUnimportedEvent, metadata: EventMetadata) {
        pendingRecordListRefusedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticPayloadEnumUnimportedRecordListRefusedPayload -> pendingRecordListRefusedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticPayloadEnumUnimportedEvent.RecordList.Refused) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRefusedPayload = StaticPayloadEnumUnimportedRecordListRefusedPayload(fields.uint32("id"), fields.uint16("field"), fields.string("kind").let { name -> StaticPayloadEnumUnimportedRecordCommandEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'kind' ($name) is not a variant of record_command") }, fields.string("reason").let { name -> StaticPayloadEnumUnimportedWhyEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'reason' ($name) is not a variant of Why") })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.refused` — binds the event name and the payload field values in one call.
    fun raiseRecordListRefused(id: UInt, field: UShort, kind: StaticPayloadEnumUnimportedRecordCommandEnum, reason: StaticPayloadEnumUnimportedWhyEnum) {
        send(
            StaticPayloadEnumUnimportedEvent.RecordList.Refused,
            EventMetadata(
                type = "external",
                typedPayload = StaticPayloadEnumUnimportedRecordListRefusedPayload(id, field, kind, reason),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "kind" to kind.declaredName, "reason" to reason.declaredName))
            )
        )
    }


    override val initialState: StaticPayloadEnumUnimportedState = StaticPayloadEnumUnimportedState.Listening

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
    override val documentInitialTargets: List<EntryTarget<StaticPayloadEnumUnimportedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticPayloadEnumUnimportedState, HistoryId>> =
            listOf(StateTarget(StaticPayloadEnumUnimportedState.Listening))

        // W3C SCXML 3.13: listening's transition 0, as the microstep reads it.
        val transitionListeningAt0 = EnabledTransition<StaticPayloadEnumUnimportedState, HistoryId>(
            StaticPayloadEnumUnimportedState.Listening,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticPayloadEnumUnimportedState? = when (stateId) {
        "listening" -> StaticPayloadEnumUnimportedState.Listening
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticPayloadEnumUnimportedState): String = when (state) {
        is StaticPayloadEnumUnimportedState.Listening -> "listening"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticPayloadEnumUnimportedState): Int = when (state) {
        is StaticPayloadEnumUnimportedState.Listening -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticPayloadEnumUnimportedEvent? = when (name) {
        "record_list.refused" -> StaticPayloadEnumUnimportedEvent.RecordList.Refused
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticPayloadEnumUnimportedEvent): String? = when (event) {
        is StaticPayloadEnumUnimportedEvent.RecordList.Refused -> "record_list.refused"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticPayloadEnumUnimportedState,
        event: StaticPayloadEnumUnimportedEvent?
    ): EnabledTransition<StaticPayloadEnumUnimportedState, HistoryId>? = when (state) {
        is StaticPayloadEnumUnimportedState.Listening -> when {
            event is StaticPayloadEnumUnimportedEvent.RecordList.Refused -> transitionListeningAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_payload_enum_unimported.scxml:16 :: _machine
    override fun onEntry(state: StaticPayloadEnumUnimportedState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticPayloadEnumUnimportedState.Listening -> {
                // SCE-MAP: static_payload_enum_unimported.scxml:26 :: listening :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_payload_enum_unimported.scxml:16 :: _machine
    override fun onExit(state: StaticPayloadEnumUnimportedState) {
        when (state) {
            is StaticPayloadEnumUnimportedState.Listening -> {
                // SCE-MAP: static_payload_enum_unimported.scxml:26 :: listening :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_payload_enum_unimported.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticPayloadEnumUnimportedState, transitionIndex: Int) {
        when (source) {
        is StaticPayloadEnumUnimportedState.Listening -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_payload_enum_unimported.scxml:27 :: listening :: _transition_0
                if (pendingRecordListRefusedPayload == null) {
                    return
                }

            vLast = pendingRecordListRefusedPayload!!.reason

            vRow = pendingRecordListRefusedPayload!!.id

            if (if (vKept.size < 2) { vKept = vKept + (StaticPayloadEnumUnimportedRefusedRecord(id = pendingRecordListRefusedPayload!!.id, field = pendingRecordListRefusedPayload!!.field, kind = pendingRecordListRefusedPayload!!.kind, reason = pendingRecordListRefusedPayload!!.reason)); false } else { true }) {
                return
            }

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
