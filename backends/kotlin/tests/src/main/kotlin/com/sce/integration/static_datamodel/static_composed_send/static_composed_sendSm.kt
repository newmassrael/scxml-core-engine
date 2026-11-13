// SCE-GENERATED — DO NOT EDIT
// source-hash: 138d54bef1f6b9b0e78600cb129e88342af00344f9fa58b6a90edf9adcaeb40d

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_composed_send.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_composed_send.scxml:41 :: _machine

package com.sce.integration.static_datamodel.static_composed_send

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticComposedSendState : State {
    data object Idle : StaticComposedSendState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticComposedSendEvent : Event {
    sealed interface Batch : StaticComposedSendEvent {
        data object Sent : Batch
    }
    sealed interface Counted : StaticComposedSendEvent {
        data object Sent : Counted
    }
    sealed interface Error : StaticComposedSendEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticComposedSendPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticComposedSendBatchSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `batch.sent`. Consumers inject it via the `raiseBatchSent` seam
// on the machine — they never name this class directly.
data class StaticComposedSendBatchSentPayload(val items: List<StaticComposedSendItemRecord>, val grid: List<List<UByte>>, val levels: List<StaticComposedSendSignalLevelEnum>, val maybe: List<UShort?>, val spare: List<UByte>?, val note: String, val late: List<UByte>) {
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

// StaticComposedSendCountedSentPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `counted.sent`. Consumers inject it via the `raiseCountedSent` seam
// on the machine — they never name this class directly.
data class StaticComposedSendCountedSentPayload(val kept: List<UByte>, val hops: UByte) {
    init {
        com.sce.runtime.ValueBounds.listWithin("kept", kept, 1, 2) { _, _ -> }
    }
}


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:SignalLevel` datamodel value. */
enum class StaticComposedSendSignalLevelEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticComposedSendSignalLevelEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of SignalLevel")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticComposedSendSignalLevelEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Item` value that holds a composed value (docs/adr/0086). */
data class StaticComposedSendItemRecord(val id: UInt, val amount: Long, val label: String, val level: StaticComposedSendSignalLevelEnum, val due: Long?, val blob: ByteArray, val tags: List<String>) {
    init {
        com.sce.runtime.ValueBounds.text("label", label, 8)
        com.sce.runtime.ValueBounds.bytes("blob", blob, 4)
        com.sce.runtime.ValueBounds.list("tags", tags, 4) { scePath0, sceValue0 -> com.sce.runtime.ValueBounds.text(scePath0, sceValue0, 8) }
    }

    /** What a value is read back from a saved state through (`fromSaved`). */
    companion object
}
// --- State Machine (W3C SCXML) ---

class StaticComposedSendStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticComposedSendState, StaticComposedSendEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `deliveries` datamodel variable, published (`sce:direction="out"`). */
    private var vDeliveries: UInt = 0.toUInt()

    /** What the published `deliveries` holds. */
    val deliveries: UInt get() = vDeliveries
    /** W3C SCXML 5.2: the `item_count` datamodel variable, published (`sce:direction="out"`). */
    private var vItemCount: UInt = 0.toUInt()

    /** What the published `item_count` holds. */
    val itemCount: UInt get() = vItemCount
    /** W3C SCXML 5.2: the `tag_count` datamodel variable, published (`sce:direction="out"`). */
    private var vTagCount: UInt = 0.toUInt()

    /** What the published `tag_count` holds. */
    val tagCount: UInt get() = vTagCount
    /** W3C SCXML 5.2: the `id_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vIdSum: UInt = 0.toUInt()

    /** What the published `id_sum` holds. */
    val idSum: UInt get() = vIdSum
    /** W3C SCXML 5.2: the `cell_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vCellSum: UInt = 0.toUInt()

    /** What the published `cell_sum` holds. */
    val cellSum: UInt get() = vCellSum
    /** W3C SCXML 5.2: the `level_count` datamodel variable, published (`sce:direction="out"`). */
    private var vLevelCount: UInt = 0.toUInt()

    /** What the published `level_count` holds. */
    val levelCount: UInt get() = vLevelCount
    /** W3C SCXML 5.2: the `first_maybe` datamodel variable, published (`sce:direction="out"`). */
    private var vFirstMaybe: UShort? = null

    /** What the published `first_maybe` holds. */
    val firstMaybe: UShort? get() = vFirstMaybe
    /** W3C SCXML 5.2: the `late_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vLateSum: UInt = 0.toUInt()

    /** What the published `late_sum` holds. */
    val lateSum: UInt get() = vLateSum
    /** W3C SCXML 5.2: the `late` datamodel variable, published (`sce:direction="out"`). */
    private var vLate: List<UByte> = emptyList()

    /** What the published `late` holds. */
    val late: List<UByte> get() = vLate
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: List<UByte> = emptyList()
    /** W3C SCXML 5.2: the `counted_deliveries` datamodel variable, published (`sce:direction="out"`). */
    private var vCountedDeliveries: UInt = 0.toUInt()

    /** What the published `counted_deliveries` holds. */
    val countedDeliveries: UInt get() = vCountedDeliveries
    /** W3C SCXML 5.2: the `counted_sum` datamodel variable, published (`sce:direction="out"`). */
    private var vCountedSum: UInt = 0.toUInt()

    /** What the published `counted_sum` holds. */
    val countedSum: UInt get() = vCountedSum
    /** W3C SCXML 5.2: the `counted_hops` datamodel variable, published (`sce:direction="out"`). */
    private var vCountedHops: UByte = 0.toUByte()

    /** What the published `counted_hops` holds. */
    val countedHops: UByte get() = vCountedHops

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var deliveries: UInt? = null
        var itemCount: UInt? = null
        var tagCount: UInt? = null
        var idSum: UInt? = null
        var cellSum: UInt? = null
        var levelCount: UInt? = null
        var firstMaybe: UShort? = null
        var lateSum: UInt? = null
        var refusals: UInt? = null
        var countedDeliveries: UInt? = null
        var countedSum: UInt? = null
        var countedHops: UByte? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.deliveries?.let { vDeliveries = it }
        params.itemCount?.let { vItemCount = it }
        params.tagCount?.let { vTagCount = it }
        params.idSum?.let { vIdSum = it }
        params.cellSum?.let { vCellSum = it }
        params.levelCount?.let { vLevelCount = it }
        params.firstMaybe?.let { vFirstMaybe = it }
        params.lateSum?.let { vLateSum = it }
        params.refusals?.let { vRefusals = it }
        params.countedDeliveries?.let { vCountedDeliveries = it }
        params.countedSum?.let { vCountedSum = it }
        params.countedHops?.let { vCountedHops = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val deliveries: UInt,
        val itemCount: UInt,
        val tagCount: UInt,
        val idSum: UInt,
        val cellSum: UInt,
        val levelCount: UInt,
        val firstMaybe: UShort?,
        val lateSum: UInt,
        val late: List<UByte>,
        val refusals: UInt,
        val countedDeliveries: UInt,
        val countedSum: UInt,
        val countedHops: UByte,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticComposedSendState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        deliveries = vDeliveries,
        itemCount = vItemCount,
        tagCount = vTagCount,
        idSum = vIdSum,
        cellSum = vCellSum,
        levelCount = vLevelCount,
        firstMaybe = vFirstMaybe,
        lateSum = vLateSum,
        late = vLate,
        refusals = vRefusals,
        countedDeliveries = vCountedDeliveries,
        countedSum = vCountedSum,
        countedHops = vCountedHops,
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
        first = "1e90a6306dff5c26ad4140c585404fe0fb4ed6167709cdddf054d940e56091b1",
        revised = "c45313f157703b7337989e2ff1d45441c09d2892a1a4a8a65e2046918f346378",
        format2First = "1e90a6306dff5c26ad4140c585404fe0fb4ed6167709cdddf054d940e56091b1",
        format2Revised = "c45313f157703b7337989e2ff1d45441c09d2892a1a4a8a65e2046918f346378",
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
        variables += "deliveries" to SavedValues.of(vDeliveries)
        variables += "item_count" to SavedValues.of(vItemCount)
        variables += "tag_count" to SavedValues.of(vTagCount)
        variables += "id_sum" to SavedValues.of(vIdSum)
        variables += "cell_sum" to SavedValues.of(vCellSum)
        variables += "level_count" to SavedValues.of(vLevelCount)
        variables += "first_maybe" to SavedValues.optional(vFirstMaybe) { SavedValues.of(it) }
        variables += "late_sum" to SavedValues.of(vLateSum)
        variables += "late" to SavedValues.list(vLate) { SavedValues.of(it) }
        variables += "refusals" to SavedValues.of(vRefusals)
        variables += "held" to SavedValues.list(vHeld) { SavedValues.of(it) }
        variables += "counted_deliveries" to SavedValues.of(vCountedDeliveries)
        variables += "counted_sum" to SavedValues.of(vCountedSum)
        variables += "counted_hops" to SavedValues.of(vCountedHops)
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
        val saved1 = SavedValues.uint32(savedGrown.variable("deliveries"), "deliveries")
        val saved2 = SavedValues.uint32(savedGrown.variable("item_count"), "item_count")
        val saved3 = SavedValues.uint32(savedGrown.variable("tag_count"), "tag_count")
        val saved4 = SavedValues.uint32(savedGrown.variable("id_sum"), "id_sum")
        val saved5 = SavedValues.uint32(savedGrown.variable("cell_sum"), "cell_sum")
        val saved6 = SavedValues.uint32(savedGrown.variable("level_count"), "level_count")
        val saved7 = SavedValues.optional(savedGrown.variable("first_maybe"), "first_maybe") { e, w -> SavedValues.uint16(e, w) }
        val saved8 = SavedValues.uint32(savedGrown.variable("late_sum"), "late_sum")
        val saved9 = SavedValues.list(savedGrown.variable("late"), "late", 2) { e, w -> SavedValues.uint8(e, w) }
        val saved10 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        val saved11 = SavedValues.list(savedGrown.variable("held"), "held", 2) { e, w -> SavedValues.uint8(e, w) }
        val saved12 = SavedValues.uint32(savedGrown.variable("counted_deliveries"), "counted_deliveries")
        val saved13 = SavedValues.uint32(savedGrown.variable("counted_sum"), "counted_sum")
        val saved14 = SavedValues.uint8(savedGrown.variable("counted_hops"), "counted_hops")
        return {
            vDeliveries = saved1
            vItemCount = saved2
            vTagCount = saved3
            vIdSum = saved4
            vCellSum = saved5
            vLevelCount = saved6
            vFirstMaybe = saved7
            vLateSum = saved8
            vLate = saved9
            vRefusals = saved10
            vHeld = saved11
            vCountedDeliveries = saved12
            vCountedSum = saved13
            vCountedHops = saved14
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
        SavedValues.uint32(savedGrown.variable("deliveries"), "deliveries")
        SavedValues.uint32(savedGrown.variable("item_count"), "item_count")
        SavedValues.uint32(savedGrown.variable("tag_count"), "tag_count")
        SavedValues.uint32(savedGrown.variable("id_sum"), "id_sum")
        SavedValues.uint32(savedGrown.variable("cell_sum"), "cell_sum")
        SavedValues.uint32(savedGrown.variable("level_count"), "level_count")
        SavedValues.optional(savedGrown.variable("first_maybe"), "first_maybe") { e, w -> SavedValues.uint16(e, w) }
        SavedValues.uint32(savedGrown.variable("late_sum"), "late_sum")
        SavedValues.list(savedGrown.variable("late"), "late", 2) { e, w -> SavedValues.uint8(e, w) }
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        SavedValues.list(savedGrown.variable("held"), "held", 2) { e, w -> SavedValues.uint8(e, w) }
        SavedValues.uint32(savedGrown.variable("counted_deliveries"), "counted_deliveries")
        SavedValues.uint32(savedGrown.variable("counted_sum"), "counted_sum")
        SavedValues.uint8(savedGrown.variable("counted_hops"), "counted_hops")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingBatchSentPayload: StaticComposedSendBatchSentPayload? = null
    private var pendingCountedSentPayload: StaticComposedSendCountedSentPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticComposedSendEvent, metadata: EventMetadata) {
        pendingBatchSentPayload = null
        pendingCountedSentPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticComposedSendBatchSentPayload -> pendingBatchSentPayload = tp
            is StaticComposedSendCountedSentPayload -> pendingCountedSentPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticComposedSendEvent.Batch.Sent) {
                    val fields = EventPayload.decode(metadata.data, "{\"late\":[]}")
                    pendingBatchSentPayload = StaticComposedSendBatchSentPayload(fields.field("items").let { it.list(3) { it.record(null) { StaticComposedSendItemRecord(id = it.field("id").let { it.uint32() }, amount = it.field("amount").let { it.int64() }, label = it.field("label").let { it.string(8) }, level = it.field("level").let { when (val sceName = it.name()) { "low" -> StaticComposedSendSignalLevelEnum.LOW; "high" -> StaticComposedSendSignalLevelEnum.HIGH; "critical" -> StaticComposedSendSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } }, due = it.field("due").let { it.optional { it.int64() } }, blob = it.field("blob").let { it.bytes(4) }, tags = it.field("tags").let { it.list(4) { it.string(8) } }) } } }, fields.field("grid").let { it.list(2) { it.list(2) { it.uint8() } } }, fields.field("levels").let { it.list(2) { when (val sceName = it.name()) { "low" -> StaticComposedSendSignalLevelEnum.LOW; "high" -> StaticComposedSendSignalLevelEnum.HIGH; "critical" -> StaticComposedSendSignalLevelEnum.CRITICAL; else -> throw it.unknownVariant(sceName, "SignalLevel") } } }, fields.field("maybe").let { it.list(2) { it.optional { it.uint16() } } }, fields.field("spare").let { it.optional { it.list(2) { it.uint8() } } }, fields.field("note").let { it.string(4) }, fields.field("late").let { it.list(2) { it.uint8() } })
                } else if (event == StaticComposedSendEvent.Counted.Sent) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingCountedSentPayload = StaticComposedSendCountedSentPayload(fields.field("kept").let { it.listWithin(1, 2) { it.uint8() } }, fields.field("hops").let { it.uint8() })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `batch.sent` — binds the event name and the payload field values in one call.
    fun raiseBatchSent(items: List<StaticComposedSendItemRecord>, grid: List<List<UByte>>, levels: List<StaticComposedSendSignalLevelEnum>, maybe: List<UShort?>, spare: List<UByte>?, note: String, late: List<UByte>) {
        val scePayload = StaticComposedSendBatchSentPayload(items, grid, levels, maybe, spare, note, late)
        send(
            StaticComposedSendEvent.Batch.Sent,
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
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `counted.sent` — binds the event name and the payload field values in one call.
    fun raiseCountedSent(kept: List<UByte>, hops: UByte) {
        val scePayload = StaticComposedSendCountedSentPayload(kept, hops)
        send(
            StaticComposedSendEvent.Counted.Sent,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("kept" to com.sce.runtime.EventPayload.listJson(scePayload.kept) { com.sce.runtime.EventPayload.valueJson(it) }, "hops" to com.sce.runtime.EventPayload.valueJson(scePayload.hops))).json
            )
        )
    }


    override val initialState: StaticComposedSendState = StaticComposedSendState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticComposedSendState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticComposedSendState, HistoryId>> =
            listOf(StateTarget(StaticComposedSendState.Idle))
    }

    private object TransitionsPart0 {

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticComposedSendState, HistoryId>(
            StaticComposedSendState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticComposedSendState, HistoryId>(
            StaticComposedSendState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticComposedSendState, HistoryId>(
            StaticComposedSendState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticComposedSendState, HistoryId>(
            StaticComposedSendState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticComposedSendState, HistoryId>(
            StaticComposedSendState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }


    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticComposedSendState? = when (stateId) {
        "idle" -> StaticComposedSendState.Idle
        else -> null
    }


    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticComposedSendState): String = when (state) {
        is StaticComposedSendState.Idle -> "idle"
    }


    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticComposedSendState): Int = when (state) {
        is StaticComposedSendState.Idle -> 0
    }


    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticComposedSendEvent? = when (name) {
        "batch.sent" -> StaticComposedSendEvent.Batch.Sent
        "counted.sent" -> StaticComposedSendEvent.Counted.Sent
        "error.execution" -> StaticComposedSendEvent.Error.Execution
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticComposedSendEvent): String? = when (event) {
        is StaticComposedSendEvent.Batch.Sent -> "batch.sent"
        is StaticComposedSendEvent.Counted.Sent -> "counted.sent"
        is StaticComposedSendEvent.Error.Execution -> "error.execution"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: StaticComposedSendState, event: StaticComposedSendEvent?): EnabledTransition<StaticComposedSendState, HistoryId>? = when (state) {
        is StaticComposedSendState.Idle -> firstEnabledTransitionOfIdle(state, event)
    }


    // idle's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfIdle(state: StaticComposedSendState, event: StaticComposedSendEvent?): EnabledTransition<StaticComposedSendState, HistoryId>? = when {
        event is StaticComposedSendEvent.Batch.Sent && vDeliveries == 0.toUInt() -> TransitionsPart0.transitionIdleAt0
        event is StaticComposedSendEvent.Batch.Sent -> TransitionsPart0.transitionIdleAt1
        event is StaticComposedSendEvent.Counted.Sent && vCountedDeliveries == 0.toUInt() -> TransitionsPart0.transitionIdleAt2
        event is StaticComposedSendEvent.Counted.Sent -> TransitionsPart0.transitionIdleAt3
        event is StaticComposedSendEvent.Error.Execution -> TransitionsPart0.transitionIdleAt4
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_composed_send.scxml:41 :: _machine
    override fun onEntry(state: StaticComposedSendState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticComposedSendState.Idle -> onEntryOfIdle()
        }
    }


    private fun onEntryOfIdle() {
                // SCE-MAP: static_composed_send.scxml:67 :: idle :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_composed_send.scxml:41 :: _machine
    override fun onExit(state: StaticComposedSendState) {
        when (state) {
            is StaticComposedSendState.Idle -> onExitOfIdle()
        }
    }


    private fun onExitOfIdle() {
                // SCE-MAP: static_composed_send.scxml:67 :: idle :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_composed_send.scxml:41 :: _machine
    override fun executeTransitionContent(source: StaticComposedSendState, transitionIndex: Int) {
        when (source) {
            is StaticComposedSendState.Idle -> transitionContentOfIdle(transitionIndex)
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
            else -> {}
        }
    }


    private fun transitionContentOfIdleAt0() {
                // SCE-MAP: static_composed_send.scxml:70 :: idle :: _transition_0
                if (pendingBatchSentPayload == null) {
                    return
                }

            vDeliveries = 1.toUInt()

            vLate = pendingBatchSentPayload!!.late


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "items", (pendingBatchSentPayload!!.items).map { mapOf<String, Any?>("id" to (it.id).toLong(), "amount" to (it.amount).toString(), "label" to it.label, "level" to it.level.declaredName, "due" to ((it.due)?.let { (it).toString() }), "blob" to com.sce.runtime.EventPayload.bytesAsText(it.blob), "tags" to (it.tags).map { it }) })

            putParam(sendPayload, "grid", (pendingBatchSentPayload!!.grid).map { (it).map { (it).toLong() } })

            putParam(sendPayload, "levels", (pendingBatchSentPayload!!.levels).map { it.declaredName })

            putParam(sendPayload, "maybe", (pendingBatchSentPayload!!.maybe).map { ((it)?.let { (it).toLong() }) })

            putParam(sendPayload, "spare", ((pendingBatchSentPayload!!.spare)?.let { (it).map { (it).toLong() } }))

            putParam(sendPayload, "note", "echo")
            putParam(sendPayload, "late", (vLate).map { (it).toLong() })

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(StaticComposedSendEvent.Batch.Sent, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "kept", run { if ((vHeld).size < 1) throw com.sce.forge.runtime.AlgorithmFailure(com.sce.forge.runtime.AlgorithmError.Precondition); (vHeld).map { (it).toLong() } })
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                // docs/adr/0086, section 15.6: the list is short of its field's lower
                // count, so the event is refused whole: one error, nothing delivered,
                // and the error ends the block (W3C SCXML 6.2.3, 4.9).
                raisePlatformError(StaticComposedSendEvent.Error.Execution, "<send> <param name='kept'> holds fewer elements than the 1 its field's type declares, so the event is not sent", "__send_1")
                return@send true
            }

            putParam(sendPayload, "hops", (3).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(StaticComposedSendEvent.Counted.Sent, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfIdleAt1() {
                // SCE-MAP: static_composed_send.scxml:90 :: idle :: _transition_1
                if (pendingBatchSentPayload == null) {
                    return
                }

            if (try { vDeliveries = com.sce.forge.runtime.SceChecked.add(vDeliveries, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='deliveries'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vItemCount = (pendingBatchSentPayload!!.items).size.toUInt()

            vLevelCount = (pendingBatchSentPayload!!.levels).size.toUInt()


            for ((sce_position_of_i, row) in pendingBatchSentPayload!!.items.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()

            if (try { vTagCount = com.sce.forge.runtime.SceChecked.add(vTagCount, (row.tags).size.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='tag_count'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vIdSum = com.sce.forge.runtime.SceChecked.add(vIdSum, com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.items, (i).toLong()).id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='id_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (cells in pendingBatchSentPayload!!.grid) {


            for (n in cells) {

            if (try { vCellSum = com.sce.forge.runtime.SceChecked.add(vCellSum, n.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='cell_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            for (each in pendingBatchSentPayload!!.late) {

            if (try { vLateSum = com.sce.forge.runtime.SceChecked.add(vLateSum, each.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='late_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if ((pendingBatchSentPayload!!.maybe).size.toUInt() > 0.toUInt()) {

            if (try { vFirstMaybe = com.sce.forge.runtime.SceChecked.at(pendingBatchSentPayload!!.maybe, (0).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='first_maybe'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (if (vHeld.size < 2) { vHeld = vHeld + (7.toUByte()); false } else { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<sce:append target='held'>: the list already holds its capacity of 2"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "kept", run { if ((vHeld).size < 1) throw com.sce.forge.runtime.AlgorithmFailure(com.sce.forge.runtime.AlgorithmError.Precondition); (vHeld).map { (it).toLong() } })
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                // docs/adr/0086, section 15.6: the list is short of its field's lower
                // count, so the event is refused whole: one error, nothing delivered,
                // and the error ends the block (W3C SCXML 6.2.3, 4.9).
                raisePlatformError(StaticComposedSendEvent.Error.Execution, "<send> <param name='kept'> holds fewer elements than the 1 its field's type declares, so the event is not sent", "__send_2")
                return@send true
            }

            putParam(sendPayload, "hops", (3).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(StaticComposedSendEvent.Counted.Sent, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfIdleAt2() {
                // SCE-MAP: static_composed_send.scxml:118 :: idle :: _transition_2
                if (pendingCountedSentPayload == null) {
                    return
                }

            vCountedDeliveries = 1.toUInt()


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kept", (pendingCountedSentPayload!!.kept).map { (it).toLong() })

            putParam(sendPayload, "hops", (pendingCountedSentPayload!!.hops).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(StaticComposedSendEvent.Counted.Sent, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfIdleAt3() {
                // SCE-MAP: static_composed_send.scxml:126 :: idle :: _transition_3
                if (pendingCountedSentPayload == null) {
                    return
                }

            if (try { vCountedDeliveries = com.sce.forge.runtime.SceChecked.add(vCountedDeliveries, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='counted_deliveries'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vCountedHops = pendingCountedSentPayload!!.hops


            for (each in pendingCountedSentPayload!!.kept) {

            if (try { vCountedSum = com.sce.forge.runtime.SceChecked.add(vCountedSum, each.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='counted_sum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfIdleAt4() {
                // SCE-MAP: static_composed_send.scxml:135 :: idle :: _transition_4

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticComposedSendEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
