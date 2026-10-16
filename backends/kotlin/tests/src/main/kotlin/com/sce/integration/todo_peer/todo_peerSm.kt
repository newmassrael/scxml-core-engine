// SCE-GENERATED — DO NOT EDIT
// source-hash: 5b579589f1ff33cd05a081b40dc01d4d0d0b2ef410a9718dd0ae11e84c00a952

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/todo_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: todo_peer.scxml:71 :: _machine
@file:OptIn(ExperimentalUnsignedTypes::class)

package com.sce.integration.todo_peer

import com.sce.runtime.*
import com.sce.generated.event_agenda.*
import com.sce.generated.event_conflict.*
import com.sce.generated.event_latest.*
import com.sce.generated.event_occurrence.*
import com.sce.generated.event_schedule.*
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
    sealed interface Todo : TodoPeerEvent {
        data object Agenda : Todo
        data object Edit : Todo
        data object Join : Todo
        data object Keep : Todo
        data object Node : Todo
        data object Resync : Todo
        data object Share : Todo
        data object Supersede : Todo
        data object Visit : Todo
        data object Write : Todo
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
data class TodoPeerScheduleRecord(val zone: Long, val start: Long, val length: Long, val freq: Long, val interval: Long, val days: Long, val count: Long, val until: Long, val bymonth: Long, val mdPos: Long, val mdNeg: Long, val dayA: Long, val dayB: Long, val spPos: Long, val spNeg: Long, val wkst: Long, val wnPos: Long, val wnNeg: Long, val yd0: Long, val yd1: Long, val yd2: Long, val yd3: Long, val yd4: Long, val yd5: Long, val ydn0: Long, val ydn1: Long, val ydn2: Long, val ydn3: Long, val ydn4: Long, val ydn5: Long, val gone: Long, val flag: Long) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("zone" to SavedValues.of(zone), "start" to SavedValues.of(start), "length" to SavedValues.of(length), "freq" to SavedValues.of(freq), "interval" to SavedValues.of(interval), "days" to SavedValues.of(days), "count" to SavedValues.of(count), "until" to SavedValues.of(until), "bymonth" to SavedValues.of(bymonth), "mdPos" to SavedValues.of(mdPos), "mdNeg" to SavedValues.of(mdNeg), "dayA" to SavedValues.of(dayA), "dayB" to SavedValues.of(dayB), "spPos" to SavedValues.of(spPos), "spNeg" to SavedValues.of(spNeg), "wkst" to SavedValues.of(wkst), "wnPos" to SavedValues.of(wnPos), "wnNeg" to SavedValues.of(wnNeg), "yd0" to SavedValues.of(yd0), "yd1" to SavedValues.of(yd1), "yd2" to SavedValues.of(yd2), "yd3" to SavedValues.of(yd3), "yd4" to SavedValues.of(yd4), "yd5" to SavedValues.of(yd5), "ydn0" to SavedValues.of(ydn0), "ydn1" to SavedValues.of(ydn1), "ydn2" to SavedValues.of(ydn2), "ydn3" to SavedValues.of(ydn3), "ydn4" to SavedValues.of(ydn4), "ydn5" to SavedValues.of(ydn5), "gone" to SavedValues.of(gone), "flag" to SavedValues.of(flag))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerScheduleRecord = TodoPeerScheduleRecord(zone = SavedValues.int64(SavedValues.field(value, what, "zone"), "$what.zone"), start = SavedValues.int64(SavedValues.field(value, what, "start"), "$what.start"), length = SavedValues.int64(SavedValues.field(value, what, "length"), "$what.length"), freq = SavedValues.int64(SavedValues.field(value, what, "freq"), "$what.freq"), interval = SavedValues.int64(SavedValues.field(value, what, "interval"), "$what.interval"), days = SavedValues.int64(SavedValues.field(value, what, "days"), "$what.days"), count = SavedValues.int64(SavedValues.field(value, what, "count"), "$what.count"), until = SavedValues.int64(SavedValues.field(value, what, "until"), "$what.until"), bymonth = SavedValues.int64(SavedValues.field(value, what, "bymonth"), "$what.bymonth"), mdPos = SavedValues.int64(SavedValues.field(value, what, "mdPos"), "$what.mdPos"), mdNeg = SavedValues.int64(SavedValues.field(value, what, "mdNeg"), "$what.mdNeg"), dayA = SavedValues.int64(SavedValues.field(value, what, "dayA"), "$what.dayA"), dayB = SavedValues.int64(SavedValues.field(value, what, "dayB"), "$what.dayB"), spPos = SavedValues.int64(SavedValues.field(value, what, "spPos"), "$what.spPos"), spNeg = SavedValues.int64(SavedValues.field(value, what, "spNeg"), "$what.spNeg"), wkst = SavedValues.int64(SavedValues.field(value, what, "wkst"), "$what.wkst"), wnPos = SavedValues.int64(SavedValues.field(value, what, "wnPos"), "$what.wnPos"), wnNeg = SavedValues.int64(SavedValues.field(value, what, "wnNeg"), "$what.wnNeg"), yd0 = SavedValues.int64(SavedValues.field(value, what, "yd0"), "$what.yd0"), yd1 = SavedValues.int64(SavedValues.field(value, what, "yd1"), "$what.yd1"), yd2 = SavedValues.int64(SavedValues.field(value, what, "yd2"), "$what.yd2"), yd3 = SavedValues.int64(SavedValues.field(value, what, "yd3"), "$what.yd3"), yd4 = SavedValues.int64(SavedValues.field(value, what, "yd4"), "$what.yd4"), yd5 = SavedValues.int64(SavedValues.field(value, what, "yd5"), "$what.yd5"), ydn0 = SavedValues.int64(SavedValues.field(value, what, "ydn0"), "$what.ydn0"), ydn1 = SavedValues.int64(SavedValues.field(value, what, "ydn1"), "$what.ydn1"), ydn2 = SavedValues.int64(SavedValues.field(value, what, "ydn2"), "$what.ydn2"), ydn3 = SavedValues.int64(SavedValues.field(value, what, "ydn3"), "$what.ydn3"), ydn4 = SavedValues.int64(SavedValues.field(value, what, "ydn4"), "$what.ydn4"), ydn5 = SavedValues.int64(SavedValues.field(value, what, "ydn5"), "$what.ydn5"), gone = SavedValues.int64(SavedValues.field(value, what, "gone"), "$what.gone"), flag = SavedValues.int64(SavedValues.field(value, what, "flag"), "$what.flag"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_EventAgenda__vv(p0: TodoPeerScheduleRecord, p1: List<TodoPeerValueRecord>, p2: ULong, p3: Long, p4: Long, p5: UInt): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerOccurrenceRecord>> =
    when (val r = eventAgenda(EventSchedulePayload(zone = p0.zone, start = p0.start, length = p0.length, freq = p0.freq, interval = p0.interval, days = p0.days, count = p0.count, until = p0.until, bymonth = p0.bymonth, mdPos = p0.mdPos, mdNeg = p0.mdNeg, dayA = p0.dayA, dayB = p0.dayB, spPos = p0.spPos, spNeg = p0.spNeg, wkst = p0.wkst, wnPos = p0.wnPos, wnNeg = p0.wnNeg, yd0 = p0.yd0, yd1 = p0.yd1, yd2 = p0.yd2, yd3 = p0.yd3, yd4 = p0.yd4, yd5 = p0.yd5, ydn0 = p0.ydn0, ydn1 = p0.ydn1, ydn2 = p0.ydn2, ydn3 = p0.ydn3, ydn4 = p0.ydn4, ydn5 = p0.ydn5, gone = p0.gone, flag = p0.flag), p1.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p2, p3, p4, p5)) {
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
private fun call_UtcFromLocal__vv(p0: List<Long>, p1: List<Int>, p2: Long): com.sce.forge.runtime.AlgorithmResult<Long> =
    when (val r = utcFromLocal(p0.toLongArray(), p1.toIntArray(), p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
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
    private var vSched: TodoPeerScheduleRecord = TodoPeerScheduleRecord(zone = 0, start = -1, length = 0, freq = 0, interval = 1, days = 0, count = 0, until = 0, bymonth = 0, mdPos = 0, mdNeg = 0, dayA = 0, dayB = 0, spPos = 0, spNeg = 0, wkst = 1, wnPos = 0, wnNeg = 0, yd0 = 0, yd1 = 0, yd2 = 0, yd3 = 0, yd4 = 0, yd5 = 0, ydn0 = 0, ydn1 = 0, ydn2 = 0, ydn3 = 0, ydn4 = 0, ydn5 = 0, gone = 0, flag = 0)
    /** W3C SCXML 5.2: the `rawCount` datamodel variable, the machine's own. */
    private var vRawCount: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `rawLast` datamodel variable, the machine's own. */
    private var vRawLast: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `utcSec` datamodel variable, the machine's own. */
    private var vUtcSec: Long = 0

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
        var agendaNext: UInt? = null
        var zoneRows: UInt? = null
        var split: Boolean? = null
        var rawCount: UInt? = null
        var rawLast: UInt? = null
        var utcSec: Long? = null
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
        params.agendaNext?.let { vAgendaNext = it }
        params.zoneRows?.let { vZoneRows = it }
        params.split?.let { vSplit = it }
        params.rawCount?.let { vRawCount = it }
        params.rawLast?.let { vRawLast = it }
        params.utcSec?.let { vUtcSec = it }
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
        val agenda: List<TodoPeerOccurrenceRecord>,
        val agendaNext: UInt,
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
        agenda = vAgenda,
        agendaNext = vAgendaNext,
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
    val savedShape: String = "0d675589a1a598d334338d7def482d151314b543b4cdc5f04d343a186a0ffcb5"

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
            "agenda" to SavedValues.list(vAgenda) { it.toSaved() },
            "local" to SavedValues.list(vLocal) { it.toSaved() },
            "occ" to vOcc.toSaved(),
            "agendaNext" to SavedValues.of(vAgendaNext),
            "zoneStarts1" to SavedValues.list(vZoneStarts1) { SavedValues.of(it) },
            "zoneOffsets1" to SavedValues.list(vZoneOffsets1) { SavedValues.of(it) },
            "zoneStarts2" to SavedValues.list(vZoneStarts2) { SavedValues.of(it) },
            "zoneOffsets2" to SavedValues.list(vZoneOffsets2) { SavedValues.of(it) },
            "zoneStarts3" to SavedValues.list(vZoneStarts3) { SavedValues.of(it) },
            "zoneOffsets3" to SavedValues.list(vZoneOffsets3) { SavedValues.of(it) },
            "zoneStarts4" to SavedValues.list(vZoneStarts4) { SavedValues.of(it) },
            "zoneOffsets4" to SavedValues.list(vZoneOffsets4) { SavedValues.of(it) },
            "zoneStarts" to SavedValues.list(vZoneStarts) { SavedValues.of(it) },
            "zoneOffsets" to SavedValues.list(vZoneOffsets) { SavedValues.of(it) },
            "zoneRows" to SavedValues.of(vZoneRows),
            "split" to SavedValues.of(vSplit),
            "sched" to vSched.toSaved(),
            "rawCount" to SavedValues.of(vRawCount),
            "rawLast" to SavedValues.of(vRawLast),
            "utcSec" to SavedValues.of(vUtcSec),
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
        val saved22 = SavedValues.list(saved.variable("agenda"), "agenda", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        val saved23 = SavedValues.list(saved.variable("local"), "local", 64) { e, w -> TodoPeerOccurrenceRecord.fromSaved(e, w) }
        val saved24 = TodoPeerOccurrenceRecord.fromSaved(saved.variable("occ"), "occ")
        val saved25 = SavedValues.uint32(saved.variable("agendaNext"), "agendaNext")
        val saved26 = SavedValues.list(saved.variable("zoneStarts1"), "zoneStarts1", 64) { e, w -> SavedValues.int64(e, w) }
        val saved27 = SavedValues.list(saved.variable("zoneOffsets1"), "zoneOffsets1", 64) { e, w -> SavedValues.int32(e, w) }
        val saved28 = SavedValues.list(saved.variable("zoneStarts2"), "zoneStarts2", 64) { e, w -> SavedValues.int64(e, w) }
        val saved29 = SavedValues.list(saved.variable("zoneOffsets2"), "zoneOffsets2", 64) { e, w -> SavedValues.int32(e, w) }
        val saved30 = SavedValues.list(saved.variable("zoneStarts3"), "zoneStarts3", 64) { e, w -> SavedValues.int64(e, w) }
        val saved31 = SavedValues.list(saved.variable("zoneOffsets3"), "zoneOffsets3", 64) { e, w -> SavedValues.int32(e, w) }
        val saved32 = SavedValues.list(saved.variable("zoneStarts4"), "zoneStarts4", 64) { e, w -> SavedValues.int64(e, w) }
        val saved33 = SavedValues.list(saved.variable("zoneOffsets4"), "zoneOffsets4", 64) { e, w -> SavedValues.int32(e, w) }
        val saved34 = SavedValues.list(saved.variable("zoneStarts"), "zoneStarts", 64) { e, w -> SavedValues.int64(e, w) }
        val saved35 = SavedValues.list(saved.variable("zoneOffsets"), "zoneOffsets", 64) { e, w -> SavedValues.int32(e, w) }
        val saved36 = SavedValues.uint32(saved.variable("zoneRows"), "zoneRows")
        val saved37 = SavedValues.bool(saved.variable("split"), "split")
        val saved38 = TodoPeerScheduleRecord.fromSaved(saved.variable("sched"), "sched")
        val saved39 = SavedValues.uint32(saved.variable("rawCount"), "rawCount")
        val saved40 = SavedValues.uint32(saved.variable("rawLast"), "rawLast")
        val saved41 = SavedValues.int64(saved.variable("utcSec"), "utcSec")
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
        vAgenda = saved22
        vLocal = saved23
        vOcc = saved24
        vAgendaNext = saved25
        vZoneStarts1 = saved26
        vZoneOffsets1 = saved27
        vZoneStarts2 = saved28
        vZoneOffsets2 = saved29
        vZoneStarts3 = saved30
        vZoneOffsets3 = saved31
        vZoneStarts4 = saved32
        vZoneOffsets4 = saved33
        vZoneStarts = saved34
        vZoneOffsets = saved35
        vZoneRows = saved36
        vSplit = saved37
        vSched = saved38
        vRawCount = saved39
        vRawLast = saved40
        vUtcSec = saved41
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTodoAgendaPayload: TodoPeerTodoAgendaPayload? = null
    private var pendingTodoEditPayload: TodoPeerTodoEditPayload? = null
    private var pendingTodoJoinPayload: TodoPeerTodoJoinPayload? = null
    private var pendingTodoKeepPayload: TodoPeerTodoKeepPayload? = null
    private var pendingTodoNodePayload: TodoPeerTodoNodePayload? = null
    private var pendingTodoSharePayload: TodoPeerTodoSharePayload? = null
    private var pendingTodoSupersedePayload: TodoPeerTodoSupersedePayload? = null
    private var pendingTodoVisitPayload: TodoPeerTodoVisitPayload? = null
    private var pendingTodoWritePayload: TodoPeerTodoWritePayload? = null
    private var pendingTodoZonePayload: TodoPeerTodoZonePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: TodoPeerEvent, metadata: EventMetadata) {
        pendingTodoAgendaPayload = null
        pendingTodoEditPayload = null
        pendingTodoJoinPayload = null
        pendingTodoKeepPayload = null
        pendingTodoNodePayload = null
        pendingTodoSharePayload = null
        pendingTodoSupersedePayload = null
        pendingTodoVisitPayload = null
        pendingTodoWritePayload = null
        pendingTodoZonePayload = null
        when (val tp = metadata.typedPayload) {
            is TodoPeerTodoAgendaPayload -> pendingTodoAgendaPayload = tp
            is TodoPeerTodoEditPayload -> pendingTodoEditPayload = tp
            is TodoPeerTodoJoinPayload -> pendingTodoJoinPayload = tp
            is TodoPeerTodoKeepPayload -> pendingTodoKeepPayload = tp
            is TodoPeerTodoNodePayload -> pendingTodoNodePayload = tp
            is TodoPeerTodoSharePayload -> pendingTodoSharePayload = tp
            is TodoPeerTodoSupersedePayload -> pendingTodoSupersedePayload = tp
            is TodoPeerTodoVisitPayload -> pendingTodoVisitPayload = tp
            is TodoPeerTodoWritePayload -> pendingTodoWritePayload = tp
            is TodoPeerTodoZonePayload -> pendingTodoZonePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == TodoPeerEvent.Todo.Agenda) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoAgendaPayload = TodoPeerTodoAgendaPayload(fields.uint32("id"), fields.int64("windowStart"), fields.int64("windowEnd"), fields.uint32("skip"))
                } else if (event == TodoPeerEvent.Todo.Edit) {
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
                } else if (event == TodoPeerEvent.Todo.Zone) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoZonePayload = TodoPeerTodoZonePayload(fields.uint8("zone"), fields.int64("start"), fields.int32("offset"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
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

        // W3C SCXML 3.13: open's transition 11, as the microstep reads it.
        val transitionOpenAt11 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            11,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 12, as the microstep reads it.
        val transitionOpenAt12 = EnabledTransition<TodoPeerState, HistoryId>(
            TodoPeerState.Open,
            emptyList(),
            12,
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
        "todo.agenda" -> TodoPeerEvent.Todo.Agenda
        "todo.edit" -> TodoPeerEvent.Todo.Edit
        "todo.join" -> TodoPeerEvent.Todo.Join
        "todo.keep" -> TodoPeerEvent.Todo.Keep
        "todo.node" -> TodoPeerEvent.Todo.Node
        "todo.resync" -> TodoPeerEvent.Todo.Resync
        "todo.share" -> TodoPeerEvent.Todo.Share
        "todo.supersede" -> TodoPeerEvent.Todo.Supersede
        "todo.visit" -> TodoPeerEvent.Todo.Visit
        "todo.write" -> TodoPeerEvent.Todo.Write
        "todo.zone" -> TodoPeerEvent.Todo.Zone
        "todo.zone_clear" -> TodoPeerEvent.Todo.ZoneClear
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TodoPeerEvent): String? = when (event) {
        is TodoPeerEvent.Error.Execution -> "error.execution"
        is TodoPeerEvent.Todo.Agenda -> "todo.agenda"
        is TodoPeerEvent.Todo.Edit -> "todo.edit"
        is TodoPeerEvent.Todo.Join -> "todo.join"
        is TodoPeerEvent.Todo.Keep -> "todo.keep"
        is TodoPeerEvent.Todo.Node -> "todo.node"
        is TodoPeerEvent.Todo.Resync -> "todo.resync"
        is TodoPeerEvent.Todo.Share -> "todo.share"
        is TodoPeerEvent.Todo.Supersede -> "todo.supersede"
        is TodoPeerEvent.Todo.Visit -> "todo.visit"
        is TodoPeerEvent.Todo.Write -> "todo.write"
        is TodoPeerEvent.Todo.Zone -> "todo.zone"
        is TodoPeerEvent.Todo.ZoneClear -> "todo.zone_clear"
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
            event is TodoPeerEvent.Todo.Agenda -> transitionOpenAt9
            event is TodoPeerEvent.Todo.Zone -> transitionOpenAt10
            event is TodoPeerEvent.Todo.ZoneClear -> transitionOpenAt11
            event is TodoPeerEvent.Error.Execution -> transitionOpenAt12
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun onEntry(state: TodoPeerState, isDefaultEntry: Boolean) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:208 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun onExit(state: TodoPeerState) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:208 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: todo_peer.scxml:71 :: _machine
    override fun executeTransitionContent(source: TodoPeerState, transitionIndex: Int) {
        when (source) {
        is TodoPeerState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: todo_peer.scxml:209 :: open :: _transition_0
                if (pendingTodoJoinPayload == null) {
                    return
                }

            vNode = pendingTodoJoinPayload!!.node

            vHub = pendingTodoJoinPayload!!.hub
            }
            1 -> {
                // SCE-MAP: todo_peer.scxml:217 :: open :: _transition_1
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
                // SCE-MAP: todo_peer.scxml:266 :: open :: _transition_2
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
                // SCE-MAP: todo_peer.scxml:348 :: open :: _transition_3
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
                // SCE-MAP: todo_peer.scxml:419 :: open :: _transition_4
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
                // SCE-MAP: todo_peer.scxml:431 :: open :: _transition_5


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
                // SCE-MAP: todo_peer.scxml:458 :: open :: _transition_6
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
                // SCE-MAP: todo_peer.scxml:499 :: open :: _transition_7
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
                // SCE-MAP: todo_peer.scxml:546 :: open :: _transition_8
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
                // SCE-MAP: todo_peer.scxml:628 :: open :: _transition_9
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

            vSched = vSched.copy(dayA = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 26.toULong(), 0))

            vSched = vSched.copy(dayB = call_EventLatest__v(vValues, pendingTodoAgendaPayload!!.id.toULong(), 27.toULong(), 0))

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
            10 -> {
                // SCE-MAP: todo_peer.scxml:735 :: open :: _transition_10
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
            11 -> {
                // SCE-MAP: todo_peer.scxml:754 :: open :: _transition_11

            vZoneStarts1 = emptyList()

            vZoneOffsets1 = emptyList()

            vZoneStarts2 = emptyList()

            vZoneOffsets2 = emptyList()

            vZoneStarts3 = emptyList()

            vZoneOffsets3 = emptyList()

            vZoneStarts4 = emptyList()

            vZoneOffsets4 = emptyList()
            }
            12 -> {
                // SCE-MAP: todo_peer.scxml:765 :: open :: _transition_12

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
