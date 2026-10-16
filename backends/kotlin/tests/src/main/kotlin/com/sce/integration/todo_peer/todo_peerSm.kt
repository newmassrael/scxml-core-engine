// SCE-GENERATED — DO NOT EDIT
// source-hash: 35dd989ec22ae23b283a4c23fe6e1a93c126d88e4bdcb9b09222b54ba76fa414

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/todo_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: todo_peer.scxml:71 :: _machine

package com.sce.integration.todo_peer

import com.sce.runtime.*
import com.sce.generated.hlc_receive.*
import com.sce.generated.hlc_send.*
import com.sce.generated.hlc_timestamp.*
import com.sce.generated.merkle_change_hash.*
import com.sce.generated.merkle_insert.*
import com.sce.generated.merkle_node.*
import com.sce.generated.mv_value.*
import com.sce.generated.mv_value_drop.*
import com.sce.generated.mv_value_holds.*
import com.sce.generated.mv_value_keep.*
import com.sce.generated.orset_entry.*
import com.sce.generated.orset_live.*
import com.sce.generated.orset_observed.*
import com.sce.generated.orset_union.*


// --- States (W3C SCXML 3.2) ---

sealed interface TodoPeerState : State {
    data object Open : TodoPeerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface TodoPeerEvent : Event {
    sealed interface Error : TodoPeerEvent {
        data object Execution : Error
    }
    sealed interface Todo : TodoPeerEvent {
        data object Edit : Todo
        data object Join : Todo
        data object Keep : Todo
        data object Node : Todo
        data object Resync : Todo
        data object Share : Todo
        data object Supersede : Todo
        data object Visit : Todo
        data object Write : Todo
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `TodoPeerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// TodoPeerTodoEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.edit`. Consumers inject it via the `raiseTodoEdit` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoEditPayload(val id: UInt, val field: UShort, val value: Long, val text: String, val now: Long)

// TodoPeerTodoJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.join`. Consumers inject it via the `raiseTodoJoin` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoJoinPayload(val node: ULong, val hub: Boolean)

// TodoPeerTodoKeepPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.keep`. Consumers inject it via the `raiseTodoKeep` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoKeepPayload(val kind: UByte, val element: ULong, val wallTime: Long, val counter: UInt, val nodeId: ULong)

// TodoPeerTodoNodePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.node`. Consumers inject it via the `raiseTodoNode` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoNodePayload(val sender: ULong, val depth: UByte, val prefix: ULong, val hash: UInt, val reply: Boolean)

// TodoPeerTodoSharePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.share`. Consumers inject it via the `raiseTodoShare` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoSharePayload(val to: ULong, val minute: ULong)

// TodoPeerTodoSupersedePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.supersede`. Consumers inject it via the `raiseTodoSupersede` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoSupersedePayload(val id: UInt, val field: UShort, val wallTime: Long, val counter: UInt, val nodeId: ULong, val by: ULong)

// TodoPeerTodoVisitPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.visit`. Consumers inject it via the `raiseTodoVisit` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoVisitPayload(val to: ULong, val depth: UByte, val prefix: ULong, val reply: Boolean)

// TodoPeerTodoWritePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.write`. Consumers inject it via the `raiseTodoWrite` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoWritePayload(val id: UInt, val field: UShort, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Hlc` datamodel value. */
data class TodoPeerHlcRecord(val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Entry` datamodel value. */
data class TodoPeerEntryRecord(val element: ULong, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("element" to SavedValues.of(element), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerEntryRecord = TodoPeerEntryRecord(element = SavedValues.uint64(SavedValues.field(value, what, "element"), "$what.element"), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Value` datamodel value. */
data class TodoPeerValueRecord(val id: ULong, val field: ULong, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerValueRecord = TodoPeerValueRecord(id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint64(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 32), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Node` datamodel value. */
data class TodoPeerNodeRecord(val depth: UByte, val prefix: ULong, val hash: UInt) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("depth" to SavedValues.of(depth), "prefix" to SavedValues.of(prefix), "hash" to SavedValues.of(hash))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerNodeRecord = TodoPeerNodeRecord(depth = SavedValues.uint8(SavedValues.field(value, what, "depth"), "$what.depth"), prefix = SavedValues.uint64(SavedValues.field(value, what, "prefix"), "$what.prefix"), hash = SavedValues.uint32(SavedValues.field(value, what, "hash"), "$what.hash"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_HlcReceive__vv(p0: TodoPeerHlcRecord, p1: TodoPeerHlcRecord, p2: Long, p3: ULong): com.sce.forge.runtime.AlgorithmResult<TodoPeerHlcRecord> =
    when (val r = hlcReceive(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), HlcTimestampPayload(wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId), p2, p3)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(TodoPeerHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_HlcSend__v(p0: TodoPeerHlcRecord, p1: Long, p2: ULong): com.sce.forge.runtime.AlgorithmResult<TodoPeerHlcRecord> =
    when (val r = hlcSend(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(TodoPeerHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MerkleChangeHash__v(p0: TodoPeerEntryRecord, p1: UByte): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = merkleChangeHash(OrsetEntryPayload(element = p0.element, wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MerkleInsert__v(p0: List<TodoPeerNodeRecord>, p1: Long, p2: UInt): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerNodeRecord>> =
    when (val r = merkleInsert(p0.map { MerkleNodePayload(depth = it.depth, prefix = it.prefix, hash = it.hash) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerNodeRecord(depth = it.depth, prefix = it.prefix, hash = it.hash) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MvValueDrop__vv(p0: List<TodoPeerValueRecord>, p1: TodoPeerValueRecord): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerValueRecord>> =
    when (val r = mvValueDrop(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MvValueHolds__vv(p0: List<TodoPeerValueRecord>, p1: TodoPeerValueRecord): Boolean {
    val r = mvValueHolds(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))
    return r
}
private fun call_MvValueKeep__vv(p0: List<TodoPeerValueRecord>, p1: TodoPeerValueRecord): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerValueRecord>> =
    when (val r = mvValueKeep(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_OrsetLive__vv(p0: List<TodoPeerEntryRecord>, p1: List<TodoPeerEntryRecord>): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerEntryRecord>> =
    when (val r = orsetLive(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_OrsetObserved__v(p0: List<TodoPeerEntryRecord>, p1: ULong): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerEntryRecord>> =
    when (val r = orsetObserved(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_OrsetUnion__vv(p0: List<TodoPeerEntryRecord>, p1: List<TodoPeerEntryRecord>): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerEntryRecord>> =
    when (val r = orsetUnion(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class TodoPeerStateMachine(
) : StateMachineEngine<TodoPeerState, TodoPeerEvent>() {

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
    private var vClock: TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `stamp` datamodel variable, the machine's own. */
    private var vStamp: TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `adds` datamodel variable, published (`sce:direction="out"`). */
    private var vAdds: List<TodoPeerEntryRecord> = emptyList()

    /** What the published `adds` holds. */
    val adds: List<TodoPeerEntryRecord> get() = vAdds
    /** W3C SCXML 5.2: the `tombstones` datamodel variable, published (`sce:direction="out"`). */
    private var vTombstones: List<TodoPeerEntryRecord> = emptyList()

    /** What the published `tombstones` holds. */
    val tombstones: List<TodoPeerEntryRecord> get() = vTombstones
    /** W3C SCXML 5.2: the `live` datamodel variable, the machine's own. */
    private var vLive: List<TodoPeerEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `observed` datamodel variable, the machine's own. */
    private var vObserved: List<TodoPeerEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `one` datamodel variable, the machine's own. */
    private var vOne: List<TodoPeerEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `entry` datamodel variable, the machine's own. */
    private var vEntry: TodoPeerEntryRecord = TodoPeerEntryRecord(element = 0.toULong(), wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `values` datamodel variable, published (`sce:direction="out"`). */
    private var vValues: List<TodoPeerValueRecord> = emptyList()

    /** What the published `values` holds. */
    val values: List<TodoPeerValueRecord> get() = vValues
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: TodoPeerValueRecord = TodoPeerValueRecord(id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `digest` datamodel variable, published (`sce:direction="out"`). */
    private var vDigest: List<TodoPeerNodeRecord> = emptyList()

    /** What the published `digest` holds. */
    val digest: List<TodoPeerNodeRecord> get() = vDigest
    /** W3C SCXML 5.2: the `mine` datamodel variable, the machine's own. */
    private var vMine: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `shareId` datamodel variable, the machine's own. */
    private var vShareId: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `shareField` datamodel variable, the machine's own. */
    private var vShareField: UShort = 0.toUShort()
    /** W3C SCXML 5.2: the `known` datamodel variable, the machine's own. */
    private var vKnown: Boolean = false
    /** W3C SCXML 5.2: the `wasLive` datamodel variable, the machine's own. */
    private var vWasLive: Boolean = false
    /** W3C SCXML 5.2: the `isLive` datamodel variable, the machine's own. */
    private var vIsLive: Boolean = false
    /** W3C SCXML 5.2: the `present` datamodel variable, the machine's own. */
    private var vPresent: Boolean = false
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
        var mine: UInt? = null
        var shareId: UInt? = null
        var shareField: UShort? = null
        var known: Boolean? = null
        var wasLive: Boolean? = null
        var isLive: Boolean? = null
        var present: Boolean? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.hub?.let { vHub = it }
        params.mine?.let { vMine = it }
        params.shareId?.let { vShareId = it }
        params.shareField?.let { vShareField = it }
        params.known?.let { vKnown = it }
        params.wasLive?.let { vWasLive = it }
        params.isLive?.let { vIsLive = it }
        params.present?.let { vPresent = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val hub: Boolean,
        val clock: TodoPeerHlcRecord,
        val adds: List<TodoPeerEntryRecord>,
        val tombstones: List<TodoPeerEntryRecord>,
        val values: List<TodoPeerValueRecord>,
        val digest: List<TodoPeerNodeRecord>,
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
        val configuration: Set<TodoPeerState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        node = vNode,
        hub = vHub,
        clock = vClock,
        adds = vAdds,
        tombstones = vTombstones,
        values = vValues,
        digest = vDigest,
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
    val savedShape: String = "fd131f879d7d2fc431407f5075e40158b7440c6df91aa09a030e271e043d6be6"

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
            "adds" to SavedValues.list(vAdds) { it.toSaved() },
            "tombstones" to SavedValues.list(vTombstones) { it.toSaved() },
            "live" to SavedValues.list(vLive) { it.toSaved() },
            "observed" to SavedValues.list(vObserved) { it.toSaved() },
            "one" to SavedValues.list(vOne) { it.toSaved() },
            "entry" to vEntry.toSaved(),
            "values" to SavedValues.list(vValues) { it.toSaved() },
            "held" to vHeld.toSaved(),
            "digest" to SavedValues.list(vDigest) { it.toSaved() },
            "mine" to SavedValues.of(vMine),
            "shareId" to SavedValues.of(vShareId),
            "shareField" to SavedValues.of(vShareField),
            "known" to SavedValues.of(vKnown),
            "wasLive" to SavedValues.of(vWasLive),
            "isLive" to SavedValues.of(vIsLive),
            "present" to SavedValues.of(vPresent),
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
        val saved3 = TodoPeerHlcRecord.fromSaved(saved.variable("clock"), "clock")
        val saved4 = TodoPeerHlcRecord.fromSaved(saved.variable("stamp"), "stamp")
        val saved5 = SavedValues.list(saved.variable("adds"), "adds", 256) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved6 = SavedValues.list(saved.variable("tombstones"), "tombstones", 256) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved7 = SavedValues.list(saved.variable("live"), "live", 256) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved8 = SavedValues.list(saved.variable("observed"), "observed", 256) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved9 = SavedValues.list(saved.variable("one"), "one", 256) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved10 = TodoPeerEntryRecord.fromSaved(saved.variable("entry"), "entry")
        val saved11 = SavedValues.list(saved.variable("values"), "values", 128) { e, w -> TodoPeerValueRecord.fromSaved(e, w) }
        val saved12 = TodoPeerValueRecord.fromSaved(saved.variable("held"), "held")
        val saved13 = SavedValues.list(saved.variable("digest"), "digest", 1024) { e, w -> TodoPeerNodeRecord.fromSaved(e, w) }
        val saved14 = SavedValues.uint32(saved.variable("mine"), "mine")
        val saved15 = SavedValues.uint32(saved.variable("shareId"), "shareId")
        val saved16 = SavedValues.uint16(saved.variable("shareField"), "shareField")
        val saved17 = SavedValues.bool(saved.variable("known"), "known")
        val saved18 = SavedValues.bool(saved.variable("wasLive"), "wasLive")
        val saved19 = SavedValues.bool(saved.variable("isLive"), "isLive")
        val saved20 = SavedValues.bool(saved.variable("present"), "present")
        val saved21 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        vNode = saved1
        vHub = saved2
        vClock = saved3
        vStamp = saved4
        vAdds = saved5
        vTombstones = saved6
        vLive = saved7
        vObserved = saved8
        vOne = saved9
        vEntry = saved10
        vValues = saved11
        vHeld = saved12
        vDigest = saved13
        vMine = saved14
        vShareId = saved15
        vShareField = saved16
        vKnown = saved17
        vWasLive = saved18
        vIsLive = saved19
        vPresent = saved20
        vRefusals = saved21
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTodoEditPayload: TodoPeerTodoEditPayload? = null
    private var pendingTodoJoinPayload: TodoPeerTodoJoinPayload? = null
    private var pendingTodoKeepPayload: TodoPeerTodoKeepPayload? = null
    private var pendingTodoNodePayload: TodoPeerTodoNodePayload? = null
    private var pendingTodoSharePayload: TodoPeerTodoSharePayload? = null
    private var pendingTodoSupersedePayload: TodoPeerTodoSupersedePayload? = null
    private var pendingTodoVisitPayload: TodoPeerTodoVisitPayload? = null
    private var pendingTodoWritePayload: TodoPeerTodoWritePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: TodoPeerEvent, metadata: EventMetadata) {
        pendingTodoEditPayload = null
        pendingTodoJoinPayload = null
        pendingTodoKeepPayload = null
        pendingTodoNodePayload = null
        pendingTodoSharePayload = null
        pendingTodoSupersedePayload = null
        pendingTodoVisitPayload = null
        pendingTodoWritePayload = null
        when (val tp = metadata.typedPayload) {
            is TodoPeerTodoEditPayload -> pendingTodoEditPayload = tp
            is TodoPeerTodoJoinPayload -> pendingTodoJoinPayload = tp
            is TodoPeerTodoKeepPayload -> pendingTodoKeepPayload = tp
            is TodoPeerTodoNodePayload -> pendingTodoNodePayload = tp
            is TodoPeerTodoSharePayload -> pendingTodoSharePayload = tp
            is TodoPeerTodoSupersedePayload -> pendingTodoSupersedePayload = tp
            is TodoPeerTodoVisitPayload -> pendingTodoVisitPayload = tp
            is TodoPeerTodoWritePayload -> pendingTodoWritePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == TodoPeerEvent.Todo.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoEditPayload = TodoPeerTodoEditPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("now"))
                } else if (event == TodoPeerEvent.Todo.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoJoinPayload = TodoPeerTodoJoinPayload(fields.uint64("node"), fields.boolean("hub"))
                } else if (event == TodoPeerEvent.Todo.Keep) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoKeepPayload = TodoPeerTodoKeepPayload(fields.uint8("kind"), fields.uint64("element"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                } else if (event == TodoPeerEvent.Todo.Node) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoNodePayload = TodoPeerTodoNodePayload(fields.uint64("sender"), fields.uint8("depth"), fields.uint64("prefix"), fields.uint32("hash"), fields.boolean("reply"))
                } else if (event == TodoPeerEvent.Todo.Share) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoSharePayload = TodoPeerTodoSharePayload(fields.uint64("to"), fields.uint64("minute"))
                } else if (event == TodoPeerEvent.Todo.Supersede) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoSupersedePayload = TodoPeerTodoSupersedePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"), fields.uint64("by"))
                } else if (event == TodoPeerEvent.Todo.Visit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoVisitPayload = TodoPeerTodoVisitPayload(fields.uint64("to"), fields.uint8("depth"), fields.uint64("prefix"), fields.boolean("reply"))
                } else if (event == TodoPeerEvent.Todo.Write) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoWritePayload = TodoPeerTodoWritePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.edit` — binds the event name and the payload field values in one call.
    fun raiseTodoEdit(id: UInt, field: UShort, value: Long, text: String, now: Long) {
        send(
            TodoPeerEvent.Todo.Edit,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoEditPayload(id, field, value, text, now),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "now" to now))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.join` — binds the event name and the payload field values in one call.
    fun raiseTodoJoin(node: ULong, hub: Boolean) {
        send(
            TodoPeerEvent.Todo.Join,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoJoinPayload(node, hub),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "hub" to hub))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.keep` — binds the event name and the payload field values in one call.
    fun raiseTodoKeep(kind: UByte, element: ULong, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            TodoPeerEvent.Todo.Keep,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoKeepPayload(kind, element, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("kind" to kind, "element" to element, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.node` — binds the event name and the payload field values in one call.
    fun raiseTodoNode(sender: ULong, depth: UByte, prefix: ULong, hash: UInt, reply: Boolean) {
        send(
            TodoPeerEvent.Todo.Node,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoNodePayload(sender, depth, prefix, hash, reply),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sender" to sender, "depth" to depth, "prefix" to prefix, "hash" to hash, "reply" to reply))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.share` — binds the event name and the payload field values in one call.
    fun raiseTodoShare(to: ULong, minute: ULong) {
        send(
            TodoPeerEvent.Todo.Share,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoSharePayload(to, minute),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("to" to to, "minute" to minute))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.supersede` — binds the event name and the payload field values in one call.
    fun raiseTodoSupersede(id: UInt, field: UShort, wallTime: Long, counter: UInt, nodeId: ULong, by: ULong) {
        send(
            TodoPeerEvent.Todo.Supersede,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoSupersedePayload(id, field, wallTime, counter, nodeId, by),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId, "by" to by))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.visit` — binds the event name and the payload field values in one call.
    fun raiseTodoVisit(to: ULong, depth: UByte, prefix: ULong, reply: Boolean) {
        send(
            TodoPeerEvent.Todo.Visit,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoVisitPayload(to, depth, prefix, reply),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("to" to to, "depth" to depth, "prefix" to prefix, "reply" to reply))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.write` — binds the event name and the payload field values in one call.
    fun raiseTodoWrite(id: UInt, field: UShort, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            TodoPeerEvent.Todo.Write,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoWritePayload(id, field, value, text, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }


    override val initialState: TodoPeerState = TodoPeerState.Open

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
    override val documentInitialTargets: List<EntryTarget<TodoPeerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<TodoPeerState, HistoryId>> =
            listOf(StateTarget(TodoPeerState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 3, as the microstep reads it.
        val transitionOpenAt3 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 4, as the microstep reads it.
        val transitionOpenAt4 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 5, as the microstep reads it.
        val transitionOpenAt5 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 6, as the microstep reads it.
        val transitionOpenAt6 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 7, as the microstep reads it.
        val transitionOpenAt7 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 8, as the microstep reads it.
        val transitionOpenAt8 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 9, as the microstep reads it.
        val transitionOpenAt9 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): TodoPeerState? = when (stateId) {
        "open" -> TodoPeerState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: TodoPeerState): String = when (state) {
        is TodoPeerState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: TodoPeerState): Int = when (state) {
        is TodoPeerState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): TodoPeerEvent? = when (name) {
        "error.execution" -> TodoPeerEvent.Error.Execution
        "todo.edit" -> TodoPeerEvent.Todo.Edit
        "todo.join" -> TodoPeerEvent.Todo.Join
        "todo.keep" -> TodoPeerEvent.Todo.Keep
        "todo.node" -> TodoPeerEvent.Todo.Node
        "todo.resync" -> TodoPeerEvent.Todo.Resync
        "todo.share" -> TodoPeerEvent.Todo.Share
        "todo.supersede" -> TodoPeerEvent.Todo.Supersede
        "todo.visit" -> TodoPeerEvent.Todo.Visit
        "todo.write" -> TodoPeerEvent.Todo.Write
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TodoPeerEvent): String? = when (event) {
        is TodoPeerEvent.Error.Execution -> "error.execution"
        is TodoPeerEvent.Todo.Edit -> "todo.edit"
        is TodoPeerEvent.Todo.Join -> "todo.join"
        is TodoPeerEvent.Todo.Keep -> "todo.keep"
        is TodoPeerEvent.Todo.Node -> "todo.node"
        is TodoPeerEvent.Todo.Resync -> "todo.resync"
        is TodoPeerEvent.Todo.Share -> "todo.share"
        is TodoPeerEvent.Todo.Supersede -> "todo.supersede"
        is TodoPeerEvent.Todo.Visit -> "todo.visit"
        is TodoPeerEvent.Todo.Write -> "todo.write"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: TodoPeerState,
        event: TodoPeerEvent?
    ): EnabledTransition<TodoPeerState, HistoryId>? = when (state) {
        is TodoPeerState.Open -> when {
            event is TodoPeerEvent.Todo.Join -> transitionOpenAt0
            event is TodoPeerEvent.Todo.Edit -> transitionOpenAt1
            event is TodoPeerEvent.Todo.Write -> transitionOpenAt2
            event is TodoPeerEvent.Todo.Supersede -> transitionOpenAt3
            event is TodoPeerEvent.Todo.Keep -> transitionOpenAt4
            event is TodoPeerEvent.Todo.Resync -> transitionOpenAt5
            event is TodoPeerEvent.Todo.Visit -> transitionOpenAt6
            event is TodoPeerEvent.Todo.Node -> transitionOpenAt7
            event is TodoPeerEvent.Todo.Share -> transitionOpenAt8
            event is TodoPeerEvent.Error.Execution -> transitionOpenAt9
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun onEntry(state: TodoPeerState, isDefaultEntry: Boolean) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:139 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun onExit(state: TodoPeerState) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:139 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun executeTransitionContent(source: TodoPeerState, transitionIndex: Int) {
        when (source) {
        is TodoPeerState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: todo_peer.scxml:140 :: open :: _transition_0
                if (pendingTodoJoinPayload == null) {
                    return
                }

            vNode = pendingTodoJoinPayload!!.node

            vHub = pendingTodoJoinPayload!!.hub
            }
            1 -> {
                // SCE-MAP: todo_peer.scxml:148 :: open :: _transition_1
                if (pendingTodoEditPayload == null) {
                    return
                }

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingTodoEditPayload!!.text, 32)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, pendingTodoEditPayload!!.now, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vLive = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vTombstones)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='live'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(element = pendingTodoEditPayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingTodoEditPayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vObserved = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetObserved__v(vLive, vEntry.element)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='observed'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (o in vObserved) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (o.wallTime).toString())

            putParam(sendPayload, "counter", (o.counter).toLong())

            putParam(sendPayload, "nodeId", (o.nodeId).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Supersede, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (o.wallTime).toString())

            putParam(sendPayload, "counter", (o.counter).toLong())

            putParam(sendPayload, "nodeId", (o.nodeId).toString())

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
                eventName = "todo.supersede",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingTodoEditPayload!!.value).toString())

            putParam(sendPayload, "text", pendingTodoEditPayload!!.text)

            putParam(sendPayload, "wallTime", (vClock.wallTime).toString())

            putParam(sendPayload, "counter", (vClock.counter).toLong())

            putParam(sendPayload, "nodeId", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Write, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingTodoEditPayload!!.value).toString())

            putParam(sendPayload, "text", pendingTodoEditPayload!!.text)

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
                eventName = "todo.write",
                target = "#server",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_3")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: todo_peer.scxml:197 :: open :: _transition_2
                if (pendingTodoWritePayload == null) {
                    return
                }

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingTodoWritePayload!!.text, 32)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (pendingTodoWritePayload!!.nodeId != vNode) {

            vStamp = vStamp.copy(wallTime = pendingTodoWritePayload!!.wallTime)

            vStamp = vStamp.copy(counter = pendingTodoWritePayload!!.counter)

            vStamp = vStamp.copy(nodeId = pendingTodoWritePayload!!.nodeId)

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcReceive__vv(vClock, vStamp, vClock.wallTime, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (pendingTodoWritePayload!!.nodeId == vNode && (pendingTodoWritePayload!!.wallTime > vClock.wallTime || pendingTodoWritePayload!!.wallTime == vClock.wallTime && pendingTodoWritePayload!!.counter > vClock.counter)) {

            vClock = vClock.copy(wallTime = pendingTodoWritePayload!!.wallTime)

            vClock = vClock.copy(counter = pendingTodoWritePayload!!.counter)
            }

            vEntry = vEntry.copy(element = pendingTodoWritePayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingTodoWritePayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(wallTime = pendingTodoWritePayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingTodoWritePayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingTodoWritePayload!!.nodeId)

            vOne = emptyList()

            if (if (vOne.size < 256) { vOne = vOne + (vEntry); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 256"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vAdds, vOne)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vLive = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vTombstones)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='live'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vIsLive = false


            for (l in vLive) {


            if (l.element == vEntry.element && l.wallTime == vEntry.wallTime && l.counter == vEntry.counter && l.nodeId == vEntry.nodeId) {

            vIsLive = true
            }
            }

            vHeld = vHeld.copy(id = pendingTodoWritePayload!!.id.toULong())

            vHeld = vHeld.copy(field = pendingTodoWritePayload!!.field.toULong())

            vHeld = vHeld.copy(value = pendingTodoWritePayload!!.value)

            vHeld = vHeld.copy(wallTime = pendingTodoWritePayload!!.wallTime)

            vHeld = vHeld.copy(counter = pendingTodoWritePayload!!.counter)

            vHeld = vHeld.copy(nodeId = pendingTodoWritePayload!!.nodeId)

            vPresent = call_MvValueHolds__vv(vValues, vHeld)


            if (vIsLive && !vPresent) {

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueKeep__vv(vValues, vHeld)), 128); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kind", (0).toLong())

            putParam(sendPayload, "element", (vEntry.element).toString())

            putParam(sendPayload, "wallTime", (pendingTodoWritePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoWritePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoWritePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Keep, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoWritePayload!!.nodeId != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoWritePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoWritePayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingTodoWritePayload!!.value).toString())

            putParam(sendPayload, "text", pendingTodoWritePayload!!.text)

            putParam(sendPayload, "wallTime", (pendingTodoWritePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoWritePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoWritePayload!!.nodeId).toString())

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
                eventName = "todo.write",
                target = "#alice",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_5")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoWritePayload!!.nodeId != 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoWritePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoWritePayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingTodoWritePayload!!.value).toString())

            putParam(sendPayload, "text", pendingTodoWritePayload!!.text)

            putParam(sendPayload, "wallTime", (pendingTodoWritePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoWritePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoWritePayload!!.nodeId).toString())

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
                eventName = "todo.write",
                target = "#bob",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_6")
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
                // SCE-MAP: todo_peer.scxml:279 :: open :: _transition_3
                if (pendingTodoSupersedePayload == null) {
                    return
                }

            vEntry = vEntry.copy(element = pendingTodoSupersedePayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingTodoSupersedePayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(wallTime = pendingTodoSupersedePayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingTodoSupersedePayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingTodoSupersedePayload!!.nodeId)

            vKnown = false


            for (t in vTombstones) {


            if (t.element == vEntry.element && t.wallTime == vEntry.wallTime && t.counter == vEntry.counter && t.nodeId == vEntry.nodeId) {

            vKnown = true
            }
            }

            vOne = emptyList()

            if (if (vOne.size < 256) { vOne = vOne + (vEntry); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 256"); true }) {
                return
            }

            if (try { vTombstones = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vTombstones, vOne)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='tombstones'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(id = pendingTodoSupersedePayload!!.id.toULong())

            vHeld = vHeld.copy(field = pendingTodoSupersedePayload!!.field.toULong())

            vHeld = vHeld.copy(wallTime = pendingTodoSupersedePayload!!.wallTime)

            vHeld = vHeld.copy(counter = pendingTodoSupersedePayload!!.counter)

            vHeld = vHeld.copy(nodeId = pendingTodoSupersedePayload!!.nodeId)

            vWasLive = call_MvValueHolds__vv(vValues, vHeld)

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueDrop__vv(vValues, vHeld)), 128); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (!vKnown) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kind", (1).toLong())

            putParam(sendPayload, "element", (vEntry.element).toString())

            putParam(sendPayload, "wallTime", (pendingTodoSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoSupersedePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Keep, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vWasLive) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kind", (0).toLong())

            putParam(sendPayload, "element", (vEntry.element).toString())

            putParam(sendPayload, "wallTime", (pendingTodoSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoSupersedePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Keep, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSupersedePayload!!.by != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoSupersedePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoSupersedePayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (pendingTodoSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoSupersedePayload!!.nodeId).toString())

            putParam(sendPayload, "by", (pendingTodoSupersedePayload!!.by).toString())

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
                eventName = "todo.supersede",
                target = "#alice",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_9")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSupersedePayload!!.by != 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoSupersedePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingTodoSupersedePayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (pendingTodoSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingTodoSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingTodoSupersedePayload!!.nodeId).toString())

            putParam(sendPayload, "by", (pendingTodoSupersedePayload!!.by).toString())

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
                eventName = "todo.supersede",
                target = "#bob",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_10")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            4 -> {
                // SCE-MAP: todo_peer.scxml:350 :: open :: _transition_4
                if (pendingTodoKeepPayload == null) {
                    return
                }

            vEntry = vEntry.copy(element = pendingTodoKeepPayload!!.element)

            vEntry = vEntry.copy(wallTime = pendingTodoKeepPayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingTodoKeepPayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingTodoKeepPayload!!.nodeId)

            if (try { vMine = com.sce.forge.runtime.SceChecked.take(call_MerkleChangeHash__v(vEntry, pendingTodoKeepPayload!!.kind)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='mine'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDigest = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MerkleInsert__v(vDigest, com.sce.forge.runtime.SceChecked.div(pendingTodoKeepPayload!!.wallTime, 60000), vMine)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='digest'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: todo_peer.scxml:362 :: open :: _transition_5


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (0).toLong())

            putParam(sendPayload, "depth", (0).toLong())

            putParam(sendPayload, "prefix", (0).toLong())

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (1).toLong())

            putParam(sendPayload, "depth", (0).toLong())

            putParam(sendPayload, "prefix", (0).toLong())

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (2).toLong())

            putParam(sendPayload, "depth", (0).toLong())

            putParam(sendPayload, "prefix", (0).toLong())

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            6 -> {
                // SCE-MAP: todo_peer.scxml:389 :: open :: _transition_6
                if (pendingTodoVisitPayload == null) {
                    return
                }

            vMine = 0.toUInt()


            for (n in vDigest) {


            if (n.depth == pendingTodoVisitPayload!!.depth && n.prefix == pendingTodoVisitPayload!!.prefix) {

            vMine = n.hash
            }
            }


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "depth", (pendingTodoVisitPayload!!.depth).toLong())

            putParam(sendPayload, "prefix", (pendingTodoVisitPayload!!.prefix).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            putParam(sendPayload, "reply", pendingTodoVisitPayload!!.reply)

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
                eventName = "todo.node",
                target = "#server",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_14")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoVisitPayload!!.to == 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "depth", (pendingTodoVisitPayload!!.depth).toLong())

            putParam(sendPayload, "prefix", (pendingTodoVisitPayload!!.prefix).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            putParam(sendPayload, "reply", pendingTodoVisitPayload!!.reply)

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
                eventName = "todo.node",
                target = "#alice",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_15")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoVisitPayload!!.to == 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "depth", (pendingTodoVisitPayload!!.depth).toLong())

            putParam(sendPayload, "prefix", (pendingTodoVisitPayload!!.prefix).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            putParam(sendPayload, "reply", pendingTodoVisitPayload!!.reply)

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
                eventName = "todo.node",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_16",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_16")
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
                // SCE-MAP: todo_peer.scxml:430 :: open :: _transition_7
                if (pendingTodoNodePayload == null) {
                    return
                }

            vMine = 0.toUInt()


            for (n in vDigest) {


            if (n.depth == pendingTodoNodePayload!!.depth && n.prefix == pendingTodoNodePayload!!.prefix) {

            vMine = n.hash
            }
            }


            if (vMine != pendingTodoNodePayload!!.hash && pendingTodoNodePayload!!.depth < 30.toUByte()) {


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingTodoNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingTodoNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.mul(pendingTodoNodePayload!!.prefix, 3.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingTodoNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingTodoNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(pendingTodoNodePayload!!.prefix, 3.toULong()), 1.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingTodoNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingTodoNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(pendingTodoNodePayload!!.prefix, 3.toULong()), 2.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vMine != pendingTodoNodePayload!!.hash && pendingTodoNodePayload!!.depth == 30.toUByte()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingTodoNodePayload!!.sender).toString())

            putParam(sendPayload, "minute", (pendingTodoNodePayload!!.prefix).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Share, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vMine != pendingTodoNodePayload!!.hash && pendingTodoNodePayload!!.depth == 30.toUByte() && !pendingTodoNodePayload!!.reply) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingTodoNodePayload!!.sender).toString())

            putParam(sendPayload, "depth", (30).toLong())

            putParam(sendPayload, "prefix", (pendingTodoNodePayload!!.prefix).toString())

            putParam(sendPayload, "reply", true)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            8 -> {
                // SCE-MAP: todo_peer.scxml:477 :: open :: _transition_8
                if (pendingTodoSharePayload == null) {
                    return
                }


            for (v in vValues) {


            var ifCondFailed22 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(v.wallTime, 60000) == pendingTodoSharePayload!!.minute.toLong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<if cond='v.wallTime / 60000 === _event.data.minute'>: an integer operation overflowed or failed"); ifCondFailed22 = true; false })) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(v.id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(v.field); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "value", (v.value).toString())

            putParam(sendPayload, "text", v.text)

            putParam(sendPayload, "wallTime", (v.wallTime).toString())

            putParam(sendPayload, "counter", (v.counter).toLong())

            putParam(sendPayload, "nodeId", (v.nodeId).toString())

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
                eventName = "todo.write",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_22",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_22")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSharePayload!!.to == 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "value", (v.value).toString())

            putParam(sendPayload, "text", v.text)

            putParam(sendPayload, "wallTime", (v.wallTime).toString())

            putParam(sendPayload, "counter", (v.counter).toLong())

            putParam(sendPayload, "nodeId", (v.nodeId).toString())

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
                eventName = "todo.write",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_23",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_23")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSharePayload!!.to == 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "value", (v.value).toString())

            putParam(sendPayload, "text", v.text)

            putParam(sendPayload, "wallTime", (v.wallTime).toString())

            putParam(sendPayload, "counter", (v.counter).toLong())

            putParam(sendPayload, "nodeId", (v.nodeId).toString())

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
                eventName = "todo.write",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_24",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_24")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed22) {
                return
            }
            }


            for (t in vTombstones) {


            var ifCondFailed26 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(t.wallTime, 60000) == pendingTodoSharePayload!!.minute.toLong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<if cond='t.wallTime / 60000 === _event.data.minute'>: an integer operation overflowed or failed"); ifCondFailed26 = true; false })) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(com.sce.forge.runtime.SceChecked.div(t.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(com.sce.forge.runtime.SceChecked.rem(t.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "wallTime", (t.wallTime).toString())

            putParam(sendPayload, "counter", (t.counter).toLong())

            putParam(sendPayload, "nodeId", (t.nodeId).toString())

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
                eventName = "todo.supersede",
                target = "#server",
                content = "",
                params = sendWireParams,
                sendId = "__send_25",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_25")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSharePayload!!.to == 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "wallTime", (t.wallTime).toString())

            putParam(sendPayload, "counter", (t.counter).toLong())

            putParam(sendPayload, "nodeId", (t.nodeId).toString())

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
                eventName = "todo.supersede",
                target = "#alice",
                content = "",
                params = sendWireParams,
                sendId = "__send_26",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_26")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vHub && pendingTodoSharePayload!!.to == 2.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "wallTime", (t.wallTime).toString())

            putParam(sendPayload, "counter", (t.counter).toLong())

            putParam(sendPayload, "nodeId", (t.nodeId).toString())

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
                eventName = "todo.supersede",
                target = "#bob",
                content = "",
                params = sendWireParams,
                sendId = "__send_27",
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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_27")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed26) {
                return
            }
            }
            }
            9 -> {
                // SCE-MAP: todo_peer.scxml:555 :: open :: _transition_9

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
