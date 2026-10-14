// SCE-GENERATED — DO NOT EDIT
// source-hash: 299edcd86435fd0aaceb858143fb6e6689d45f9269775736c7c99fe0feb97e4c

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/sheet_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: sheet_peer.scxml:34 :: _machine

package com.sce.integration.sheet_peer

import com.sce.runtime.*
import com.sce.generated.hlc_receive.*
import com.sce.generated.hlc_send.*
import com.sce.generated.hlc_timestamp.*
import com.sce.generated.lww_cell.*
import com.sce.generated.lww_write.*
import com.sce.generated.rga_gone.*
import com.sce.generated.rga_holds.*
import com.sce.generated.rga_insert.*
import com.sce.generated.rga_node.*
import com.sce.generated.rga_remove.*


// --- States (W3C SCXML 3.2) ---

sealed interface SheetPeerState : State {
    data object Open : SheetPeerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface SheetPeerEvent : Event {
    sealed interface Error : SheetPeerEvent {
        data object Execution : Error
    }
    sealed interface Sheet : SheetPeerEvent {
        data object AddCol : Sheet
        data object AddRow : Sheet
        data object Cell : Sheet
        data object Col : Sheet
        data object DropCol : Sheet
        data object DropRow : Sheet
        data object Edit : Sheet
        data object Gone : Sheet
        data object GoneCol : Sheet
        data object Join : Sheet
        data object Row : Sheet
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `SheetPeerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// SheetPeerSheetAddColPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.add_col`. Consumers inject it via the `raiseSheetAddCol` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetAddColPayload(val after: ULong)

// SheetPeerSheetAddRowPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.add_row`. Consumers inject it via the `raiseSheetAddRow` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetAddRowPayload(val after: ULong)

// SheetPeerSheetCellPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.cell`. Consumers inject it via the `raiseSheetCell` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetCellPayload(val row: ULong, val col: ULong, val value: Long, val wallTime: Long, val counter: UInt, val nodeId: ULong)

// SheetPeerSheetColPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.col`. Consumers inject it via the `raiseSheetCol` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetColPayload(val key: ULong, val after: ULong)

// SheetPeerSheetDropColPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.drop_col`. Consumers inject it via the `raiseSheetDropCol` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetDropColPayload(val col: ULong)

// SheetPeerSheetDropRowPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.drop_row`. Consumers inject it via the `raiseSheetDropRow` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetDropRowPayload(val row: ULong)

// SheetPeerSheetEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.edit`. Consumers inject it via the `raiseSheetEdit` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetEditPayload(val row: ULong, val col: ULong, val value: Long, val now: Long)

// SheetPeerSheetGonePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.gone`. Consumers inject it via the `raiseSheetGone` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetGonePayload(val key: ULong, val by: ULong)

// SheetPeerSheetGoneColPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.gone_col`. Consumers inject it via the `raiseSheetGoneCol` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetGoneColPayload(val key: ULong, val by: ULong)

// SheetPeerSheetJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.join`. Consumers inject it via the `raiseSheetJoin` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetJoinPayload(val node: ULong, val hub: Boolean)

// SheetPeerSheetRowPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `sheet.row`. Consumers inject it via the `raiseSheetRow` seam
// on the machine — they never name this class directly.
data class SheetPeerSheetRowPayload(val key: ULong, val after: ULong)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Hlc` datamodel value. */
data class SheetPeerHlcRecord(val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): SheetPeerHlcRecord = SheetPeerHlcRecord(wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:LwwCell` datamodel value. */
data class SheetPeerLwwCellRecord(val row: ULong, val col: ULong, val value: Long, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("row" to SavedValues.of(row), "col" to SavedValues.of(col), "value" to SavedValues.of(value), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): SheetPeerLwwCellRecord = SheetPeerLwwCellRecord(row = SavedValues.uint64(SavedValues.field(value, what, "row"), "$what.row"), col = SavedValues.uint64(SavedValues.field(value, what, "col"), "$what.col"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:RgaNode` datamodel value. */
data class SheetPeerRgaNodeRecord(val key: ULong, val after: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("key" to SavedValues.of(key), "after" to SavedValues.of(after))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): SheetPeerRgaNodeRecord = SheetPeerRgaNodeRecord(key = SavedValues.uint64(SavedValues.field(value, what, "key"), "$what.key"), after = SavedValues.uint64(SavedValues.field(value, what, "after"), "$what.after"))
    }
}
/** SCE Accepted Subset §2.15: a `record:RgaGone` datamodel value. */
data class SheetPeerRgaGoneRecord(val key: ULong, val by: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("key" to SavedValues.of(key), "by" to SavedValues.of(by))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): SheetPeerRgaGoneRecord = SheetPeerRgaGoneRecord(key = SavedValues.uint64(SavedValues.field(value, what, "key"), "$what.key"), by = SavedValues.uint64(SavedValues.field(value, what, "by"), "$what.by"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_HlcReceive__vv(p0: SheetPeerHlcRecord, p1: SheetPeerHlcRecord, p2: Long, p3: ULong): com.sce.forge.runtime.AlgorithmResult<SheetPeerHlcRecord> =
    when (val r = hlcReceive(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), HlcTimestampPayload(wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId), p2, p3)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(SheetPeerHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_HlcSend__v(p0: SheetPeerHlcRecord, p1: Long, p2: ULong): com.sce.forge.runtime.AlgorithmResult<SheetPeerHlcRecord> =
    when (val r = hlcSend(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(SheetPeerHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_LwwWrite__vv(p0: List<SheetPeerLwwCellRecord>, p1: SheetPeerLwwCellRecord): com.sce.forge.runtime.AlgorithmResult<List<SheetPeerLwwCellRecord>> =
    when (val r = lwwWrite(p0.map { LwwCellPayload(row = it.row, col = it.col, value = it.value, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, LwwCellPayload(row = p1.row, col = p1.col, value = p1.value, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { SheetPeerLwwCellRecord(row = it.row, col = it.col, value = it.value, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_RgaHolds__v(p0: List<SheetPeerRgaNodeRecord>, p1: ULong): Boolean {
    val r = rgaHolds(p0.map { RgaNodePayload(key = it.key, after = it.after) }, p1)
    return r
}
private fun call_RgaInsert__vv(p0: List<SheetPeerRgaNodeRecord>, p1: SheetPeerRgaNodeRecord): com.sce.forge.runtime.AlgorithmResult<List<SheetPeerRgaNodeRecord>> =
    when (val r = rgaInsert(p0.map { RgaNodePayload(key = it.key, after = it.after) }, RgaNodePayload(key = p1.key, after = p1.after))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { SheetPeerRgaNodeRecord(key = it.key, after = it.after) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_RgaRemove__vv(p0: List<SheetPeerRgaGoneRecord>, p1: SheetPeerRgaGoneRecord): com.sce.forge.runtime.AlgorithmResult<List<SheetPeerRgaGoneRecord>> =
    when (val r = rgaRemove(p0.map { RgaGonePayload(key = it.key, by = it.by) }, RgaGonePayload(key = p1.key, by = p1.by))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { SheetPeerRgaGoneRecord(key = it.key, by = it.by) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class SheetPeerStateMachine(
) : StateMachineEngine<SheetPeerState, SheetPeerEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `node` datamodel variable, published (`sce:direction="out"`). */
    private var vNode: ULong = 0.toULong()

    /** What the published `node` holds. */
    val node: ULong get() = vNode
    /** W3C SCXML 5.2: the `hub` datamodel variable, published (`sce:direction="out"`). */
    private var vHub: Boolean = false

    /** What the published `hub` holds. */
    val hub: Boolean get() = vHub
    /** W3C SCXML 5.2: the `clock` datamodel variable, published (`sce:direction="out"`). */
    private var vClock: SheetPeerHlcRecord = SheetPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `stamp` datamodel variable, the machine's own. */
    private var vStamp: SheetPeerHlcRecord = SheetPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `cells` datamodel variable, published (`sce:direction="out"`). */
    private var vCells: List<SheetPeerLwwCellRecord> = emptyList()

    /** What the published `cells` holds. */
    val cells: List<SheetPeerLwwCellRecord> get() = vCells
    /** W3C SCXML 5.2: the `rows` datamodel variable, published (`sce:direction="out"`). */
    private var vRows: List<SheetPeerRgaNodeRecord> = emptyList()

    /** What the published `rows` holds. */
    val rows: List<SheetPeerRgaNodeRecord> get() = vRows
    /** W3C SCXML 5.2: the `gone` datamodel variable, published (`sce:direction="out"`). */
    private var vGone: List<SheetPeerRgaGoneRecord> = emptyList()

    /** What the published `gone` holds. */
    val gone: List<SheetPeerRgaGoneRecord> get() = vGone
    /** W3C SCXML 5.2: the `cols` datamodel variable, published (`sce:direction="out"`). */
    private var vCols: List<SheetPeerRgaNodeRecord> = emptyList()

    /** What the published `cols` holds. */
    val cols: List<SheetPeerRgaNodeRecord> get() = vCols
    /** W3C SCXML 5.2: the `goneCols` datamodel variable, published (`sce:direction="out"`). */
    private var vGoneCols: List<SheetPeerRgaGoneRecord> = emptyList()

    /** What the published `goneCols` holds. */
    val goneCols: List<SheetPeerRgaGoneRecord> get() = vGoneCols
    /** W3C SCXML 5.2: the `write` datamodel variable, the machine's own. */
    private var vWrite: SheetPeerLwwCellRecord = SheetPeerLwwCellRecord(row = 0.toULong(), col = 0.toULong(), value = 0, wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `placed` datamodel variable, the machine's own. */
    private var vPlaced: SheetPeerRgaNodeRecord = SheetPeerRgaNodeRecord(key = 0.toULong(), after = 0.toULong())
    /** W3C SCXML 5.2: the `removal` datamodel variable, the machine's own. */
    private var vRemoval: SheetPeerRgaGoneRecord = SheetPeerRgaGoneRecord(key = 0.toULong(), by = 0.toULong())
    /** W3C SCXML 5.2: the `lamport` datamodel variable, published (`sce:direction="out"`). */
    private var vLamport: ULong = 0.toULong()

    /** What the published `lamport` holds. */
    val lamport: ULong get() = vLamport
    /** W3C SCXML 5.2: the `newKey` datamodel variable, the machine's own. */
    private var vNewKey: ULong = 0.toULong()
    /** W3C SCXML 5.2: the `orphans` datamodel variable, published (`sce:direction="out"`). */
    private var vOrphans: UInt = 0.toUInt()

    /** What the published `orphans` holds. */
    val orphans: UInt get() = vOrphans
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
        var node: ULong? = null
        var hub: Boolean? = null
        var lamport: ULong? = null
        var newKey: ULong? = null
        var orphans: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.hub?.let { vHub = it }
        params.lamport?.let { vLamport = it }
        params.newKey?.let { vNewKey = it }
        params.orphans?.let { vOrphans = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val hub: Boolean,
        val clock: SheetPeerHlcRecord,
        val cells: List<SheetPeerLwwCellRecord>,
        val rows: List<SheetPeerRgaNodeRecord>,
        val gone: List<SheetPeerRgaGoneRecord>,
        val cols: List<SheetPeerRgaNodeRecord>,
        val goneCols: List<SheetPeerRgaGoneRecord>,
        val lamport: ULong,
        val orphans: UInt,
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
        val configuration: Set<SheetPeerState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        node = vNode,
        hub = vHub,
        clock = vClock,
        cells = vCells,
        rows = vRows,
        gone = vGone,
        cols = vCols,
        goneCols = vGoneCols,
        lamport = vLamport,
        orphans = vOrphans,
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
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "9d83d09021256edef19aee8bd083ef8a53cec06e13bbd100b0d9f5979a2b6fb7"

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
            "node" to SavedValues.of(vNode),
            "hub" to SavedValues.of(vHub),
            "clock" to vClock.toSaved(),
            "stamp" to vStamp.toSaved(),
            "cells" to SavedValues.list(vCells) { it.toSaved() },
            "rows" to SavedValues.list(vRows) { it.toSaved() },
            "gone" to SavedValues.list(vGone) { it.toSaved() },
            "cols" to SavedValues.list(vCols) { it.toSaved() },
            "goneCols" to SavedValues.list(vGoneCols) { it.toSaved() },
            "write" to vWrite.toSaved(),
            "placed" to vPlaced.toSaved(),
            "removal" to vRemoval.toSaved(),
            "lamport" to SavedValues.of(vLamport),
            "newKey" to SavedValues.of(vNewKey),
            "orphans" to SavedValues.of(vOrphans),
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
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.uint64(saved.variable("node"), "node")
        val saved2 = SavedValues.bool(saved.variable("hub"), "hub")
        val saved3 = SheetPeerHlcRecord.fromSaved(saved.variable("clock"), "clock")
        val saved4 = SheetPeerHlcRecord.fromSaved(saved.variable("stamp"), "stamp")
        val saved5 = SavedValues.list(saved.variable("cells"), "cells", 256) { e, w -> SheetPeerLwwCellRecord.fromSaved(e, w) }
        val saved6 = SavedValues.list(saved.variable("rows"), "rows", 256) { e, w -> SheetPeerRgaNodeRecord.fromSaved(e, w) }
        val saved7 = SavedValues.list(saved.variable("gone"), "gone", 256) { e, w -> SheetPeerRgaGoneRecord.fromSaved(e, w) }
        val saved8 = SavedValues.list(saved.variable("cols"), "cols", 256) { e, w -> SheetPeerRgaNodeRecord.fromSaved(e, w) }
        val saved9 = SavedValues.list(saved.variable("goneCols"), "goneCols", 256) { e, w -> SheetPeerRgaGoneRecord.fromSaved(e, w) }
        val saved10 = SheetPeerLwwCellRecord.fromSaved(saved.variable("write"), "write")
        val saved11 = SheetPeerRgaNodeRecord.fromSaved(saved.variable("placed"), "placed")
        val saved12 = SheetPeerRgaGoneRecord.fromSaved(saved.variable("removal"), "removal")
        val saved13 = SavedValues.uint64(saved.variable("lamport"), "lamport")
        val saved14 = SavedValues.uint64(saved.variable("newKey"), "newKey")
        val saved15 = SavedValues.uint32(saved.variable("orphans"), "orphans")
        val saved16 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        vNode = saved1
        vHub = saved2
        vClock = saved3
        vStamp = saved4
        vCells = saved5
        vRows = saved6
        vGone = saved7
        vCols = saved8
        vGoneCols = saved9
        vWrite = saved10
        vPlaced = saved11
        vRemoval = saved12
        vLamport = saved13
        vNewKey = saved14
        vOrphans = saved15
        vRefusals = saved16
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingSheetAddColPayload: SheetPeerSheetAddColPayload? = null
    private var pendingSheetAddRowPayload: SheetPeerSheetAddRowPayload? = null
    private var pendingSheetCellPayload: SheetPeerSheetCellPayload? = null
    private var pendingSheetColPayload: SheetPeerSheetColPayload? = null
    private var pendingSheetDropColPayload: SheetPeerSheetDropColPayload? = null
    private var pendingSheetDropRowPayload: SheetPeerSheetDropRowPayload? = null
    private var pendingSheetEditPayload: SheetPeerSheetEditPayload? = null
    private var pendingSheetGonePayload: SheetPeerSheetGonePayload? = null
    private var pendingSheetGoneColPayload: SheetPeerSheetGoneColPayload? = null
    private var pendingSheetJoinPayload: SheetPeerSheetJoinPayload? = null
    private var pendingSheetRowPayload: SheetPeerSheetRowPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: SheetPeerEvent, metadata: EventMetadata) {
        pendingSheetAddColPayload = null
        pendingSheetAddRowPayload = null
        pendingSheetCellPayload = null
        pendingSheetColPayload = null
        pendingSheetDropColPayload = null
        pendingSheetDropRowPayload = null
        pendingSheetEditPayload = null
        pendingSheetGonePayload = null
        pendingSheetGoneColPayload = null
        pendingSheetJoinPayload = null
        pendingSheetRowPayload = null
        when (val tp = metadata.typedPayload) {
            is SheetPeerSheetAddColPayload -> pendingSheetAddColPayload = tp
            is SheetPeerSheetAddRowPayload -> pendingSheetAddRowPayload = tp
            is SheetPeerSheetCellPayload -> pendingSheetCellPayload = tp
            is SheetPeerSheetColPayload -> pendingSheetColPayload = tp
            is SheetPeerSheetDropColPayload -> pendingSheetDropColPayload = tp
            is SheetPeerSheetDropRowPayload -> pendingSheetDropRowPayload = tp
            is SheetPeerSheetEditPayload -> pendingSheetEditPayload = tp
            is SheetPeerSheetGonePayload -> pendingSheetGonePayload = tp
            is SheetPeerSheetGoneColPayload -> pendingSheetGoneColPayload = tp
            is SheetPeerSheetJoinPayload -> pendingSheetJoinPayload = tp
            is SheetPeerSheetRowPayload -> pendingSheetRowPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == SheetPeerEvent.Sheet.AddCol) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetAddColPayload = SheetPeerSheetAddColPayload(fields.uint64("after"))
                } else if (event == SheetPeerEvent.Sheet.AddRow) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetAddRowPayload = SheetPeerSheetAddRowPayload(fields.uint64("after"))
                } else if (event == SheetPeerEvent.Sheet.Cell) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetCellPayload = SheetPeerSheetCellPayload(fields.uint64("row"), fields.uint64("col"), fields.int64("value"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                } else if (event == SheetPeerEvent.Sheet.Col) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetColPayload = SheetPeerSheetColPayload(fields.uint64("key"), fields.uint64("after"))
                } else if (event == SheetPeerEvent.Sheet.DropCol) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetDropColPayload = SheetPeerSheetDropColPayload(fields.uint64("col"))
                } else if (event == SheetPeerEvent.Sheet.DropRow) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetDropRowPayload = SheetPeerSheetDropRowPayload(fields.uint64("row"))
                } else if (event == SheetPeerEvent.Sheet.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetEditPayload = SheetPeerSheetEditPayload(fields.uint64("row"), fields.uint64("col"), fields.int64("value"), fields.int64("now"))
                } else if (event == SheetPeerEvent.Sheet.Gone) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetGonePayload = SheetPeerSheetGonePayload(fields.uint64("key"), fields.uint64("by"))
                } else if (event == SheetPeerEvent.Sheet.GoneCol) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetGoneColPayload = SheetPeerSheetGoneColPayload(fields.uint64("key"), fields.uint64("by"))
                } else if (event == SheetPeerEvent.Sheet.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetJoinPayload = SheetPeerSheetJoinPayload(fields.uint64("node"), fields.boolean("hub"))
                } else if (event == SheetPeerEvent.Sheet.Row) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingSheetRowPayload = SheetPeerSheetRowPayload(fields.uint64("key"), fields.uint64("after"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.add_col` — binds the event name and the payload field values in one call.
    fun raiseSheetAddCol(after: ULong) {
        send(
            SheetPeerEvent.Sheet.AddCol,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetAddColPayload(after),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("after" to after))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.add_row` — binds the event name and the payload field values in one call.
    fun raiseSheetAddRow(after: ULong) {
        send(
            SheetPeerEvent.Sheet.AddRow,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetAddRowPayload(after),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("after" to after))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.cell` — binds the event name and the payload field values in one call.
    fun raiseSheetCell(row: ULong, col: ULong, value: Long, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            SheetPeerEvent.Sheet.Cell,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetCellPayload(row, col, value, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("row" to row, "col" to col, "value" to value, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.col` — binds the event name and the payload field values in one call.
    fun raiseSheetCol(key: ULong, after: ULong) {
        send(
            SheetPeerEvent.Sheet.Col,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetColPayload(key, after),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key, "after" to after))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.drop_col` — binds the event name and the payload field values in one call.
    fun raiseSheetDropCol(col: ULong) {
        send(
            SheetPeerEvent.Sheet.DropCol,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetDropColPayload(col),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("col" to col))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.drop_row` — binds the event name and the payload field values in one call.
    fun raiseSheetDropRow(row: ULong) {
        send(
            SheetPeerEvent.Sheet.DropRow,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetDropRowPayload(row),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("row" to row))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.edit` — binds the event name and the payload field values in one call.
    fun raiseSheetEdit(row: ULong, col: ULong, value: Long, now: Long) {
        send(
            SheetPeerEvent.Sheet.Edit,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetEditPayload(row, col, value, now),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("row" to row, "col" to col, "value" to value, "now" to now))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.gone` — binds the event name and the payload field values in one call.
    fun raiseSheetGone(key: ULong, by: ULong) {
        send(
            SheetPeerEvent.Sheet.Gone,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetGonePayload(key, by),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key, "by" to by))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.gone_col` — binds the event name and the payload field values in one call.
    fun raiseSheetGoneCol(key: ULong, by: ULong) {
        send(
            SheetPeerEvent.Sheet.GoneCol,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetGoneColPayload(key, by),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key, "by" to by))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.join` — binds the event name and the payload field values in one call.
    fun raiseSheetJoin(node: ULong, hub: Boolean) {
        send(
            SheetPeerEvent.Sheet.Join,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetJoinPayload(node, hub),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "hub" to hub))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `sheet.row` — binds the event name and the payload field values in one call.
    fun raiseSheetRow(key: ULong, after: ULong) {
        send(
            SheetPeerEvent.Sheet.Row,
            EventMetadata(
                type = "external",
                typedPayload = SheetPeerSheetRowPayload(key, after),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("key" to key, "after" to after))
            )
        )
    }


    override val initialState: SheetPeerState = SheetPeerState.Open

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
    override val documentInitialTargets: List<EntryTarget<SheetPeerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<SheetPeerState, HistoryId>> =
            listOf(StateTarget(SheetPeerState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 3, as the microstep reads it.
        val transitionOpenAt3 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 4, as the microstep reads it.
        val transitionOpenAt4 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 5, as the microstep reads it.
        val transitionOpenAt5 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 6, as the microstep reads it.
        val transitionOpenAt6 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 7, as the microstep reads it.
        val transitionOpenAt7 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 8, as the microstep reads it.
        val transitionOpenAt8 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 9, as the microstep reads it.
        val transitionOpenAt9 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 10, as the microstep reads it.
        val transitionOpenAt10 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 11, as the microstep reads it.
        val transitionOpenAt11 = EnabledTransition<SheetPeerState, HistoryId>(
            SheetPeerState.Open,
            emptyList(),
            11,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): SheetPeerState? = when (stateId) {
        "open" -> SheetPeerState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: SheetPeerState): String = when (state) {
        is SheetPeerState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: SheetPeerState): Int = when (state) {
        is SheetPeerState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): SheetPeerEvent? = when (name) {
        "error.execution" -> SheetPeerEvent.Error.Execution
        "sheet.add_col" -> SheetPeerEvent.Sheet.AddCol
        "sheet.add_row" -> SheetPeerEvent.Sheet.AddRow
        "sheet.cell" -> SheetPeerEvent.Sheet.Cell
        "sheet.col" -> SheetPeerEvent.Sheet.Col
        "sheet.drop_col" -> SheetPeerEvent.Sheet.DropCol
        "sheet.drop_row" -> SheetPeerEvent.Sheet.DropRow
        "sheet.edit" -> SheetPeerEvent.Sheet.Edit
        "sheet.gone" -> SheetPeerEvent.Sheet.Gone
        "sheet.gone_col" -> SheetPeerEvent.Sheet.GoneCol
        "sheet.join" -> SheetPeerEvent.Sheet.Join
        "sheet.row" -> SheetPeerEvent.Sheet.Row
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: SheetPeerEvent): String? = when (event) {
        is SheetPeerEvent.Error.Execution -> "error.execution"
        is SheetPeerEvent.Sheet.AddCol -> "sheet.add_col"
        is SheetPeerEvent.Sheet.AddRow -> "sheet.add_row"
        is SheetPeerEvent.Sheet.Cell -> "sheet.cell"
        is SheetPeerEvent.Sheet.Col -> "sheet.col"
        is SheetPeerEvent.Sheet.DropCol -> "sheet.drop_col"
        is SheetPeerEvent.Sheet.DropRow -> "sheet.drop_row"
        is SheetPeerEvent.Sheet.Edit -> "sheet.edit"
        is SheetPeerEvent.Sheet.Gone -> "sheet.gone"
        is SheetPeerEvent.Sheet.GoneCol -> "sheet.gone_col"
        is SheetPeerEvent.Sheet.Join -> "sheet.join"
        is SheetPeerEvent.Sheet.Row -> "sheet.row"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: SheetPeerState,
        event: SheetPeerEvent?
    ): EnabledTransition<SheetPeerState, HistoryId>? = when (state) {
        is SheetPeerState.Open -> when {
            event is SheetPeerEvent.Sheet.Join -> transitionOpenAt0
            event is SheetPeerEvent.Sheet.Edit -> transitionOpenAt1
            event is SheetPeerEvent.Sheet.Cell -> transitionOpenAt2
            event is SheetPeerEvent.Sheet.AddRow -> transitionOpenAt3
            event is SheetPeerEvent.Sheet.Row -> transitionOpenAt4
            event is SheetPeerEvent.Sheet.DropRow -> transitionOpenAt5
            event is SheetPeerEvent.Sheet.Gone -> transitionOpenAt6
            event is SheetPeerEvent.Sheet.AddCol -> transitionOpenAt7
            event is SheetPeerEvent.Sheet.Col -> transitionOpenAt8
            event is SheetPeerEvent.Sheet.DropCol -> transitionOpenAt9
            event is SheetPeerEvent.Sheet.GoneCol -> transitionOpenAt10
            event is SheetPeerEvent.Error.Execution -> transitionOpenAt11
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: sheet_peer.scxml:34 :: _machine
    override fun onEntry(state: SheetPeerState, isDefaultEntry: Boolean) {
        when (state) {
            is SheetPeerState.Open -> {
                // SCE-MAP: sheet_peer.scxml:100 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: sheet_peer.scxml:34 :: _machine
    override fun onExit(state: SheetPeerState) {
        when (state) {
            is SheetPeerState.Open -> {
                // SCE-MAP: sheet_peer.scxml:100 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: sheet_peer.scxml:34 :: _machine
    override fun executeTransitionContent(source: SheetPeerState, transitionIndex: Int) {
        when (source) {
        is SheetPeerState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: sheet_peer.scxml:101 :: open :: _transition_0
                if (pendingSheetJoinPayload == null) {
                    return
                }

            vNode = pendingSheetJoinPayload!!.node

            vHub = pendingSheetJoinPayload!!.hub
            }
            1 -> {
                // SCE-MAP: sheet_peer.scxml:107 :: open :: _transition_1
                if (pendingSheetEditPayload == null) {
                    return
                }

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, pendingSheetEditPayload!!.now, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "row", (pendingSheetEditPayload!!.row).toString())

            putParam(sendPayload, "col", (pendingSheetEditPayload!!.col).toString())

            putParam(sendPayload, "value", (pendingSheetEditPayload!!.value).toString())

            putParam(sendPayload, "wallTime", (vClock.wallTime).toString())

            putParam(sendPayload, "counter", (vClock.counter).toLong())

            putParam(sendPayload, "nodeId", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SheetPeerEvent.Sheet.Cell, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "row", (pendingSheetEditPayload!!.row).toString())

            putParam(sendPayload, "col", (pendingSheetEditPayload!!.col).toString())

            putParam(sendPayload, "value", (pendingSheetEditPayload!!.value).toString())

            putParam(sendPayload, "wallTime", (vClock.wallTime).toString())

            putParam(sendPayload, "counter", (vClock.counter).toLong())

            putParam(sendPayload, "nodeId", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.cell",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_1",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: sheet_peer.scxml:128 :: open :: _transition_2
                if (pendingSheetCellPayload == null) {
                    return
                }


            if (pendingSheetCellPayload!!.nodeId != vNode) {

            vStamp = vStamp.copy(wallTime = pendingSheetCellPayload!!.wallTime)

            vStamp = vStamp.copy(counter = pendingSheetCellPayload!!.counter)

            vStamp = vStamp.copy(nodeId = pendingSheetCellPayload!!.nodeId)

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcReceive__vv(vClock, vStamp, vClock.wallTime, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vWrite = vWrite.copy(row = pendingSheetCellPayload!!.row)

            vWrite = vWrite.copy(col = pendingSheetCellPayload!!.col)

            vWrite = vWrite.copy(value = pendingSheetCellPayload!!.value)

            vWrite = vWrite.copy(wallTime = pendingSheetCellPayload!!.wallTime)

            vWrite = vWrite.copy(counter = pendingSheetCellPayload!!.counter)

            vWrite = vWrite.copy(nodeId = pendingSheetCellPayload!!.nodeId)

            if (try { vCells = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_LwwWrite__vv(vCells, vWrite)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='cells'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vHub && pendingSheetCellPayload!!.nodeId != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "row", (pendingSheetCellPayload!!.row).toString())

            putParam(sendPayload, "col", (pendingSheetCellPayload!!.col).toString())

            putParam(sendPayload, "value", (pendingSheetCellPayload!!.value).toString())

            putParam(sendPayload, "wallTime", (pendingSheetCellPayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingSheetCellPayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingSheetCellPayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.cell",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_2",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_2")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingSheetCellPayload!!.nodeId != 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "row", (pendingSheetCellPayload!!.row).toString())

            putParam(sendPayload, "col", (pendingSheetCellPayload!!.col).toString())

            putParam(sendPayload, "value", (pendingSheetCellPayload!!.value).toString())

            putParam(sendPayload, "wallTime", (pendingSheetCellPayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingSheetCellPayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingSheetCellPayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.cell",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_3",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_3")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            3 -> {
                // SCE-MAP: sheet_peer.scxml:166 :: open :: _transition_3
                if (pendingSheetAddRowPayload == null) {
                    return
                }

            if (try { vLamport = com.sce.forge.runtime.SceChecked.add(vLamport, 1.toULong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='lamport'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vNewKey = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vLamport, 256.toULong()), vNode); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='newKey'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (vNewKey).toString())

            putParam(sendPayload, "after", (pendingSheetAddRowPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SheetPeerEvent.Sheet.Row, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (vNewKey).toString())

            putParam(sendPayload, "after", (pendingSheetAddRowPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.row",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_5",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_5")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            4 -> {
                // SCE-MAP: sheet_peer.scxml:183 :: open :: _transition_4
                if (pendingSheetRowPayload == null) {
                    return
                }


            var ifCondFailed4 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(pendingSheetRowPayload!!.key, 256.toULong()) > vLamport } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='_event.data.key / 256 > lamport'>: an integer operation overflowed or failed"); ifCondFailed4 = true; false })) {

            if (try { vLamport = com.sce.forge.runtime.SceChecked.div(pendingSheetRowPayload!!.key, 256.toULong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='lamport'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            if (ifCondFailed4) {
                return
            }

            vPlaced = vPlaced.copy(key = pendingSheetRowPayload!!.key)

            vPlaced = vPlaced.copy(after = pendingSheetRowPayload!!.after)


            if (pendingSheetRowPayload!!.after != 0.toULong() && !call_RgaHolds__v(vRows, pendingSheetRowPayload!!.key) && !call_RgaHolds__v(vRows, pendingSheetRowPayload!!.after)) {

            if (try { vOrphans = com.sce.forge.runtime.SceChecked.add(vOrphans, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='orphans'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { vRows = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RgaInsert__vv(vRows, vPlaced)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='rows'>: an integer operation overflowed or failed"); true }) {
                return
            }


            var ifCondFailed6 = false
            if ((try { vHub && com.sce.forge.runtime.SceChecked.rem(pendingSheetRowPayload!!.key, 256.toULong()) != 1.toULong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='hub && _event.data.key % 256 !== 1'>: an integer operation overflowed or failed"); ifCondFailed6 = true; false })) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetRowPayload!!.key).toString())

            putParam(sendPayload, "after", (pendingSheetRowPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.row",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_6",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_6")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            if (ifCondFailed6) {
                return
            }


            var ifCondFailed7 = false
            if ((try { vHub && com.sce.forge.runtime.SceChecked.rem(pendingSheetRowPayload!!.key, 256.toULong()) != 2.toULong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='hub && _event.data.key % 256 !== 2'>: an integer operation overflowed or failed"); ifCondFailed7 = true; false })) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetRowPayload!!.key).toString())

            putParam(sendPayload, "after", (pendingSheetRowPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.row",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_7",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_7")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            if (ifCondFailed7) {
                return
            }
            }
            5 -> {
                // SCE-MAP: sheet_peer.scxml:208 :: open :: _transition_5
                if (pendingSheetDropRowPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetDropRowPayload!!.row).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SheetPeerEvent.Sheet.Gone, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetDropRowPayload!!.row).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_9",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_9")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            6 -> {
                // SCE-MAP: sheet_peer.scxml:221 :: open :: _transition_6
                if (pendingSheetGonePayload == null) {
                    return
                }

            vRemoval = vRemoval.copy(key = pendingSheetGonePayload!!.key)

            vRemoval = vRemoval.copy(by = pendingSheetGonePayload!!.by)

            if (try { vGone = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RgaRemove__vv(vGone, vRemoval)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='gone'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vHub && pendingSheetGonePayload!!.by != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetGonePayload!!.key).toString())

            putParam(sendPayload, "by", (pendingSheetGonePayload!!.by).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_10",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_10")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingSheetGonePayload!!.by != 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetGonePayload!!.key).toString())

            putParam(sendPayload, "by", (pendingSheetGonePayload!!.by).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_11",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_11")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            7 -> {
                // SCE-MAP: sheet_peer.scxml:242 :: open :: _transition_7
                if (pendingSheetAddColPayload == null) {
                    return
                }

            if (try { vLamport = com.sce.forge.runtime.SceChecked.add(vLamport, 1.toULong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='lamport'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vNewKey = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vLamport, 256.toULong()), vNode); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='newKey'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (vNewKey).toString())

            putParam(sendPayload, "after", (pendingSheetAddColPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SheetPeerEvent.Sheet.Col, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (vNewKey).toString())

            putParam(sendPayload, "after", (pendingSheetAddColPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.col",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_13",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_13")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            8 -> {
                // SCE-MAP: sheet_peer.scxml:255 :: open :: _transition_8
                if (pendingSheetColPayload == null) {
                    return
                }


            var ifCondFailed10 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(pendingSheetColPayload!!.key, 256.toULong()) > vLamport } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='_event.data.key / 256 > lamport'>: an integer operation overflowed or failed"); ifCondFailed10 = true; false })) {

            if (try { vLamport = com.sce.forge.runtime.SceChecked.div(pendingSheetColPayload!!.key, 256.toULong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='lamport'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            if (ifCondFailed10) {
                return
            }

            vPlaced = vPlaced.copy(key = pendingSheetColPayload!!.key)

            vPlaced = vPlaced.copy(after = pendingSheetColPayload!!.after)


            if (pendingSheetColPayload!!.after != 0.toULong() && !call_RgaHolds__v(vCols, pendingSheetColPayload!!.key) && !call_RgaHolds__v(vCols, pendingSheetColPayload!!.after)) {

            if (try { vOrphans = com.sce.forge.runtime.SceChecked.add(vOrphans, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='orphans'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { vCols = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RgaInsert__vv(vCols, vPlaced)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='cols'>: an integer operation overflowed or failed"); true }) {
                return
            }


            var ifCondFailed12 = false
            if ((try { vHub && com.sce.forge.runtime.SceChecked.rem(pendingSheetColPayload!!.key, 256.toULong()) != 1.toULong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='hub && _event.data.key % 256 !== 1'>: an integer operation overflowed or failed"); ifCondFailed12 = true; false })) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetColPayload!!.key).toString())

            putParam(sendPayload, "after", (pendingSheetColPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.col",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_14",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_14")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            if (ifCondFailed12) {
                return
            }


            var ifCondFailed13 = false
            if ((try { vHub && com.sce.forge.runtime.SceChecked.rem(pendingSheetColPayload!!.key, 256.toULong()) != 2.toULong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<if cond='hub && _event.data.key % 256 !== 2'>: an integer operation overflowed or failed"); ifCondFailed13 = true; false })) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetColPayload!!.key).toString())

            putParam(sendPayload, "after", (pendingSheetColPayload!!.after).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.col",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_15",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_15")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            if (ifCondFailed13) {
                return
            }
            }
            9 -> {
                // SCE-MAP: sheet_peer.scxml:279 :: open :: _transition_9
                if (pendingSheetDropColPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetDropColPayload!!.col).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(SheetPeerEvent.Sheet.GoneCol, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetDropColPayload!!.col).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone_col",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_17",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_17")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            10 -> {
                // SCE-MAP: sheet_peer.scxml:290 :: open :: _transition_10
                if (pendingSheetGoneColPayload == null) {
                    return
                }

            vRemoval = vRemoval.copy(key = pendingSheetGoneColPayload!!.key)

            vRemoval = vRemoval.copy(by = pendingSheetGoneColPayload!!.by)

            if (try { vGoneCols = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RgaRemove__vv(vGoneCols, vRemoval)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='goneCols'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vHub && pendingSheetGoneColPayload!!.by != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetGoneColPayload!!.key).toString())

            putParam(sendPayload, "by", (pendingSheetGoneColPayload!!.by).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone_col",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_18",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_18")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingSheetGoneColPayload!!.by != 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "key", (pendingSheetGoneColPayload!!.key).toString())

            putParam(sendPayload, "by", (pendingSheetGoneColPayload!!.by).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "sce:mesh" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "sce:mesh",
                eventName = "sheet.gone_col",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_19",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            val hostServed = performHostSend(hostRequest)
            // W3C SCXML 6.2: a declared type with no handler registered is,
            // from the document's side, a processor the platform does not
            // support — the act it asked for was performed by nobody. Same
            // event as an undeclared type, so a wiring mistake cannot read
            // as success.
            if (hostServed == null && !hasEventProcessor("sce:mesh")) {
                raisePlatformError(SheetPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_19")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            11 -> {
                // SCE-MAP: sheet_peer.scxml:308 :: open :: _transition_11

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(SheetPeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
