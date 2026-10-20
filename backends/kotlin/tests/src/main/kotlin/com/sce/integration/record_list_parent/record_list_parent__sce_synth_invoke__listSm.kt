// SCE-GENERATED — DO NOT EDIT
// source-hash: 438f3c44a0f49109d71c51cacca59b20d771e2147409672affa707c8d139f3c1

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/record_list_parent__sce_synth_invoke__list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: record_list.scxml:156 :: _machine

package com.sce.integration.record_list_parent

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
import com.sce.generated.orset_union.*
import com.sce.generated.record_member.*
import com.sce.generated.record_roster.*
import com.sce.generated.record_settled.*
import com.sce.generated.register_values.*


// --- States (W3C SCXML 3.2) ---

sealed interface RecordListParentSceSynthInvokeListState : State {
    data object Open : RecordListParentSceSynthInvokeListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface RecordListParentSceSynthInvokeListEvent : Event {
    sealed interface Error : RecordListParentSceSynthInvokeListEvent {
        data object Communication : Error
        data object Execution : Error
    }
    sealed interface RecordList : RecordListParentSceSynthInvokeListEvent {
        data object Ack : RecordList
        data object Announce : RecordList
        data object Compact : RecordList
        data object Edit : RecordList
        data object Floor : RecordList
        data object Join : RecordList
        data object Keep : RecordList
        data object Node : RecordList
        data object Propose : RecordList
        data object Register : RecordList
        data object Resync : RecordList
        data object Role : RecordList
        data object Room : RecordList
        data object Settle : RecordList
        data object Share : RecordList
        data object Supersede : RecordList
        data object Tell : RecordList
        data object Value : RecordList
        data object Visit : RecordList
        data object Write : RecordList
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `RecordListParentSceSynthInvokeListPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// RecordListParentSceSynthInvokeListRecordListAckPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.ack`. Consumers inject it via the `raiseRecordListAck` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListAckPayload(val sender: ULong, val before: Long, val hash: UInt)

// RecordListParentSceSynthInvokeListRecordListAnnouncePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.announce`. Consumers inject it via the `raiseRecordListAnnounce` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListAnnouncePayload(val id: UInt, val field: UShort)

// RecordListParentSceSynthInvokeListRecordListCompactPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.compact`. Consumers inject it via the `raiseRecordListCompact` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListCompactPayload(val before: Long)

// RecordListParentSceSynthInvokeListRecordListEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.edit`. Consumers inject it via the `raiseRecordListEdit` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListEditPayload(val id: UInt, val field: UShort, val value: Long, val text: String, val now: Long)

// RecordListParentSceSynthInvokeListRecordListFloorPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.floor`. Consumers inject it via the `raiseRecordListFloor` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListFloorPayload(val before: Long, val hash: UInt)

// RecordListParentSceSynthInvokeListRecordListJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.join`. Consumers inject it via the `raiseRecordListJoin` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListJoinPayload(val node: ULong, val hub: Boolean)

// RecordListParentSceSynthInvokeListRecordListKeepPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.keep`. Consumers inject it via the `raiseRecordListKeep` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListKeepPayload(val kind: UByte, val element: ULong, val wallTime: Long, val counter: UInt, val nodeId: ULong)

// RecordListParentSceSynthInvokeListRecordListNodePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.node`. Consumers inject it via the `raiseRecordListNode` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListNodePayload(val sender: ULong, val to: ULong, val depth: UByte, val prefix: ULong, val hash: UInt, val reply: Boolean)

// RecordListParentSceSynthInvokeListRecordListProposePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.propose`. Consumers inject it via the `raiseRecordListPropose` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListProposePayload(val before: Long)

// RecordListParentSceSynthInvokeListRecordListRolePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.role`. Consumers inject it via the `raiseRecordListRole` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRolePayload(val node: ULong, val role: UByte)

// RecordListParentSceSynthInvokeListRecordListRoomPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.room`. Consumers inject it via the `raiseRecordListRoom` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRoomPayload(val values: UInt, val history: UInt, val digest: UInt)

// RecordListParentSceSynthInvokeListRecordListSettlePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.settle`. Consumers inject it via the `raiseRecordListSettle` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListSettlePayload(val before: Long, val members: UInt)

// RecordListParentSceSynthInvokeListRecordListSharePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.share`. Consumers inject it via the `raiseRecordListShare` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListSharePayload(val to: ULong, val minute: ULong)

// RecordListParentSceSynthInvokeListRecordListSupersedePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.supersede`. Consumers inject it via the `raiseRecordListSupersede` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListSupersedePayload(val id: UInt, val field: UShort, val wallTime: Long, val counter: UInt, val nodeId: ULong, val by: ULong)

// RecordListParentSceSynthInvokeListRecordListVisitPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.visit`. Consumers inject it via the `raiseRecordListVisit` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListVisitPayload(val to: ULong, val depth: UByte, val prefix: ULong, val reply: Boolean)

// RecordListParentSceSynthInvokeListRecordListWritePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.write`. Consumers inject it via the `raiseRecordListWrite` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListWritePayload(val id: UInt, val field: UShort, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Hlc` datamodel value. */
data class RecordListParentSceSynthInvokeListHlcRecord(val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Entry` datamodel value. */
data class RecordListParentSceSynthInvokeListEntryRecord(val element: ULong, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("element" to SavedValues.of(element), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListEntryRecord = RecordListParentSceSynthInvokeListEntryRecord(element = SavedValues.uint64(SavedValues.field(value, what, "element"), "$what.element"), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Value` datamodel value. */
data class RecordListParentSceSynthInvokeListValueRecord(val id: ULong, val field: ULong, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListValueRecord = RecordListParentSceSynthInvokeListValueRecord(id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint64(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Node` datamodel value. */
data class RecordListParentSceSynthInvokeListNodeRecord(val depth: UByte, val prefix: ULong, val hash: UInt) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("depth" to SavedValues.of(depth), "prefix" to SavedValues.of(prefix), "hash" to SavedValues.of(hash))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListNodeRecord = RecordListParentSceSynthInvokeListNodeRecord(depth = SavedValues.uint8(SavedValues.field(value, what, "depth"), "$what.depth"), prefix = SavedValues.uint64(SavedValues.field(value, what, "prefix"), "$what.prefix"), hash = SavedValues.uint32(SavedValues.field(value, what, "hash"), "$what.hash"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Answer` datamodel value. */
data class RecordListParentSceSynthInvokeListAnswerRecord(val sender: ULong, val hash: UInt) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("sender" to SavedValues.of(sender), "hash" to SavedValues.of(hash))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListAnswerRecord = RecordListParentSceSynthInvokeListAnswerRecord(sender = SavedValues.uint64(SavedValues.field(value, what, "sender"), "$what.sender"), hash = SavedValues.uint32(SavedValues.field(value, what, "hash"), "$what.hash"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Member` datamodel value. */
data class RecordListParentSceSynthInvokeListMemberRecord(val node: ULong, val role: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("node" to SavedValues.of(node), "role" to SavedValues.of(role))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListMemberRecord = RecordListParentSceSynthInvokeListMemberRecord(node = SavedValues.uint64(SavedValues.field(value, what, "node"), "$what.node"), role = SavedValues.uint8(SavedValues.field(value, what, "role"), "$what.role"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_HlcReceive__vv(p0: RecordListParentSceSynthInvokeListHlcRecord, p1: RecordListParentSceSynthInvokeListHlcRecord, p2: Long, p3: ULong): com.sce.forge.runtime.AlgorithmResult<RecordListParentSceSynthInvokeListHlcRecord> =
    when (val r = hlcReceive(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), HlcTimestampPayload(wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId), p2, p3)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(RecordListParentSceSynthInvokeListHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_HlcSend__v(p0: RecordListParentSceSynthInvokeListHlcRecord, p1: Long, p2: ULong): com.sce.forge.runtime.AlgorithmResult<RecordListParentSceSynthInvokeListHlcRecord> =
    when (val r = hlcSend(HlcTimestampPayload(wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(RecordListParentSceSynthInvokeListHlcRecord(wallTime = r.value.wallTime, counter = r.value.counter, nodeId = r.value.nodeId))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MerkleChangeHash__i(p0: RecordListParentSceSynthInvokeListEntryRecord, p1: UByte): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = merkleChangeHash(OrsetEntryPayload(element = p0.element, wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MerkleChangeHash__v(p0: RecordListParentSceSynthInvokeListEntryRecord, p1: UByte): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = merkleChangeHash(OrsetEntryPayload(element = p0.element, wallTime = p0.wallTime, counter = p0.counter, nodeId = p0.nodeId), p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MerkleInsert__v(p0: List<RecordListParentSceSynthInvokeListNodeRecord>, p1: Long, p2: UInt): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListNodeRecord>> =
    when (val r = merkleInsert(p0.map { MerkleNodePayload(depth = it.depth, prefix = it.prefix, hash = it.hash) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListNodeRecord(depth = it.depth, prefix = it.prefix, hash = it.hash) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MvValueDrop__vv(p0: List<RecordListParentSceSynthInvokeListValueRecord>, p1: RecordListParentSceSynthInvokeListValueRecord): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListValueRecord>> =
    when (val r = mvValueDrop(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_MvValueHolds__vv(p0: List<RecordListParentSceSynthInvokeListValueRecord>, p1: RecordListParentSceSynthInvokeListValueRecord): Boolean {
    val r = mvValueHolds(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))
    return r
}
private fun call_MvValueKeep__vv(p0: List<RecordListParentSceSynthInvokeListValueRecord>, p1: RecordListParentSceSynthInvokeListValueRecord): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListValueRecord>> =
    when (val r = mvValueKeep(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, MvValuePayload(id = p1.id, field = p1.field, value = p1.value, text = p1.text, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_OrsetLive__vv(p0: List<RecordListParentSceSynthInvokeListEntryRecord>, p1: List<RecordListParentSceSynthInvokeListEntryRecord>): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListEntryRecord>> =
    when (val r = orsetLive(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_OrsetUnion__vv(p0: List<RecordListParentSceSynthInvokeListEntryRecord>, p1: List<RecordListParentSceSynthInvokeListEntryRecord>): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListEntryRecord>> =
    when (val r = orsetUnion(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_RecordRoster__v(p0: List<RecordListParentSceSynthInvokeListMemberRecord>, p1: ULong, p2: UByte): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListMemberRecord>> =
    when (val r = recordRoster(p0.map { RecordMemberPayload(node = it.node, role = it.role) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListMemberRecord(node = it.node, role = it.role) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_RecordSettled__vv(p0: List<RecordListParentSceSynthInvokeListEntryRecord>, p1: List<RecordListParentSceSynthInvokeListEntryRecord>, p2: Long): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = recordSettled(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_RegisterValues__v(p0: List<RecordListParentSceSynthInvokeListValueRecord>, p1: ULong, p2: ULong): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListValueRecord>> =
    when (val r = registerValues(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class RecordListParentSceSynthInvokeListStateMachine(
) : StateMachineEngine<RecordListParentSceSynthInvokeListState, RecordListParentSceSynthInvokeListEvent>() {

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
    private var vClock: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `stamp` datamodel variable, the machine's own. */
    private var vStamp: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `adds` datamodel variable, published (`sce:direction="out"`). */
    private var vAdds: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()

    /** What the published `adds` holds. */
    val adds: List<RecordListParentSceSynthInvokeListEntryRecord> get() = vAdds
    /** W3C SCXML 5.2: the `tombstones` datamodel variable, published (`sce:direction="out"`). */
    private var vTombstones: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()

    /** What the published `tombstones` holds. */
    val tombstones: List<RecordListParentSceSynthInvokeListEntryRecord> get() = vTombstones
    /** W3C SCXML 5.2: the `one` datamodel variable, the machine's own. */
    private var vOne: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `stable` datamodel variable, the machine's own. */
    private var vStable: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `entry` datamodel variable, the machine's own. */
    private var vEntry: RecordListParentSceSynthInvokeListEntryRecord = RecordListParentSceSynthInvokeListEntryRecord(element = 0.toULong(), wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `values` datamodel variable, published (`sce:direction="out"`). */
    private var vValues: List<RecordListParentSceSynthInvokeListValueRecord> = emptyList()

    /** What the published `values` holds. */
    val values: List<RecordListParentSceSynthInvokeListValueRecord> get() = vValues
    /** W3C SCXML 5.2: the `typed` datamodel variable, the machine's own. */
    private var vTyped: String = ""
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: RecordListParentSceSynthInvokeListValueRecord = RecordListParentSceSynthInvokeListValueRecord(id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `digest` datamodel variable, published (`sce:direction="out"`). */
    private var vDigest: List<RecordListParentSceSynthInvokeListNodeRecord> = emptyList()

    /** What the published `digest` holds. */
    val digest: List<RecordListParentSceSynthInvokeListNodeRecord> get() = vDigest
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
    /** W3C SCXML 5.2: the `floor` datamodel variable, published (`sce:direction="out"`). */
    private var vFloor: Long = 0

    /** What the published `floor` holds. */
    val floor: Long get() = vFloor
    /** W3C SCXML 5.2: the `stale` datamodel variable, published (`sce:direction="out"`). */
    private var vStale: Boolean = false

    /** What the published `stale` holds. */
    val stale: Boolean get() = vStale
    /** W3C SCXML 5.2: the `late` datamodel variable, published (`sce:direction="out"`). */
    private var vLate: UInt = 0.toUInt()

    /** What the published `late` holds. */
    val late: UInt get() = vLate
    /** W3C SCXML 5.2: the `frozenFor` datamodel variable, the machine's own. */
    private var vFrozenFor: Long = -1
    /** W3C SCXML 5.2: the `frozenHash` datamodel variable, the machine's own. */
    private var vFrozenHash: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `pending` datamodel variable, the machine's own. */
    private var vPending: Long = -1
    /** W3C SCXML 5.2: the `members` datamodel variable, the machine's own. */
    private var vMembers: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `agreeing` datamodel variable, the machine's own. */
    private var vAgreeing: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `answers` datamodel variable, the machine's own. */
    private var vAnswers: List<RecordListParentSceSynthInvokeListAnswerRecord> = emptyList()
    /** W3C SCXML 5.2: the `answer` datamodel variable, the machine's own. */
    private var vAnswer: RecordListParentSceSynthInvokeListAnswerRecord = RecordListParentSceSynthInvokeListAnswerRecord(sender = 0.toULong(), hash = 0.toUInt())
    /** W3C SCXML 5.2: the `agreed` datamodel variable, the machine's own. */
    private var vAgreed: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `roomValues` datamodel variable, the machine's own. */
    private var vRoomValues: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `roomHistory` datamodel variable, the machine's own. */
    private var vRoomHistory: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `roomDigest` datamodel variable, the machine's own. */
    private var vRoomDigest: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `full` datamodel variable, published (`sce:direction="out"`). */
    private var vFull: UInt = 0.toUInt()

    /** What the published `full` holds. */
    val full: UInt get() = vFull
    /** W3C SCXML 5.2: the `fresh` datamodel variable, the machine's own. */
    private var vFresh: Boolean = false
    /** W3C SCXML 5.2: the `blocked` datamodel variable, the machine's own. */
    private var vBlocked: Boolean = false
    /** W3C SCXML 5.2: the `roster` datamodel variable, the machine's own. */
    private var vRoster: List<RecordListParentSceSynthInvokeListMemberRecord> = emptyList()
    /** W3C SCXML 5.2: the `denied` datamodel variable, published (`sce:direction="out"`). */
    private var vDenied: UInt = 0.toUInt()

    /** What the published `denied` holds. */
    val denied: UInt get() = vDenied
    /** W3C SCXML 5.2: the `forbidden` datamodel variable, the machine's own. */
    private var vForbidden: Boolean = false
    /** W3C SCXML 5.2: the `authorized` datamodel variable, the machine's own. */
    private var vAuthorized: Boolean = true
    /** W3C SCXML 5.2: the `announce` datamodel variable, the machine's own. */
    private var vAnnounce: Boolean = false
    /** W3C SCXML 5.2: the `announced` datamodel variable, published (`sce:direction="out"`). */
    private var vAnnounced: UInt = 0.toUInt()

    /** What the published `announced` holds. */
    val announced: UInt get() = vAnnounced
    /** W3C SCXML 5.2: the `moved` datamodel variable, the machine's own. */
    private var vMoved: List<ULong> = emptyList()
    /** W3C SCXML 5.2: the `listed` datamodel variable, the machine's own. */
    private var vListed: Boolean = false
    /** W3C SCXML 5.2: the `live` datamodel variable, the machine's own. */
    private var vLive: List<RecordListParentSceSynthInvokeListValueRecord> = emptyList()
    /** W3C SCXML 5.2: the `ordinal` datamodel variable, the machine's own. */
    private var vOrdinal: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `empty` datamodel variable, the machine's own. */
    private var vEmpty: RecordListParentSceSynthInvokeListValueRecord = RecordListParentSceSynthInvokeListValueRecord(id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var node: ULong? = null
        var hub: Boolean? = null
        var typed: String? = null
        var mine: UInt? = null
        var shareId: UInt? = null
        var shareField: UShort? = null
        var known: Boolean? = null
        var wasLive: Boolean? = null
        var isLive: Boolean? = null
        var present: Boolean? = null
        var refusals: UInt? = null
        var floor: Long? = null
        var stale: Boolean? = null
        var late: UInt? = null
        var frozenFor: Long? = null
        var frozenHash: UInt? = null
        var pending: Long? = null
        var members: UInt? = null
        var agreeing: UInt? = null
        var agreed: UInt? = null
        var roomValues: UInt? = null
        var roomHistory: UInt? = null
        var roomDigest: UInt? = null
        var full: UInt? = null
        var fresh: Boolean? = null
        var blocked: Boolean? = null
        var denied: UInt? = null
        var forbidden: Boolean? = null
        var authorized: Boolean? = null
        var announce: Boolean? = null
        var announced: UInt? = null
        var listed: Boolean? = null
        var ordinal: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.hub?.let { vHub = it }
        params.typed?.let { vTyped = it }
        params.mine?.let { vMine = it }
        params.shareId?.let { vShareId = it }
        params.shareField?.let { vShareField = it }
        params.known?.let { vKnown = it }
        params.wasLive?.let { vWasLive = it }
        params.isLive?.let { vIsLive = it }
        params.present?.let { vPresent = it }
        params.refusals?.let { vRefusals = it }
        params.floor?.let { vFloor = it }
        params.stale?.let { vStale = it }
        params.late?.let { vLate = it }
        params.frozenFor?.let { vFrozenFor = it }
        params.frozenHash?.let { vFrozenHash = it }
        params.pending?.let { vPending = it }
        params.members?.let { vMembers = it }
        params.agreeing?.let { vAgreeing = it }
        params.agreed?.let { vAgreed = it }
        params.roomValues?.let { vRoomValues = it }
        params.roomHistory?.let { vRoomHistory = it }
        params.roomDigest?.let { vRoomDigest = it }
        params.full?.let { vFull = it }
        params.fresh?.let { vFresh = it }
        params.blocked?.let { vBlocked = it }
        params.denied?.let { vDenied = it }
        params.forbidden?.let { vForbidden = it }
        params.authorized?.let { vAuthorized = it }
        params.announce?.let { vAnnounce = it }
        params.announced?.let { vAnnounced = it }
        params.listed?.let { vListed = it }
        params.ordinal?.let { vOrdinal = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val hub: Boolean,
        val clock: RecordListParentSceSynthInvokeListHlcRecord,
        val adds: List<RecordListParentSceSynthInvokeListEntryRecord>,
        val tombstones: List<RecordListParentSceSynthInvokeListEntryRecord>,
        val values: List<RecordListParentSceSynthInvokeListValueRecord>,
        val digest: List<RecordListParentSceSynthInvokeListNodeRecord>,
        val refusals: UInt,
        val floor: Long,
        val stale: Boolean,
        val late: UInt,
        val full: UInt,
        val denied: UInt,
        val announced: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<RecordListParentSceSynthInvokeListState>,
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
        floor = vFloor,
        stale = vStale,
        late = vLate,
        full = vFull,
        denied = vDenied,
        announced = vAnnounced,
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
        first = "5dc7614d1325a4df4b3e5f02c9118a98386acef5798da6ee07a0a213bee98b14",
        revised = "da0b6da5dff444c8bbc3da0b1c703a8ce54c2ad5c4ce3ca5f731502eeaaa44ff",
        records = listOf(
            RecordLineage(
                event = "merge.hlc_timestamp",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("clock", false),
                    RecordHolder("stamp", false),
                ),
            ),
            RecordLineage(
                event = "merge.merkle_node",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("digest", true),
                ),
            ),
            RecordLineage(
                event = "merge.mv_value",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("values", true),
                    RecordHolder("held", false),
                    RecordHolder("live", true),
                    RecordHolder("empty", false),
                ),
            ),
            RecordLineage(
                event = "merge.orset_entry",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("adds", true),
                    RecordHolder("tombstones", true),
                    RecordHolder("one", true),
                    RecordHolder("stable", true),
                    RecordHolder("entry", false),
                ),
            ),
            RecordLineage(
                event = "record_list.answer",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("answers", true),
                    RecordHolder("answer", false),
                ),
            ),
            RecordLineage(
                event = "record_list.member",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("roster", true),
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
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "node" to SavedValues.of(vNode),
            "hub" to SavedValues.of(vHub),
            "clock" to vClock.toSaved(),
            "stamp" to vStamp.toSaved(),
            "adds" to SavedValues.list(vAdds) { it.toSaved() },
            "tombstones" to SavedValues.list(vTombstones) { it.toSaved() },
            "one" to SavedValues.list(vOne) { it.toSaved() },
            "stable" to SavedValues.list(vStable) { it.toSaved() },
            "entry" to vEntry.toSaved(),
            "values" to SavedValues.list(vValues) { it.toSaved() },
            "typed" to SavedValues.of(vTyped),
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
            "floor" to SavedValues.of(vFloor),
            "stale" to SavedValues.of(vStale),
            "late" to SavedValues.of(vLate),
            "frozenFor" to SavedValues.of(vFrozenFor),
            "frozenHash" to SavedValues.of(vFrozenHash),
            "pending" to SavedValues.of(vPending),
            "members" to SavedValues.of(vMembers),
            "agreeing" to SavedValues.of(vAgreeing),
            "answers" to SavedValues.list(vAnswers) { it.toSaved() },
            "answer" to vAnswer.toSaved(),
            "agreed" to SavedValues.of(vAgreed),
            "roomValues" to SavedValues.of(vRoomValues),
            "roomHistory" to SavedValues.of(vRoomHistory),
            "roomDigest" to SavedValues.of(vRoomDigest),
            "full" to SavedValues.of(vFull),
            "fresh" to SavedValues.of(vFresh),
            "blocked" to SavedValues.of(vBlocked),
            "roster" to SavedValues.list(vRoster) { it.toSaved() },
            "denied" to SavedValues.of(vDenied),
            "forbidden" to SavedValues.of(vForbidden),
            "authorized" to SavedValues.of(vAuthorized),
            "announce" to SavedValues.of(vAnnounce),
            "announced" to SavedValues.of(vAnnounced),
            "moved" to SavedValues.list(vMoved) { SavedValues.of(it) },
            "listed" to SavedValues.of(vListed),
            "live" to SavedValues.list(vLive) { it.toSaved() },
            "ordinal" to SavedValues.of(vOrdinal),
            "empty" to vEmpty.toSaved(),
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
        val saved1 = SavedValues.uint64(savedGrown.variable("node"), "node")
        val saved2 = SavedValues.bool(savedGrown.variable("hub"), "hub")
        val saved3 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("clock"), "clock")
        val saved4 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("stamp"), "stamp")
        val saved5 = SavedValues.list(savedGrown.variable("adds"), "adds", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved6 = SavedValues.list(savedGrown.variable("tombstones"), "tombstones", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved7 = SavedValues.list(savedGrown.variable("one"), "one", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved8 = SavedValues.list(savedGrown.variable("stable"), "stable", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved9 = RecordListParentSceSynthInvokeListEntryRecord.fromSaved(savedGrown.variable("entry"), "entry")
        val saved10 = SavedValues.list(savedGrown.variable("values"), "values", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        val saved11 = SavedValues.string(savedGrown.variable("typed"), "typed", 32)
        val saved12 = RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("held"), "held")
        val saved13 = SavedValues.list(savedGrown.variable("digest"), "digest", 2048) { e, w -> RecordListParentSceSynthInvokeListNodeRecord.fromSaved(e, w) }
        val saved14 = SavedValues.uint32(savedGrown.variable("mine"), "mine")
        val saved15 = SavedValues.uint32(savedGrown.variable("shareId"), "shareId")
        val saved16 = SavedValues.uint16(savedGrown.variable("shareField"), "shareField")
        val saved17 = SavedValues.bool(savedGrown.variable("known"), "known")
        val saved18 = SavedValues.bool(savedGrown.variable("wasLive"), "wasLive")
        val saved19 = SavedValues.bool(savedGrown.variable("isLive"), "isLive")
        val saved20 = SavedValues.bool(savedGrown.variable("present"), "present")
        val saved21 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        val saved22 = SavedValues.int64(savedGrown.variable("floor"), "floor")
        val saved23 = SavedValues.bool(savedGrown.variable("stale"), "stale")
        val saved24 = SavedValues.uint32(savedGrown.variable("late"), "late")
        val saved25 = SavedValues.int64(savedGrown.variable("frozenFor"), "frozenFor")
        val saved26 = SavedValues.uint32(savedGrown.variable("frozenHash"), "frozenHash")
        val saved27 = SavedValues.int64(savedGrown.variable("pending"), "pending")
        val saved28 = SavedValues.uint32(savedGrown.variable("members"), "members")
        val saved29 = SavedValues.uint32(savedGrown.variable("agreeing"), "agreeing")
        val saved30 = SavedValues.list(savedGrown.variable("answers"), "answers", 512) { e, w -> RecordListParentSceSynthInvokeListAnswerRecord.fromSaved(e, w) }
        val saved31 = RecordListParentSceSynthInvokeListAnswerRecord.fromSaved(savedGrown.variable("answer"), "answer")
        val saved32 = SavedValues.uint32(savedGrown.variable("agreed"), "agreed")
        val saved33 = SavedValues.uint32(savedGrown.variable("roomValues"), "roomValues")
        val saved34 = SavedValues.uint32(savedGrown.variable("roomHistory"), "roomHistory")
        val saved35 = SavedValues.uint32(savedGrown.variable("roomDigest"), "roomDigest")
        val saved36 = SavedValues.uint32(savedGrown.variable("full"), "full")
        val saved37 = SavedValues.bool(savedGrown.variable("fresh"), "fresh")
        val saved38 = SavedValues.bool(savedGrown.variable("blocked"), "blocked")
        val saved39 = SavedValues.list(savedGrown.variable("roster"), "roster", 64) { e, w -> RecordListParentSceSynthInvokeListMemberRecord.fromSaved(e, w) }
        val saved40 = SavedValues.uint32(savedGrown.variable("denied"), "denied")
        val saved41 = SavedValues.bool(savedGrown.variable("forbidden"), "forbidden")
        val saved42 = SavedValues.bool(savedGrown.variable("authorized"), "authorized")
        val saved43 = SavedValues.bool(savedGrown.variable("announce"), "announce")
        val saved44 = SavedValues.uint32(savedGrown.variable("announced"), "announced")
        val saved45 = SavedValues.list(savedGrown.variable("moved"), "moved", 16) { e, w -> SavedValues.uint64(e, w) }
        val saved46 = SavedValues.bool(savedGrown.variable("listed"), "listed")
        val saved47 = SavedValues.list(savedGrown.variable("live"), "live", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        val saved48 = SavedValues.uint32(savedGrown.variable("ordinal"), "ordinal")
        val saved49 = RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("empty"), "empty")
        vNode = saved1
        vHub = saved2
        vClock = saved3
        vStamp = saved4
        vAdds = saved5
        vTombstones = saved6
        vOne = saved7
        vStable = saved8
        vEntry = saved9
        vValues = saved10
        vTyped = saved11
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
        vFloor = saved22
        vStale = saved23
        vLate = saved24
        vFrozenFor = saved25
        vFrozenHash = saved26
        vPending = saved27
        vMembers = saved28
        vAgreeing = saved29
        vAnswers = saved30
        vAnswer = saved31
        vAgreed = saved32
        vRoomValues = saved33
        vRoomHistory = saved34
        vRoomDigest = saved35
        vFull = saved36
        vFresh = saved37
        vBlocked = saved38
        vRoster = saved39
        vDenied = saved40
        vForbidden = saved41
        vAuthorized = saved42
        vAnnounce = saved43
        vAnnounced = saved44
        vMoved = saved45
        vListed = saved46
        vLive = saved47
        vOrdinal = saved48
        vEmpty = saved49
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingRecordListAckPayload: RecordListParentSceSynthInvokeListRecordListAckPayload? = null
    private var pendingRecordListAnnouncePayload: RecordListParentSceSynthInvokeListRecordListAnnouncePayload? = null
    private var pendingRecordListCompactPayload: RecordListParentSceSynthInvokeListRecordListCompactPayload? = null
    private var pendingRecordListEditPayload: RecordListParentSceSynthInvokeListRecordListEditPayload? = null
    private var pendingRecordListFloorPayload: RecordListParentSceSynthInvokeListRecordListFloorPayload? = null
    private var pendingRecordListJoinPayload: RecordListParentSceSynthInvokeListRecordListJoinPayload? = null
    private var pendingRecordListKeepPayload: RecordListParentSceSynthInvokeListRecordListKeepPayload? = null
    private var pendingRecordListNodePayload: RecordListParentSceSynthInvokeListRecordListNodePayload? = null
    private var pendingRecordListProposePayload: RecordListParentSceSynthInvokeListRecordListProposePayload? = null
    private var pendingRecordListRolePayload: RecordListParentSceSynthInvokeListRecordListRolePayload? = null
    private var pendingRecordListRoomPayload: RecordListParentSceSynthInvokeListRecordListRoomPayload? = null
    private var pendingRecordListSettlePayload: RecordListParentSceSynthInvokeListRecordListSettlePayload? = null
    private var pendingRecordListSharePayload: RecordListParentSceSynthInvokeListRecordListSharePayload? = null
    private var pendingRecordListSupersedePayload: RecordListParentSceSynthInvokeListRecordListSupersedePayload? = null
    private var pendingRecordListVisitPayload: RecordListParentSceSynthInvokeListRecordListVisitPayload? = null
    private var pendingRecordListWritePayload: RecordListParentSceSynthInvokeListRecordListWritePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: RecordListParentSceSynthInvokeListEvent, metadata: EventMetadata) {
        pendingRecordListAckPayload = null
        pendingRecordListAnnouncePayload = null
        pendingRecordListCompactPayload = null
        pendingRecordListEditPayload = null
        pendingRecordListFloorPayload = null
        pendingRecordListJoinPayload = null
        pendingRecordListKeepPayload = null
        pendingRecordListNodePayload = null
        pendingRecordListProposePayload = null
        pendingRecordListRolePayload = null
        pendingRecordListRoomPayload = null
        pendingRecordListSettlePayload = null
        pendingRecordListSharePayload = null
        pendingRecordListSupersedePayload = null
        pendingRecordListVisitPayload = null
        pendingRecordListWritePayload = null
        when (val tp = metadata.typedPayload) {
            is RecordListParentSceSynthInvokeListRecordListAckPayload -> pendingRecordListAckPayload = tp
            is RecordListParentSceSynthInvokeListRecordListAnnouncePayload -> pendingRecordListAnnouncePayload = tp
            is RecordListParentSceSynthInvokeListRecordListCompactPayload -> pendingRecordListCompactPayload = tp
            is RecordListParentSceSynthInvokeListRecordListEditPayload -> pendingRecordListEditPayload = tp
            is RecordListParentSceSynthInvokeListRecordListFloorPayload -> pendingRecordListFloorPayload = tp
            is RecordListParentSceSynthInvokeListRecordListJoinPayload -> pendingRecordListJoinPayload = tp
            is RecordListParentSceSynthInvokeListRecordListKeepPayload -> pendingRecordListKeepPayload = tp
            is RecordListParentSceSynthInvokeListRecordListNodePayload -> pendingRecordListNodePayload = tp
            is RecordListParentSceSynthInvokeListRecordListProposePayload -> pendingRecordListProposePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRolePayload -> pendingRecordListRolePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRoomPayload -> pendingRecordListRoomPayload = tp
            is RecordListParentSceSynthInvokeListRecordListSettlePayload -> pendingRecordListSettlePayload = tp
            is RecordListParentSceSynthInvokeListRecordListSharePayload -> pendingRecordListSharePayload = tp
            is RecordListParentSceSynthInvokeListRecordListSupersedePayload -> pendingRecordListSupersedePayload = tp
            is RecordListParentSceSynthInvokeListRecordListVisitPayload -> pendingRecordListVisitPayload = tp
            is RecordListParentSceSynthInvokeListRecordListWritePayload -> pendingRecordListWritePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Ack) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListAckPayload = RecordListParentSceSynthInvokeListRecordListAckPayload(fields.uint64("sender"), fields.int64("before"), fields.uint32("hash"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Announce) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListAnnouncePayload = RecordListParentSceSynthInvokeListRecordListAnnouncePayload(fields.uint32("id"), fields.uint16("field"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Compact) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListCompactPayload = RecordListParentSceSynthInvokeListRecordListCompactPayload(fields.int64("before"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListEditPayload = RecordListParentSceSynthInvokeListRecordListEditPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("now"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Floor) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListFloorPayload = RecordListParentSceSynthInvokeListRecordListFloorPayload(fields.int64("before"), fields.uint32("hash"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListJoinPayload = RecordListParentSceSynthInvokeListRecordListJoinPayload(fields.uint64("node"), fields.boolean("hub"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Keep) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListKeepPayload = RecordListParentSceSynthInvokeListRecordListKeepPayload(fields.uint8("kind"), fields.uint64("element"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Node) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListNodePayload = RecordListParentSceSynthInvokeListRecordListNodePayload(fields.uint64("sender"), fields.uint64("to"), fields.uint8("depth"), fields.uint64("prefix"), fields.uint32("hash"), fields.boolean("reply"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Propose) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListProposePayload = RecordListParentSceSynthInvokeListRecordListProposePayload(fields.int64("before"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Role) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRolePayload = RecordListParentSceSynthInvokeListRecordListRolePayload(fields.uint64("node"), fields.uint8("role"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Room) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRoomPayload = RecordListParentSceSynthInvokeListRecordListRoomPayload(fields.uint32("values"), fields.uint32("history"), fields.uint32("digest"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Settle) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListSettlePayload = RecordListParentSceSynthInvokeListRecordListSettlePayload(fields.int64("before"), fields.uint32("members"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Share) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListSharePayload = RecordListParentSceSynthInvokeListRecordListSharePayload(fields.uint64("to"), fields.uint64("minute"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Supersede) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListSupersedePayload = RecordListParentSceSynthInvokeListRecordListSupersedePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"), fields.uint64("by"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Visit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListVisitPayload = RecordListParentSceSynthInvokeListRecordListVisitPayload(fields.uint64("to"), fields.uint8("depth"), fields.uint64("prefix"), fields.boolean("reply"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Write) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListWritePayload = RecordListParentSceSynthInvokeListRecordListWritePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.ack` — binds the event name and the payload field values in one call.
    fun raiseRecordListAck(sender: ULong, before: Long, hash: UInt) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Ack,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListAckPayload(sender, before, hash),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sender" to sender, "before" to before, "hash" to hash))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.announce` — binds the event name and the payload field values in one call.
    fun raiseRecordListAnnounce(id: UInt, field: UShort) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Announce,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListAnnouncePayload(id, field),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.compact` — binds the event name and the payload field values in one call.
    fun raiseRecordListCompact(before: Long) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Compact,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListCompactPayload(before),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("before" to before))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.edit` — binds the event name and the payload field values in one call.
    fun raiseRecordListEdit(id: UInt, field: UShort, value: Long, text: String, now: Long) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Edit,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListEditPayload(id, field, value, text, now),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "now" to now))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.floor` — binds the event name and the payload field values in one call.
    fun raiseRecordListFloor(before: Long, hash: UInt) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Floor,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListFloorPayload(before, hash),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("before" to before, "hash" to hash))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.join` — binds the event name and the payload field values in one call.
    fun raiseRecordListJoin(node: ULong, hub: Boolean) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Join,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListJoinPayload(node, hub),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "hub" to hub))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.keep` — binds the event name and the payload field values in one call.
    fun raiseRecordListKeep(kind: UByte, element: ULong, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Keep,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListKeepPayload(kind, element, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("kind" to kind, "element" to element, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.node` — binds the event name and the payload field values in one call.
    fun raiseRecordListNode(sender: ULong, to: ULong, depth: UByte, prefix: ULong, hash: UInt, reply: Boolean) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Node,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListNodePayload(sender, to, depth, prefix, hash, reply),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("sender" to sender, "to" to to, "depth" to depth, "prefix" to prefix, "hash" to hash, "reply" to reply))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.propose` — binds the event name and the payload field values in one call.
    fun raiseRecordListPropose(before: Long) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Propose,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListProposePayload(before),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("before" to before))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.role` — binds the event name and the payload field values in one call.
    fun raiseRecordListRole(node: ULong, role: UByte) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Role,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListRolePayload(node, role),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "role" to role))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.room` — binds the event name and the payload field values in one call.
    fun raiseRecordListRoom(values: UInt, history: UInt, digest: UInt) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Room,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListRoomPayload(values, history, digest),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("values" to values, "history" to history, "digest" to digest))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.settle` — binds the event name and the payload field values in one call.
    fun raiseRecordListSettle(before: Long, members: UInt) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Settle,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListSettlePayload(before, members),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("before" to before, "members" to members))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.share` — binds the event name and the payload field values in one call.
    fun raiseRecordListShare(to: ULong, minute: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Share,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListSharePayload(to, minute),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("to" to to, "minute" to minute))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.supersede` — binds the event name and the payload field values in one call.
    fun raiseRecordListSupersede(id: UInt, field: UShort, wallTime: Long, counter: UInt, nodeId: ULong, by: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Supersede,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListSupersedePayload(id, field, wallTime, counter, nodeId, by),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId, "by" to by))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.visit` — binds the event name and the payload field values in one call.
    fun raiseRecordListVisit(to: ULong, depth: UByte, prefix: ULong, reply: Boolean) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Visit,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListVisitPayload(to, depth, prefix, reply),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("to" to to, "depth" to depth, "prefix" to prefix, "reply" to reply))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.write` — binds the event name and the payload field values in one call.
    fun raiseRecordListWrite(id: UInt, field: UShort, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Write,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListWritePayload(id, field, value, text, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }


    override val initialState: RecordListParentSceSynthInvokeListState = RecordListParentSceSynthInvokeListState.Open

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
    override val documentInitialTargets: List<EntryTarget<RecordListParentSceSynthInvokeListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<RecordListParentSceSynthInvokeListState, HistoryId>> =
            listOf(StateTarget(RecordListParentSceSynthInvokeListState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 3, as the microstep reads it.
        val transitionOpenAt3 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 4, as the microstep reads it.
        val transitionOpenAt4 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 5, as the microstep reads it.
        val transitionOpenAt5 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 6, as the microstep reads it.
        val transitionOpenAt6 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 7, as the microstep reads it.
        val transitionOpenAt7 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 8, as the microstep reads it.
        val transitionOpenAt8 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 9, as the microstep reads it.
        val transitionOpenAt9 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 10, as the microstep reads it.
        val transitionOpenAt10 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 11, as the microstep reads it.
        val transitionOpenAt11 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            11,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 12, as the microstep reads it.
        val transitionOpenAt12 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            12,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 13, as the microstep reads it.
        val transitionOpenAt13 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            13,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 14, as the microstep reads it.
        val transitionOpenAt14 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            14,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 15, as the microstep reads it.
        val transitionOpenAt15 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            15,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 16, as the microstep reads it.
        val transitionOpenAt16 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            16,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 17, as the microstep reads it.
        val transitionOpenAt17 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            17,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 18, as the microstep reads it.
        val transitionOpenAt18 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            18,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 19, as the microstep reads it.
        val transitionOpenAt19 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            19,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): RecordListParentSceSynthInvokeListState? = when (stateId) {
        "open" -> RecordListParentSceSynthInvokeListState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: RecordListParentSceSynthInvokeListState): String = when (state) {
        is RecordListParentSceSynthInvokeListState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: RecordListParentSceSynthInvokeListState): Int = when (state) {
        is RecordListParentSceSynthInvokeListState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): RecordListParentSceSynthInvokeListEvent? = when (name) {
        "error.communication" -> RecordListParentSceSynthInvokeListEvent.Error.Communication
        "error.execution" -> RecordListParentSceSynthInvokeListEvent.Error.Execution
        "record_list.ack" -> RecordListParentSceSynthInvokeListEvent.RecordList.Ack
        "record_list.announce" -> RecordListParentSceSynthInvokeListEvent.RecordList.Announce
        "record_list.compact" -> RecordListParentSceSynthInvokeListEvent.RecordList.Compact
        "record_list.edit" -> RecordListParentSceSynthInvokeListEvent.RecordList.Edit
        "record_list.floor" -> RecordListParentSceSynthInvokeListEvent.RecordList.Floor
        "record_list.join" -> RecordListParentSceSynthInvokeListEvent.RecordList.Join
        "record_list.keep" -> RecordListParentSceSynthInvokeListEvent.RecordList.Keep
        "record_list.node" -> RecordListParentSceSynthInvokeListEvent.RecordList.Node
        "record_list.propose" -> RecordListParentSceSynthInvokeListEvent.RecordList.Propose
        "record_list.register" -> RecordListParentSceSynthInvokeListEvent.RecordList.Register
        "record_list.resync" -> RecordListParentSceSynthInvokeListEvent.RecordList.Resync
        "record_list.role" -> RecordListParentSceSynthInvokeListEvent.RecordList.Role
        "record_list.room" -> RecordListParentSceSynthInvokeListEvent.RecordList.Room
        "record_list.settle" -> RecordListParentSceSynthInvokeListEvent.RecordList.Settle
        "record_list.share" -> RecordListParentSceSynthInvokeListEvent.RecordList.Share
        "record_list.supersede" -> RecordListParentSceSynthInvokeListEvent.RecordList.Supersede
        "record_list.tell" -> RecordListParentSceSynthInvokeListEvent.RecordList.Tell
        "record_list.value" -> RecordListParentSceSynthInvokeListEvent.RecordList.Value
        "record_list.visit" -> RecordListParentSceSynthInvokeListEvent.RecordList.Visit
        "record_list.write" -> RecordListParentSceSynthInvokeListEvent.RecordList.Write
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: RecordListParentSceSynthInvokeListEvent): String? = when (event) {
        is RecordListParentSceSynthInvokeListEvent.Error.Communication -> "error.communication"
        is RecordListParentSceSynthInvokeListEvent.Error.Execution -> "error.execution"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Ack -> "record_list.ack"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Announce -> "record_list.announce"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Compact -> "record_list.compact"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Edit -> "record_list.edit"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Floor -> "record_list.floor"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Join -> "record_list.join"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Keep -> "record_list.keep"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Node -> "record_list.node"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Propose -> "record_list.propose"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Register -> "record_list.register"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Resync -> "record_list.resync"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Role -> "record_list.role"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Room -> "record_list.room"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Settle -> "record_list.settle"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Share -> "record_list.share"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Supersede -> "record_list.supersede"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Tell -> "record_list.tell"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Value -> "record_list.value"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Visit -> "record_list.visit"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Write -> "record_list.write"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: RecordListParentSceSynthInvokeListState,
        event: RecordListParentSceSynthInvokeListEvent?
    ): EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>? = when (state) {
        is RecordListParentSceSynthInvokeListState.Open -> when {
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Join -> transitionOpenAt0
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Room -> transitionOpenAt1
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Role -> transitionOpenAt2
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Edit -> transitionOpenAt3
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Write && pendingRecordListWritePayload != null && (vFloor > 0 && pendingRecordListWritePayload!!.wallTime <= vFloor) -> transitionOpenAt4
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Write -> transitionOpenAt5
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Supersede -> transitionOpenAt6
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Announce && vAnnounce -> transitionOpenAt7
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Tell && vAnnounce -> transitionOpenAt8
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Keep -> transitionOpenAt9
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Compact -> transitionOpenAt10
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Settle -> transitionOpenAt11
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Propose -> transitionOpenAt12
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Ack -> transitionOpenAt13
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Floor -> transitionOpenAt14
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Resync -> transitionOpenAt15
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Visit -> transitionOpenAt16
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Node -> transitionOpenAt17
            event is RecordListParentSceSynthInvokeListEvent.RecordList.Share -> transitionOpenAt18
            event is RecordListParentSceSynthInvokeListEvent.Error.Execution -> transitionOpenAt19
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: record_list.scxml:156 :: _machine
    override fun onEntry(state: RecordListParentSceSynthInvokeListState, isDefaultEntry: Boolean) {
        when (state) {
            is RecordListParentSceSynthInvokeListState.Open -> {
                // SCE-MAP: record_list.scxml:309 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: record_list.scxml:156 :: _machine
    override fun onExit(state: RecordListParentSceSynthInvokeListState) {
        when (state) {
            is RecordListParentSceSynthInvokeListState.Open -> {
                // SCE-MAP: record_list.scxml:309 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: record_list.scxml:156 :: _machine
    override fun executeTransitionContent(source: RecordListParentSceSynthInvokeListState, transitionIndex: Int) {
        when (source) {
        is RecordListParentSceSynthInvokeListState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: record_list.scxml:310 :: open :: _transition_0
                if (pendingRecordListJoinPayload == null) {
                    return
                }

            vNode = pendingRecordListJoinPayload!!.node

            vHub = pendingRecordListJoinPayload!!.hub
            }
            1 -> {
                // SCE-MAP: record_list.scxml:317 :: open :: _transition_1
                if (pendingRecordListRoomPayload == null) {
                    return
                }

            vRoomValues = pendingRecordListRoomPayload!!.values

            vRoomHistory = pendingRecordListRoomPayload!!.history

            vRoomDigest = pendingRecordListRoomPayload!!.digest
            }
            2 -> {
                // SCE-MAP: record_list.scxml:328 :: open :: _transition_2
                if (pendingRecordListRolePayload == null) {
                    return
                }

            if (try { vRoster = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RecordRoster__v(vRoster, pendingRecordListRolePayload!!.node, pendingRecordListRolePayload!!.role)), 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='roster'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: record_list.scxml:343 :: open :: _transition_3
                if (pendingRecordListEditPayload == null) {
                    return
                }

            if (try { vTyped = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListEditPayload!!.text, 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='typed'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListEditPayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(element = pendingRecordListEditPayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingRecordListEditPayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vFresh = true


            for (o in vAdds) {


            if (o.element == vEntry.element) {

            vFresh = false
            }
            }

            vForbidden = false


            if ((vRoster).size.toUInt() > 0.toUInt()) {

            vForbidden = true


            for (m in vRoster) {


            if (m.node == vNode && m.role >= 2.toUByte()) {

            vForbidden = false
            }
            }
            }

            vBlocked = !vForbidden && (vRoomValues > 0.toUInt() && vFresh && (vValues).size.toUInt() >= vRoomValues || vRoomHistory > 0.toUInt() && (vTombstones).size.toUInt() >= vRoomHistory || vRoomDigest > 0.toUInt() && (vDigest).size.toUInt() >= vRoomDigest)


            if (vForbidden) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vBlocked) {

            if (try { vFull = com.sce.forge.runtime.SceChecked.add(vFull, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='full'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (!vBlocked && !vForbidden) {

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, pendingRecordListEditPayload!!.now, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (o in vAdds) {


            if (o.element == vEntry.element) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListEditPayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (o.wallTime).toString())

            putParam(sendPayload, "counter", (o.counter).toLong())

            putParam(sendPayload, "nodeId", (o.nodeId).toString())

            putParam(sendPayload, "by", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Supersede, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListEditPayload!!.field).toLong())

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
                eventName = "record_list.supersede",
                target = "#hub",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_1")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListEditPayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingRecordListEditPayload!!.value).toString())

            putParam(sendPayload, "text", pendingRecordListEditPayload!!.text)

            putParam(sendPayload, "wallTime", (vClock.wallTime).toString())

            putParam(sendPayload, "counter", (vClock.counter).toLong())

            putParam(sendPayload, "nodeId", (vNode).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Write, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListEditPayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingRecordListEditPayload!!.value).toString())

            putParam(sendPayload, "text", pendingRecordListEditPayload!!.text)

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
                eventName = "record_list.write",
                target = "#hub",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_3")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            4 -> {
                // SCE-MAP: record_list.scxml:433 :: open :: _transition_4
                if (pendingRecordListWritePayload == null) {
                    return
                }

            vHeld = vHeld.copy(id = pendingRecordListWritePayload!!.id.toULong())

            vHeld = vHeld.copy(field = pendingRecordListWritePayload!!.field.toULong())

            vHeld = vHeld.copy(wallTime = pendingRecordListWritePayload!!.wallTime)

            vHeld = vHeld.copy(counter = pendingRecordListWritePayload!!.counter)

            vHeld = vHeld.copy(nodeId = pendingRecordListWritePayload!!.nodeId)

            vPresent = call_MvValueHolds__vv(vValues, vHeld)


            if (!vPresent) {

            if (try { vLate = com.sce.forge.runtime.SceChecked.add(vLate, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='late'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            5 -> {
                // SCE-MAP: record_list.scxml:454 :: open :: _transition_5
                if (pendingRecordListWritePayload == null) {
                    return
                }

            vAuthorized = true


            if (vHub && (vRoster).size.toUInt() > 0.toUInt()) {

            vAuthorized = false


            for (m in vRoster) {


            if (m.node == pendingRecordListWritePayload!!.nodeId && m.role >= 2.toUByte()) {

            vAuthorized = true
            }
            }
            }


            if (!vAuthorized) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vAuthorized) {

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListWritePayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (pendingRecordListWritePayload!!.nodeId != vNode) {

            vStamp = vStamp.copy(wallTime = pendingRecordListWritePayload!!.wallTime)

            vStamp = vStamp.copy(counter = pendingRecordListWritePayload!!.counter)

            vStamp = vStamp.copy(nodeId = pendingRecordListWritePayload!!.nodeId)

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcReceive__vv(vClock, vStamp, vClock.wallTime, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (pendingRecordListWritePayload!!.nodeId == vNode && (pendingRecordListWritePayload!!.wallTime > vClock.wallTime || pendingRecordListWritePayload!!.wallTime == vClock.wallTime && pendingRecordListWritePayload!!.counter > vClock.counter)) {

            vClock = vClock.copy(wallTime = pendingRecordListWritePayload!!.wallTime)

            vClock = vClock.copy(counter = pendingRecordListWritePayload!!.counter)
            }

            vEntry = vEntry.copy(element = pendingRecordListWritePayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingRecordListWritePayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(wallTime = pendingRecordListWritePayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingRecordListWritePayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingRecordListWritePayload!!.nodeId)

            vIsLive = true


            for (t in vTombstones) {


            if (t.element == vEntry.element && t.wallTime == vEntry.wallTime && t.counter == vEntry.counter && t.nodeId == vEntry.nodeId) {

            vIsLive = false
            }
            }


            if (vIsLive) {

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vHeld = vHeld.copy(id = pendingRecordListWritePayload!!.id.toULong())

            vHeld = vHeld.copy(field = pendingRecordListWritePayload!!.field.toULong())

            vHeld = vHeld.copy(value = pendingRecordListWritePayload!!.value)

            vHeld = vHeld.copy(wallTime = pendingRecordListWritePayload!!.wallTime)

            vHeld = vHeld.copy(counter = pendingRecordListWritePayload!!.counter)

            vHeld = vHeld.copy(nodeId = pendingRecordListWritePayload!!.nodeId)

            vPresent = call_MvValueHolds__vv(vValues, vHeld)


            if (vIsLive && !vPresent) {

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueKeep__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kind", (0).toLong())

            putParam(sendPayload, "element", (vEntry.element).toString())

            putParam(sendPayload, "wallTime", (pendingRecordListWritePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListWritePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListWritePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Keep, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vIsLive && !vPresent && vAnnounce) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListWritePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListWritePayload!!.field).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Announce, EventMetadata.internal(sendData))
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
            putParam(sendPayload, "id", (pendingRecordListWritePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListWritePayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingRecordListWritePayload!!.value).toString())

            putParam(sendPayload, "text", pendingRecordListWritePayload!!.text)

            putParam(sendPayload, "wallTime", (pendingRecordListWritePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListWritePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListWritePayload!!.nodeId).toString())

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
                eventName = "record_list.write",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_6")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            6 -> {
                // SCE-MAP: record_list.scxml:549 :: open :: _transition_6
                if (pendingRecordListSupersedePayload == null) {
                    return
                }

            vAuthorized = true


            if (vHub && (vRoster).size.toUInt() > 0.toUInt()) {

            vAuthorized = false


            for (m in vRoster) {


            if (m.node == pendingRecordListSupersedePayload!!.by && m.role >= 2.toUByte()) {

            vAuthorized = true
            }
            }
            }


            if (!vAuthorized) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vAuthorized) {

            vEntry = vEntry.copy(element = pendingRecordListSupersedePayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingRecordListSupersedePayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(wallTime = pendingRecordListSupersedePayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingRecordListSupersedePayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingRecordListSupersedePayload!!.nodeId)

            vKnown = false


            for (t in vTombstones) {


            if (t.element == vEntry.element && t.wallTime == vEntry.wallTime && t.counter == vEntry.counter && t.nodeId == vEntry.nodeId) {

            vKnown = true
            }
            }

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vTombstones = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vTombstones, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='tombstones'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(id = pendingRecordListSupersedePayload!!.id.toULong())

            vHeld = vHeld.copy(field = pendingRecordListSupersedePayload!!.field.toULong())

            vHeld = vHeld.copy(wallTime = pendingRecordListSupersedePayload!!.wallTime)

            vHeld = vHeld.copy(counter = pendingRecordListSupersedePayload!!.counter)

            vHeld = vHeld.copy(nodeId = pendingRecordListSupersedePayload!!.nodeId)

            vWasLive = call_MvValueHolds__vv(vValues, vHeld)

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueDrop__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (!vKnown) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "kind", (1).toLong())

            putParam(sendPayload, "element", (vEntry.element).toString())

            putParam(sendPayload, "wallTime", (pendingRecordListSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListSupersedePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Keep, EventMetadata.internal(sendData))
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

            putParam(sendPayload, "wallTime", (pendingRecordListSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListSupersedePayload!!.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Keep, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vWasLive && vAnnounce) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListSupersedePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListSupersedePayload!!.field).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Announce, EventMetadata.internal(sendData))
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
            putParam(sendPayload, "id", (pendingRecordListSupersedePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListSupersedePayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (pendingRecordListSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListSupersedePayload!!.nodeId).toString())

            putParam(sendPayload, "by", (pendingRecordListSupersedePayload!!.by).toString())

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
                eventName = "record_list.supersede",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_10")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            7 -> {
                // SCE-MAP: record_list.scxml:634 :: open :: _transition_7
                if (pendingRecordListAnnouncePayload == null) {
                    return
                }

            vEntry = vEntry.copy(element = pendingRecordListAnnouncePayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingRecordListAnnouncePayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vListed = false


            for (named in vMoved) {


            if (named == vEntry.element) {

            vListed = true
            }
            }


            if ((vMoved).size.toUInt() == 0.toUInt()) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Tell, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (!vListed) {

            if (if (vMoved.size < 16) { vMoved = vMoved + (vEntry.element); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='moved'>: the list already holds its capacity of 16"); true }) {
                return
            }
            }
            }
            8 -> {
                // SCE-MAP: record_list.scxml:657 :: open :: _transition_8


            for (named in vMoved) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(com.sce.forge.runtime.SceChecked.div(named, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(com.sce.forge.runtime.SceChecked.rem(named, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vLive = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RegisterValues__v(vValues, vShareId.toULong(), vShareField.toULong())), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='live'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if ((vLive).size.toUInt() == 0.toUInt()) {


            if (run send@{
            // docs/adr/0005, decision 3: the target is a string computed from the
            // machine's fields now, and the send goes only by a route the document
            // declared as `sce:targets`. A value in none of them names no route this
            // send may take — an address nobody answers at, which is reported below as
            // error.communication (W3C SCXML C.1) — and an operation that fails cannot
            // be evaluated, the argument error.
            val _computed: String = "#_parent"
            val _rt: String = if (_computed == "#_parent") _computed else ""
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_12")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "count", (0).toLong())

            putParam(sendPayload, "value", (vEmpty.value).toString())

            putParam(sendPayload, "text", vEmpty.text)

            putParam(sendPayload, "wallTime", (vEmpty.wallTime).toString())

            putParam(sendPayload, "counter", (vEmpty.counter).toLong())

            putParam(sendPayload, "nodeId", (vEmpty.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Register,
"record_list.register",
                sendData,
0L,
                "__send_12",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_12")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_12")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            if (try { vAnnounced = com.sce.forge.runtime.SceChecked.add(vAnnounced, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='announced'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vOrdinal = 0.toUInt()


            for (r in vLive) {


            if (vOrdinal == 0.toUInt()) {


            if (run send@{
            // docs/adr/0005, decision 3: the target is a string computed from the
            // machine's fields now, and the send goes only by a route the document
            // declared as `sce:targets`. A value in none of them names no route this
            // send may take — an address nobody answers at, which is reported below as
            // error.communication (W3C SCXML C.1) — and an operation that fails cannot
            // be evaluated, the argument error.
            val _computed: String = "#_parent"
            val _rt: String = if (_computed == "#_parent") _computed else ""
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_13")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "count", ((vLive).size.toUInt()).toLong())

            putParam(sendPayload, "value", (r.value).toString())

            putParam(sendPayload, "text", r.text)

            putParam(sendPayload, "wallTime", (r.wallTime).toString())

            putParam(sendPayload, "counter", (r.counter).toLong())

            putParam(sendPayload, "nodeId", (r.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Register,
"record_list.register",
                sendData,
0L,
                "__send_13",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_13")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_13")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vOrdinal != 0.toUInt()) {


            if (run send@{
            // docs/adr/0005, decision 3: the target is a string computed from the
            // machine's fields now, and the send goes only by a route the document
            // declared as `sce:targets`. A value in none of them names no route this
            // send may take — an address nobody answers at, which is reported below as
            // error.communication (W3C SCXML C.1) — and an operation that fails cannot
            // be evaluated, the argument error.
            val _computed: String = "#_parent"
            val _rt: String = if (_computed == "#_parent") _computed else ""
            if (com.sce.runtime.SendHelper.isUnreachableTarget(_rt)) {
                // W3C SCXML C.1 (test496): a target that names nothing is not
                // reachable — error.communication, nothing delivered, and the
                // error ends the block as any other would (W3C SCXML 4.9).
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_14")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

            putParam(sendPayload, "ordinal", (vOrdinal).toLong())

            putParam(sendPayload, "value", (r.value).toString())

            putParam(sendPayload, "text", r.text)

            putParam(sendPayload, "wallTime", (r.wallTime).toString())

            putParam(sendPayload, "counter", (r.counter).toLong())

            putParam(sendPayload, "nodeId", (r.nodeId).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Value,
"record_list.value",
                sendData,
0L,
                "__send_14",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_14")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_14")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.SENT -> {}
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }

            if (try { vAnnounced = com.sce.forge.runtime.SceChecked.add(vAnnounced, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='announced'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOrdinal = com.sce.forge.runtime.SceChecked.add(vOrdinal, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='ordinal'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }

            vMoved = emptyList()
            }
            9 -> {
                // SCE-MAP: record_list.scxml:712 :: open :: _transition_9
                if (pendingRecordListKeepPayload == null) {
                    return
                }

            vEntry = vEntry.copy(element = pendingRecordListKeepPayload!!.element)

            vEntry = vEntry.copy(wallTime = pendingRecordListKeepPayload!!.wallTime)

            vEntry = vEntry.copy(counter = pendingRecordListKeepPayload!!.counter)

            vEntry = vEntry.copy(nodeId = pendingRecordListKeepPayload!!.nodeId)

            if (try { vMine = com.sce.forge.runtime.SceChecked.take(call_MerkleChangeHash__v(vEntry, pendingRecordListKeepPayload!!.kind)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='mine'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDigest = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MerkleInsert__v(vDigest, com.sce.forge.runtime.SceChecked.div(pendingRecordListKeepPayload!!.wallTime, 60000), vMine)), 2048); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='digest'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            10 -> {
                // SCE-MAP: record_list.scxml:729 :: open :: _transition_10
                if (pendingRecordListCompactPayload == null) {
                    return
                }

            vStable = emptyList()

            vFrozenFor = -1


            for (t in vTombstones) {


            if (t.wallTime <= pendingRecordListCompactPayload!!.before) {

            if (if (vStable.size < 1024) { vStable = vStable + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='stable'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vMine = com.sce.forge.runtime.SceChecked.take(call_MerkleChangeHash__i(t, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='mine'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDigest = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MerkleInsert__v(vDigest, com.sce.forge.runtime.SceChecked.div(t.wallTime, 60000), vMine)), 2048); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='digest'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }

            if (try { vTombstones = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vTombstones, vStable)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='tombstones'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (pendingRecordListCompactPayload!!.before > vFloor) {

            vFloor = pendingRecordListCompactPayload!!.before
            }
            }
            11 -> {
                // SCE-MAP: record_list.scxml:750 :: open :: _transition_11
                if (pendingRecordListSettlePayload == null) {
                    return
                }


            if (vHub && pendingRecordListSettlePayload!!.before > vFloor) {

            vPending = pendingRecordListSettlePayload!!.before

            vMembers = pendingRecordListSettlePayload!!.members

            vAnswers = emptyList()


            if (vClock.wallTime <= pendingRecordListSettlePayload!!.before) {

            if (try { vClock = vClock.copy(wallTime = com.sce.forge.runtime.SceChecked.add(pendingRecordListSettlePayload!!.before, 1)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock.wallTime'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vClock = vClock.copy(counter = 0.toUInt())
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "before", (pendingRecordListSettlePayload!!.before).toString())

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
                eventName = "record_list.propose",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_15")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            12 -> {
                // SCE-MAP: record_list.scxml:771 :: open :: _transition_12
                if (pendingRecordListProposePayload == null) {
                    return
                }


            if (!vHub && pendingRecordListProposePayload!!.before > vFloor) {


            if (vClock.wallTime <= pendingRecordListProposePayload!!.before) {

            if (try { vClock = vClock.copy(wallTime = com.sce.forge.runtime.SceChecked.add(pendingRecordListProposePayload!!.before, 1)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock.wallTime'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vClock = vClock.copy(counter = 0.toUInt())
            }

            vStable = emptyList()


            for (t in vTombstones) {


            if (t.wallTime <= pendingRecordListProposePayload!!.before) {

            if (if (vStable.size < 1024) { vStable = vStable + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='stable'>: the list already holds its capacity of 1024"); true }) {
                return
            }
            }
            }

            vFrozenFor = pendingRecordListProposePayload!!.before

            if (try { vFrozenHash = com.sce.forge.runtime.SceChecked.take(call_RecordSettled__vv(vAdds, vTombstones, pendingRecordListProposePayload!!.before)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='frozenHash'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "before", (pendingRecordListProposePayload!!.before).toString())

            putParam(sendPayload, "hash", (vFrozenHash).toLong())

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
                eventName = "record_list.ack",
                target = "#hub",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_16")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            13 -> {
                // SCE-MAP: record_list.scxml:801 :: open :: _transition_13
                if (pendingRecordListAckPayload == null) {
                    return
                }


            if (vHub && pendingRecordListAckPayload!!.before == vPending) {

            if (try { vMine = com.sce.forge.runtime.SceChecked.take(call_RecordSettled__vv(vAdds, vTombstones, vPending)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='mine'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vKnown = false


            for (a in vAnswers) {


            if (a.sender == pendingRecordListAckPayload!!.sender && a.hash == pendingRecordListAckPayload!!.hash) {

            vKnown = true
            }
            }


            if (!vKnown) {

            vAnswer = vAnswer.copy(sender = pendingRecordListAckPayload!!.sender)

            vAnswer = vAnswer.copy(hash = pendingRecordListAckPayload!!.hash)

            if (if (vAnswers.size < 512) { vAnswers = vAnswers + (vAnswer); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='answers'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }

            vAgreeing = 0.toUInt()


            for (a in vAnswers) {


            if (a.hash == vMine) {

            if (try { vAgreeing = com.sce.forge.runtime.SceChecked.add(vAgreeing, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='agreeing'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            if (vMembers > 0.toUInt() && vAgreeing >= vMembers) {

            vStable = emptyList()


            for (t in vTombstones) {


            if (t.wallTime <= vPending) {

            if (if (vStable.size < 1024) { vStable = vStable + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='stable'>: the list already holds its capacity of 1024"); true }) {
                return
            }
            }
            }

            vFrozenFor = vPending

            vFrozenHash = vMine


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "before", (vPending).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

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
                eventName = "record_list.floor",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_17")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "before", (vPending).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Floor, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)

            vPending = -1
            }


            if (pendingRecordListAckPayload!!.hash != vMine && vFloor > 0) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "before", (vFloor).toString())

            putParam(sendPayload, "hash", (vAgreed).toLong())

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
                eventName = "record_list.floor",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_19")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            14 -> {
                // SCE-MAP: record_list.scxml:862 :: open :: _transition_14
                if (pendingRecordListFloorPayload == null) {
                    return
                }


            if (pendingRecordListFloorPayload!!.before > vFloor) {


            if (vFrozenFor != pendingRecordListFloorPayload!!.before) {

            vStable = emptyList()


            for (t in vTombstones) {


            if (t.wallTime <= pendingRecordListFloorPayload!!.before) {

            if (if (vStable.size < 1024) { vStable = vStable + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='stable'>: the list already holds its capacity of 1024"); true }) {
                return
            }
            }
            }

            vFrozenFor = pendingRecordListFloorPayload!!.before

            if (try { vFrozenHash = com.sce.forge.runtime.SceChecked.take(call_RecordSettled__vv(vAdds, vTombstones, pendingRecordListFloorPayload!!.before)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='frozenHash'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vFrozenHash == pendingRecordListFloorPayload!!.hash) {


            if (vClock.wallTime <= pendingRecordListFloorPayload!!.before) {

            if (try { vClock = vClock.copy(wallTime = com.sce.forge.runtime.SceChecked.add(pendingRecordListFloorPayload!!.before, 1)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock.wallTime'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vClock = vClock.copy(counter = 0.toUInt())
            }


            for (t in vStable) {

            if (try { vMine = com.sce.forge.runtime.SceChecked.take(call_MerkleChangeHash__i(t, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='mine'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDigest = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MerkleInsert__v(vDigest, com.sce.forge.runtime.SceChecked.div(t.wallTime, 60000), vMine)), 2048); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='digest'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { vTombstones = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vTombstones, vStable)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='tombstones'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vFloor = pendingRecordListFloorPayload!!.before

            vAgreed = pendingRecordListFloorPayload!!.hash

            vStale = false

            vFrozenFor = -1
            }


            if (vFrozenHash != pendingRecordListFloorPayload!!.hash) {

            vStale = true
            }
            }
            }
            15 -> {
                // SCE-MAP: record_list.scxml:898 :: open :: _transition_15


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (0).toLong())

            putParam(sendPayload, "depth", (0).toLong())

            putParam(sendPayload, "prefix", (0).toLong())

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            16 -> {
                // SCE-MAP: record_list.scxml:909 :: open :: _transition_16
                if (pendingRecordListVisitPayload == null) {
                    return
                }

            vMine = 0.toUInt()


            for (n in vDigest) {


            if (n.depth == pendingRecordListVisitPayload!!.depth && n.prefix == pendingRecordListVisitPayload!!.prefix) {

            vMine = n.hash
            }
            }


            if (!vHub) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "to", (0).toLong())

            putParam(sendPayload, "depth", (pendingRecordListVisitPayload!!.depth).toLong())

            putParam(sendPayload, "prefix", (pendingRecordListVisitPayload!!.prefix).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            putParam(sendPayload, "reply", pendingRecordListVisitPayload!!.reply)

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
                eventName = "record_list.node",
                target = "#hub",
                content = "",
                params = sendWireParams,
                sendId = "__send_21",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_21")
            }
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
            putParam(sendPayload, "sender", (vNode).toString())

            putParam(sendPayload, "to", (pendingRecordListVisitPayload!!.to).toString())

            putParam(sendPayload, "depth", (pendingRecordListVisitPayload!!.depth).toLong())

            putParam(sendPayload, "prefix", (pendingRecordListVisitPayload!!.prefix).toString())

            putParam(sendPayload, "hash", (vMine).toLong())

            putParam(sendPayload, "reply", pendingRecordListVisitPayload!!.reply)

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
                eventName = "record_list.node",
                target = "#people",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_22")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            17 -> {
                // SCE-MAP: record_list.scxml:946 :: open :: _transition_17
                if (pendingRecordListNodePayload == null) {
                    return
                }


            if (vHub || pendingRecordListNodePayload!!.to == 0.toULong() || pendingRecordListNodePayload!!.to == vNode) {

            vMine = 0.toUInt()


            for (n in vDigest) {


            if (n.depth == pendingRecordListNodePayload!!.depth && n.prefix == pendingRecordListNodePayload!!.prefix) {

            vMine = n.hash
            }
            }


            if (vMine != pendingRecordListNodePayload!!.hash && pendingRecordListNodePayload!!.depth < 30.toUByte()) {


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingRecordListNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingRecordListNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.mul(pendingRecordListNodePayload!!.prefix, 3.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingRecordListNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingRecordListNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(pendingRecordListNodePayload!!.prefix, 3.toULong()), 1.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingRecordListNodePayload!!.sender).toString())

            try {
                putParam(sendPayload, "depth", (com.sce.forge.runtime.SceChecked.add(pendingRecordListNodePayload!!.depth, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='depth'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "prefix", (com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(pendingRecordListNodePayload!!.prefix, 3.toULong()), 2.toULong())).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='prefix'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "reply", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Visit, EventMetadata.internal(sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vMine != pendingRecordListNodePayload!!.hash && pendingRecordListNodePayload!!.depth == 30.toUByte()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingRecordListNodePayload!!.sender).toString())

            putParam(sendPayload, "minute", (pendingRecordListNodePayload!!.prefix).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Share, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vMine != pendingRecordListNodePayload!!.hash && pendingRecordListNodePayload!!.depth == 30.toUByte() && !pendingRecordListNodePayload!!.reply) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "to", (pendingRecordListNodePayload!!.sender).toString())

            putParam(sendPayload, "depth", (30).toLong())

            putParam(sendPayload, "prefix", (pendingRecordListNodePayload!!.prefix).toString())

            putParam(sendPayload, "reply", true)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Visit, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
            18 -> {
                // SCE-MAP: record_list.scxml:995 :: open :: _transition_18
                if (pendingRecordListSharePayload == null) {
                    return
                }


            for (v in vValues) {


            var ifCondFailed65 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(v.wallTime, 60000) == pendingRecordListSharePayload!!.minute.toLong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='v.wallTime / 60000 === _event.data.minute'>: an integer operation overflowed or failed"); ifCondFailed65 = true; false })) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(v.id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(v.field); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
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
                eventName = "record_list.write",
                target = "#hub",
                content = "",
                params = sendWireParams,
                sendId = "__send_28",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_28")
            }
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
                eventName = "record_list.write",
                target = "#people",
                content = "",
                params = sendWireParams,
                sendId = "__send_29",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_29")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed65) {
                return
            }
            }


            for (t in vTombstones) {


            var ifCondFailed68 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(t.wallTime, 60000) == pendingRecordListSharePayload!!.minute.toLong() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='t.wallTime / 60000 === _event.data.minute'>: an integer operation overflowed or failed"); ifCondFailed68 = true; false })) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(com.sce.forge.runtime.SceChecked.div(t.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(com.sce.forge.runtime.SceChecked.rem(t.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
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
                eventName = "record_list.supersede",
                target = "#hub",
                content = "",
                params = sendWireParams,
                sendId = "__send_30",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_30")
            }
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
                eventName = "record_list.supersede",
                target = "#people",
                content = "",
                params = sendWireParams,
                sendId = "__send_31",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_31")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed68) {
                return
            }
            }
            }
            19 -> {
                // SCE-MAP: record_list.scxml:1052 :: open :: _transition_19

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
