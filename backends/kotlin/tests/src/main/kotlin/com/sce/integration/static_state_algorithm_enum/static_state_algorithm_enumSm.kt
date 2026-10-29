// SCE-GENERATED — DO NOT EDIT
// source-hash: adbae20e9ca5d62b09870f4a9e5e88119b90dbcf69963958bb021c42c5c2c6a4

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_state_algorithm_enum.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_state_algorithm_enum.scxml:22 :: _machine

package com.sce.integration.static_state_algorithm_enum

import com.sce.runtime.*
import com.sce.generated.enum_item_status.*
import com.sce.generated.item_status_count.*
import com.sce.generated.item_status_of.*
import com.sce.generated.item_status_set.*
import com.sce.generated.schema_item.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticStateAlgorithmEnumState : State {
    data object Board : StaticStateAlgorithmEnumState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticStateAlgorithmEnumEvent : Event {
    data object Check : StaticStateAlgorithmEnumEvent
    data object Count : StaticStateAlgorithmEnumEvent
    sealed interface Error : StaticStateAlgorithmEnumEvent {
        data object Execution : Error
    }
    sealed interface Item : StaticStateAlgorithmEnumEvent {
        data object Added : Item
        data object Moved : Item
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticStateAlgorithmEnumPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticStateAlgorithmEnumItemAddedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `item.added`. Consumers inject it via the `raiseItemAdded` seam
// on the machine — they never name this class directly.
data class StaticStateAlgorithmEnumItemAddedPayload(val id: UInt, val status: StaticStateAlgorithmEnumStatusEnum)

// StaticStateAlgorithmEnumItemMovedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `item.moved`. Consumers inject it via the `raiseItemMoved` seam
// on the machine — they never name this class directly.
data class StaticStateAlgorithmEnumItemMovedPayload(val id: UInt, val status: StaticStateAlgorithmEnumStatusEnum)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Status` datamodel value. */
enum class StaticStateAlgorithmEnumStatusEnum(val declaredName: String) {
    OPEN("open"),
    IN_PROGRESS("in_progress"),
    DONE("done");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        OPEN -> 2u.toUByte()
        IN_PROGRESS -> 5u.toUByte()
        DONE -> 7u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticStateAlgorithmEnumStatusEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Status")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticStateAlgorithmEnumStatusEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Item` datamodel value. */
data class StaticStateAlgorithmEnumItemRecord(val id: UInt, val status: StaticStateAlgorithmEnumStatusEnum) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "status" to status.toSaved())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticStateAlgorithmEnumItemRecord = StaticStateAlgorithmEnumItemRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), status = StaticStateAlgorithmEnumStatusEnum.fromSaved(SavedValues.field(value, what, "status"), "$what.status"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_CountStatus__v(p0: List<StaticStateAlgorithmEnumItemRecord>, p1: StaticStateAlgorithmEnumStatusEnum): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = itemStatusCount(p0.map { SchemaItemPayload(id = it.id, status = when (it.status) { StaticStateAlgorithmEnumStatusEnum.OPEN -> EnumItemStatus.OPEN; StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS -> EnumItemStatus.IN_PROGRESS; StaticStateAlgorithmEnumStatusEnum.DONE -> EnumItemStatus.DONE }) }, when (p1) { StaticStateAlgorithmEnumStatusEnum.OPEN -> EnumItemStatus.OPEN; StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS -> EnumItemStatus.IN_PROGRESS; StaticStateAlgorithmEnumStatusEnum.DONE -> EnumItemStatus.DONE })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_SetStatus__v(p0: List<StaticStateAlgorithmEnumItemRecord>, p1: UInt, p2: StaticStateAlgorithmEnumStatusEnum): com.sce.forge.runtime.AlgorithmResult<List<StaticStateAlgorithmEnumItemRecord>> =
    when (val r = itemStatusSet(p0.map { SchemaItemPayload(id = it.id, status = when (it.status) { StaticStateAlgorithmEnumStatusEnum.OPEN -> EnumItemStatus.OPEN; StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS -> EnumItemStatus.IN_PROGRESS; StaticStateAlgorithmEnumStatusEnum.DONE -> EnumItemStatus.DONE }) }, p1, when (p2) { StaticStateAlgorithmEnumStatusEnum.OPEN -> EnumItemStatus.OPEN; StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS -> EnumItemStatus.IN_PROGRESS; StaticStateAlgorithmEnumStatusEnum.DONE -> EnumItemStatus.DONE })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { StaticStateAlgorithmEnumItemRecord(id = it.id, status = when (it.status) { EnumItemStatus.OPEN -> StaticStateAlgorithmEnumStatusEnum.OPEN; EnumItemStatus.IN_PROGRESS -> StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS; EnumItemStatus.DONE -> StaticStateAlgorithmEnumStatusEnum.DONE }) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_StatusOf__v(p0: List<StaticStateAlgorithmEnumItemRecord>, p1: UInt): StaticStateAlgorithmEnumStatusEnum {
    val r = itemStatusOf(p0.map { SchemaItemPayload(id = it.id, status = when (it.status) { StaticStateAlgorithmEnumStatusEnum.OPEN -> EnumItemStatus.OPEN; StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS -> EnumItemStatus.IN_PROGRESS; StaticStateAlgorithmEnumStatusEnum.DONE -> EnumItemStatus.DONE }) }, p1)
    return when (r) { EnumItemStatus.OPEN -> StaticStateAlgorithmEnumStatusEnum.OPEN; EnumItemStatus.IN_PROGRESS -> StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS; EnumItemStatus.DONE -> StaticStateAlgorithmEnumStatusEnum.DONE }
}

// --- State Machine (W3C SCXML) ---

class StaticStateAlgorithmEnumStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticStateAlgorithmEnumState, StaticStateAlgorithmEnumEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `items` datamodel variable, published (`sce:direction="out"`). */
    private var vItems: List<StaticStateAlgorithmEnumItemRecord> = emptyList()

    /** What the published `items` holds. */
    val items: List<StaticStateAlgorithmEnumItemRecord> get() = vItems
    /** W3C SCXML 5.2: the `draft` datamodel variable, the machine's own. */
    private var vDraft: StaticStateAlgorithmEnumItemRecord = StaticStateAlgorithmEnumItemRecord(id = 0.toUInt(), status = StaticStateAlgorithmEnumStatusEnum.OPEN)
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticStateAlgorithmEnumStatusEnum = StaticStateAlgorithmEnumStatusEnum.OPEN

    /** What the published `last` holds. */
    val last: StaticStateAlgorithmEnumStatusEnum get() = vLast
    /** W3C SCXML 5.2: the `openCount` datamodel variable, published (`sce:direction="out"`). */
    private var vOpenCount: UInt = 0.toUInt()

    /** What the published `openCount` holds. */
    val openCount: UInt get() = vOpenCount
    /** W3C SCXML 5.2: the `startedCount` datamodel variable, published (`sce:direction="out"`). */
    private var vStartedCount: UInt = 0.toUInt()

    /** What the published `startedCount` holds. */
    val startedCount: UInt get() = vStartedCount
    /** W3C SCXML 5.2: the `doneCount` datamodel variable, published (`sce:direction="out"`). */
    private var vDoneCount: UInt = 0.toUInt()

    /** What the published `doneCount` holds. */
    val doneCount: UInt get() = vDoneCount
    /** W3C SCXML 5.2: the `checks` datamodel variable, published (`sce:direction="out"`). */
    private var vChecks: UInt = 0.toUInt()

    /** What the published `checks` holds. */
    val checks: UInt get() = vChecks
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
        var openCount: UInt? = null
        var startedCount: UInt? = null
        var doneCount: UInt? = null
        var checks: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.openCount?.let { vOpenCount = it }
        params.startedCount?.let { vStartedCount = it }
        params.doneCount?.let { vDoneCount = it }
        params.checks?.let { vChecks = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val items: List<StaticStateAlgorithmEnumItemRecord>,
        val last: StaticStateAlgorithmEnumStatusEnum,
        val openCount: UInt,
        val startedCount: UInt,
        val doneCount: UInt,
        val checks: UInt,
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
        val configuration: Set<StaticStateAlgorithmEnumState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        items = vItems,
        last = vLast,
        openCount = vOpenCount,
        startedCount = vStartedCount,
        doneCount = vDoneCount,
        checks = vChecks,
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
        first = "1281fa7afe0032f4ca9df6e2eab14d8e902566321a56264f5a071aa354e7b0a0",
        revised = "bd414277a1759a184a40341a429d5f4c069357ddf31a20ac51ba23bfbeca2bd3",
        format2First = "5addb038b50a9b6dd7ccc1d18de4809fbaed360a881124cf17816629947adb50",
        format2Revised = "5aa2ed9946edb35a3b75bb5bd278885e9d4926e0c2409ccf94c0221fbf435c3c",
        records = listOf(
            RecordLineage(
                event = "item.added",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("items", true),
                    RecordHolder("draft", false),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_item_status.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("items", "status", true),
                    EnumHolder("draft", "status", false),
                    EnumHolder("last", null, false),
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
            "items" to SavedValues.list(vItems) { it.toSaved() },
            "draft" to vDraft.toSaved(),
            "last" to vLast.toSaved(),
            "openCount" to SavedValues.of(vOpenCount),
            "startedCount" to SavedValues.of(vStartedCount),
            "doneCount" to SavedValues.of(vDoneCount),
            "checks" to SavedValues.of(vChecks),
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
        val saved1 = SavedValues.list(savedGrown.variable("items"), "items", 4) { e, w -> StaticStateAlgorithmEnumItemRecord.fromSaved(e, w) }
        val saved2 = StaticStateAlgorithmEnumItemRecord.fromSaved(savedGrown.variable("draft"), "draft")
        val saved3 = StaticStateAlgorithmEnumStatusEnum.fromSaved(savedGrown.variable("last"), "last")
        val saved4 = SavedValues.uint32(savedGrown.variable("openCount"), "openCount")
        val saved5 = SavedValues.uint32(savedGrown.variable("startedCount"), "startedCount")
        val saved6 = SavedValues.uint32(savedGrown.variable("doneCount"), "doneCount")
        val saved7 = SavedValues.uint32(savedGrown.variable("checks"), "checks")
        val saved8 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vItems = saved1
        vDraft = saved2
        vLast = saved3
        vOpenCount = saved4
        vStartedCount = saved5
        vDoneCount = saved6
        vChecks = saved7
        vRefusals = saved8
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        SavedValues.list(savedGrown.variable("items"), "items", 4) { e, w -> StaticStateAlgorithmEnumItemRecord.fromSaved(e, w) }
        StaticStateAlgorithmEnumItemRecord.fromSaved(savedGrown.variable("draft"), "draft")
        StaticStateAlgorithmEnumStatusEnum.fromSaved(savedGrown.variable("last"), "last")
        SavedValues.uint32(savedGrown.variable("openCount"), "openCount")
        SavedValues.uint32(savedGrown.variable("startedCount"), "startedCount")
        SavedValues.uint32(savedGrown.variable("doneCount"), "doneCount")
        SavedValues.uint32(savedGrown.variable("checks"), "checks")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingItemAddedPayload: StaticStateAlgorithmEnumItemAddedPayload? = null
    private var pendingItemMovedPayload: StaticStateAlgorithmEnumItemMovedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticStateAlgorithmEnumEvent, metadata: EventMetadata) {
        pendingItemAddedPayload = null
        pendingItemMovedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticStateAlgorithmEnumItemAddedPayload -> pendingItemAddedPayload = tp
            is StaticStateAlgorithmEnumItemMovedPayload -> pendingItemMovedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticStateAlgorithmEnumEvent.Item.Added) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingItemAddedPayload = StaticStateAlgorithmEnumItemAddedPayload(fields.uint32("id"), fields.string("status").let { name -> StaticStateAlgorithmEnumStatusEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'status' ($name) is not a variant of Status") })
                } else if (event == StaticStateAlgorithmEnumEvent.Item.Moved) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingItemMovedPayload = StaticStateAlgorithmEnumItemMovedPayload(fields.uint32("id"), fields.string("status").let { name -> StaticStateAlgorithmEnumStatusEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'status' ($name) is not a variant of Status") })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `item.added` — binds the event name and the payload field values in one call.
    fun raiseItemAdded(id: UInt, status: StaticStateAlgorithmEnumStatusEnum) {
        send(
            StaticStateAlgorithmEnumEvent.Item.Added,
            EventMetadata(
                type = "external",
                typedPayload = StaticStateAlgorithmEnumItemAddedPayload(id, status),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "status" to status.declaredName))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `item.moved` — binds the event name and the payload field values in one call.
    fun raiseItemMoved(id: UInt, status: StaticStateAlgorithmEnumStatusEnum) {
        send(
            StaticStateAlgorithmEnumEvent.Item.Moved,
            EventMetadata(
                type = "external",
                typedPayload = StaticStateAlgorithmEnumItemMovedPayload(id, status),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "status" to status.declaredName))
            )
        )
    }


    override val initialState: StaticStateAlgorithmEnumState = StaticStateAlgorithmEnumState.Board

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
    override val documentInitialTargets: List<EntryTarget<StaticStateAlgorithmEnumState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticStateAlgorithmEnumState, HistoryId>> =
            listOf(StateTarget(StaticStateAlgorithmEnumState.Board))

        // W3C SCXML 3.13: board's transition 0, as the microstep reads it.
        val transitionBoardAt0 = EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>(
            StaticStateAlgorithmEnumState.Board,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 1, as the microstep reads it.
        val transitionBoardAt1 = EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>(
            StaticStateAlgorithmEnumState.Board,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 2, as the microstep reads it.
        val transitionBoardAt2 = EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>(
            StaticStateAlgorithmEnumState.Board,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 3, as the microstep reads it.
        val transitionBoardAt3 = EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>(
            StaticStateAlgorithmEnumState.Board,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 4, as the microstep reads it.
        val transitionBoardAt4 = EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>(
            StaticStateAlgorithmEnumState.Board,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticStateAlgorithmEnumState? = when (stateId) {
        "board" -> StaticStateAlgorithmEnumState.Board
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticStateAlgorithmEnumState): String = when (state) {
        is StaticStateAlgorithmEnumState.Board -> "board"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticStateAlgorithmEnumState): Int = when (state) {
        is StaticStateAlgorithmEnumState.Board -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticStateAlgorithmEnumEvent? = when (name) {
        "check" -> StaticStateAlgorithmEnumEvent.Check
        "count" -> StaticStateAlgorithmEnumEvent.Count
        "error.execution" -> StaticStateAlgorithmEnumEvent.Error.Execution
        "item.added" -> StaticStateAlgorithmEnumEvent.Item.Added
        "item.moved" -> StaticStateAlgorithmEnumEvent.Item.Moved
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticStateAlgorithmEnumEvent): String? = when (event) {
        is StaticStateAlgorithmEnumEvent.Check -> "check"
        is StaticStateAlgorithmEnumEvent.Count -> "count"
        is StaticStateAlgorithmEnumEvent.Error.Execution -> "error.execution"
        is StaticStateAlgorithmEnumEvent.Item.Added -> "item.added"
        is StaticStateAlgorithmEnumEvent.Item.Moved -> "item.moved"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticStateAlgorithmEnumState,
        event: StaticStateAlgorithmEnumEvent?
    ): EnabledTransition<StaticStateAlgorithmEnumState, HistoryId>? = when (state) {
        is StaticStateAlgorithmEnumState.Board -> when {
            event is StaticStateAlgorithmEnumEvent.Item.Added -> transitionBoardAt0
            event is StaticStateAlgorithmEnumEvent.Item.Moved -> transitionBoardAt1
            event is StaticStateAlgorithmEnumEvent.Count -> transitionBoardAt2
            event is StaticStateAlgorithmEnumEvent.Check && call_StatusOf__v(vItems, 1.toUInt()) == StaticStateAlgorithmEnumStatusEnum.DONE -> transitionBoardAt3
            event is StaticStateAlgorithmEnumEvent.Error.Execution -> transitionBoardAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_state_algorithm_enum.scxml:22 :: _machine
    override fun onEntry(state: StaticStateAlgorithmEnumState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticStateAlgorithmEnumState.Board -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:43 :: board :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_state_algorithm_enum.scxml:22 :: _machine
    override fun onExit(state: StaticStateAlgorithmEnumState) {
        when (state) {
            is StaticStateAlgorithmEnumState.Board -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:43 :: board :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_state_algorithm_enum.scxml:22 :: _machine
    override fun executeTransitionContent(source: StaticStateAlgorithmEnumState, transitionIndex: Int) {
        when (source) {
        is StaticStateAlgorithmEnumState.Board -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:44 :: board :: _transition_0
                if (pendingItemAddedPayload == null) {
                    return
                }

            vDraft = StaticStateAlgorithmEnumItemRecord(id = pendingItemAddedPayload!!.id, status = pendingItemAddedPayload!!.status)

            if (if (vItems.size < 4) { vItems = vItems + (vDraft); false } else { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<sce:append target='items'>: the list already holds its capacity of 4"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:48 :: board :: _transition_1
                if (pendingItemMovedPayload == null) {
                    return
                }

            if (try { vItems = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_SetStatus__v(vItems, pendingItemMovedPayload!!.id, pendingItemMovedPayload!!.status)), 4); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='items'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLast = call_StatusOf__v(vItems, pendingItemMovedPayload!!.id)
            }
            2 -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:52 :: board :: _transition_2

            if (try { vOpenCount = com.sce.forge.runtime.SceChecked.take(call_CountStatus__v(vItems, StaticStateAlgorithmEnumStatusEnum.OPEN)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='openCount'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vStartedCount = com.sce.forge.runtime.SceChecked.take(call_CountStatus__v(vItems, StaticStateAlgorithmEnumStatusEnum.IN_PROGRESS)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='startedCount'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDoneCount = com.sce.forge.runtime.SceChecked.take(call_CountStatus__v(vItems, StaticStateAlgorithmEnumStatusEnum.DONE)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='doneCount'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:57 :: board :: _transition_3

            if (try { vChecks = com.sce.forge.runtime.SceChecked.add(vChecks, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='checks'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_state_algorithm_enum.scxml:60 :: board :: _transition_4

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEnumEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
