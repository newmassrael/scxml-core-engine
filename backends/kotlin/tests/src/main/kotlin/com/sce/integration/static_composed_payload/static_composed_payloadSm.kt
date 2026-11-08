// SCE-GENERATED — DO NOT EDIT
// source-hash: ea764ab5524bc7a7e7d4553640b3fb26547d13f0d2053c0f5e7214161efbc6d0

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_composed_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_composed_payload.scxml:28 :: _machine

package com.sce.integration.static_composed_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticComposedPayloadState : State {
    data object Idle : StaticComposedPayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticComposedPayloadEvent : Event {
    sealed interface Batch : StaticComposedPayloadEvent {
        data object Sent : Batch
    }
    sealed interface Error : StaticComposedPayloadEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticComposedPayloadPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticComposedPayloadBatchSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `batch.sent`. Consumers inject it via the `raiseBatchSent` seam
// on the machine — they never name this class directly.
data class StaticComposedPayloadBatchSentPayload(val items: List<StaticComposedPayloadItemRecord>, val grid: List<List<UByte>>, val levels: List<StaticComposedPayloadSignalLevelEnum>, val maybe: List<UShort?>, val spare: List<UByte>?, val note: String, val late: List<UByte>) {
    init {
        com.sce.runtime.ValueBounds.list("items", items, 3) { _, _ -> }
        com.sce.runtime.ValueBounds.list("grid", grid, 2) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.list(scePath0, sceValue0, 2) { _, _ -> } }
        com.sce.runtime.ValueBounds.list("levels", levels, 2) { _, _ -> }
        com.sce.runtime.ValueBounds.list("maybe", maybe, 2) { _, _ -> }
        com.sce.runtime.ValueBounds.optional("spare", spare) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.list(scePath0, sceValue0, 2) { _, _ -> } }
        com.sce.runtime.ValueBounds.text("note", note, 4)
        com.sce.runtime.ValueBounds.list("late", late, 2) { _, _ -> }
    }
}


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:SignalLevel` datamodel value. */
enum class StaticComposedPayloadSignalLevelEnum(val declaredName: String) {
    LOW("low"),
    HIGH("high"),
    CRITICAL("critical");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        LOW -> 0u.toUByte()
        HIGH -> 1u.toUByte()
        CRITICAL -> 2u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticComposedPayloadSignalLevelEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of SignalLevel")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticComposedPayloadSignalLevelEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Item` value that holds a composed value (docs/adr/0086). */
data class StaticComposedPayloadItemRecord(val id: UInt, val amount: Long, val label: String, val level: StaticComposedPayloadSignalLevelEnum, val due: Long?, val blob: ByteArray, val tags: List<String>) {
    init {
        com.sce.runtime.ValueBounds.text("label", label, 8)
        com.sce.runtime.ValueBounds.bytes("blob", blob, 4)
        com.sce.runtime.ValueBounds.list("tags", tags, 4) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 8) }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticComposedPayloadStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticComposedPayloadState, StaticComposedPayloadEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `item_count` datamodel variable, published (`sce:direction="out"`). */
    private var vItemCount: UInt = 0.toUInt()

    /** What the published `item_count` holds. */
    val itemCount: UInt get() = vItemCount
    /** W3C SCXML 5.2: the `tag_count` datamodel variable, published (`sce:direction="out"`). */
    private var vTagCount: UInt = 0.toUInt()

    /** What the published `tag_count` holds. */
    val tagCount: UInt get() = vTagCount
    /** W3C SCXML 5.2: the `word_count` datamodel variable, published (`sce:direction="out"`). */
    private var vWordCount: UInt = 0.toUInt()

    /** What the published `word_count` holds. */
    val wordCount: UInt get() = vWordCount
    /** W3C SCXML 5.2: the `id_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vIdSum: UInt = 0.toUInt()

    /** What the published `id_sum` holds. */
    val idSum: UInt get() = vIdSum
    /** W3C SCXML 5.2: the `cell_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vCellSum: UInt = 0.toUInt()

    /** What the published `cell_sum` holds. */
    val cellSum: UInt get() = vCellSum
    /** W3C SCXML 5.2: the `cell` datamodel variable, published (`sce:direction="out"`). */
    private var vCell: UByte = 0.toUByte()

    /** What the published `cell` holds. */
    val cell: UByte get() = vCell
    /** W3C SCXML 5.2: the `first_tags` datamodel variable, published (`sce:direction="out"`). */
    private var vFirstTags: UInt = 0.toUInt()

    /** What the published `first_tags` holds. */
    val firstTags: UInt get() = vFirstTags
    /** W3C SCXML 5.2: the `first_maybe` datamodel variable, published (`sce:direction="out"`). */
    private var vFirstMaybe: UShort? = null

    /** What the published `first_maybe` holds. */
    val firstMaybe: UShort? get() = vFirstMaybe
    /** W3C SCXML 5.2: the `late` datamodel variable, published (`sce:direction="out"`). */
    private var vLate: List<UByte> = emptyList()

    /** What the published `late` holds. */
    val late: List<UByte> get() = vLate
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
        var itemCount: UInt? = null
        var tagCount: UInt? = null
        var wordCount: UInt? = null
        var idSum: UInt? = null
        var cellSum: UInt? = null
        var cell: UByte? = null
        var firstTags: UInt? = null
        var firstMaybe: UShort? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.itemCount?.let { vItemCount = it }
        params.tagCount?.let { vTagCount = it }
        params.wordCount?.let { vWordCount = it }
        params.idSum?.let { vIdSum = it }
        params.cellSum?.let { vCellSum = it }
        params.cell?.let { vCell = it }
        params.firstTags?.let { vFirstTags = it }
        params.firstMaybe?.let { vFirstMaybe = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val itemCount: UInt,
        val tagCount: UInt,
        val wordCount: UInt,
        val idSum: UInt,
        val cellSum: UInt,
        val cell: UByte,
        val firstTags: UInt,
        val firstMaybe: UShort?,
        val late: List<UByte>,
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
        val configuration: Set<StaticComposedPayloadState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        itemCount = vItemCount,
        tagCount = vTagCount,
        wordCount = vWordCount,
        idSum = vIdSum,
        cellSum = vCellSum,
        cell = vCell,
        firstTags = vFirstTags,
        firstMaybe = vFirstMaybe,
        late = vLate,
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
        first = "9985104c525252fa61d9625e712b2dc2abed32d09af56313bf83994feb1e08a6",
        revised = "8a14de4a250c6df7ab2260410d5ef4bf9805edf2b7c8cbd7956f008d807eb01e",
        format2First = "9985104c525252fa61d9625e712b2dc2abed32d09af56313bf83994feb1e08a6",
        format2Revised = "8a14de4a250c6df7ab2260410d5ef4bf9805edf2b7c8cbd7956f008d807eb01e",
        records = listOf(
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
        variables += "item_count" to SavedValues.of(vItemCount)
        variables += "tag_count" to SavedValues.of(vTagCount)
        variables += "word_count" to SavedValues.of(vWordCount)
        variables += "id_sum" to SavedValues.of(vIdSum)
        variables += "cell_sum" to SavedValues.of(vCellSum)
        variables += "cell" to SavedValues.of(vCell)
        variables += "first_tags" to SavedValues.of(vFirstTags)
        variables += "first_maybe" to SavedValues.optional(vFirstMaybe) { SavedValues.of(it) }
        variables += "late" to SavedValues.list(vLate) { SavedValues.of(it) }
        variables += "refusals" to SavedValues.of(vRefusals)
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
        val saved1 = SavedValues.uint32(savedGrown.variable("item_count"), "item_count")
        val saved2 = SavedValues.uint32(savedGrown.variable("tag_count"), "tag_count")
        val saved3 = SavedValues.uint32(savedGrown.variable("word_count"), "word_count")
        val saved4 = SavedValues.uint32(savedGrown.variable("id_sum"), "id_sum")
        val saved5 = SavedValues.uint32(savedGrown.variable("cell_sum"), "cell_sum")
        val saved6 = SavedValues.uint8(savedGrown.variable("cell"), "cell")
        val saved7 = SavedValues.uint32(savedGrown.variable("first_tags"), "first_tags")
        val saved8 = SavedValues.optional(savedGrown.variable("first_maybe"), "first_maybe") { e, w -> SavedValues.uint16(e, w) }
        val saved9 = SavedValues.list(savedGrown.variable("late"), "late", 4) { e, w -> SavedValues.uint8(e, w) }
        val saved10 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        return {
            vItemCount = saved1
            vTagCount = saved2
            vWordCount = saved3
            vIdSum = saved4
            vCellSum = saved5
            vCell = saved6
            vFirstTags = saved7
            vFirstMaybe = saved8
            vLate = saved9
            vRefusals = saved10
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
        SavedValues.uint32(savedGrown.variable("item_count"), "item_count")
        SavedValues.uint32(savedGrown.variable("tag_count"), "tag_count")
        SavedValues.uint32(savedGrown.variable("word_count"), "word_count")
        SavedValues.uint32(savedGrown.variable("id_sum"), "id_sum")
        SavedValues.uint32(savedGrown.variable("cell_sum"), "cell_sum")
        SavedValues.uint8(savedGrown.variable("cell"), "cell")
        SavedValues.uint32(savedGrown.variable("first_tags"), "first_tags")
        SavedValues.optional(savedGrown.variable("first_maybe"), "first_maybe") { e, w -> SavedValues.uint16(e, w) }
        SavedValues.list(savedGrown.variable("late"), "late", 4) { e, w -> SavedValues.uint8(e, w) }
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingBatchSentPayload: StaticComposedPayloadBatchSentPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticComposedPayloadEvent, metadata: EventMetadata) {
        pendingBatchSentPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticComposedPayloadBatchSentPayload -> pendingBatchSentPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticComposedPayloadEvent.Batch.Sent) {
                    val fields = EventPayload.decode(metadata.data, "{\"late\":[]}")
                    pendingBatchSentPayload = StaticComposedPayloadBatchSentPayload(fields.field("items").let { it.list(3) { it.record(null) { StaticComposedPayloadItemRecord(id = it.field("id").let { it.uint32() }, amount = it.field("amount").let { it.int64() }, label = it.field("label").let { it.string(8) }, level = it.field("level").let { when (val sceName = it.name()) { "low" -> StaticComposedPayloadSignalLevelEnum.LOW; "high" -> StaticComposedPayloadSignalLevelEnum.HIGH; "critical" -> StaticComposedPayloadSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } }, due = it.field("due").let { it.optional { it.int64() } }, blob = it.field("blob").let { it.bytes(4) }, tags = it.field("tags").let { it.list(4) { it.string(8) } }) } } }, fields.field("grid").let { it.list(2) { it.list(2) { it.uint8() } } }, fields.field("levels").let { it.list(2) { when (val sceName = it.name()) { "low" -> StaticComposedPayloadSignalLevelEnum.LOW; "high" -> StaticComposedPayloadSignalLevelEnum.HIGH; "critical" -> StaticComposedPayloadSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } } }, fields.field("maybe").let { it.list(2) { it.optional { it.uint16() } } }, fields.field("spare").let { it.optional { it.list(2) { it.uint8() } } }, fields.field("note").let { it.string(4) }, fields.field("late").let { it.list(2) { it.uint8() } })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `batch.sent` — binds the event name and the payload field values in one call.
    fun raiseBatchSent(items: List<StaticComposedPayloadItemRecord>, grid: List<List<UByte>>, levels: List<StaticComposedPayloadSignalLevelEnum>, maybe: List<UShort?>, spare: List<UByte>?, note: String, late: List<UByte>) {
        val scePayload = StaticComposedPayloadBatchSentPayload(items, grid, levels, maybe, spare, note, late)
        send(
            StaticComposedPayloadEvent.Batch.Sent,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("items" to com.sce.runtime.EventPayload.listJson(scePayload.items) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("id" to com.sce.runtime.EventPayload.valueJson(it.id), "amount" to com.sce.runtime.EventPayload.valueJson(it.amount), "label" to com.sce.runtime.EventPayload.valueJson(it.label), "level" to com.sce.runtime.EventPayload.valueJson(it.level.declaredName), "due" to ((it.due)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)), "blob" to com.sce.runtime.EventPayload.valueJson(it.blob), "tags" to com.sce.runtime.EventPayload.listJson(it.tags) { com.sce.runtime.EventPayload.valueJson(it) })) }, "grid" to com.sce.runtime.EventPayload.listJson(scePayload.grid) { com.sce.runtime.EventPayload.listJson(it) { com.sce.runtime.EventPayload.valueJson(it) } }, "levels" to com.sce.runtime.EventPayload.listJson(scePayload.levels) { com.sce.runtime.EventPayload.valueJson(it.declaredName) }, "maybe" to com.sce.runtime.EventPayload.listJson(scePayload.maybe) { ((it)?.let { com.sce.runtime.EventPayload.valueJson(it) } ?: com.sce.runtime.EventPayload.valueJson(null)) }, "spare" to ((scePayload.spare)?.let { com.sce.runtime.EventPayload.listJson(it) { com.sce.runtime.EventPayload.valueJson(it) } } ?: com.sce.runtime.EventPayload.valueJson(null)), "note" to com.sce.runtime.EventPayload.valueJson(scePayload.note), "late" to com.sce.runtime.EventPayload.listJson(scePayload.late) { com.sce.runtime.EventPayload.valueJson(it) })).json
            )
        )
    }


    override val initialState: StaticComposedPayloadState = StaticComposedPayloadState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticComposedPayloadState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticComposedPayloadState, HistoryId>> =
            listOf(StateTarget(StaticComposedPayloadState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticComposedPayloadState, HistoryId>(
            StaticComposedPayloadState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticComposedPayloadState, HistoryId>(
            StaticComposedPayloadState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticComposedPayloadState? = when (stateId) {
        "idle" -> StaticComposedPayloadState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticComposedPayloadState): String = when (state) {
        is StaticComposedPayloadState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticComposedPayloadState): Int = when (state) {
        is StaticComposedPayloadState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticComposedPayloadEvent? = when (name) {
        "batch.sent" -> StaticComposedPayloadEvent.Batch.Sent
        "error.execution" -> StaticComposedPayloadEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticComposedPayloadEvent): String? = when (event) {
        is StaticComposedPayloadEvent.Batch.Sent -> "batch.sent"
        is StaticComposedPayloadEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticComposedPayloadState, event: StaticComposedPayloadEvent?): EnabledTransition<StaticComposedPayloadState, HistoryId>? = when (state) {
        is StaticComposedPayloadState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticComposedPayloadState, event: StaticComposedPayloadEvent?): EnabledTransition<StaticComposedPayloadState, HistoryId>? = when {
        event is StaticComposedPayloadEvent.Batch.Sent -> TransitionsPart0.transitionIdleAt0
        event is StaticComposedPayloadEvent.Error.Execution -> TransitionsPart0.transitionIdleAt1
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_composed_payload.scxml:28 :: _machine
    override fun onEntry(state: StaticComposedPayloadState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticComposedPayloadState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_composed_payload.scxml:49 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_composed_payload.scxml:28 :: _machine
    override fun onExit(state: StaticComposedPayloadState) {
        when (state) {
            is StaticComposedPayloadState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_composed_payload.scxml:49 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_composed_payload.scxml:28 :: _machine
    override fun executeTransitionContent(source: StaticComposedPayloadState, transitionIndex: Int) {
        when (source) {
            is StaticComposedPayloadState.Idle -> transitionContentOfIdle(transitionIndex)
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
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_composed_payload.scxml:50 :: idle :: _transition_0
                if (pendingBatchSentPayload == null) {
                    return
                }

            vItemCount = (pendingBatchSentPayload!!.items).size.toUInt()

            vLate = pendingBatchSentPayload!!.late


            for ((sce_position_of_i, row) in pendingBatchSentPayload!!.items.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()

            if (try { vTagCount = com.sce.forge.runtime.SceChecked.add(vTagCount, (row.tags).size.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='tag_count'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vIdSum = com.sce.forge.runtime.SceChecked.add(vIdSum, com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.items, (i).toLong()).id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='id_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (word in row.tags) {

            if (try { vWordCount = com.sce.forge.runtime.SceChecked.add(vWordCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='word_count'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            for (cells in pendingBatchSentPayload!!.grid) {


            for (n in cells) {

            if (try { vCellSum = com.sce.forge.runtime.SceChecked.add(vCellSum, n.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='cell_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            var ifCondFailed1 = false
            if ((try { (pendingBatchSentPayload!!.grid).size.toUInt() > 1.toUInt() && (com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.grid, (1).toLong())).size.toUInt() > 0.toUInt() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<if cond='len(_event.data.grid) > 1 && len(_event.data.grid[1]) > 0'>: an integer operation overflowed or failed"); ifCondFailed1 = true; false })) {

            if (try { vCell = com.sce.forge.runtime.SceChecked.at(com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.grid, (1).toLong()), (0).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='cell'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            if (ifCondFailed1) {
                return
            }


            if ((pendingBatchSentPayload!!.items).size.toUInt() > 0.toUInt()) {

            if (try { vFirstTags = (com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.items, (0).toLong()).tags).size.toUInt(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='first_tags'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if ((pendingBatchSentPayload!!.maybe).size.toUInt() > 0.toUInt()) {

            if (try { vFirstMaybe = com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.maybe, (0).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='first_maybe'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_composed_payload.scxml:77 :: idle :: _transition_1

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedPayloadEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
