// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/todo_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: todo_peer.scxml:78 :: _machine
@file:OptIn(ExperimentalUnsignedTypes::class)

package com.sce.integration.todo_peer

import com.sce.runtime.*
import com.sce.generated.event_agenda.*
import com.sce.generated.event_conflict.*
import com.sce.generated.event_latest.*
import com.sce.generated.event_member.*
import com.sce.generated.event_occurrence.*
import com.sce.generated.event_roster.*
import com.sce.generated.event_schedule.*
import com.sce.generated.hlc_receive.*
import com.sce.generated.hlc_send.*
import com.sce.generated.hlc_timestamp.*
import com.sce.generated.mv_value.*
import com.sce.generated.mv_value_drop.*
import com.sce.generated.mv_value_holds.*
import com.sce.generated.mv_value_keep.*
import com.sce.generated.orset_entry.*
import com.sce.generated.orset_live.*
import com.sce.generated.orset_union.*
import com.sce.generated.utc_from_local.*


// --- States (W3C SCXML 3.2) ---

sealed interface TodoPeerState : State {
    data object Open : TodoPeerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface TodoPeerEvent : Event {
    sealed interface Error : TodoPeerEvent {
        data object Execution : Error
    }
    sealed interface RecordList : TodoPeerEvent {
        data object Change : RecordList
    }
    sealed interface Todo : TodoPeerEvent {
        data object Agenda : Todo
        data object Edit : Todo
        data object Join : Todo
        data object Pass : Todo
        data object Role : Todo
        data object Room : Todo
        data object Take : Todo
        data object Zone : Todo
        data object ZoneClear : Todo
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `TodoPeerPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// TodoPeerRecordListChangePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.change`. Consumers inject it via the `raiseRecordListChange` seam
// on the machine — they never name this class directly.
data class TodoPeerRecordListChangePayload(val by: ULong, val wallTime: Long, val counter: UInt, val ops: List<TodoPeerOpRecord>) {
    init {
        com.sce.runtime.ValueBounds.listWithin("ops", ops, 1, 64) { _, _ -> }
    }
}

// TodoPeerTodoAgendaPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.agenda`. Consumers inject it via the `raiseTodoAgenda` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoAgendaPayload(val id: UInt, val windowStart: Long, val windowEnd: Long, val skip: UInt)

// TodoPeerTodoEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.edit`. Consumers inject it via the `raiseTodoEdit` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoEditPayload(val id: UInt, val field: UShort, val value: Long, val text: String, val now: Long)

// TodoPeerTodoJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.join`. Consumers inject it via the `raiseTodoJoin` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoJoinPayload(val node: ULong)

// TodoPeerTodoRolePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.role`. Consumers inject it via the `raiseTodoRole` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoRolePayload(val node: ULong, val role: UByte)

// TodoPeerTodoRoomPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.room`. Consumers inject it via the `raiseTodoRoom` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoRoomPayload(val values: UInt)

// TodoPeerTodoZonePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.zone`. Consumers inject it via the `raiseTodoZone` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoZonePayload(val zone: UByte, val start: Long, val offset: Int)


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
        fun fromSaved(value: Any?, what: String): TodoPeerValueRecord = TodoPeerValueRecord(id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint64(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Occurrence` datamodel value. */
data class TodoPeerOccurrenceRecord(val id: ULong, val start: Long, val end: Long, val nth: UInt, val conflict: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "start" to SavedValues.of(start), "end" to SavedValues.of(end), "nth" to SavedValues.of(nth), "conflict" to SavedValues.of(conflict))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerOccurrenceRecord = TodoPeerOccurrenceRecord(id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), start = SavedValues.int64(SavedValues.field(value, what, "start"), "$what.start"), end = SavedValues.int64(SavedValues.field(value, what, "end"), "$what.end"), nth = SavedValues.uint32(SavedValues.field(value, what, "nth"), "$what.nth"), conflict = SavedValues.uint8(SavedValues.field(value, what, "conflict"), "$what.conflict"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Schedule` datamodel value. */
data class TodoPeerScheduleRecord(val zone: Long, val start: Long, val length: Long, val freq: Long, val interval: Long, val days: Long, val count: Long, val until: Long, val bymonth: Long, val mdPos: Long, val mdNeg: Long, val invalid: Long, val op0: Long, val op1: Long, val op2: Long, val op3: Long, val op4: Long, val op5: Long, val op6: Long, val on0: Long, val on1: Long, val on2: Long, val on3: Long, val on4: Long, val on5: Long, val on6: Long, val spPos: Long, val spNeg: Long, val wkst: Long, val wnPos: Long, val wnNeg: Long, val yd0: Long, val yd1: Long, val yd2: Long, val yd3: Long, val yd4: Long, val yd5: Long, val ydn0: Long, val ydn1: Long, val ydn2: Long, val ydn3: Long, val ydn4: Long, val ydn5: Long, val gone: Long, val flag: Long) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("zone" to SavedValues.of(zone), "start" to SavedValues.of(start), "length" to SavedValues.of(length), "freq" to SavedValues.of(freq), "interval" to SavedValues.of(interval), "days" to SavedValues.of(days), "count" to SavedValues.of(count), "until" to SavedValues.of(until), "bymonth" to SavedValues.of(bymonth), "mdPos" to SavedValues.of(mdPos), "mdNeg" to SavedValues.of(mdNeg), "invalid" to SavedValues.of(invalid), "op0" to SavedValues.of(op0), "op1" to SavedValues.of(op1), "op2" to SavedValues.of(op2), "op3" to SavedValues.of(op3), "op4" to SavedValues.of(op4), "op5" to SavedValues.of(op5), "op6" to SavedValues.of(op6), "on0" to SavedValues.of(on0), "on1" to SavedValues.of(on1), "on2" to SavedValues.of(on2), "on3" to SavedValues.of(on3), "on4" to SavedValues.of(on4), "on5" to SavedValues.of(on5), "on6" to SavedValues.of(on6), "spPos" to SavedValues.of(spPos), "spNeg" to SavedValues.of(spNeg), "wkst" to SavedValues.of(wkst), "wnPos" to SavedValues.of(wnPos), "wnNeg" to SavedValues.of(wnNeg), "yd0" to SavedValues.of(yd0), "yd1" to SavedValues.of(yd1), "yd2" to SavedValues.of(yd2), "yd3" to SavedValues.of(yd3), "yd4" to SavedValues.of(yd4), "yd5" to SavedValues.of(yd5), "ydn0" to SavedValues.of(ydn0), "ydn1" to SavedValues.of(ydn1), "ydn2" to SavedValues.of(ydn2), "ydn3" to SavedValues.of(ydn3), "ydn4" to SavedValues.of(ydn4), "ydn5" to SavedValues.of(ydn5), "gone" to SavedValues.of(gone), "flag" to SavedValues.of(flag))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerScheduleRecord = TodoPeerScheduleRecord(zone = SavedValues.int64(SavedValues.field(value, what, "zone"), "$what.zone"), start = SavedValues.int64(SavedValues.field(value, what, "start"), "$what.start"), length = SavedValues.int64(SavedValues.field(value, what, "length"), "$what.length"), freq = SavedValues.int64(SavedValues.field(value, what, "freq"), "$what.freq"), interval = SavedValues.int64(SavedValues.field(value, what, "interval"), "$what.interval"), days = SavedValues.int64(SavedValues.field(value, what, "days"), "$what.days"), count = SavedValues.int64(SavedValues.field(value, what, "count"), "$what.count"), until = SavedValues.int64(SavedValues.field(value, what, "until"), "$what.until"), bymonth = SavedValues.int64(SavedValues.field(value, what, "bymonth"), "$what.bymonth"), mdPos = SavedValues.int64(SavedValues.field(value, what, "mdPos"), "$what.mdPos"), mdNeg = SavedValues.int64(SavedValues.field(value, what, "mdNeg"), "$what.mdNeg"), invalid = SavedValues.int64(SavedValues.field(value, what, "invalid"), "$what.invalid"), op0 = SavedValues.int64(SavedValues.field(value, what, "op0"), "$what.op0"), op1 = SavedValues.int64(SavedValues.field(value, what, "op1"), "$what.op1"), op2 = SavedValues.int64(SavedValues.field(value, what, "op2"), "$what.op2"), op3 = SavedValues.int64(SavedValues.field(value, what, "op3"), "$what.op3"), op4 = SavedValues.int64(SavedValues.field(value, what, "op4"), "$what.op4"), op5 = SavedValues.int64(SavedValues.field(value, what, "op5"), "$what.op5"), op6 = SavedValues.int64(SavedValues.field(value, what, "op6"), "$what.op6"), on0 = SavedValues.int64(SavedValues.field(value, what, "on0"), "$what.on0"), on1 = SavedValues.int64(SavedValues.field(value, what, "on1"), "$what.on1"), on2 = SavedValues.int64(SavedValues.field(value, what, "on2"), "$what.on2"), on3 = SavedValues.int64(SavedValues.field(value, what, "on3"), "$what.on3"), on4 = SavedValues.int64(SavedValues.field(value, what, "on4"), "$what.on4"), on5 = SavedValues.int64(SavedValues.field(value, what, "on5"), "$what.on5"), on6 = SavedValues.int64(SavedValues.field(value, what, "on6"), "$what.on6"), spPos = SavedValues.int64(SavedValues.field(value, what, "spPos"), "$what.spPos"), spNeg = SavedValues.int64(SavedValues.field(value, what, "spNeg"), "$what.spNeg"), wkst = SavedValues.int64(SavedValues.field(value, what, "wkst"), "$what.wkst"), wnPos = SavedValues.int64(SavedValues.field(value, what, "wnPos"), "$what.wnPos"), wnNeg = SavedValues.int64(SavedValues.field(value, what, "wnNeg"), "$what.wnNeg"), yd0 = SavedValues.int64(SavedValues.field(value, what, "yd0"), "$what.yd0"), yd1 = SavedValues.int64(SavedValues.field(value, what, "yd1"), "$what.yd1"), yd2 = SavedValues.int64(SavedValues.field(value, what, "yd2"), "$what.yd2"), yd3 = SavedValues.int64(SavedValues.field(value, what, "yd3"), "$what.yd3"), yd4 = SavedValues.int64(SavedValues.field(value, what, "yd4"), "$what.yd4"), yd5 = SavedValues.int64(SavedValues.field(value, what, "yd5"), "$what.yd5"), ydn0 = SavedValues.int64(SavedValues.field(value, what, "ydn0"), "$what.ydn0"), ydn1 = SavedValues.int64(SavedValues.field(value, what, "ydn1"), "$what.ydn1"), ydn2 = SavedValues.int64(SavedValues.field(value, what, "ydn2"), "$what.ydn2"), ydn3 = SavedValues.int64(SavedValues.field(value, what, "ydn3"), "$what.ydn3"), ydn4 = SavedValues.int64(SavedValues.field(value, what, "ydn4"), "$what.ydn4"), ydn5 = SavedValues.int64(SavedValues.field(value, what, "ydn5"), "$what.ydn5"), gone = SavedValues.int64(SavedValues.field(value, what, "gone"), "$what.gone"), flag = SavedValues.int64(SavedValues.field(value, what, "flag"), "$what.flag"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Member` datamodel value. */
data class TodoPeerMemberRecord(val node: ULong, val role: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("node" to SavedValues.of(node), "role" to SavedValues.of(role))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerMemberRecord = TodoPeerMemberRecord(node = SavedValues.uint64(SavedValues.field(value, what, "node"), "$what.node"), role = SavedValues.uint8(SavedValues.field(value, what, "role"), "$what.role"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Op` datamodel value. */
data class TodoPeerOpRecord(val kind: UByte, val id: UInt, val field: UShort, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("kind" to SavedValues.of(kind), "id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerOpRecord = TodoPeerOpRecord(kind = SavedValues.uint8(SavedValues.field(value, what, "kind"), "$what.kind"), id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object TodoPeerLimits {
    /** The bound `sce:limit="merge_entries"` names, as this build resolved it. */
    const val MERGE_ENTRIES: UInt = 1024u
    /** The bound `sce:limit="merge_text"` names, as this build resolved it. */
    const val MERGE_TEXT: UInt = 96u
    /** The bound `sce:limit="merge_values"` names, as this build resolved it. */
    const val MERGE_VALUES: UInt = 512u
    /** The bound `sce:limit="shared_parts"` names, as this build resolved it. */
    const val SHARED_PARTS: UInt = 64u
    /** The bound `sce:limit="todo_roster"` names, as this build resolved it. */
    const val TODO_ROSTER: UInt = 64u
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_EventAgenda__vv(p0: TodoPeerScheduleRecord, p1: List<TodoPeerValueRecord>, p2: ULong, p3: Long, p4: Long, p5: UInt): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerOccurrenceRecord>> =
    when (val r = eventAgenda(EventSchedulePayload(zone = p0.zone, start = p0.start, length = p0.length, freq = p0.freq, interval = p0.interval, days = p0.days, count = p0.count, until = p0.until, bymonth = p0.bymonth, mdPos = p0.mdPos, mdNeg = p0.mdNeg, invalid = p0.invalid, op0 = p0.op0, op1 = p0.op1, op2 = p0.op2, op3 = p0.op3, op4 = p0.op4, op5 = p0.op5, op6 = p0.op6, on0 = p0.on0, on1 = p0.on1, on2 = p0.on2, on3 = p0.on3, on4 = p0.on4, on5 = p0.on5, on6 = p0.on6, spPos = p0.spPos, spNeg = p0.spNeg, wkst = p0.wkst, wnPos = p0.wnPos, wnNeg = p0.wnNeg, yd0 = p0.yd0, yd1 = p0.yd1, yd2 = p0.yd2, yd3 = p0.yd3, yd4 = p0.yd4, yd5 = p0.yd5, ydn0 = p0.ydn0, ydn1 = p0.ydn1, ydn2 = p0.ydn2, ydn3 = p0.ydn3, ydn4 = p0.ydn4, ydn5 = p0.ydn5, gone = p0.gone, flag = p0.flag), p1.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p2, p3, p4, p5)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerOccurrenceRecord(id = it.id, start = it.start, end = it.end, nth = it.nth, conflict = it.conflict) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_EventConflict__v(p0: List<TodoPeerValueRecord>, p1: ULong): Boolean {
    val r = eventConflict(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1)
    return r
}
private fun call_EventLatest__v(p0: List<TodoPeerValueRecord>, p1: ULong, p2: ULong, p3: Long): Long {
    val r = eventLatest(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1, p2, p3)
    return r
}
private fun call_EventRoster__v(p0: List<TodoPeerMemberRecord>, p1: ULong, p2: UByte): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerMemberRecord>> =
    when (val r = eventRoster(p0.map { EventMemberPayload(node = it.node, role = it.role) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerMemberRecord(node = it.node, role = it.role) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
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
private fun call_OrsetUnion__vv(p0: List<TodoPeerEntryRecord>, p1: List<TodoPeerEntryRecord>): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerEntryRecord>> =
    when (val r = orsetUnion(p0.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1.map { OrsetEntryPayload(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerEntryRecord(element = it.element, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_UtcFromLocal__vv(p0: List<Long>, p1: List<Int>, p2: Long): com.sce.forge.runtime.AlgorithmResult<Long> =
    when (val r = utcFromLocal(p0.toLongArray(), p1.toIntArray(), p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class TodoPeerStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<TodoPeerState, TodoPeerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `node` datamodel variable, published (`sce:direction="out"`). */
    private var vNode: ULong = 0.toULong()

    /** What the published `node` holds. */
    val node: ULong get() = vNode
    /** W3C SCXML 5.2: the `clock` datamodel variable, published (`sce:direction="out"`). */
    private var vClock: TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `stamp` datamodel variable, the machine's own. */
    private var vStamp: TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `dot` datamodel variable, the machine's own. */
    private var vDot: TodoPeerHlcRecord = TodoPeerHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `adds` datamodel variable, published (`sce:direction="out"`). */
    private var vAdds: List<TodoPeerEntryRecord> = emptyList()

    /** What the published `adds` holds. */
    val adds: List<TodoPeerEntryRecord> get() = vAdds
    /** W3C SCXML 5.2: the `one` datamodel variable, the machine's own. */
    private var vOne: List<TodoPeerEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `entry` datamodel variable, the machine's own. */
    private var vEntry: TodoPeerEntryRecord = TodoPeerEntryRecord(element = 0.toULong(), wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `values` datamodel variable, published (`sce:direction="out"`). */
    private var vValues: List<TodoPeerValueRecord> = emptyList()

    /** What the published `values` holds. */
    val values: List<TodoPeerValueRecord> get() = vValues
    /** W3C SCXML 5.2: the `typed` datamodel variable, the machine's own. */
    private var vTyped: String = ""
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: TodoPeerValueRecord = TodoPeerValueRecord(id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `wasLive` datamodel variable, the machine's own. */
    private var vWasLive: Boolean = false
    /** W3C SCXML 5.2: the `present` datamodel variable, the machine's own. */
    private var vPresent: Boolean = false
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals
    /** W3C SCXML 5.2: the `agenda` datamodel variable, published (`sce:direction="out"`). */
    private var vAgenda: List<TodoPeerOccurrenceRecord> = emptyList()

    /** What the published `agenda` holds. */
    val agenda: List<TodoPeerOccurrenceRecord> get() = vAgenda
    /** W3C SCXML 5.2: the `local` datamodel variable, the machine's own. */
    private var vLocal: List<TodoPeerOccurrenceRecord> = emptyList()
    /** W3C SCXML 5.2: the `occ` datamodel variable, the machine's own. */
    private var vOcc: TodoPeerOccurrenceRecord = TodoPeerOccurrenceRecord(id = 0.toULong(), start = 0, end = 0, nth = 0.toUInt(), conflict = 0.toUByte())
    /** W3C SCXML 5.2: the `agendaNext` datamodel variable, published (`sce:direction="out"`). */
    private var vAgendaNext: UInt = 0.toUInt()

    /** What the published `agendaNext` holds. */
    val agendaNext: UInt get() = vAgendaNext
    /** W3C SCXML 5.2: the `zoneStarts1` datamodel variable, the machine's own. */
    private var vZoneStarts1: List<Long> = emptyList()
    /** W3C SCXML 5.2: the `zoneOffsets1` datamodel variable, the machine's own. */
    private var vZoneOffsets1: List<Int> = emptyList()
    /** W3C SCXML 5.2: the `zoneStarts2` datamodel variable, the machine's own. */
    private var vZoneStarts2: List<Long> = emptyList()
    /** W3C SCXML 5.2: the `zoneOffsets2` datamodel variable, the machine's own. */
    private var vZoneOffsets2: List<Int> = emptyList()
    /** W3C SCXML 5.2: the `zoneStarts3` datamodel variable, the machine's own. */
    private var vZoneStarts3: List<Long> = emptyList()
    /** W3C SCXML 5.2: the `zoneOffsets3` datamodel variable, the machine's own. */
    private var vZoneOffsets3: List<Int> = emptyList()
    /** W3C SCXML 5.2: the `zoneStarts4` datamodel variable, the machine's own. */
    private var vZoneStarts4: List<Long> = emptyList()
    /** W3C SCXML 5.2: the `zoneOffsets4` datamodel variable, the machine's own. */
    private var vZoneOffsets4: List<Int> = emptyList()
    /** W3C SCXML 5.2: the `zoneStarts` datamodel variable, the machine's own. */
    private var vZoneStarts: List<Long> = emptyList()
    /** W3C SCXML 5.2: the `zoneOffsets` datamodel variable, the machine's own. */
    private var vZoneOffsets: List<Int> = emptyList()
    /** W3C SCXML 5.2: the `zoneRows` datamodel variable, the machine's own. */
    private var vZoneRows: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `split` datamodel variable, the machine's own. */
    private var vSplit: Boolean = false
    /** W3C SCXML 5.2: the `sched` datamodel variable, the machine's own. */
    private var vSched: TodoPeerScheduleRecord = TodoPeerScheduleRecord(zone = 0, start = -1, length = 0, freq = 0, interval = 1, days = 0, count = 0, until = 0, bymonth = 0, mdPos = 0, mdNeg = 0, invalid = 0, op0 = 0, op1 = 0, op2 = 0, op3 = 0, op4 = 0, op5 = 0, op6 = 0, on0 = 0, on1 = 0, on2 = 0, on3 = 0, on4 = 0, on5 = 0, on6 = 0, spPos = 0, spNeg = 0, wkst = 1, wnPos = 0, wnNeg = 0, yd0 = 0, yd1 = 0, yd2 = 0, yd3 = 0, yd4 = 0, yd5 = 0, ydn0 = 0, ydn1 = 0, ydn2 = 0, ydn3 = 0, ydn4 = 0, ydn5 = 0, gone = 0, flag = 0)
    /** W3C SCXML 5.2: the `rawCount` datamodel variable, the machine's own. */
    private var vRawCount: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `rawLast` datamodel variable, the machine's own. */
    private var vRawLast: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `utcSec` datamodel variable, the machine's own. */
    private var vUtcSec: Long = 0
    /** W3C SCXML 5.2: the `roomValues` datamodel variable, the machine's own. */
    private var vRoomValues: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `full` datamodel variable, published (`sce:direction="out"`). */
    private var vFull: UInt = 0.toUInt()

    /** What the published `full` holds. */
    val full: UInt get() = vFull
    /** W3C SCXML 5.2: the `gone` datamodel variable, the machine's own. */
    private var vGone: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `roster` datamodel variable, the machine's own. */
    private var vRoster: List<TodoPeerMemberRecord> = emptyList()
    /** W3C SCXML 5.2: the `denied` datamodel variable, published (`sce:direction="out"`). */
    private var vDenied: UInt = 0.toUInt()

    /** What the published `denied` holds. */
    val denied: UInt get() = vDenied
    /** W3C SCXML 5.2: the `forbidden` datamodel variable, the machine's own. */
    private var vForbidden: Boolean = false
    /** W3C SCXML 5.2: the `authorized` datamodel variable, the machine's own. */
    private var vAuthorized: Boolean = true
    /** W3C SCXML 5.2: the `taking` datamodel variable, the machine's own. */
    private var vTaking: List<TodoPeerOpRecord> = emptyList()
    /** W3C SCXML 5.2: the `took` datamodel variable, the machine's own. */
    private var vTook: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `op` datamodel variable, the machine's own. */
    private var vOp: TodoPeerOpRecord = TodoPeerOpRecord(kind = 0.toUByte(), id = 0.toUInt(), field = 0.toUShort(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var node: ULong? = null
        var typed: String? = null
        var wasLive: Boolean? = null
        var present: Boolean? = null
        var refusals: UInt? = null
        var agendaNext: UInt? = null
        var zoneRows: UInt? = null
        var split: Boolean? = null
        var rawCount: UInt? = null
        var rawLast: UInt? = null
        var utcSec: Long? = null
        var roomValues: UInt? = null
        var full: UInt? = null
        var gone: UInt? = null
        var denied: UInt? = null
        var forbidden: Boolean? = null
        var authorized: Boolean? = null
        var took: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.typed?.let { vTyped = it }
        params.wasLive?.let { vWasLive = it }
        params.present?.let { vPresent = it }
        params.refusals?.let { vRefusals = it }
        params.agendaNext?.let { vAgendaNext = it }
        params.zoneRows?.let { vZoneRows = it }
        params.split?.let { vSplit = it }
        params.rawCount?.let { vRawCount = it }
        params.rawLast?.let { vRawLast = it }
        params.utcSec?.let { vUtcSec = it }
        params.roomValues?.let { vRoomValues = it }
        params.full?.let { vFull = it }
        params.gone?.let { vGone = it }
        params.denied?.let { vDenied = it }
        params.forbidden?.let { vForbidden = it }
        params.authorized?.let { vAuthorized = it }
        params.took?.let { vTook = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val clock: TodoPeerHlcRecord,
        val adds: List<TodoPeerEntryRecord>,
        val values: List<TodoPeerValueRecord>,
        val refusals: UInt,
        val agenda: List<TodoPeerOccurrenceRecord>,
        val agendaNext: UInt,
        val full: UInt,
        val denied: UInt,
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
        clock = vClock,
        adds = vAdds,
        values = vValues,
        refusals = vRefusals,
        agenda = vAgenda,
        agendaNext = vAgendaNext,
        full = vFull,
        denied = vDenied,
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
        first = "379f1ea604eeed8d97ddd3e711e4ee2dc7d9dbe9fe8ad6209b525ef0cdffa8d8",
        revised = "058fbac9afe11b05ed04e463167e6e9c6f314e0e9e008a3e88e4216a917ed9a1",
        format2First = "379f1ea604eeed8d97ddd3e711e4ee2dc7d9dbe9fe8ad6209b525ef0cdffa8d8",
        format2Revised = "058fbac9afe11b05ed04e463167e6e9c6f314e0e9e008a3e88e4216a917ed9a1",
        records = listOf(
            RecordLineage(
                event = "calendar.member",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("roster", true),
                ),
            ),
            RecordLineage(
                event = "calendar.occurrence",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("agenda", true),
                    RecordHolder("local", true),
                    RecordHolder("occ", false),
                ),
            ),
            RecordLineage(
                event = "calendar.schedule",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("sched", false),
                ),
            ),
            RecordLineage(
                event = "merge.hlc_timestamp",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("clock", false),
                    RecordHolder("stamp", false),
                    RecordHolder("dot", false),
                ),
            ),
            RecordLineage(
                event = "merge.mv_value",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("values", true),
                    RecordHolder("held", false),
                ),
            ),
            RecordLineage(
                event = "merge.orset_entry",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("adds", true),
                    RecordHolder("one", true),
                    RecordHolder("entry", false),
                ),
            ),
            RecordLineage(
                event = "record_list.op",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("taking", true),
                    RecordHolder("op", false),
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
        variables += "node" to SavedValues.of(vNode)
        variables += "clock" to vClock.toSaved()
        variables += "stamp" to vStamp.toSaved()
        variables += "dot" to vDot.toSaved()
        variables += "adds" to SavedValues.list(vAdds) { it.toSaved() }
        variables += "one" to SavedValues.list(vOne) { it.toSaved() }
        variables += "entry" to vEntry.toSaved()
        variables += "values" to SavedValues.list(vValues) { it.toSaved() }
        variables += "typed" to SavedValues.of(vTyped)
        variables += "held" to vHeld.toSaved()
        variables += "wasLive" to SavedValues.of(vWasLive)
        variables += "present" to SavedValues.of(vPresent)
        variables += "refusals" to SavedValues.of(vRefusals)
        variables += "agenda" to SavedValues.list(vAgenda) { it.toSaved() }
        variables += "local" to SavedValues.list(vLocal) { it.toSaved() }
        variables += "occ" to vOcc.toSaved()
        variables += "agendaNext" to SavedValues.of(vAgendaNext)
        variables += "zoneStarts1" to SavedValues.list(vZoneStarts1) { SavedValues.of(it) }
        variables += "zoneOffsets1" to SavedValues.list(vZoneOffsets1) { SavedValues.of(it) }
        variables += "zoneStarts2" to SavedValues.list(vZoneStarts2) { SavedValues.of(it) }
        variables += "zoneOffsets2" to SavedValues.list(vZoneOffsets2) { SavedValues.of(it) }
        variables += "zoneStarts3" to SavedValues.list(vZoneStarts3) { SavedValues.of(it) }
        variables += "zoneOffsets3" to SavedValues.list(vZoneOffsets3) { SavedValues.of(it) }
        variables += "zoneStarts4" to SavedValues.list(vZoneStarts4) { SavedValues.of(it) }
        variables += "zoneOffsets4" to SavedValues.list(vZoneOffsets4) { SavedValues.of(it) }
        variables += "zoneStarts" to SavedValues.list(vZoneStarts) { SavedValues.of(it) }
        variables += "zoneOffsets" to SavedValues.list(vZoneOffsets) { SavedValues.of(it) }
        variables += "zoneRows" to SavedValues.of(vZoneRows)
        variables += "split" to SavedValues.of(vSplit)
        variables += "sched" to vSched.toSaved()
        variables += "rawCount" to SavedValues.of(vRawCount)
        variables += "rawLast" to SavedValues.of(vRawLast)
        variables += "utcSec" to SavedValues.of(vUtcSec)
        variables += "roomValues" to SavedValues.of(vRoomValues)
        variables += "full" to SavedValues.of(vFull)
        variables += "gone" to SavedValues.of(vGone)
        variables += "roster" to SavedValues.list(vRoster) { it.toSaved() }
        variables += "denied" to SavedValues.of(vDenied)
        variables += "forbidden" to SavedValues.of(vForbidden)
        variables += "authorized" to SavedValues.of(vAuthorized)
        variables += "taking" to SavedValues.list(vTaking) { it.toSaved() }
        variables += "took" to SavedValues.of(vTook)
        variables += "op" to vOp.toSaved()
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
        val saved1 = SavedValues.uint64(savedGrown.variable("node"), "node")
        val saved2 = TodoPeerHlcRecord.fromSaved(savedGrown.variable("clock"), "clock")
        val saved3 = TodoPeerHlcRecord.fromSaved(savedGrown.variable("stamp"), "stamp")
        val saved4 = TodoPeerHlcRecord.fromSaved(savedGrown.variable("dot"), "dot")
        val saved5 = SavedValues.list(savedGrown.variable("adds"), "adds", 1024) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved6 = SavedValues.list(savedGrown.variable("one"), "one", 1024) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        val saved7 = TodoPeerEntryRecord.fromSaved(savedGrown.variable("entry"), "entry")
        val saved8 = SavedValues.list(savedGrown.variable("values"), "values", 512) { e, w -> TodoPeerValueRecord.fromSaved(e, w) }
        val saved9 = SavedValues.string(savedGrown.variable("typed"), "typed", 32)
        val saved10 = TodoPeerValueRecord.fromSaved(savedGrown.variable("held"), "held")
        val saved11 = SavedValues.bool(savedGrown.variable("wasLive"), "wasLive")
        val saved12 = SavedValues.bool(savedGrown.variable("present"), "present")
        val saved13 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        val saved14 = SavedValues.list(savedGrown.variable("agenda"), "agenda", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        val saved15 = SavedValues.list(savedGrown.variable("local"), "local", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        val saved16 = TodoPeerOccurrenceRecord.fromSaved(savedGrown.variable("occ"), "occ")
        val saved17 = SavedValues.uint32(savedGrown.variable("agendaNext"), "agendaNext")
        val saved18 = SavedValues.list(savedGrown.variable("zoneStarts1"), "zoneStarts1", 64) { e, w -> SavedValues.int64(e, w) }
        val saved19 = SavedValues.list(savedGrown.variable("zoneOffsets1"), "zoneOffsets1", 64) { e, w -> SavedValues.int32(e, w) }
        val saved20 = SavedValues.list(savedGrown.variable("zoneStarts2"), "zoneStarts2", 64) { e, w -> SavedValues.int64(e, w) }
        val saved21 = SavedValues.list(savedGrown.variable("zoneOffsets2"), "zoneOffsets2", 64) { e, w -> SavedValues.int32(e, w) }
        val saved22 = SavedValues.list(savedGrown.variable("zoneStarts3"), "zoneStarts3", 64) { e, w -> SavedValues.int64(e, w) }
        val saved23 = SavedValues.list(savedGrown.variable("zoneOffsets3"), "zoneOffsets3", 64) { e, w -> SavedValues.int32(e, w) }
        val saved24 = SavedValues.list(savedGrown.variable("zoneStarts4"), "zoneStarts4", 64) { e, w -> SavedValues.int64(e, w) }
        val saved25 = SavedValues.list(savedGrown.variable("zoneOffsets4"), "zoneOffsets4", 64) { e, w -> SavedValues.int32(e, w) }
        val saved26 = SavedValues.list(savedGrown.variable("zoneStarts"), "zoneStarts", 64) { e, w -> SavedValues.int64(e, w) }
        val saved27 = SavedValues.list(savedGrown.variable("zoneOffsets"), "zoneOffsets", 64) { e, w -> SavedValues.int32(e, w) }
        val saved28 = SavedValues.uint32(savedGrown.variable("zoneRows"), "zoneRows")
        val saved29 = SavedValues.bool(savedGrown.variable("split"), "split")
        val saved30 = TodoPeerScheduleRecord.fromSaved(savedGrown.variable("sched"), "sched")
        val saved31 = SavedValues.uint32(savedGrown.variable("rawCount"), "rawCount")
        val saved32 = SavedValues.uint32(savedGrown.variable("rawLast"), "rawLast")
        val saved33 = SavedValues.int64(savedGrown.variable("utcSec"), "utcSec")
        val saved34 = SavedValues.uint32(savedGrown.variable("roomValues"), "roomValues")
        val saved35 = SavedValues.uint32(savedGrown.variable("full"), "full")
        val saved36 = SavedValues.uint32(savedGrown.variable("gone"), "gone")
        val saved37 = SavedValues.list(savedGrown.variable("roster"), "roster", 64) { e, w -> TodoPeerMemberRecord.fromSaved(e, w) }
        val saved38 = SavedValues.uint32(savedGrown.variable("denied"), "denied")
        val saved39 = SavedValues.bool(savedGrown.variable("forbidden"), "forbidden")
        val saved40 = SavedValues.bool(savedGrown.variable("authorized"), "authorized")
        val saved41 = SavedValues.list(savedGrown.variable("taking"), "taking", 64) { e, w -> TodoPeerOpRecord.fromSaved(e, w) }
        val saved42 = SavedValues.uint32(savedGrown.variable("took"), "took")
        val saved43 = TodoPeerOpRecord.fromSaved(savedGrown.variable("op"), "op")
        return {
            vNode = saved1
            vClock = saved2
            vStamp = saved3
            vDot = saved4
            vAdds = saved5
            vOne = saved6
            vEntry = saved7
            vValues = saved8
            vTyped = saved9
            vHeld = saved10
            vWasLive = saved11
            vPresent = saved12
            vRefusals = saved13
            vAgenda = saved14
            vLocal = saved15
            vOcc = saved16
            vAgendaNext = saved17
            vZoneStarts1 = saved18
            vZoneOffsets1 = saved19
            vZoneStarts2 = saved20
            vZoneOffsets2 = saved21
            vZoneStarts3 = saved22
            vZoneOffsets3 = saved23
            vZoneStarts4 = saved24
            vZoneOffsets4 = saved25
            vZoneStarts = saved26
            vZoneOffsets = saved27
            vZoneRows = saved28
            vSplit = saved29
            vSched = saved30
            vRawCount = saved31
            vRawLast = saved32
            vUtcSec = saved33
            vRoomValues = saved34
            vFull = saved35
            vGone = saved36
            vRoster = saved37
            vDenied = saved38
            vForbidden = saved39
            vAuthorized = saved40
            vTaking = saved41
            vTook = saved42
            vOp = saved43
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
        SavedValues.uint64(savedGrown.variable("node"), "node")
        TodoPeerHlcRecord.fromSaved(savedGrown.variable("clock"), "clock")
        TodoPeerHlcRecord.fromSaved(savedGrown.variable("stamp"), "stamp")
        TodoPeerHlcRecord.fromSaved(savedGrown.variable("dot"), "dot")
        SavedValues.list(savedGrown.variable("adds"), "adds", 1024) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("one"), "one", 1024) { e, w -> TodoPeerEntryRecord.fromSaved(e, w) }
        TodoPeerEntryRecord.fromSaved(savedGrown.variable("entry"), "entry")
        SavedValues.list(savedGrown.variable("values"), "values", 512) { e, w -> TodoPeerValueRecord.fromSaved(e, w) }
        SavedValues.string(savedGrown.variable("typed"), "typed", 32)
        TodoPeerValueRecord.fromSaved(savedGrown.variable("held"), "held")
        SavedValues.bool(savedGrown.variable("wasLive"), "wasLive")
        SavedValues.bool(savedGrown.variable("present"), "present")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        SavedValues.list(savedGrown.variable("agenda"), "agenda", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("local"), "local", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        TodoPeerOccurrenceRecord.fromSaved(savedGrown.variable("occ"), "occ")
        SavedValues.uint32(savedGrown.variable("agendaNext"), "agendaNext")
        SavedValues.list(savedGrown.variable("zoneStarts1"), "zoneStarts1", 64) { e, w -> SavedValues.int64(e, w) }
        SavedValues.list(savedGrown.variable("zoneOffsets1"), "zoneOffsets1", 64) { e, w -> SavedValues.int32(e, w) }
        SavedValues.list(savedGrown.variable("zoneStarts2"), "zoneStarts2", 64) { e, w -> SavedValues.int64(e, w) }
        SavedValues.list(savedGrown.variable("zoneOffsets2"), "zoneOffsets2", 64) { e, w -> SavedValues.int32(e, w) }
        SavedValues.list(savedGrown.variable("zoneStarts3"), "zoneStarts3", 64) { e, w -> SavedValues.int64(e, w) }
        SavedValues.list(savedGrown.variable("zoneOffsets3"), "zoneOffsets3", 64) { e, w -> SavedValues.int32(e, w) }
        SavedValues.list(savedGrown.variable("zoneStarts4"), "zoneStarts4", 64) { e, w -> SavedValues.int64(e, w) }
        SavedValues.list(savedGrown.variable("zoneOffsets4"), "zoneOffsets4", 64) { e, w -> SavedValues.int32(e, w) }
        SavedValues.list(savedGrown.variable("zoneStarts"), "zoneStarts", 64) { e, w -> SavedValues.int64(e, w) }
        SavedValues.list(savedGrown.variable("zoneOffsets"), "zoneOffsets", 64) { e, w -> SavedValues.int32(e, w) }
        SavedValues.uint32(savedGrown.variable("zoneRows"), "zoneRows")
        SavedValues.bool(savedGrown.variable("split"), "split")
        TodoPeerScheduleRecord.fromSaved(savedGrown.variable("sched"), "sched")
        SavedValues.uint32(savedGrown.variable("rawCount"), "rawCount")
        SavedValues.uint32(savedGrown.variable("rawLast"), "rawLast")
        SavedValues.int64(savedGrown.variable("utcSec"), "utcSec")
        SavedValues.uint32(savedGrown.variable("roomValues"), "roomValues")
        SavedValues.uint32(savedGrown.variable("full"), "full")
        SavedValues.uint32(savedGrown.variable("gone"), "gone")
        SavedValues.list(savedGrown.variable("roster"), "roster", 64) { e, w -> TodoPeerMemberRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("denied"), "denied")
        SavedValues.bool(savedGrown.variable("forbidden"), "forbidden")
        SavedValues.bool(savedGrown.variable("authorized"), "authorized")
        SavedValues.list(savedGrown.variable("taking"), "taking", 64) { e, w -> TodoPeerOpRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("took"), "took")
        TodoPeerOpRecord.fromSaved(savedGrown.variable("op"), "op")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingRecordListChangePayload: TodoPeerRecordListChangePayload? = null
    private var pendingTodoAgendaPayload: TodoPeerTodoAgendaPayload? = null
    private var pendingTodoEditPayload: TodoPeerTodoEditPayload? = null
    private var pendingTodoJoinPayload: TodoPeerTodoJoinPayload? = null
    private var pendingTodoRolePayload: TodoPeerTodoRolePayload? = null
    private var pendingTodoRoomPayload: TodoPeerTodoRoomPayload? = null
    private var pendingTodoZonePayload: TodoPeerTodoZonePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: TodoPeerEvent, metadata: EventMetadata) {
        pendingRecordListChangePayload = null
        pendingTodoAgendaPayload = null
        pendingTodoEditPayload = null
        pendingTodoJoinPayload = null
        pendingTodoRolePayload = null
        pendingTodoRoomPayload = null
        pendingTodoZonePayload = null
        when (val tp = metadata.typedPayload) {
            is TodoPeerRecordListChangePayload -> pendingRecordListChangePayload = tp
            is TodoPeerTodoAgendaPayload -> pendingTodoAgendaPayload = tp
            is TodoPeerTodoEditPayload -> pendingTodoEditPayload = tp
            is TodoPeerTodoJoinPayload -> pendingTodoJoinPayload = tp
            is TodoPeerTodoRolePayload -> pendingTodoRolePayload = tp
            is TodoPeerTodoRoomPayload -> pendingTodoRoomPayload = tp
            is TodoPeerTodoZonePayload -> pendingTodoZonePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == TodoPeerEvent.RecordList.Change) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListChangePayload = TodoPeerRecordListChangePayload(fields.field("by").let { it.uint64() }, fields.field("wallTime").let { it.int64() }, fields.field("counter").let { it.uint32() }, fields.field("ops").let { it.listWithin(1, 64) { it.record(null) { TodoPeerOpRecord(kind = it.field("kind").let { it.uint8() }, id = it.field("id").let { it.uint32() }, field = it.field("field").let { it.uint16() }, value = it.field("value").let { it.int64() }, text = it.field("text").let { it.string(96) }, wallTime = it.field("wallTime").let { it.int64() }, counter = it.field("counter").let { it.uint32() }, nodeId = it.field("nodeId").let { it.uint64() }) } } })
                } else if (event == TodoPeerEvent.Todo.Agenda) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoAgendaPayload = TodoPeerTodoAgendaPayload(fields.uint32("id"), fields.int64("windowStart"), fields.int64("windowEnd"), fields.uint32("skip"))
                } else if (event == TodoPeerEvent.Todo.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoEditPayload = TodoPeerTodoEditPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text", 32), fields.int64("now"))
                } else if (event == TodoPeerEvent.Todo.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoJoinPayload = TodoPeerTodoJoinPayload(fields.uint64("node"))
                } else if (event == TodoPeerEvent.Todo.Role) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoRolePayload = TodoPeerTodoRolePayload(fields.uint64("node"), fields.uint8("role"))
                } else if (event == TodoPeerEvent.Todo.Room) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoRoomPayload = TodoPeerTodoRoomPayload(fields.uint32("values"))
                } else if (event == TodoPeerEvent.Todo.Zone) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoZonePayload = TodoPeerTodoZonePayload(fields.uint8("zone"), fields.int64("start"), fields.int32("offset"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.change` — binds the event name and the payload field values in one call.
    fun raiseRecordListChange(by: ULong, wallTime: Long, counter: UInt, ops: List<TodoPeerOpRecord>) {
        val scePayload = TodoPeerRecordListChangePayload(by, wallTime, counter, ops)
        send(
            TodoPeerEvent.RecordList.Change,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("by" to com.sce.runtime.EventPayload.valueJson(scePayload.by), "wallTime" to com.sce.runtime.EventPayload.valueJson(scePayload.wallTime), "counter" to com.sce.runtime.EventPayload.valueJson(scePayload.counter), "ops" to com.sce.runtime.EventPayload.listJson(scePayload.ops) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("kind" to com.sce.runtime.EventPayload.valueJson(it.kind), "id" to com.sce.runtime.EventPayload.valueJson(it.id), "field" to com.sce.runtime.EventPayload.valueJson(it.field), "value" to com.sce.runtime.EventPayload.valueJson(it.value), "text" to com.sce.runtime.EventPayload.valueJson(it.text), "wallTime" to com.sce.runtime.EventPayload.valueJson(it.wallTime), "counter" to com.sce.runtime.EventPayload.valueJson(it.counter), "nodeId" to com.sce.runtime.EventPayload.valueJson(it.nodeId))) })).json
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.agenda` — binds the event name and the payload field values in one call.
    fun raiseTodoAgenda(id: UInt, windowStart: Long, windowEnd: Long, skip: UInt) {
        send(
            TodoPeerEvent.Todo.Agenda,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoAgendaPayload(id, windowStart, windowEnd, skip),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "windowStart" to windowStart, "windowEnd" to windowEnd, "skip" to skip))
            )
        )
    }
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
    fun raiseTodoJoin(node: ULong) {
        send(
            TodoPeerEvent.Todo.Join,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoJoinPayload(node),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.role` — binds the event name and the payload field values in one call.
    fun raiseTodoRole(node: ULong, role: UByte) {
        send(
            TodoPeerEvent.Todo.Role,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoRolePayload(node, role),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "role" to role))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.room` — binds the event name and the payload field values in one call.
    fun raiseTodoRoom(values: UInt) {
        send(
            TodoPeerEvent.Todo.Room,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoRoomPayload(values),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("values" to values))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.zone` — binds the event name and the payload field values in one call.
    fun raiseTodoZone(zone: UByte, start: Long, offset: Int) {
        send(
            TodoPeerEvent.Todo.Zone,
            EventMetadata(
                type = "external",
                typedPayload = TodoPeerTodoZonePayload(zone, start, offset),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("zone" to zone, "start" to start, "offset" to offset))
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
    }

    private object TransitionsPart0 {

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

        // W3C SCXML 3.13: open's transition 10, as the microstep reads it.
        val transitionOpenAt10 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            10,
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
        "record_list.change" -> TodoPeerEvent.RecordList.Change
        "todo.agenda" -> TodoPeerEvent.Todo.Agenda
        "todo.edit" -> TodoPeerEvent.Todo.Edit
        "todo.join" -> TodoPeerEvent.Todo.Join
        "todo.pass" -> TodoPeerEvent.Todo.Pass
        "todo.role" -> TodoPeerEvent.Todo.Role
        "todo.room" -> TodoPeerEvent.Todo.Room
        "todo.take" -> TodoPeerEvent.Todo.Take
        "todo.zone" -> TodoPeerEvent.Todo.Zone
        "todo.zone_clear" -> TodoPeerEvent.Todo.ZoneClear
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TodoPeerEvent): String? = when (event) {
        is TodoPeerEvent.Error.Execution -> "error.execution"
        is TodoPeerEvent.RecordList.Change -> "record_list.change"
        is TodoPeerEvent.Todo.Agenda -> "todo.agenda"
        is TodoPeerEvent.Todo.Edit -> "todo.edit"
        is TodoPeerEvent.Todo.Join -> "todo.join"
        is TodoPeerEvent.Todo.Pass -> "todo.pass"
        is TodoPeerEvent.Todo.Role -> "todo.role"
        is TodoPeerEvent.Todo.Room -> "todo.room"
        is TodoPeerEvent.Todo.Take -> "todo.take"
        is TodoPeerEvent.Todo.Zone -> "todo.zone"
        is TodoPeerEvent.Todo.ZoneClear -> "todo.zone_clear"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: TodoPeerState, event: TodoPeerEvent?): EnabledTransition<TodoPeerState, HistoryId>? = when (state) {
        is TodoPeerState.Open -> firstEnabledTransitionOfOpen(state, event)
    }


    // open's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfOpen(state: TodoPeerState, event: TodoPeerEvent?): EnabledTransition<TodoPeerState, HistoryId>? = when {
        event is TodoPeerEvent.Todo.Join -> TransitionsPart0.transitionOpenAt0
        event is TodoPeerEvent.Todo.Room -> TransitionsPart0.transitionOpenAt1
        event is TodoPeerEvent.Todo.Role -> TransitionsPart0.transitionOpenAt2
        event is TodoPeerEvent.Todo.Edit -> TransitionsPart0.transitionOpenAt3
        event is TodoPeerEvent.RecordList.Change -> TransitionsPart0.transitionOpenAt4
        event is TodoPeerEvent.Todo.Take -> TransitionsPart0.transitionOpenAt5
        event is TodoPeerEvent.Todo.Pass -> TransitionsPart0.transitionOpenAt6
        event is TodoPeerEvent.Todo.Agenda -> TransitionsPart0.transitionOpenAt7
        event is TodoPeerEvent.Todo.Zone -> TransitionsPart0.transitionOpenAt8
        event is TodoPeerEvent.Todo.ZoneClear -> TransitionsPart0.transitionOpenAt9
        event is TodoPeerEvent.Error.Execution -> TransitionsPart0.transitionOpenAt10
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: todo_peer.scxml:78 :: _machine
    override fun onEntry(state: TodoPeerState, isDefaultEntry: Boolean) {
        when (state) {
            is TodoPeerState.Open -> onEntryOfOpen()
        }
    }


    private fun onEntryOfOpen() {
                // SCE-MAP: todo_peer.scxml:259 :: open :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: todo_peer.scxml:78 :: _machine
    override fun onExit(state: TodoPeerState) {
        when (state) {
            is TodoPeerState.Open -> onExitOfOpen()
        }
    }


    private fun onExitOfOpen() {
                // SCE-MAP: todo_peer.scxml:259 :: open :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: todo_peer.scxml:78 :: _machine
    override fun executeTransitionContent(source: TodoPeerState, transitionIndex: Int) {
        when (source) {
            is TodoPeerState.Open -> transitionContentOfOpen(transitionIndex)
        }
    }


    // The content of `open`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfOpen(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfOpenAt0()
            1 -> transitionContentOfOpenAt1()
            2 -> transitionContentOfOpenAt2()
            3 -> transitionContentOfOpenAt3()
            4 -> transitionContentOfOpenAt4()
            5 -> transitionContentOfOpenAt5()
            6 -> transitionContentOfOpenAt6()
            7 -> transitionContentOfOpenAt7()
            8 -> transitionContentOfOpenAt8()
            9 -> transitionContentOfOpenAt9()
            10 -> transitionContentOfOpenAt10()
            else -> {}
        }
    }


    private fun transitionContentOfOpenAt0() {
                // SCE-MAP: todo_peer.scxml:260 :: open :: _transition_0
                if (pendingTodoJoinPayload == null) {
                    return
                }

            vNode = pendingTodoJoinPayload!!.node
    }

    private fun transitionContentOfOpenAt1() {
                // SCE-MAP: todo_peer.scxml:265 :: open :: _transition_1
                if (pendingTodoRoomPayload == null) {
                    return
                }

            vRoomValues = pendingTodoRoomPayload!!.values
    }

    private fun transitionContentOfOpenAt2() {
                // SCE-MAP: todo_peer.scxml:272 :: open :: _transition_2
                if (pendingTodoRolePayload == null) {
                    return
                }

            if (try { vRoster = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_EventRoster__v(vRoster, pendingTodoRolePayload!!.node, pendingTodoRolePayload!!.role)), 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='roster'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt3() {
                // SCE-MAP: todo_peer.scxml:293 :: open :: _transition_3
                if (pendingTodoEditPayload == null) {
                    return
                }

            if (try { vTyped = com.sce.forge.runtime.SceChecked.bounded(pendingTodoEditPayload!!.text, 32); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='typed'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(element = pendingTodoEditPayload!!.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), pendingTodoEditPayload!!.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
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


            if (vForbidden) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vGone = 0.toUInt()


            for (o in vAdds) {


            if (o.element == vEntry.element) {

            if (try { vGone = com.sce.forge.runtime.SceChecked.add(vGone, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='gone'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            if (!vForbidden && vRoomValues > 0.toUInt() && vGone == 0.toUInt() && (vValues).size.toUInt() >= vRoomValues) {

            if (try { vFull = com.sce.forge.runtime.SceChecked.add(vFull, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='full'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vForbidden = true
            }


            if (!vForbidden) {

            if (try { vStamp = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, pendingTodoEditPayload!!.now, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='stamp'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vTaking = emptyList()

            vTook = 0.toUInt()


            for (o in vAdds) {


            if (o.element == vEntry.element) {

            vOp = vOp.copy(kind = 1.toUByte())

            vOp = vOp.copy(id = pendingTodoEditPayload!!.id)

            vOp = vOp.copy(field = pendingTodoEditPayload!!.field)

            vOp = vOp.copy(value = 0)

            if (try { vOp = vOp.copy(text = com.sce.forge.runtime.SceChecked.bounded("", 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='op.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOp = vOp.copy(wallTime = o.wallTime)

            vOp = vOp.copy(counter = o.counter)

            vOp = vOp.copy(nodeId = o.nodeId)

            if (if (vTaking.size < 64) { vTaking = vTaking + (vOp); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='taking'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }

            vOp = vOp.copy(kind = 0.toUByte())

            vOp = vOp.copy(id = pendingTodoEditPayload!!.id)

            vOp = vOp.copy(field = pendingTodoEditPayload!!.field)

            vOp = vOp.copy(value = pendingTodoEditPayload!!.value)

            if (try { vOp = vOp.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingTodoEditPayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='op.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOp = vOp.copy(wallTime = 0)

            vOp = vOp.copy(counter = 0.toUInt())

            vOp = vOp.copy(nodeId = 0.toULong())

            if (if (vTaking.size < 64) { vTaking = vTaking + (vOp); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='taking'>: the list already holds its capacity of 64"); true }) {
                return
            }

            vClock = vStamp

            vDot = vStamp


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Take, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Pass, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
    }

    private fun transitionContentOfOpenAt4() {
                // SCE-MAP: todo_peer.scxml:364 :: open :: _transition_4
                if (pendingRecordListChangePayload == null) {
                    return
                }

            vAuthorized = true


            if ((vRoster).size.toUInt() > 0.toUInt()) {

            vAuthorized = false


            for (m in vRoster) {


            if (m.node == pendingRecordListChangePayload!!.by && m.role >= 2.toUByte()) {

            vAuthorized = true
            }
            }
            }


            if (!vAuthorized) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vAuthorized) {

            vTaking = pendingRecordListChangePayload!!.ops

            vTook = 0.toUInt()

            vDot = vDot.copy(wallTime = pendingRecordListChangePayload!!.wallTime)

            vDot = vDot.copy(counter = pendingRecordListChangePayload!!.counter)

            vDot = vDot.copy(nodeId = pendingRecordListChangePayload!!.by)


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(TodoPeerEvent.Todo.Take, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vDot.nodeId == vNode && (vDot.wallTime > vClock.wallTime || vDot.wallTime == vClock.wallTime && vDot.counter > vClock.counter)) {

            vClock = vClock.copy(wallTime = vDot.wallTime)

            vClock = vClock.copy(counter = vDot.counter)
            }


            if (vDot.nodeId != vNode) {

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcReceive__vv(vClock, vDot, vClock.wallTime, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
    }

    private fun transitionContentOfOpenAt5() {
                // SCE-MAP: todo_peer.scxml:403 :: open :: _transition_5

            if (try { vOp = com.sce.forge.runtime.SceChecked.at(vTaking, (vTook).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='op'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTook = com.sce.forge.runtime.SceChecked.add(vTook, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='took'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(element = vOp.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), vOp.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vOp.kind == 0.toUByte()) {

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(vOp.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(id = vOp.id.toULong())

            vHeld = vHeld.copy(field = vOp.field.toULong())

            vHeld = vHeld.copy(value = vOp.value)

            vHeld = vHeld.copy(wallTime = vDot.wallTime)

            vHeld = vHeld.copy(counter = vDot.counter)

            vHeld = vHeld.copy(nodeId = vDot.nodeId)

            vEntry = vEntry.copy(wallTime = vDot.wallTime)

            vEntry = vEntry.copy(counter = vDot.counter)

            vEntry = vEntry.copy(nodeId = vDot.nodeId)

            vPresent = call_MvValueHolds__vv(vValues, vHeld)


            if (!vPresent) {

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueKeep__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            if (vOp.kind == 1.toUByte()) {

            vEntry = vEntry.copy(wallTime = vOp.wallTime)

            vEntry = vEntry.copy(counter = vOp.counter)

            vEntry = vEntry.copy(nodeId = vOp.nodeId)

            vHeld = vHeld.copy(id = vOp.id.toULong())

            vHeld = vHeld.copy(field = vOp.field.toULong())

            vHeld = vHeld.copy(wallTime = vOp.wallTime)

            vHeld = vHeld.copy(counter = vOp.counter)

            vHeld = vHeld.copy(nodeId = vOp.nodeId)

            vWasLive = call_MvValueHolds__vv(vValues, vHeld)

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueDrop__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfOpenAt6() {
                // SCE-MAP: todo_peer.scxml:451 :: open :: _transition_6


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "by", (vDot.nodeId).toString())

            putParam(sendPayload, "wallTime", (vDot.wallTime).toString())

            putParam(sendPayload, "counter", (vDot.counter).toLong())

            try {
                putParam(sendPayload, "ops", run { if ((vTaking).size < 1) throw com.sce.forge.runtime.AlgorithmFailure(com.sce.forge.runtime.AlgorithmError.Precondition); (vTaking).map { mapOf<String, Any?>("kind" to (it.kind).toLong(), "id" to (it.id).toLong(), "field" to (it.field).toLong(), "value" to (it.value).toString(), "text" to it.text, "wallTime" to (it.wallTime).toString(), "counter" to (it.counter).toLong(), "nodeId" to (it.nodeId).toString()) } })
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                // docs/adr/0086, section 15.6: the list is short of its field's lower
                // count, so the event is refused whole: one error, nothing delivered,
                // and the error ends the block (W3C SCXML 6.2.3, 4.9).
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send> <param name='ops'> holds fewer elements than the 1 its field's type declares, so the event is not sent", "__send_3")
                return@send true
            }

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
                eventName = "record_list.change",
                target = "#people",
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

    private fun transitionContentOfOpenAt7() {
                // SCE-MAP: todo_peer.scxml:464 :: open :: _transition_7
                if (pendingTodoAgendaPayload == null) {
                    return
                }

            vAgenda = emptyList()

            vAgendaNext = 0.toUInt()

            vRawCount = 0.toUInt()

            vRawLast = 0.toUInt()

            vSched = vSched.copy(zone = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 15.toULong(), 0))

            vSched = vSched.copy(start = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 16.toULong(), -1))

            vSched = vSched.copy(length = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 17.toULong(), 0))

            vSched = vSched.copy(freq = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 18.toULong(), 0))

            vSched = vSched.copy(interval = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 19.toULong(), 1))

            vSched = vSched.copy(days = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 20.toULong(), 0))

            vSched = vSched.copy(count = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 21.toULong(), 0))

            vSched = vSched.copy(until = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 22.toULong(), 0))

            vSched = vSched.copy(bymonth = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 23.toULong(), 0))

            vSched = vSched.copy(mdPos = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 24.toULong(), 0))

            vSched = vSched.copy(mdNeg = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 25.toULong(), 0))

            vSched = vSched.copy(invalid = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 26.toULong(), 0))

            vSched = vSched.copy(op0 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 45.toULong(), 0))

            vSched = vSched.copy(op1 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 46.toULong(), 0))

            vSched = vSched.copy(op2 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 47.toULong(), 0))

            vSched = vSched.copy(op3 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 48.toULong(), 0))

            vSched = vSched.copy(op4 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 49.toULong(), 0))

            vSched = vSched.copy(op5 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 50.toULong(), 0))

            vSched = vSched.copy(op6 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 51.toULong(), 0))

            vSched = vSched.copy(on0 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 52.toULong(), 0))

            vSched = vSched.copy(on1 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 53.toULong(), 0))

            vSched = vSched.copy(on2 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 54.toULong(), 0))

            vSched = vSched.copy(on3 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 55.toULong(), 0))

            vSched = vSched.copy(on4 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 56.toULong(), 0))

            vSched = vSched.copy(on5 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 57.toULong(), 0))

            vSched = vSched.copy(on6 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 58.toULong(), 0))

            vSched = vSched.copy(spPos = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 28.toULong(), 0))

            vSched = vSched.copy(spNeg = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 29.toULong(), 0))

            vSched = vSched.copy(wkst = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 30.toULong(), 1))

            vSched = vSched.copy(wnPos = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 31.toULong(), 0))

            vSched = vSched.copy(wnNeg = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 32.toULong(), 0))

            vSched = vSched.copy(yd0 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 33.toULong(), 0))

            vSched = vSched.copy(yd1 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 34.toULong(), 0))

            vSched = vSched.copy(yd2 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 35.toULong(), 0))

            vSched = vSched.copy(yd3 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 36.toULong(), 0))

            vSched = vSched.copy(yd4 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 37.toULong(), 0))

            vSched = vSched.copy(yd5 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 38.toULong(), 0))

            vSched = vSched.copy(ydn0 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 39.toULong(), 0))

            vSched = vSched.copy(ydn1 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 40.toULong(), 0))

            vSched = vSched.copy(ydn2 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 41.toULong(), 0))

            vSched = vSched.copy(ydn3 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 42.toULong(), 0))

            vSched = vSched.copy(ydn4 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 43.toULong(), 0))

            vSched = vSched.copy(ydn5 = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 44.toULong(), 0))

            vSched = vSched.copy(gone = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 9.toULong(), 0))

            vSplit = call_EventConflict__v(vValues, pendingTodoAgendaPayload!!.id.toULong())

            vSched = vSched.copy(flag = 0)


            if (vSplit) {

            vSched = vSched.copy(flag = 1)
            }

            vZoneStarts = emptyList()

            vZoneOffsets = emptyList()

            vZoneRows = 0.toUInt()


            if (vSched.zone == 1L) {


            for (s1 in vZoneStarts1) {

            if (if (vZoneStarts.size < 64) { vZoneStarts = vZoneStarts + (s1); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (try { vZoneRows = com.sce.forge.runtime.SceChecked.add(vZoneRows, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='zoneRows'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (o1 in vZoneOffsets1) {

            if (if (vZoneOffsets.size < 64) { vZoneOffsets = vZoneOffsets + (o1); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            if (vSched.zone == 2L) {


            for (s2 in vZoneStarts2) {

            if (if (vZoneStarts.size < 64) { vZoneStarts = vZoneStarts + (s2); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (try { vZoneRows = com.sce.forge.runtime.SceChecked.add(vZoneRows, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='zoneRows'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (o2 in vZoneOffsets2) {

            if (if (vZoneOffsets.size < 64) { vZoneOffsets = vZoneOffsets + (o2); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            if (vSched.zone == 3L) {


            for (s3 in vZoneStarts3) {

            if (if (vZoneStarts.size < 64) { vZoneStarts = vZoneStarts + (s3); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (try { vZoneRows = com.sce.forge.runtime.SceChecked.add(vZoneRows, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='zoneRows'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (o3 in vZoneOffsets3) {

            if (if (vZoneOffsets.size < 64) { vZoneOffsets = vZoneOffsets + (o3); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            if (vSched.zone == 4L) {


            for (s4 in vZoneStarts4) {

            if (if (vZoneStarts.size < 64) { vZoneStarts = vZoneStarts + (s4); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (try { vZoneRows = com.sce.forge.runtime.SceChecked.add(vZoneRows, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='zoneRows'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            for (o4 in vZoneOffsets4) {

            if (if (vZoneOffsets.size < 64) { vZoneOffsets = vZoneOffsets + (o4); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            if (vZoneRows == 0.toUInt()) {

            if (try { vAgenda = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_EventAgenda__vv(vSched, vValues, pendingTodoAgendaPayload!!.id.toULong(), pendingTodoAgendaPayload!!.windowStart, pendingTodoAgendaPayload!!.windowEnd, pendingTodoAgendaPayload!!.skip)), 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='agenda'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (a in vAgenda) {

            if (try { vRawCount = com.sce.forge.runtime.SceChecked.add(vRawCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='rawCount'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vRawLast = a.nth
            }
            }


            if (vZoneRows != 0.toUInt()) {

            if (try { vLocal = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_EventAgenda__vv(vSched, vValues, pendingTodoAgendaPayload!!.id.toULong(), com.sce.forge.runtime.SceChecked.sub(pendingTodoAgendaPayload!!.windowStart, 86400000), com.sce.forge.runtime.SceChecked.add(pendingTodoAgendaPayload!!.windowEnd, 86400000), pendingTodoAgendaPayload!!.skip)), 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='local'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (l in vLocal) {

            if (try { vRawCount = com.sce.forge.runtime.SceChecked.add(vRawCount, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='rawCount'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vRawLast = l.nth

            vOcc = vOcc.copy(id = l.id)

            vOcc = vOcc.copy(nth = l.nth)

            vOcc = vOcc.copy(conflict = l.conflict)

            if (try { vUtcSec = com.sce.forge.runtime.SceChecked.take(call_UtcFromLocal__vv(vZoneStarts, vZoneOffsets, com.sce.forge.runtime.SceChecked.div(l.start, 1000))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='utcSec'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOcc = vOcc.copy(start = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vUtcSec, 1000), com.sce.forge.runtime.SceChecked.rem(l.start, 1000))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='occ.start'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOcc = vOcc.copy(end = com.sce.forge.runtime.SceChecked.add(vOcc.start, com.sce.forge.runtime.SceChecked.sub(l.end, l.start))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='occ.end'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if ((vOcc.end > pendingTodoAgendaPayload!!.windowStart || vOcc.start >= pendingTodoAgendaPayload!!.windowStart) && vOcc.start < pendingTodoAgendaPayload!!.windowEnd) {

            if (if (vAgenda.size < 64) { vAgenda = vAgenda + (vOcc); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='agenda'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }
            }


            if (vRawCount == 64.toUInt()) {

            if (try { vAgendaNext = com.sce.forge.runtime.SceChecked.add(vRawLast, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='agendaNext'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfOpenAt8() {
                // SCE-MAP: todo_peer.scxml:584 :: open :: _transition_8
                if (pendingTodoZonePayload == null) {
                    return
                }


            if (pendingTodoZonePayload!!.zone == 1.toUByte()) {

            if (if (vZoneStarts1.size < 64) { vZoneStarts1 = vZoneStarts1 + (pendingTodoZonePayload!!.start); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts1'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (if (vZoneOffsets1.size < 64) { vZoneOffsets1 = vZoneOffsets1 + (pendingTodoZonePayload!!.offset); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets1'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }


            if (pendingTodoZonePayload!!.zone == 2.toUByte()) {

            if (if (vZoneStarts2.size < 64) { vZoneStarts2 = vZoneStarts2 + (pendingTodoZonePayload!!.start); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts2'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (if (vZoneOffsets2.size < 64) { vZoneOffsets2 = vZoneOffsets2 + (pendingTodoZonePayload!!.offset); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets2'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }


            if (pendingTodoZonePayload!!.zone == 3.toUByte()) {

            if (if (vZoneStarts3.size < 64) { vZoneStarts3 = vZoneStarts3 + (pendingTodoZonePayload!!.start); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts3'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (if (vZoneOffsets3.size < 64) { vZoneOffsets3 = vZoneOffsets3 + (pendingTodoZonePayload!!.offset); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets3'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }


            if (pendingTodoZonePayload!!.zone == 4.toUByte()) {

            if (if (vZoneStarts4.size < 64) { vZoneStarts4 = vZoneStarts4 + (pendingTodoZonePayload!!.start); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneStarts4'>: the list already holds its capacity of 64"); true }) {
                return
            }

            if (if (vZoneOffsets4.size < 64) { vZoneOffsets4 = vZoneOffsets4 + (pendingTodoZonePayload!!.offset); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='zoneOffsets4'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfOpenAt9() {
                // SCE-MAP: todo_peer.scxml:603 :: open :: _transition_9

            vZoneStarts1 = emptyList()

            vZoneOffsets1 = emptyList()

            vZoneStarts2 = emptyList()

            vZoneOffsets2 = emptyList()

            vZoneStarts3 = emptyList()

            vZoneOffsets3 = emptyList()

            vZoneStarts4 = emptyList()

            vZoneOffsets4 = emptyList()
    }

    private fun transitionContentOfOpenAt10() {
                // SCE-MAP: todo_peer.scxml:614 :: open :: _transition_10

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
