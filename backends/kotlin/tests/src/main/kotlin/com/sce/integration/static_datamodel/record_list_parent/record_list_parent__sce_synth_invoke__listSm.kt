// SCE-GENERATED — DO NOT EDIT
// source-hash: d8c75e4d0233623996dc9a7cc3bdbe850af8d4f91ab023212ebd9f938cf7955f

// GENERATED CODE — DO NOT EDIT
// Source: sce:std/shared/record_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: record_list.scxml:210 :: _machine

package com.sce.integration.static_datamodel.record_list_parent

import com.sce.runtime.*
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
import com.sce.generated.record_member.*
import com.sce.generated.record_roster.*
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
        data object Announce : RecordList
        data object Change : RecordList
        data object Clear : RecordList
        data object Close : RecordList
        data object Closed : RecordList
        data object Conclude : RecordList
        data object Confirm : RecordList
        data object Confirmed : RecordList
        data object Deliver : RecordList
        data object Edit : RecordList
        data object Join : RecordList
        data object Made : RecordList
        data object Offer : RecordList
        data object Pass : RecordList
        data object Pending : RecordList
        data object Raise : RecordList
        data object Recall : RecordList
        data object Refused : RecordList
        data object Register : RecordList
        data object Restore : RecordList
        data object Retire : RecordList
        data object Role : RecordList
        data object Room : RecordList
        data object Take : RecordList
        data object Tell : RecordList
        data object Unwind : RecordList
        data object Value : RecordList
        data object Verdict : RecordList
        data object Withdraw : RecordList
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `RecordListParentSceSynthInvokeListPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// RecordListParentSceSynthInvokeListRecordListAnnouncePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.announce`. Consumers inject it via the `raiseRecordListAnnounce` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListAnnouncePayload(val id: UInt, val field: UShort)

// RecordListParentSceSynthInvokeListRecordListChangePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.change`. Consumers inject it via the `raiseRecordListChange` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListChangePayload(val by: ULong, val wallTime: Long, val counter: UInt, val ops: List<RecordListParentSceSynthInvokeListOpRecord>) {
    init {
        com.sce.runtime.ValueBounds.listWithin("ops", ops, 1, 64) { _, _ -> }
    }
}

// RecordListParentSceSynthInvokeListRecordListClearPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.clear`. Consumers inject it via the `raiseRecordListClear` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListClearPayload(val id: UInt, val field: UShort, val now: Long, val more: Boolean, val command: ULong)

// RecordListParentSceSynthInvokeListRecordListConfirmPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.confirm`. Consumers inject it via the `raiseRecordListConfirm` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListConfirmPayload(val wallTime: Long, val counter: UInt, val command: ULong)

// RecordListParentSceSynthInvokeListRecordListEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.edit`. Consumers inject it via the `raiseRecordListEdit` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListEditPayload(val id: UInt, val field: UShort, val value: Long, val text: String, val now: Long, val more: Boolean, val command: ULong)

// RecordListParentSceSynthInvokeListRecordListJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.join`. Consumers inject it via the `raiseRecordListJoin` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListJoinPayload(val node: ULong, val journal: Boolean)

// RecordListParentSceSynthInvokeListRecordListRaisePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.raise`. Consumers inject it via the `raiseRecordListRaise` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRaisePayload(val wallTime: Long, val counter: UInt, val command: ULong)

// RecordListParentSceSynthInvokeListRecordListRecallPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.recall`. Consumers inject it via the `raiseRecordListRecall` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRecallPayload(val by: ULong, val wallTime: Long, val counter: UInt, val ops: List<RecordListParentSceSynthInvokeListOpRecord>, val command: ULong, val provisional: Boolean) {
    init {
        com.sce.runtime.ValueBounds.listWithin("ops", ops, 1, 64) { _, _ -> }
    }
}

// RecordListParentSceSynthInvokeListRecordListRestorePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.restore`. Consumers inject it via the `raiseRecordListRestore` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRestorePayload(val by: ULong, val wallTime: Long, val counter: UInt, val ops: List<RecordListParentSceSynthInvokeListOpRecord>) {
    init {
        com.sce.runtime.ValueBounds.listWithin("ops", ops, 1, 64) { _, _ -> }
    }
}

// RecordListParentSceSynthInvokeListRecordListRetirePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.retire`. Consumers inject it via the `raiseRecordListRetire` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRetirePayload(val id: UInt, val now: Long, val more: Boolean, val command: ULong)

// RecordListParentSceSynthInvokeListRecordListRolePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.role`. Consumers inject it via the `raiseRecordListRole` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRolePayload(val node: ULong, val role: UByte)

// RecordListParentSceSynthInvokeListRecordListRoomPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.room`. Consumers inject it via the `raiseRecordListRoom` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListRoomPayload(val values: UInt, val history: UInt, val digest: UInt)

// RecordListParentSceSynthInvokeListRecordListVerdictPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.verdict`. Consumers inject it via the `raiseRecordListVerdict` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListVerdictPayload(val wallTime: Long, val counter: UInt, val outcome: RecordListParentSceSynthInvokeListOutcomeEnum, val reason: RecordListParentSceSynthInvokeListRefusalEnum?)

// RecordListParentSceSynthInvokeListRecordListWithdrawPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.withdraw`. Consumers inject it via the `raiseRecordListWithdraw` seam
// on the machine — they never name this class directly.
data class RecordListParentSceSynthInvokeListRecordListWithdrawPayload(val wallTime: Long, val counter: UInt, val command: ULong)


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
/** SCE Accepted Subset §2.15: a `record:Member` datamodel value. */
data class RecordListParentSceSynthInvokeListMemberRecord(val node: ULong, val role: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("node" to SavedValues.of(node), "role" to SavedValues.of(role))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListMemberRecord = RecordListParentSceSynthInvokeListMemberRecord(node = SavedValues.uint64(SavedValues.field(value, what, "node"), "$what.node"), role = SavedValues.uint8(SavedValues.field(value, what, "role"), "$what.role"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Staged` datamodel value. */
data class RecordListParentSceSynthInvokeListStagedRecord(val kind: UByte, val id: UInt, val field: UShort, val value: Long, val text: String) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("kind" to SavedValues.of(kind), "id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListStagedRecord = RecordListParentSceSynthInvokeListStagedRecord(kind = SavedValues.uint8(SavedValues.field(value, what, "kind"), "$what.kind"), id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96))
    }
}
/** SCE Accepted Subset §2.15: an `enum:Refusal` datamodel value. */
enum class RecordListParentSceSynthInvokeListRefusalEnum(val declaredName: String) {
    NOT_WRITER("notWriter"),
    NO_ROOM_FOR_VALUE("noRoomForValue"),
    NO_ROOM_FOR_HISTORY("noRoomForHistory"),
    NO_ROOM_FOR_DIGEST("noRoomForDigest"),
    CHANGE_TOO_LARGE("changeTooLarge"),
    REGISTER_NAMED_TWICE("registerNamedTwice"),
    CLOCK_EXHAUSTED("clockExhausted"),
    PEER_REFUSED("peerRefused");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NOT_WRITER -> 1u.toUByte()
        NO_ROOM_FOR_VALUE -> 2u.toUByte()
        NO_ROOM_FOR_HISTORY -> 3u.toUByte()
        NO_ROOM_FOR_DIGEST -> 4u.toUByte()
        CHANGE_TOO_LARGE -> 6u.toUByte()
        REGISTER_NAMED_TWICE -> 7u.toUByte()
        CLOCK_EXHAUSTED -> 8u.toUByte()
        PEER_REFUSED -> 9u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListRefusalEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Refusal")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): RecordListParentSceSynthInvokeListRefusalEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: an `enum:Command` datamodel value. */
enum class RecordListParentSceSynthInvokeListCommandEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListCommandEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Command")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): RecordListParentSceSynthInvokeListCommandEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Op` datamodel value. */
data class RecordListParentSceSynthInvokeListOpRecord(val kind: UByte, val id: UInt, val field: UShort, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("kind" to SavedValues.of(kind), "id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListOpRecord = RecordListParentSceSynthInvokeListOpRecord(kind = SavedValues.uint8(SavedValues.field(value, what, "kind"), "$what.kind"), id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Provisional` datamodel value. */
data class RecordListParentSceSynthInvokeListProvisionalRecord(val wallTime: Long, val counter: UInt, val command: ULong, val id: UInt, val field: UShort, val kind: RecordListParentSceSynthInvokeListCommandEnum) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "command" to SavedValues.of(command), "id" to SavedValues.of(id), "field" to SavedValues.of(field), "kind" to kind.toSaved())

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListProvisionalRecord = RecordListParentSceSynthInvokeListProvisionalRecord(wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), command = SavedValues.uint64(SavedValues.field(value, what, "command"), "$what.command"), id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), kind = RecordListParentSceSynthInvokeListCommandEnum.fromSaved(SavedValues.field(value, what, "kind"), "$what.kind"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Undo` datamodel value. */
data class RecordListParentSceSynthInvokeListUndoRecord(val atWallTime: Long, val atCounter: UInt, val id: ULong, val field: ULong, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("atWallTime" to SavedValues.of(atWallTime), "atCounter" to SavedValues.of(atCounter), "id" to SavedValues.of(id), "field" to SavedValues.of(field), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListUndoRecord = RecordListParentSceSynthInvokeListUndoRecord(atWallTime = SavedValues.int64(SavedValues.field(value, what, "atWallTime"), "$what.atWallTime"), atCounter = SavedValues.uint32(SavedValues.field(value, what, "atCounter"), "$what.atCounter"), id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint64(SavedValues.field(value, what, "field"), "$what.field"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: an `enum:Outcome` datamodel value. */
enum class RecordListParentSceSynthInvokeListOutcomeEnum(val declaredName: String) {
    NONE("none"),
    ACCEPTED("accepted"),
    HELD("held"),
    WAITING("waiting"),
    REFUSED("refused");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        NONE -> 0u.toUByte()
        ACCEPTED -> 1u.toUByte()
        HELD -> 2u.toUByte()
        WAITING -> 3u.toUByte()
        REFUSED -> 4u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentSceSynthInvokeListOutcomeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Outcome")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): RecordListParentSceSynthInvokeListOutcomeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** The bounds this machine is built with, each by the name `sce:limit` gives it (docs/adr/0050). */
object RecordListParentSceSynthInvokeListLimits {
    /** The bound `sce:limit="merge_entries"` names, as this build resolved it. */
    const val MERGE_ENTRIES: UInt = 1024u
    /** The bound `sce:limit="merge_text"` names, as this build resolved it. */
    const val MERGE_TEXT: UInt = 96u
    /** The bound `sce:limit="merge_values"` names, as this build resolved it. */
    const val MERGE_VALUES: UInt = 512u
    /** The bound `sce:limit="shared_moved"` names, as this build resolved it. */
    const val SHARED_MOVED: UInt = 64u
    /** The bound `sce:limit="shared_parts"` names, as this build resolved it. */
    const val SHARED_PARTS: UInt = 64u
    /** The bound `sce:limit="shared_provisional"` names, as this build resolved it. */
    const val SHARED_PROVISIONAL: UInt = 8u
    /** The bound `sce:limit="shared_roster"` names, as this build resolved it. */
    const val SHARED_ROSTER: UInt = 64u
    /** The bound `sce:limit="shared_staged"` names, as this build resolved it. */
    const val SHARED_STAGED: UInt = 32u
    /** The bound `sce:limit="shared_typed_text"` names, as this build resolved it. */
    const val SHARED_TYPED_TEXT: UInt = 32u
    /** The bound `sce:limit="shared_undo"` names, as this build resolved it. */
    const val SHARED_UNDO: UInt = 512u
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
private fun call_RegisterValues__v(p0: List<RecordListParentSceSynthInvokeListValueRecord>, p1: ULong, p2: ULong): com.sce.forge.runtime.AlgorithmResult<List<RecordListParentSceSynthInvokeListValueRecord>> =
    when (val r = registerValues(p0.map { MvValuePayload(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { RecordListParentSceSynthInvokeListValueRecord(id = it.id, field = it.field, value = it.value, text = it.text, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class RecordListParentSceSynthInvokeListStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<RecordListParentSceSynthInvokeListState, RecordListParentSceSynthInvokeListEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `node` datamodel variable, published (`sce:direction="out"`). */
    private var vNode: ULong = 0.toULong()

    /** What the published `node` holds. */
    val node: ULong get() = vNode
    /** W3C SCXML 5.2: the `journal` datamodel variable, the machine's own. */
    private var vJournal: Boolean = true
    /** W3C SCXML 5.2: the `clock` datamodel variable, published (`sce:direction="out"`). */
    private var vClock: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `stamp` datamodel variable, the machine's own. */
    private var vStamp: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `dot` datamodel variable, the machine's own. */
    private var vDot: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `adds` datamodel variable, published (`sce:direction="out"`). */
    private var vAdds: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()

    /** What the published `adds` holds. */
    val adds: List<RecordListParentSceSynthInvokeListEntryRecord> get() = vAdds
    /** W3C SCXML 5.2: the `one` datamodel variable, the machine's own. */
    private var vOne: List<RecordListParentSceSynthInvokeListEntryRecord> = emptyList()
    /** W3C SCXML 5.2: the `entry` datamodel variable, the machine's own. */
    private var vEntry: RecordListParentSceSynthInvokeListEntryRecord = RecordListParentSceSynthInvokeListEntryRecord(element = 0.toULong(), wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `values` datamodel variable, published (`sce:direction="out"`). */
    private var vValues: List<RecordListParentSceSynthInvokeListValueRecord> = emptyList()

    /** What the published `values` holds. */
    val values: List<RecordListParentSceSynthInvokeListValueRecord> get() = vValues
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: RecordListParentSceSynthInvokeListValueRecord = RecordListParentSceSynthInvokeListValueRecord(id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `shareId` datamodel variable, the machine's own. */
    private var vShareId: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `shareField` datamodel variable, the machine's own. */
    private var vShareField: UShort = 0.toUShort()
    /** W3C SCXML 5.2: the `wasLive` datamodel variable, the machine's own. */
    private var vWasLive: Boolean = false
    /** W3C SCXML 5.2: the `present` datamodel variable, the machine's own. */
    private var vPresent: Boolean = false
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    private var vRefusals: UInt = 0.toUInt()

    /** What the published `refusals` holds. */
    val refusals: UInt get() = vRefusals
    /** W3C SCXML 5.2: the `roomValues` datamodel variable, the machine's own. */
    private var vRoomValues: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `full` datamodel variable, published (`sce:direction="out"`). */
    private var vFull: UInt = 0.toUInt()

    /** What the published `full` holds. */
    val full: UInt get() = vFull
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
    /** W3C SCXML 5.2: the `staged` datamodel variable, the machine's own. */
    private var vStaged: List<RecordListParentSceSynthInvokeListStagedRecord> = emptyList()
    /** W3C SCXML 5.2: the `staging` datamodel variable, the machine's own. */
    private var vStaging: RecordListParentSceSynthInvokeListStagedRecord = RecordListParentSceSynthInvokeListStagedRecord(kind = 0.toUByte(), id = 0.toUInt(), field = 0.toUShort(), value = 0, text = "")
    /** W3C SCXML 5.2: the `holding` datamodel variable, the machine's own. */
    private var vHolding: Boolean = false
    /** W3C SCXML 5.2: the `closingNow` datamodel variable, the machine's own. */
    private var vClosingNow: Long = 0
    /** W3C SCXML 5.2: the `spoiled` datamodel variable, the machine's own. */
    private var vSpoiled: Boolean = false
    /** W3C SCXML 5.2: the `probing` datamodel variable, the machine's own. */
    private var vProbing: Boolean = false
    /** W3C SCXML 5.2: the `gone` datamodel variable, the machine's own. */
    private var vGone: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `writes` datamodel variable, the machine's own. */
    private var vWrites: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `covered` datamodel variable, the machine's own. */
    private var vCovered: Boolean = false
    /** W3C SCXML 5.2: the `probe` datamodel variable, the machine's own. */
    private var vProbe: RecordListParentSceSynthInvokeListRefusalEnum = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE
    /** W3C SCXML 5.2: the `spoiledFor` datamodel variable, the machine's own. */
    private var vSpoiledFor: RecordListParentSceSynthInvokeListRefusalEnum = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE
    /** W3C SCXML 5.2: the `namedId` datamodel variable, the machine's own. */
    private var vNamedId: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `namedField` datamodel variable, the machine's own. */
    private var vNamedField: UShort = 0.toUShort()
    /** W3C SCXML 5.2: the `namedKind` datamodel variable, the machine's own. */
    private var vNamedKind: RecordListParentSceSynthInvokeListCommandEnum = RecordListParentSceSynthInvokeListCommandEnum.EDIT
    /** W3C SCXML 5.2: the `taking` datamodel variable, the machine's own. */
    private var vTaking: List<RecordListParentSceSynthInvokeListOpRecord> = emptyList()
    /** W3C SCXML 5.2: the `took` datamodel variable, the machine's own. */
    private var vTook: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `op` datamodel variable, the machine's own. */
    private var vOp: RecordListParentSceSynthInvokeListOpRecord = RecordListParentSceSynthInvokeListOpRecord(kind = 0.toUByte(), id = 0.toUInt(), field = 0.toUShort(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `restoring` datamodel variable, the machine's own. */
    private var vRestoring: Boolean = false
    /** W3C SCXML 5.2: the `tentative` datamodel variable, the machine's own. */
    private var vTentative: Boolean = false
    /** W3C SCXML 5.2: the `closingCommand` datamodel variable, the machine's own. */
    private var vClosingCommand: ULong = 0.toULong()
    /** W3C SCXML 5.2: the `offered` datamodel variable, the machine's own. */
    private var vOffered: List<RecordListParentSceSynthInvokeListOpRecord> = emptyList()
    /** W3C SCXML 5.2: the `offeredAt` datamodel variable, the machine's own. */
    private var vOfferedAt: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `refusedFor` datamodel variable, the machine's own. */
    private var vRefusedFor: RecordListParentSceSynthInvokeListRefusalEnum = RecordListParentSceSynthInvokeListRefusalEnum.PEER_REFUSED
    /** W3C SCXML 5.2: the `closedOps` datamodel variable, the machine's own. */
    private var vClosedOps: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `closedFor` datamodel variable, the machine's own. */
    private var vClosedFor: RecordListParentSceSynthInvokeListRefusalEnum? = null
    /** W3C SCXML 5.2: the `closedAt` datamodel variable, the machine's own. */
    private var vClosedAt: RecordListParentSceSynthInvokeListHlcRecord = RecordListParentSceSynthInvokeListHlcRecord(wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `provisional` datamodel variable, the machine's own. */
    private var vProvisional: List<RecordListParentSceSynthInvokeListProvisionalRecord> = emptyList()
    /** W3C SCXML 5.2: the `provisionalKept` datamodel variable, the machine's own. */
    private var vProvisionalKept: List<RecordListParentSceSynthInvokeListProvisionalRecord> = emptyList()
    /** W3C SCXML 5.2: the `standing` datamodel variable, the machine's own. */
    private var vStanding: RecordListParentSceSynthInvokeListProvisionalRecord = RecordListParentSceSynthInvokeListProvisionalRecord(wallTime = 0, counter = 0.toUInt(), command = 0.toULong(), id = 0.toUInt(), field = 0.toUShort(), kind = RecordListParentSceSynthInvokeListCommandEnum.EDIT)
    /** W3C SCXML 5.2: the `provisionalFound` datamodel variable, the machine's own. */
    private var vProvisionalFound: Boolean = false
    /** W3C SCXML 5.2: the `withdrawFrom` datamodel variable, the machine's own. */
    private var vWithdrawFrom: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `undo` datamodel variable, the machine's own. */
    private var vUndo: List<RecordListParentSceSynthInvokeListUndoRecord> = emptyList()
    /** W3C SCXML 5.2: the `undoKept` datamodel variable, the machine's own. */
    private var vUndoKept: List<RecordListParentSceSynthInvokeListUndoRecord> = emptyList()
    /** W3C SCXML 5.2: the `undoing` datamodel variable, the machine's own. */
    private var vUndoing: RecordListParentSceSynthInvokeListUndoRecord = RecordListParentSceSynthInvokeListUndoRecord(atWallTime = 0, atCounter = 0.toUInt(), id = 0.toULong(), field = 0.toULong(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var node: ULong? = null
        var journal: Boolean? = null
        var shareId: UInt? = null
        var shareField: UShort? = null
        var wasLive: Boolean? = null
        var present: Boolean? = null
        var refusals: UInt? = null
        var roomValues: UInt? = null
        var full: UInt? = null
        var denied: UInt? = null
        var forbidden: Boolean? = null
        var authorized: Boolean? = null
        var announce: Boolean? = null
        var announced: UInt? = null
        var listed: Boolean? = null
        var ordinal: UInt? = null
        var holding: Boolean? = null
        var closingNow: Long? = null
        var spoiled: Boolean? = null
        var probing: Boolean? = null
        var gone: UInt? = null
        var writes: UInt? = null
        var covered: Boolean? = null
        var namedId: UInt? = null
        var namedField: UShort? = null
        var took: UInt? = null
        var restoring: Boolean? = null
        var tentative: Boolean? = null
        var closingCommand: ULong? = null
        var closedOps: UInt? = null
        var provisionalFound: Boolean? = null
        var withdrawFrom: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.journal?.let { vJournal = it }
        params.shareId?.let { vShareId = it }
        params.shareField?.let { vShareField = it }
        params.wasLive?.let { vWasLive = it }
        params.present?.let { vPresent = it }
        params.refusals?.let { vRefusals = it }
        params.roomValues?.let { vRoomValues = it }
        params.full?.let { vFull = it }
        params.denied?.let { vDenied = it }
        params.forbidden?.let { vForbidden = it }
        params.authorized?.let { vAuthorized = it }
        params.announce?.let { vAnnounce = it }
        params.announced?.let { vAnnounced = it }
        params.listed?.let { vListed = it }
        params.ordinal?.let { vOrdinal = it }
        params.holding?.let { vHolding = it }
        params.closingNow?.let { vClosingNow = it }
        params.spoiled?.let { vSpoiled = it }
        params.probing?.let { vProbing = it }
        params.gone?.let { vGone = it }
        params.writes?.let { vWrites = it }
        params.covered?.let { vCovered = it }
        params.namedId?.let { vNamedId = it }
        params.namedField?.let { vNamedField = it }
        params.took?.let { vTook = it }
        params.restoring?.let { vRestoring = it }
        params.tentative?.let { vTentative = it }
        params.closingCommand?.let { vClosingCommand = it }
        params.closedOps?.let { vClosedOps = it }
        params.provisionalFound?.let { vProvisionalFound = it }
        params.withdrawFrom?.let { vWithdrawFrom = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val clock: RecordListParentSceSynthInvokeListHlcRecord,
        val adds: List<RecordListParentSceSynthInvokeListEntryRecord>,
        val values: List<RecordListParentSceSynthInvokeListValueRecord>,
        val refusals: UInt,
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
        clock = vClock,
        adds = vAdds,
        values = vValues,
        refusals = vRefusals,
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
        first = "54ae020ce3da36cbee2cb77e732b4a4d1632678b6aeefc2b1779783ccc1103af",
        revised = "4bdb42f302b58cee24b7a9a585b8c61922676c367d9f9d9e2a85a4479016f03f",
        format2First = "c6779279e566c8bceb786919ac42a1b6e825b9906726e5ea2e9970ab8dd06ac2",
        format2Revised = "6de25742cdda2d2175b8af7453fd27733dbd4b9a54c816dcdeaa0a1c5375c98d",
        records = listOf(
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
                    RecordHolder("offeredAt", false),
                    RecordHolder("closedAt", false),
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
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("adds", true),
                    RecordHolder("one", true),
                    RecordHolder("entry", false),
                ),
            ),
            RecordLineage(
                event = "record_list.member",
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
                    RecordHolder("offered", true),
                ),
            ),
            RecordLineage(
                event = "record_list.provisional",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("provisional", true),
                    RecordHolder("provisionalKept", true),
                    RecordHolder("standing", false),
                ),
            ),
            RecordLineage(
                event = "record_list.staged",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("staged", true),
                    RecordHolder("staging", false),
                ),
            ),
            RecordLineage(
                event = "record_list.undo",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("undo", true),
                    RecordHolder("undoKept", true),
                    RecordHolder("undoing", false),
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
                    EnumHolder("namedKind", null, false),
                    EnumHolder("provisional", "kind", true),
                    EnumHolder("provisionalKept", "kind", true),
                    EnumHolder("standing", "kind", false),
                ),
            ),
            EnumLineage(
                key = "enum:sce:std/shared/record_refusal.scxml",
                revision = 2,
                revisions = listOf("3ac588a3c4a21b24f14d28b1fcef233ad4398105757a2598b93d4d89f93275e8"),
                added = listOf(
                    AddedVariant("peerRefused", 2),
                ),
                holders = listOf(
                    EnumHolder("probe", null, false),
                    EnumHolder("spoiledFor", null, false),
                    EnumHolder("refusedFor", null, false),
                    EnumHolder("closedFor", null, false),
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "node" to SavedValues.of(vNode)
        variables += "journal" to SavedValues.of(vJournal)
        variables += "clock" to vClock.toSaved()
        variables += "stamp" to vStamp.toSaved()
        variables += "dot" to vDot.toSaved()
        variables += "adds" to SavedValues.list(vAdds) { it.toSaved() }
        variables += "one" to SavedValues.list(vOne) { it.toSaved() }
        variables += "entry" to vEntry.toSaved()
        variables += "values" to SavedValues.list(vValues) { it.toSaved() }
        variables += "held" to vHeld.toSaved()
        variables += "shareId" to SavedValues.of(vShareId)
        variables += "shareField" to SavedValues.of(vShareField)
        variables += "wasLive" to SavedValues.of(vWasLive)
        variables += "present" to SavedValues.of(vPresent)
        variables += "refusals" to SavedValues.of(vRefusals)
        variables += "roomValues" to SavedValues.of(vRoomValues)
        variables += "full" to SavedValues.of(vFull)
        variables += "roster" to SavedValues.list(vRoster) { it.toSaved() }
        variables += "denied" to SavedValues.of(vDenied)
        variables += "forbidden" to SavedValues.of(vForbidden)
        variables += "authorized" to SavedValues.of(vAuthorized)
        variables += "announce" to SavedValues.of(vAnnounce)
        variables += "announced" to SavedValues.of(vAnnounced)
        variables += "moved" to SavedValues.list(vMoved) { SavedValues.of(it) }
        variables += "listed" to SavedValues.of(vListed)
        variables += "live" to SavedValues.list(vLive) { it.toSaved() }
        variables += "ordinal" to SavedValues.of(vOrdinal)
        variables += "empty" to vEmpty.toSaved()
        variables += "staged" to SavedValues.list(vStaged) { it.toSaved() }
        variables += "staging" to vStaging.toSaved()
        variables += "holding" to SavedValues.of(vHolding)
        variables += "closingNow" to SavedValues.of(vClosingNow)
        variables += "spoiled" to SavedValues.of(vSpoiled)
        variables += "probing" to SavedValues.of(vProbing)
        variables += "gone" to SavedValues.of(vGone)
        variables += "writes" to SavedValues.of(vWrites)
        variables += "covered" to SavedValues.of(vCovered)
        variables += "probe" to vProbe.toSaved()
        variables += "spoiledFor" to vSpoiledFor.toSaved()
        variables += "namedId" to SavedValues.of(vNamedId)
        variables += "namedField" to SavedValues.of(vNamedField)
        variables += "namedKind" to vNamedKind.toSaved()
        variables += "taking" to SavedValues.list(vTaking) { it.toSaved() }
        variables += "took" to SavedValues.of(vTook)
        variables += "op" to vOp.toSaved()
        variables += "restoring" to SavedValues.of(vRestoring)
        variables += "tentative" to SavedValues.of(vTentative)
        variables += "closingCommand" to SavedValues.of(vClosingCommand)
        variables += "offered" to SavedValues.list(vOffered) { it.toSaved() }
        variables += "offeredAt" to vOfferedAt.toSaved()
        variables += "refusedFor" to vRefusedFor.toSaved()
        variables += "closedOps" to SavedValues.of(vClosedOps)
        variables += "closedFor" to SavedValues.optional(vClosedFor) { it.toSaved() }
        variables += "closedAt" to vClosedAt.toSaved()
        variables += "provisional" to SavedValues.list(vProvisional) { it.toSaved() }
        variables += "provisionalKept" to SavedValues.list(vProvisionalKept) { it.toSaved() }
        variables += "standing" to vStanding.toSaved()
        variables += "provisionalFound" to SavedValues.of(vProvisionalFound)
        variables += "withdrawFrom" to SavedValues.of(vWithdrawFrom)
        variables += "undo" to SavedValues.list(vUndo) { it.toSaved() }
        variables += "undoKept" to SavedValues.list(vUndoKept) { it.toSaved() }
        variables += "undoing" to vUndoing.toSaved()
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
        val saved2 = SavedValues.bool(savedGrown.variable("journal"), "journal")
        val saved3 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("clock"), "clock")
        val saved4 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("stamp"), "stamp")
        val saved5 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("dot"), "dot")
        val saved6 = SavedValues.list(savedGrown.variable("adds"), "adds", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved7 = SavedValues.list(savedGrown.variable("one"), "one", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        val saved8 = RecordListParentSceSynthInvokeListEntryRecord.fromSaved(savedGrown.variable("entry"), "entry")
        val saved9 = SavedValues.list(savedGrown.variable("values"), "values", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        val saved10 = RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("held"), "held")
        val saved11 = SavedValues.uint32(savedGrown.variable("shareId"), "shareId")
        val saved12 = SavedValues.uint16(savedGrown.variable("shareField"), "shareField")
        val saved13 = SavedValues.bool(savedGrown.variable("wasLive"), "wasLive")
        val saved14 = SavedValues.bool(savedGrown.variable("present"), "present")
        val saved15 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        val saved16 = SavedValues.uint32(savedGrown.variable("roomValues"), "roomValues")
        val saved17 = SavedValues.uint32(savedGrown.variable("full"), "full")
        val saved18 = SavedValues.list(savedGrown.variable("roster"), "roster", 64) { e, w -> RecordListParentSceSynthInvokeListMemberRecord.fromSaved(e, w) }
        val saved19 = SavedValues.uint32(savedGrown.variable("denied"), "denied")
        val saved20 = SavedValues.bool(savedGrown.variable("forbidden"), "forbidden")
        val saved21 = SavedValues.bool(savedGrown.variable("authorized"), "authorized")
        val saved22 = SavedValues.bool(savedGrown.variable("announce"), "announce")
        val saved23 = SavedValues.uint32(savedGrown.variable("announced"), "announced")
        val saved24 = SavedValues.list(savedGrown.variable("moved"), "moved", 64) { e, w -> SavedValues.uint64(e, w) }
        val saved25 = SavedValues.bool(savedGrown.variable("listed"), "listed")
        val saved26 = SavedValues.list(savedGrown.variable("live"), "live", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        val saved27 = SavedValues.uint32(savedGrown.variable("ordinal"), "ordinal")
        val saved28 = RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("empty"), "empty")
        val saved29 = SavedValues.list(savedGrown.variable("staged"), "staged", 32) { e, w -> RecordListParentSceSynthInvokeListStagedRecord.fromSaved(e, w) }
        val saved30 = RecordListParentSceSynthInvokeListStagedRecord.fromSaved(savedGrown.variable("staging"), "staging")
        val saved31 = SavedValues.bool(savedGrown.variable("holding"), "holding")
        val saved32 = SavedValues.int64(savedGrown.variable("closingNow"), "closingNow")
        val saved33 = SavedValues.bool(savedGrown.variable("spoiled"), "spoiled")
        val saved34 = SavedValues.bool(savedGrown.variable("probing"), "probing")
        val saved35 = SavedValues.uint32(savedGrown.variable("gone"), "gone")
        val saved36 = SavedValues.uint32(savedGrown.variable("writes"), "writes")
        val saved37 = SavedValues.bool(savedGrown.variable("covered"), "covered")
        val saved38 = RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("probe"), "probe")
        val saved39 = RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("spoiledFor"), "spoiledFor")
        val saved40 = SavedValues.uint32(savedGrown.variable("namedId"), "namedId")
        val saved41 = SavedValues.uint16(savedGrown.variable("namedField"), "namedField")
        val saved42 = RecordListParentSceSynthInvokeListCommandEnum.fromSaved(savedGrown.variable("namedKind"), "namedKind")
        val saved43 = SavedValues.list(savedGrown.variable("taking"), "taking", 64) { e, w -> RecordListParentSceSynthInvokeListOpRecord.fromSaved(e, w) }
        val saved44 = SavedValues.uint32(savedGrown.variable("took"), "took")
        val saved45 = RecordListParentSceSynthInvokeListOpRecord.fromSaved(savedGrown.variable("op"), "op")
        val saved46 = SavedValues.bool(savedGrown.variable("restoring"), "restoring")
        val saved47 = SavedValues.bool(savedGrown.variable("tentative"), "tentative")
        val saved48 = SavedValues.uint64(savedGrown.variable("closingCommand"), "closingCommand")
        val saved49 = SavedValues.list(savedGrown.variable("offered"), "offered", 64) { e, w -> RecordListParentSceSynthInvokeListOpRecord.fromSaved(e, w) }
        val saved50 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("offeredAt"), "offeredAt")
        val saved51 = RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("refusedFor"), "refusedFor")
        val saved52 = SavedValues.uint32(savedGrown.variable("closedOps"), "closedOps")
        val saved53 = SavedValues.optional(savedGrown.variable("closedFor"), "closedFor") { e, w -> RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(e, w) }
        val saved54 = RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("closedAt"), "closedAt")
        val saved55 = SavedValues.list(savedGrown.variable("provisional"), "provisional", 8) { e, w -> RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(e, w) }
        val saved56 = SavedValues.list(savedGrown.variable("provisionalKept"), "provisionalKept", 8) { e, w -> RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(e, w) }
        val saved57 = RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(savedGrown.variable("standing"), "standing")
        val saved58 = SavedValues.bool(savedGrown.variable("provisionalFound"), "provisionalFound")
        val saved59 = SavedValues.uint32(savedGrown.variable("withdrawFrom"), "withdrawFrom")
        val saved60 = SavedValues.list(savedGrown.variable("undo"), "undo", 512) { e, w -> RecordListParentSceSynthInvokeListUndoRecord.fromSaved(e, w) }
        val saved61 = SavedValues.list(savedGrown.variable("undoKept"), "undoKept", 512) { e, w -> RecordListParentSceSynthInvokeListUndoRecord.fromSaved(e, w) }
        val saved62 = RecordListParentSceSynthInvokeListUndoRecord.fromSaved(savedGrown.variable("undoing"), "undoing")
        return {
            vNode = saved1
            vJournal = saved2
            vClock = saved3
            vStamp = saved4
            vDot = saved5
            vAdds = saved6
            vOne = saved7
            vEntry = saved8
            vValues = saved9
            vHeld = saved10
            vShareId = saved11
            vShareField = saved12
            vWasLive = saved13
            vPresent = saved14
            vRefusals = saved15
            vRoomValues = saved16
            vFull = saved17
            vRoster = saved18
            vDenied = saved19
            vForbidden = saved20
            vAuthorized = saved21
            vAnnounce = saved22
            vAnnounced = saved23
            vMoved = saved24
            vListed = saved25
            vLive = saved26
            vOrdinal = saved27
            vEmpty = saved28
            vStaged = saved29
            vStaging = saved30
            vHolding = saved31
            vClosingNow = saved32
            vSpoiled = saved33
            vProbing = saved34
            vGone = saved35
            vWrites = saved36
            vCovered = saved37
            vProbe = saved38
            vSpoiledFor = saved39
            vNamedId = saved40
            vNamedField = saved41
            vNamedKind = saved42
            vTaking = saved43
            vTook = saved44
            vOp = saved45
            vRestoring = saved46
            vTentative = saved47
            vClosingCommand = saved48
            vOffered = saved49
            vOfferedAt = saved50
            vRefusedFor = saved51
            vClosedOps = saved52
            vClosedFor = saved53
            vClosedAt = saved54
            vProvisional = saved55
            vProvisionalKept = saved56
            vStanding = saved57
            vProvisionalFound = saved58
            vWithdrawFrom = saved59
            vUndo = saved60
            vUndoKept = saved61
            vUndoing = saved62
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
        SavedValues.bool(savedGrown.variable("journal"), "journal")
        RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("clock"), "clock")
        RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("stamp"), "stamp")
        RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("dot"), "dot")
        SavedValues.list(savedGrown.variable("adds"), "adds", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("one"), "one", 1024) { e, w -> RecordListParentSceSynthInvokeListEntryRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListEntryRecord.fromSaved(savedGrown.variable("entry"), "entry")
        SavedValues.list(savedGrown.variable("values"), "values", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("held"), "held")
        SavedValues.uint32(savedGrown.variable("shareId"), "shareId")
        SavedValues.uint16(savedGrown.variable("shareField"), "shareField")
        SavedValues.bool(savedGrown.variable("wasLive"), "wasLive")
        SavedValues.bool(savedGrown.variable("present"), "present")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        SavedValues.uint32(savedGrown.variable("roomValues"), "roomValues")
        SavedValues.uint32(savedGrown.variable("full"), "full")
        SavedValues.list(savedGrown.variable("roster"), "roster", 64) { e, w -> RecordListParentSceSynthInvokeListMemberRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("denied"), "denied")
        SavedValues.bool(savedGrown.variable("forbidden"), "forbidden")
        SavedValues.bool(savedGrown.variable("authorized"), "authorized")
        SavedValues.bool(savedGrown.variable("announce"), "announce")
        SavedValues.uint32(savedGrown.variable("announced"), "announced")
        SavedValues.list(savedGrown.variable("moved"), "moved", 64) { e, w -> SavedValues.uint64(e, w) }
        SavedValues.bool(savedGrown.variable("listed"), "listed")
        SavedValues.list(savedGrown.variable("live"), "live", 512) { e, w -> RecordListParentSceSynthInvokeListValueRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("ordinal"), "ordinal")
        RecordListParentSceSynthInvokeListValueRecord.fromSaved(savedGrown.variable("empty"), "empty")
        SavedValues.list(savedGrown.variable("staged"), "staged", 32) { e, w -> RecordListParentSceSynthInvokeListStagedRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListStagedRecord.fromSaved(savedGrown.variable("staging"), "staging")
        SavedValues.bool(savedGrown.variable("holding"), "holding")
        SavedValues.int64(savedGrown.variable("closingNow"), "closingNow")
        SavedValues.bool(savedGrown.variable("spoiled"), "spoiled")
        SavedValues.bool(savedGrown.variable("probing"), "probing")
        SavedValues.uint32(savedGrown.variable("gone"), "gone")
        SavedValues.uint32(savedGrown.variable("writes"), "writes")
        SavedValues.bool(savedGrown.variable("covered"), "covered")
        RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("probe"), "probe")
        RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("spoiledFor"), "spoiledFor")
        SavedValues.uint32(savedGrown.variable("namedId"), "namedId")
        SavedValues.uint16(savedGrown.variable("namedField"), "namedField")
        RecordListParentSceSynthInvokeListCommandEnum.fromSaved(savedGrown.variable("namedKind"), "namedKind")
        SavedValues.list(savedGrown.variable("taking"), "taking", 64) { e, w -> RecordListParentSceSynthInvokeListOpRecord.fromSaved(e, w) }
        SavedValues.uint32(savedGrown.variable("took"), "took")
        RecordListParentSceSynthInvokeListOpRecord.fromSaved(savedGrown.variable("op"), "op")
        SavedValues.bool(savedGrown.variable("restoring"), "restoring")
        SavedValues.bool(savedGrown.variable("tentative"), "tentative")
        SavedValues.uint64(savedGrown.variable("closingCommand"), "closingCommand")
        SavedValues.list(savedGrown.variable("offered"), "offered", 64) { e, w -> RecordListParentSceSynthInvokeListOpRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("offeredAt"), "offeredAt")
        RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(savedGrown.variable("refusedFor"), "refusedFor")
        SavedValues.uint32(savedGrown.variable("closedOps"), "closedOps")
        SavedValues.optional(savedGrown.variable("closedFor"), "closedFor") { e, w -> RecordListParentSceSynthInvokeListRefusalEnum.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListHlcRecord.fromSaved(savedGrown.variable("closedAt"), "closedAt")
        SavedValues.list(savedGrown.variable("provisional"), "provisional", 8) { e, w -> RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("provisionalKept"), "provisionalKept", 8) { e, w -> RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListProvisionalRecord.fromSaved(savedGrown.variable("standing"), "standing")
        SavedValues.bool(savedGrown.variable("provisionalFound"), "provisionalFound")
        SavedValues.uint32(savedGrown.variable("withdrawFrom"), "withdrawFrom")
        SavedValues.list(savedGrown.variable("undo"), "undo", 512) { e, w -> RecordListParentSceSynthInvokeListUndoRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("undoKept"), "undoKept", 512) { e, w -> RecordListParentSceSynthInvokeListUndoRecord.fromSaved(e, w) }
        RecordListParentSceSynthInvokeListUndoRecord.fromSaved(savedGrown.variable("undoing"), "undoing")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingRecordListAnnouncePayload: RecordListParentSceSynthInvokeListRecordListAnnouncePayload? = null
    private var pendingRecordListChangePayload: RecordListParentSceSynthInvokeListRecordListChangePayload? = null
    private var pendingRecordListClearPayload: RecordListParentSceSynthInvokeListRecordListClearPayload? = null
    private var pendingRecordListConfirmPayload: RecordListParentSceSynthInvokeListRecordListConfirmPayload? = null
    private var pendingRecordListEditPayload: RecordListParentSceSynthInvokeListRecordListEditPayload? = null
    private var pendingRecordListJoinPayload: RecordListParentSceSynthInvokeListRecordListJoinPayload? = null
    private var pendingRecordListRaisePayload: RecordListParentSceSynthInvokeListRecordListRaisePayload? = null
    private var pendingRecordListRecallPayload: RecordListParentSceSynthInvokeListRecordListRecallPayload? = null
    private var pendingRecordListRestorePayload: RecordListParentSceSynthInvokeListRecordListRestorePayload? = null
    private var pendingRecordListRetirePayload: RecordListParentSceSynthInvokeListRecordListRetirePayload? = null
    private var pendingRecordListRolePayload: RecordListParentSceSynthInvokeListRecordListRolePayload? = null
    private var pendingRecordListRoomPayload: RecordListParentSceSynthInvokeListRecordListRoomPayload? = null
    private var pendingRecordListVerdictPayload: RecordListParentSceSynthInvokeListRecordListVerdictPayload? = null
    private var pendingRecordListWithdrawPayload: RecordListParentSceSynthInvokeListRecordListWithdrawPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, on which the engine refuses the delivery whole and
    // takes an error.execution in its place (docs/adr/0086, section 4.2). Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: RecordListParentSceSynthInvokeListEvent, metadata: EventMetadata) {
        pendingRecordListAnnouncePayload = null
        pendingRecordListChangePayload = null
        pendingRecordListClearPayload = null
        pendingRecordListConfirmPayload = null
        pendingRecordListEditPayload = null
        pendingRecordListJoinPayload = null
        pendingRecordListRaisePayload = null
        pendingRecordListRecallPayload = null
        pendingRecordListRestorePayload = null
        pendingRecordListRetirePayload = null
        pendingRecordListRolePayload = null
        pendingRecordListRoomPayload = null
        pendingRecordListVerdictPayload = null
        pendingRecordListWithdrawPayload = null
        when (val tp = metadata.typedPayload) {
            is RecordListParentSceSynthInvokeListRecordListAnnouncePayload -> pendingRecordListAnnouncePayload = tp
            is RecordListParentSceSynthInvokeListRecordListChangePayload -> pendingRecordListChangePayload = tp
            is RecordListParentSceSynthInvokeListRecordListClearPayload -> pendingRecordListClearPayload = tp
            is RecordListParentSceSynthInvokeListRecordListConfirmPayload -> pendingRecordListConfirmPayload = tp
            is RecordListParentSceSynthInvokeListRecordListEditPayload -> pendingRecordListEditPayload = tp
            is RecordListParentSceSynthInvokeListRecordListJoinPayload -> pendingRecordListJoinPayload = tp
            is RecordListParentSceSynthInvokeListRecordListRaisePayload -> pendingRecordListRaisePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRecallPayload -> pendingRecordListRecallPayload = tp
            is RecordListParentSceSynthInvokeListRecordListRestorePayload -> pendingRecordListRestorePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRetirePayload -> pendingRecordListRetirePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRolePayload -> pendingRecordListRolePayload = tp
            is RecordListParentSceSynthInvokeListRecordListRoomPayload -> pendingRecordListRoomPayload = tp
            is RecordListParentSceSynthInvokeListRecordListVerdictPayload -> pendingRecordListVerdictPayload = tp
            is RecordListParentSceSynthInvokeListRecordListWithdrawPayload -> pendingRecordListWithdrawPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Announce) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListAnnouncePayload = RecordListParentSceSynthInvokeListRecordListAnnouncePayload(fields.uint32("id"), fields.uint16("field"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Change) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListChangePayload = RecordListParentSceSynthInvokeListRecordListChangePayload(fields.field("by").let { it.uint64() }, fields.field("wallTime").let { it.int64() }, fields.field("counter").let { it.uint32() }, fields.field("ops").let { it.listWithin(1, 64) { it.record(null) { RecordListParentSceSynthInvokeListOpRecord(kind = it.field("kind").let { it.uint8() }, id = it.field("id").let { it.uint32() }, field = it.field("field").let { it.uint16() }, value = it.field("value").let { it.int64() }, text = it.field("text").let { it.string(96) }, wallTime = it.field("wallTime").let { it.int64() }, counter = it.field("counter").let { it.uint32() }, nodeId = it.field("nodeId").let { it.uint64() }) } } })
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Clear) {
                    val fields = EventPayload.decode(metadata.data, "{\"command\":\"0\"}")
                    pendingRecordListClearPayload = RecordListParentSceSynthInvokeListRecordListClearPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("now"), fields.boolean("more"), fields.uint64("command"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Confirm) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListConfirmPayload = RecordListParentSceSynthInvokeListRecordListConfirmPayload(fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("command"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Edit) {
                    val fields = EventPayload.decode(metadata.data, "{\"more\":false,\"command\":\"0\"}")
                    pendingRecordListEditPayload = RecordListParentSceSynthInvokeListRecordListEditPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text", 32), fields.int64("now"), fields.boolean("more"), fields.uint64("command"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Join) {
                    val fields = EventPayload.decode(metadata.data, "{\"journal\":true}")
                    pendingRecordListJoinPayload = RecordListParentSceSynthInvokeListRecordListJoinPayload(fields.uint64("node"), fields.boolean("journal"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Raise) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRaisePayload = RecordListParentSceSynthInvokeListRecordListRaisePayload(fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("command"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Recall) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRecallPayload = RecordListParentSceSynthInvokeListRecordListRecallPayload(fields.field("by").let { it.uint64() }, fields.field("wallTime").let { it.int64() }, fields.field("counter").let { it.uint32() }, fields.field("ops").let { it.listWithin(1, 64) { it.record(null) { RecordListParentSceSynthInvokeListOpRecord(kind = it.field("kind").let { it.uint8() }, id = it.field("id").let { it.uint32() }, field = it.field("field").let { it.uint16() }, value = it.field("value").let { it.int64() }, text = it.field("text").let { it.string(96) }, wallTime = it.field("wallTime").let { it.int64() }, counter = it.field("counter").let { it.uint32() }, nodeId = it.field("nodeId").let { it.uint64() }) } } }, fields.field("command").let { it.uint64() }, fields.field("provisional").let { it.boolean() })
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Restore) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRestorePayload = RecordListParentSceSynthInvokeListRecordListRestorePayload(fields.field("by").let { it.uint64() }, fields.field("wallTime").let { it.int64() }, fields.field("counter").let { it.uint32() }, fields.field("ops").let { it.listWithin(1, 64) { it.record(null) { RecordListParentSceSynthInvokeListOpRecord(kind = it.field("kind").let { it.uint8() }, id = it.field("id").let { it.uint32() }, field = it.field("field").let { it.uint16() }, value = it.field("value").let { it.int64() }, text = it.field("text").let { it.string(96) }, wallTime = it.field("wallTime").let { it.int64() }, counter = it.field("counter").let { it.uint32() }, nodeId = it.field("nodeId").let { it.uint64() }) } } })
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Retire) {
                    val fields = EventPayload.decode(metadata.data, "{\"command\":\"0\"}")
                    pendingRecordListRetirePayload = RecordListParentSceSynthInvokeListRecordListRetirePayload(fields.uint32("id"), fields.int64("now"), fields.boolean("more"), fields.uint64("command"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Role) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRolePayload = RecordListParentSceSynthInvokeListRecordListRolePayload(fields.uint64("node"), fields.uint8("role"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Room) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRoomPayload = RecordListParentSceSynthInvokeListRecordListRoomPayload(fields.uint32("values"), fields.uint32("history"), fields.uint32("digest"))
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Verdict) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListVerdictPayload = RecordListParentSceSynthInvokeListRecordListVerdictPayload(fields.int64("wallTime"), fields.uint32("counter"), fields.string("outcome").let { name -> RecordListParentSceSynthInvokeListOutcomeEnum.entries.firstOrNull { it.declaredName == name } ?: throw fields.field("outcome").unknownVariant(name, "Outcome") }, fields.optional("reason") { _ -> fields.string("reason").let { name -> RecordListParentSceSynthInvokeListRefusalEnum.entries.firstOrNull { it.declaredName == name } ?: throw fields.field("reason").unknownVariant(name, "Refusal") } })
                } else if (event == RecordListParentSceSynthInvokeListEvent.RecordList.Withdraw) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListWithdrawPayload = RecordListParentSceSynthInvokeListRecordListWithdrawPayload(fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("command"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
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
    // `record_list.change` — binds the event name and the payload field values in one call.
    fun raiseRecordListChange(by: ULong, wallTime: Long, counter: UInt, ops: List<RecordListParentSceSynthInvokeListOpRecord>) {
        val scePayload = RecordListParentSceSynthInvokeListRecordListChangePayload(by, wallTime, counter, ops)
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Change,
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
    // `record_list.clear` — binds the event name and the payload field values in one call.
    fun raiseRecordListClear(id: UInt, field: UShort, now: Long, more: Boolean, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Clear,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListClearPayload(id, field, now, more, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "now" to now, "more" to more, "command" to command))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.confirm` — binds the event name and the payload field values in one call.
    fun raiseRecordListConfirm(wallTime: Long, counter: UInt, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Confirm,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListConfirmPayload(wallTime, counter, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("wallTime" to wallTime, "counter" to counter, "command" to command))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.edit` — binds the event name and the payload field values in one call.
    fun raiseRecordListEdit(id: UInt, field: UShort, value: Long, text: String, now: Long, more: Boolean, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Edit,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListEditPayload(id, field, value, text, now, more, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "now" to now, "more" to more, "command" to command))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.join` — binds the event name and the payload field values in one call.
    fun raiseRecordListJoin(node: ULong, journal: Boolean) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Join,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListJoinPayload(node, journal),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "journal" to journal))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.raise` — binds the event name and the payload field values in one call.
    fun raiseRecordListRaise(wallTime: Long, counter: UInt, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Raise,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListRaisePayload(wallTime, counter, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("wallTime" to wallTime, "counter" to counter, "command" to command))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.recall` — binds the event name and the payload field values in one call.
    fun raiseRecordListRecall(by: ULong, wallTime: Long, counter: UInt, ops: List<RecordListParentSceSynthInvokeListOpRecord>, command: ULong, provisional: Boolean) {
        val scePayload = RecordListParentSceSynthInvokeListRecordListRecallPayload(by, wallTime, counter, ops, command, provisional)
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Recall,
            EventMetadata(
                type = "external",
                typedPayload = scePayload,
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("by" to com.sce.runtime.EventPayload.valueJson(scePayload.by), "wallTime" to com.sce.runtime.EventPayload.valueJson(scePayload.wallTime), "counter" to com.sce.runtime.EventPayload.valueJson(scePayload.counter), "ops" to com.sce.runtime.EventPayload.listJson(scePayload.ops) { com.sce.runtime.EventPayload.recordJson(mapOf<String, Any?>("kind" to com.sce.runtime.EventPayload.valueJson(it.kind), "id" to com.sce.runtime.EventPayload.valueJson(it.id), "field" to com.sce.runtime.EventPayload.valueJson(it.field), "value" to com.sce.runtime.EventPayload.valueJson(it.value), "text" to com.sce.runtime.EventPayload.valueJson(it.text), "wallTime" to com.sce.runtime.EventPayload.valueJson(it.wallTime), "counter" to com.sce.runtime.EventPayload.valueJson(it.counter), "nodeId" to com.sce.runtime.EventPayload.valueJson(it.nodeId))) }, "command" to com.sce.runtime.EventPayload.valueJson(scePayload.command), "provisional" to com.sce.runtime.EventPayload.valueJson(scePayload.provisional))).json
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.restore` — binds the event name and the payload field values in one call.
    fun raiseRecordListRestore(by: ULong, wallTime: Long, counter: UInt, ops: List<RecordListParentSceSynthInvokeListOpRecord>) {
        val scePayload = RecordListParentSceSynthInvokeListRecordListRestorePayload(by, wallTime, counter, ops)
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Restore,
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
    // `record_list.retire` — binds the event name and the payload field values in one call.
    fun raiseRecordListRetire(id: UInt, now: Long, more: Boolean, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Retire,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListRetirePayload(id, now, more, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "now" to now, "more" to more, "command" to command))
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
    // `record_list.verdict` — binds the event name and the payload field values in one call.
    fun raiseRecordListVerdict(wallTime: Long, counter: UInt, outcome: RecordListParentSceSynthInvokeListOutcomeEnum, reason: RecordListParentSceSynthInvokeListRefusalEnum?) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Verdict,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListVerdictPayload(wallTime, counter, outcome, reason),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("wallTime" to wallTime, "counter" to counter, "outcome" to outcome.declaredName, "reason" to reason?.declaredName))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.withdraw` — binds the event name and the payload field values in one call.
    fun raiseRecordListWithdraw(wallTime: Long, counter: UInt, command: ULong) {
        send(
            RecordListParentSceSynthInvokeListEvent.RecordList.Withdraw,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentSceSynthInvokeListRecordListWithdrawPayload(wallTime, counter, command),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("wallTime" to wallTime, "counter" to counter, "command" to command))
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
    }

    private object TransitionsPart0 {

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

        // W3C SCXML 3.13: open's transition 20, as the microstep reads it.
        val transitionOpenAt20 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            20,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 21, as the microstep reads it.
        val transitionOpenAt21 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            21,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 22, as the microstep reads it.
        val transitionOpenAt22 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            22,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 23, as the microstep reads it.
        val transitionOpenAt23 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            23,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 24, as the microstep reads it.
        val transitionOpenAt24 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            24,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 25, as the microstep reads it.
        val transitionOpenAt25 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            25,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 26, as the microstep reads it.
        val transitionOpenAt26 = EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>(
            RecordListParentSceSynthInvokeListState.Open,
            emptyList(),
            26,
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
        "record_list.announce" -> RecordListParentSceSynthInvokeListEvent.RecordList.Announce
        "record_list.change" -> RecordListParentSceSynthInvokeListEvent.RecordList.Change
        "record_list.clear" -> RecordListParentSceSynthInvokeListEvent.RecordList.Clear
        "record_list.close" -> RecordListParentSceSynthInvokeListEvent.RecordList.Close
        "record_list.closed" -> RecordListParentSceSynthInvokeListEvent.RecordList.Closed
        "record_list.conclude" -> RecordListParentSceSynthInvokeListEvent.RecordList.Conclude
        "record_list.confirm" -> RecordListParentSceSynthInvokeListEvent.RecordList.Confirm
        "record_list.confirmed" -> RecordListParentSceSynthInvokeListEvent.RecordList.Confirmed
        "record_list.deliver" -> RecordListParentSceSynthInvokeListEvent.RecordList.Deliver
        "record_list.edit" -> RecordListParentSceSynthInvokeListEvent.RecordList.Edit
        "record_list.join" -> RecordListParentSceSynthInvokeListEvent.RecordList.Join
        "record_list.made" -> RecordListParentSceSynthInvokeListEvent.RecordList.Made
        "record_list.offer" -> RecordListParentSceSynthInvokeListEvent.RecordList.Offer
        "record_list.pass" -> RecordListParentSceSynthInvokeListEvent.RecordList.Pass
        "record_list.pending" -> RecordListParentSceSynthInvokeListEvent.RecordList.Pending
        "record_list.raise" -> RecordListParentSceSynthInvokeListEvent.RecordList.Raise
        "record_list.recall" -> RecordListParentSceSynthInvokeListEvent.RecordList.Recall
        "record_list.refused" -> RecordListParentSceSynthInvokeListEvent.RecordList.Refused
        "record_list.register" -> RecordListParentSceSynthInvokeListEvent.RecordList.Register
        "record_list.restore" -> RecordListParentSceSynthInvokeListEvent.RecordList.Restore
        "record_list.retire" -> RecordListParentSceSynthInvokeListEvent.RecordList.Retire
        "record_list.role" -> RecordListParentSceSynthInvokeListEvent.RecordList.Role
        "record_list.room" -> RecordListParentSceSynthInvokeListEvent.RecordList.Room
        "record_list.take" -> RecordListParentSceSynthInvokeListEvent.RecordList.Take
        "record_list.tell" -> RecordListParentSceSynthInvokeListEvent.RecordList.Tell
        "record_list.unwind" -> RecordListParentSceSynthInvokeListEvent.RecordList.Unwind
        "record_list.value" -> RecordListParentSceSynthInvokeListEvent.RecordList.Value
        "record_list.verdict" -> RecordListParentSceSynthInvokeListEvent.RecordList.Verdict
        "record_list.withdraw" -> RecordListParentSceSynthInvokeListEvent.RecordList.Withdraw
        else -> null
    }


    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: RecordListParentSceSynthInvokeListEvent): String? = when (event) {
        is RecordListParentSceSynthInvokeListEvent.Error.Communication -> "error.communication"
        is RecordListParentSceSynthInvokeListEvent.Error.Execution -> "error.execution"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Announce -> "record_list.announce"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Change -> "record_list.change"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Clear -> "record_list.clear"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Close -> "record_list.close"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Closed -> "record_list.closed"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Conclude -> "record_list.conclude"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Confirm -> "record_list.confirm"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Confirmed -> "record_list.confirmed"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Deliver -> "record_list.deliver"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Edit -> "record_list.edit"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Join -> "record_list.join"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Made -> "record_list.made"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Offer -> "record_list.offer"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Pass -> "record_list.pass"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Pending -> "record_list.pending"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Raise -> "record_list.raise"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Recall -> "record_list.recall"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Refused -> "record_list.refused"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Register -> "record_list.register"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Restore -> "record_list.restore"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Retire -> "record_list.retire"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Role -> "record_list.role"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Room -> "record_list.room"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Take -> "record_list.take"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Tell -> "record_list.tell"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Unwind -> "record_list.unwind"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Value -> "record_list.value"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Verdict -> "record_list.verdict"
        is RecordListParentSceSynthInvokeListEvent.RecordList.Withdraw -> "record_list.withdraw"
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(state: RecordListParentSceSynthInvokeListState, event: RecordListParentSceSynthInvokeListEvent?): EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>? = when (state) {
        is RecordListParentSceSynthInvokeListState.Open -> firstEnabledTransitionOfOpen(state, event)
    }


    // open's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfOpen(state: RecordListParentSceSynthInvokeListState, event: RecordListParentSceSynthInvokeListEvent?): EnabledTransition<RecordListParentSceSynthInvokeListState, HistoryId>? = when {
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Join -> TransitionsPart0.transitionOpenAt0
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Room -> TransitionsPart0.transitionOpenAt1
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Role -> TransitionsPart0.transitionOpenAt2
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Edit && (vOffered).size.toUInt() > 0.toUInt() -> TransitionsPart0.transitionOpenAt3
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Clear && (vOffered).size.toUInt() > 0.toUInt() -> TransitionsPart0.transitionOpenAt4
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Retire && (vOffered).size.toUInt() > 0.toUInt() -> TransitionsPart0.transitionOpenAt5
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Edit -> TransitionsPart0.transitionOpenAt6
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Clear -> TransitionsPart0.transitionOpenAt7
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Retire -> TransitionsPart0.transitionOpenAt8
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Close -> TransitionsPart0.transitionOpenAt9
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Offer -> TransitionsPart0.transitionOpenAt10
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Verdict && pendingRecordListVerdictPayload != null && ((vOffered).size.toUInt() > 0.toUInt() && pendingRecordListVerdictPayload!!.outcome != RecordListParentSceSynthInvokeListOutcomeEnum.NONE && pendingRecordListVerdictPayload!!.wallTime == vOfferedAt.wallTime && pendingRecordListVerdictPayload!!.counter == vOfferedAt.counter) -> TransitionsPart0.transitionOpenAt11
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Verdict -> TransitionsPart0.transitionOpenAt12
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Conclude -> TransitionsPart0.transitionOpenAt13
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Change -> TransitionsPart0.transitionOpenAt14
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Recall -> TransitionsPart0.transitionOpenAt15
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Deliver -> TransitionsPart0.transitionOpenAt16
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Restore -> TransitionsPart0.transitionOpenAt17
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Raise -> TransitionsPart0.transitionOpenAt18
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Take -> TransitionsPart0.transitionOpenAt19
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Pass -> TransitionsPart0.transitionOpenAt20
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Confirm -> TransitionsPart0.transitionOpenAt21
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Withdraw -> TransitionsPart0.transitionOpenAt22
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Unwind -> TransitionsPart0.transitionOpenAt23
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Announce && vAnnounce -> TransitionsPart0.transitionOpenAt24
        event is RecordListParentSceSynthInvokeListEvent.RecordList.Tell && vAnnounce -> TransitionsPart0.transitionOpenAt25
        event is RecordListParentSceSynthInvokeListEvent.Error.Execution -> TransitionsPart0.transitionOpenAt26
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: record_list.scxml:210 :: _machine
    override fun onEntry(state: RecordListParentSceSynthInvokeListState, isDefaultEntry: Boolean) {
        when (state) {
            is RecordListParentSceSynthInvokeListState.Open -> onEntryOfOpen()
        }
    }


    private fun onEntryOfOpen() {
                // SCE-MAP: record_list.scxml:449 :: open :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: record_list.scxml:210 :: _machine
    override fun onExit(state: RecordListParentSceSynthInvokeListState) {
        when (state) {
            is RecordListParentSceSynthInvokeListState.Open -> onExitOfOpen()
        }
    }


    private fun onExitOfOpen() {
                // SCE-MAP: record_list.scxml:449 :: open :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: record_list.scxml:210 :: _machine
    override fun executeTransitionContent(source: RecordListParentSceSynthInvokeListState, transitionIndex: Int) {
        when (source) {
            is RecordListParentSceSynthInvokeListState.Open -> transitionContentOfOpen(transitionIndex)
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
            11 -> transitionContentOfOpenAt11()
            12 -> transitionContentOfOpenAt12()
            13 -> transitionContentOfOpenAt13()
            14 -> transitionContentOfOpenAt14()
            15 -> transitionContentOfOpenAt15()
            16 -> transitionContentOfOpenAt16()
            17 -> transitionContentOfOpenAt17()
            18 -> transitionContentOfOpenAt18()
            19 -> transitionContentOfOpenAt19()
            20 -> transitionContentOfOpenAt20()
            21 -> transitionContentOfOpenAt21()
            22 -> transitionContentOfOpenAt22()
            23 -> transitionContentOfOpenAt23()
            24 -> transitionContentOfOpenAt24()
            25 -> transitionContentOfOpenAt25()
            26 -> transitionContentOfOpenAt26()
            else -> {}
        }
    }


    private fun transitionContentOfOpenAt0() {
                // SCE-MAP: record_list.scxml:450 :: open :: _transition_0
                if (pendingRecordListJoinPayload == null) {
                    return
                }

            vNode = pendingRecordListJoinPayload!!.node

            vJournal = pendingRecordListJoinPayload!!.journal
    }

    private fun transitionContentOfOpenAt1() {
                // SCE-MAP: record_list.scxml:458 :: open :: _transition_1
                if (pendingRecordListRoomPayload == null) {
                    return
                }

            vRoomValues = pendingRecordListRoomPayload!!.values
    }

    private fun transitionContentOfOpenAt2() {
                // SCE-MAP: record_list.scxml:467 :: open :: _transition_2
                if (pendingRecordListRolePayload == null) {
                    return
                }

            if (try { vRoster = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_RecordRoster__v(vRoster, pendingRecordListRolePayload!!.node, pendingRecordListRolePayload!!.role)), 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='roster'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt3() {
                // SCE-MAP: record_list.scxml:476 :: open :: _transition_3

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt4() {
                // SCE-MAP: record_list.scxml:479 :: open :: _transition_4

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt5() {
                // SCE-MAP: record_list.scxml:482 :: open :: _transition_5

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt6() {
                // SCE-MAP: record_list.scxml:496 :: open :: _transition_6
                if (pendingRecordListEditPayload == null) {
                    return
                }

            vClosingCommand = pendingRecordListEditPayload!!.command

            vHolding = pendingRecordListEditPayload!!.more


            if (!vSpoiled) {

            vNamedId = pendingRecordListEditPayload!!.id

            vNamedField = pendingRecordListEditPayload!!.field

            vNamedKind = RecordListParentSceSynthInvokeListCommandEnum.EDIT

            vProbing = true

            vStaging = vStaging.copy(kind = 0.toUByte())

            vStaging = vStaging.copy(id = pendingRecordListEditPayload!!.id)

            vStaging = vStaging.copy(field = pendingRecordListEditPayload!!.field)

            vStaging = vStaging.copy(value = pendingRecordListEditPayload!!.value)

            if (try { vStaging = vStaging.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListEditPayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='staging.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vProbe = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE

            if (if (vStaged.size < 32) { vStaged = vStaged + (vStaging); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='staged'>: the list already holds its capacity of 32"); true }) {
                return
            }

            vProbing = false
            }


            if (!vHolding) {

            vClosingNow = pendingRecordListEditPayload!!.now


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Close, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
    }

    private fun transitionContentOfOpenAt7() {
                // SCE-MAP: record_list.scxml:522 :: open :: _transition_7
                if (pendingRecordListClearPayload == null) {
                    return
                }

            vClosingCommand = pendingRecordListClearPayload!!.command

            vHolding = pendingRecordListClearPayload!!.more


            if (!vSpoiled) {

            vNamedId = pendingRecordListClearPayload!!.id

            vNamedField = pendingRecordListClearPayload!!.field

            vNamedKind = RecordListParentSceSynthInvokeListCommandEnum.CLEAR

            vProbing = true

            vProbe = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE

            vStaging = vStaging.copy(kind = 1.toUByte())

            vStaging = vStaging.copy(id = pendingRecordListClearPayload!!.id)

            vStaging = vStaging.copy(field = pendingRecordListClearPayload!!.field)

            vStaging = vStaging.copy(value = 0)

            if (try { vStaging = vStaging.copy(text = com.sce.forge.runtime.SceChecked.bounded("", 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='staging.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (if (vStaged.size < 32) { vStaged = vStaged + (vStaging); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='staged'>: the list already holds its capacity of 32"); true }) {
                return
            }

            vProbing = false
            }


            if (!vHolding) {

            vClosingNow = pendingRecordListClearPayload!!.now


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Close, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
    }

    private fun transitionContentOfOpenAt8() {
                // SCE-MAP: record_list.scxml:548 :: open :: _transition_8
                if (pendingRecordListRetirePayload == null) {
                    return
                }

            vClosingCommand = pendingRecordListRetirePayload!!.command

            vHolding = pendingRecordListRetirePayload!!.more


            if (!vSpoiled) {

            vNamedId = pendingRecordListRetirePayload!!.id

            vNamedField = 0.toUShort()

            vNamedKind = RecordListParentSceSynthInvokeListCommandEnum.RETIRE

            vProbing = true

            vProbe = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE

            vStaging = vStaging.copy(kind = 2.toUByte())

            vStaging = vStaging.copy(id = pendingRecordListRetirePayload!!.id)

            vStaging = vStaging.copy(field = 0.toUShort())

            vStaging = vStaging.copy(value = 0)

            if (try { vStaging = vStaging.copy(text = com.sce.forge.runtime.SceChecked.bounded("", 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='staging.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (if (vStaged.size < 32) { vStaged = vStaged + (vStaging); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='staged'>: the list already holds its capacity of 32"); true }) {
                return
            }

            vProbing = false
            }


            if (!vHolding) {

            vClosingNow = pendingRecordListRetirePayload!!.now


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Close, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
    }

    private fun transitionContentOfOpenAt9() {
                // SCE-MAP: record_list.scxml:598 :: open :: _transition_9


            if (!vSpoiled && (vRoster).size.toUInt() > 0.toUInt()) {

            vForbidden = true


            for (m in vRoster) {


            if (m.node == vNode && m.role >= 2.toUByte()) {

            vForbidden = false
            }
            }


            if (vForbidden) {

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vSpoiled = true

            vSpoiledFor = RecordListParentSceSynthInvokeListRefusalEnum.NOT_WRITER
            }
            }


            if (!vSpoiled) {


            for ((sce_position_of_earlierAt, earlier) in vStaged.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val earlierAt = sce_position_of_earlierAt.toUInt()


            for ((sce_position_of_laterAt, later) in vStaged.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val laterAt = sce_position_of_laterAt.toUInt()


            if (!vSpoiled && laterAt > earlierAt && earlier.kind != 2.toUByte() && later.kind != 2.toUByte() && earlier.id == later.id && earlier.field == later.field) {

            vSpoiled = true

            vSpoiledFor = RecordListParentSceSynthInvokeListRefusalEnum.REGISTER_NAMED_TWICE

            vNamedId = later.id

            vNamedField = later.field


            if (later.kind == 1.toUByte()) {

            vNamedKind = RecordListParentSceSynthInvokeListCommandEnum.CLEAR
            } else {

            vNamedKind = RecordListParentSceSynthInvokeListCommandEnum.EDIT
            }
            }
            }
            }
            }

            vGone = 0.toUInt()

            vWrites = 0.toUInt()


            if (!vSpoiled) {


            for (a in vAdds) {

            vCovered = false


            for (s in vStaged) {


            var ifCondFailed14 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(a.element, 65536.toULong()) == s.id.toULong() && (s.kind == 2.toUByte() || com.sce.forge.runtime.SceChecked.rem(a.element, 65536.toULong()) == s.field.toULong()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='a.element / 65536 === s.id && (s.kind === 2 || a.element % 65536 === s.field)'>: an integer operation overflowed or failed"); ifCondFailed14 = true; false })) {

            vCovered = true
            }
            if (ifCondFailed14) {
                return
            }
            }


            if (vCovered) {

            if (try { vGone = com.sce.forge.runtime.SceChecked.add(vGone, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='gone'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }


            for (s in vStaged) {


            if (s.kind == 0.toUByte()) {

            if (try { vWrites = com.sce.forge.runtime.SceChecked.add(vWrites, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='writes'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            }


            var ifCondFailed17 = false
            if ((try { !vSpoiled && com.sce.forge.runtime.SceChecked.add(vGone, vWrites) > 0.toUInt() && vRoomValues > 0.toUInt() && vWrites > vGone && com.sce.forge.runtime.SceChecked.add((vValues).size.toUInt(), vWrites) > com.sce.forge.runtime.SceChecked.add(vRoomValues, vGone) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='!spoiled && gone + writes > 0 && roomValues > 0 && writes > gone && len(values) + writes > roomValues + gone'>: an integer operation overflowed or failed"); ifCondFailed17 = true; false })) {

            if (try { vFull = com.sce.forge.runtime.SceChecked.add(vFull, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='full'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vSpoiled = true

            vSpoiledFor = RecordListParentSceSynthInvokeListRefusalEnum.NO_ROOM_FOR_VALUE
            }
            if (ifCondFailed17) {
                return
            }


            var ifCondFailed18 = false
            if ((try { !vSpoiled && vJournal && com.sce.forge.runtime.SceChecked.add(vGone, vWrites) > 0.toUInt() && (vProvisional).size.toUInt() >= 8.toUInt() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='!spoiled && journal && gone + writes > 0 && len(provisional) >= _x.limits.shared_provisional'>: an integer operation overflowed or failed"); ifCondFailed18 = true; false })) {

            vSpoiled = true

            vSpoiledFor = RecordListParentSceSynthInvokeListRefusalEnum.NO_ROOM_FOR_HISTORY
            }
            if (ifCondFailed18) {
                return
            }


            var ifCondFailed19 = false
            if ((try { !vSpoiled && com.sce.forge.runtime.SceChecked.add(vGone, vWrites) > 0.toUInt() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='!spoiled && gone + writes > 0'>: an integer operation overflowed or failed"); ifCondFailed19 = true; false })) {

            vProbing = true

            vProbe = RecordListParentSceSynthInvokeListRefusalEnum.CLOCK_EXHAUSTED

            if (try { vStamp = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, vClosingNow, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='stamp'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vProbe = RecordListParentSceSynthInvokeListRefusalEnum.CHANGE_TOO_LARGE

            vTaking = emptyList()

            vTook = 0.toUInt()


            for (a in vAdds) {

            vCovered = false


            for (s in vStaged) {


            var ifCondFailed20 = false
            if ((try { com.sce.forge.runtime.SceChecked.div(a.element, 65536.toULong()) == s.id.toULong() && (s.kind == 2.toUByte() || com.sce.forge.runtime.SceChecked.rem(a.element, 65536.toULong()) == s.field.toULong()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='a.element / 65536 === s.id && (s.kind === 2 || a.element % 65536 === s.field)'>: an integer operation overflowed or failed"); ifCondFailed20 = true; false })) {

            vCovered = true
            }
            if (ifCondFailed20) {
                return
            }
            }


            if (vCovered) {

            vOp = vOp.copy(kind = 1.toUByte())

            if (try { vOp = vOp.copy(id = com.sce.forge.runtime.SceChecked.narrowToUInt(com.sce.forge.runtime.SceChecked.div(a.element, 65536.toULong()))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='op.id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vOp = vOp.copy(field = com.sce.forge.runtime.SceChecked.narrowToUShort(com.sce.forge.runtime.SceChecked.rem(a.element, 65536.toULong()))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='op.field'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOp = vOp.copy(value = 0)

            if (try { vOp = vOp.copy(text = com.sce.forge.runtime.SceChecked.bounded("", 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='op.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOp = vOp.copy(wallTime = a.wallTime)

            vOp = vOp.copy(counter = a.counter)

            vOp = vOp.copy(nodeId = a.nodeId)

            if (if (vTaking.size < 64) { vTaking = vTaking + (vOp); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='taking'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            for (s in vStaged) {


            if (s.kind == 0.toUByte()) {

            vOp = vOp.copy(kind = 0.toUByte())

            vOp = vOp.copy(id = s.id)

            vOp = vOp.copy(field = s.field)

            vOp = vOp.copy(value = s.value)

            if (try { vOp = vOp.copy(text = com.sce.forge.runtime.SceChecked.bounded(s.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='op.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOp = vOp.copy(wallTime = 0)

            vOp = vOp.copy(counter = 0.toUInt())

            vOp = vOp.copy(nodeId = 0.toULong())

            if (if (vTaking.size < 64) { vTaking = vTaking + (vOp); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='taking'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }

            vProbing = false

            vStaged = emptyList()

            vClock = vStamp

            vDot = vStamp


            if (!vJournal) {

            vRestoring = false

            vTentative = false


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Take, EventMetadata.internal(sendData))
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
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Pass, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vJournal) {

            vOffered = emptyList()


            for (t in vTaking) {

            if (if (vOffered.size < 64) { vOffered = vOffered + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='offered'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }

            vOfferedAt = vStamp


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Offer, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed19) {
                return
            }


            var ifCondFailed25 = false
            if ((try { !vSpoiled && com.sce.forge.runtime.SceChecked.add(vGone, vWrites) == 0.toUInt() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='!spoiled && gone + writes === 0'>: an integer operation overflowed or failed"); ifCondFailed25 = true; false })) {

            vStaged = emptyList()


            if (vJournal) {

            vClosedOps = 0.toUInt()

            vClosedFor = null

            vClosedAt = vClosedAt.copy(wallTime = 0)

            vClosedAt = vClosedAt.copy(counter = 0.toUInt())

            vClosedAt = vClosedAt.copy(nodeId = 0.toULong())


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Conclude, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            if (ifCondFailed25) {
                return
            }


            if (vSpoiled) {

            vStaged = emptyList()

            vSpoiled = false


            if (vJournal) {

            vClosedOps = 0.toUInt()

            vClosedFor = vSpoiledFor

            vClosedAt = vClosedAt.copy(wallTime = 0)

            vClosedAt = vClosedAt.copy(counter = 0.toUInt())

            vClosedAt = vClosedAt.copy(nodeId = 0.toULong())


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Conclude, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (vAnnounce) {


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_8")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vNamedId).toLong())

            putParam(sendPayload, "field", (vNamedField).toLong())

            putParam(sendPayload, "kind", (vNamedKind).declaredName)

            putParam(sendPayload, "reason", (vSpoiledFor).declaredName)

            putParam(sendPayload, "command", (vClosingCommand).toString())

            putParam(sendPayload, "withdrawn", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Refused,
"record_list.refused",
                sendData,
0L,
                "__send_8",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_8")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_8")
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
            }
    }

    private fun transitionContentOfOpenAt10() {
                // SCE-MAP: record_list.scxml:766 :: open :: _transition_10


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "by", (vOfferedAt.nodeId).toString())

            putParam(sendPayload, "wallTime", (vOfferedAt.wallTime).toString())

            putParam(sendPayload, "counter", (vOfferedAt.counter).toLong())

            try {
                putParam(sendPayload, "ops", run { if ((vOffered).size < 1) throw com.sce.forge.runtime.AlgorithmFailure(com.sce.forge.runtime.AlgorithmError.Precondition); (vOffered).map { mapOf<String, Any?>("kind" to (it.kind).toLong(), "id" to (it.id).toLong(), "field" to (it.field).toLong(), "value" to (it.value).toString(), "text" to it.text, "wallTime" to (it.wallTime).toString(), "counter" to (it.counter).toLong(), "nodeId" to (it.nodeId).toString()) } })
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                // docs/adr/0086, section 15.6: the list is short of its field's lower
                // count, so the event is refused whole: one error, nothing delivered,
                // and the error ends the block (W3C SCXML 6.2.3, 4.9).
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='ops'> holds fewer elements than the 1 its field's type declares, so the event is not sent", "__send_9")
                return@send true
            }

            putParam(sendPayload, "command", (vClosingCommand).toString())

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
                eventName = "record_list.made",
                target = "#journal",
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_9")
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfOpenAt11() {
                // SCE-MAP: record_list.scxml:793 :: open :: _transition_11
                if (pendingRecordListVerdictPayload == null) {
                    return
                }

            vStanding = vStanding.copy(wallTime = vOfferedAt.wallTime)

            vStanding = vStanding.copy(counter = vOfferedAt.counter)

            vStanding = vStanding.copy(command = vClosingCommand)

            if (try { vStanding = vStanding.copy(id = com.sce.forge.runtime.SceChecked.at(vOffered, (0).toLong()).id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='standing.id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vStanding = vStanding.copy(field = com.sce.forge.runtime.SceChecked.at(vOffered, (0).toLong()).field); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='standing.field'>: an integer operation overflowed or failed"); true }) {
                return
            }


            var ifCondFailed30 = false
            if ((try { com.sce.forge.runtime.SceChecked.at(vOffered, (0).toLong()).kind == 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='offered[0].kind === 0'>: an integer operation overflowed or failed"); ifCondFailed30 = true; false })) {

            vStanding = vStanding.copy(kind = RecordListParentSceSynthInvokeListCommandEnum.EDIT)
            } else {

            vStanding = vStanding.copy(kind = RecordListParentSceSynthInvokeListCommandEnum.CLEAR)
            }
            if (ifCondFailed30) {
                return
            }

            vClosedAt = vOfferedAt


            if (pendingRecordListVerdictPayload!!.outcome == RecordListParentSceSynthInvokeListOutcomeEnum.REFUSED) {


            if (pendingRecordListVerdictPayload!!.reason != null) {

            vRefusedFor = com.sce.forge.runtime.SceOptional.proven(pendingRecordListVerdictPayload!!.reason)
            } else {

            vRefusedFor = RecordListParentSceSynthInvokeListRefusalEnum.PEER_REFUSED
            }

            vClosedOps = 0.toUInt()

            vClosedFor = vRefusedFor

            vOffered = emptyList()


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Conclude, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (vAnnounce) {


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_11")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vStanding.id).toLong())

            putParam(sendPayload, "field", (vStanding.field).toLong())

            putParam(sendPayload, "kind", (vStanding.kind).declaredName)

            putParam(sendPayload, "reason", (vRefusedFor).declaredName)

            putParam(sendPayload, "command", (vStanding.command).toString())

            putParam(sendPayload, "withdrawn", false)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Refused,
"record_list.refused",
                sendData,
0L,
                "__send_11",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_11")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_11")
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
            }


            if (pendingRecordListVerdictPayload!!.outcome != RecordListParentSceSynthInvokeListOutcomeEnum.REFUSED) {

            vClosedOps = (vOffered).size.toUInt()

            vClosedFor = null

            vTaking = emptyList()


            for (t in vOffered) {

            if (if (vTaking.size < 64) { vTaking = vTaking + (t); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='taking'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }

            vTook = 0.toUInt()

            vDot = vOfferedAt

            vRestoring = false

            vTentative = pendingRecordListVerdictPayload!!.outcome == RecordListParentSceSynthInvokeListOutcomeEnum.WAITING


            if (vTentative) {

            if (if (vProvisional.size < 8) { vProvisional = vProvisional + (vStanding); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisional'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }

            vOffered = emptyList()


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Take, EventMetadata.internal(sendData))
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
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Conclude, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (vTentative && vAnnounce) {


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
            putParam(sendPayload, "wallTime", (vStanding.wallTime).toString())

            putParam(sendPayload, "counter", (vStanding.counter).toLong())

            putParam(sendPayload, "command", (vStanding.command).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Pending,
"record_list.pending",
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
            }
    }

    private fun transitionContentOfOpenAt12() {
                // SCE-MAP: record_list.scxml:857 :: open :: _transition_12

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }

    private fun transitionContentOfOpenAt13() {
                // SCE-MAP: record_list.scxml:864 :: open :: _transition_13


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "command", (vClosingCommand).toString())

            putParam(sendPayload, "operations", (vClosedOps).toLong())

            putParam(sendPayload, "reason", (vClosedFor)?.let { scePresent -> (scePresent).declaredName })

            putParam(sendPayload, "wallTime", (vClosedAt.wallTime).toString())

            putParam(sendPayload, "counter", (vClosedAt.counter).toLong())

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
                eventName = "record_list.closed",
                target = "#journal",
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

    private fun transitionContentOfOpenAt14() {
                // SCE-MAP: record_list.scxml:879 :: open :: _transition_14
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

            if (try { vDenied = com.sce.forge.runtime.SceChecked.add(vDenied, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='denied'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vAuthorized) {

            vTaking = pendingRecordListChangePayload!!.ops

            vTook = 0.toUInt()

            vDot = vDot.copy(wallTime = pendingRecordListChangePayload!!.wallTime)

            vDot = vDot.copy(counter = pendingRecordListChangePayload!!.counter)

            vDot = vDot.copy(nodeId = pendingRecordListChangePayload!!.by)

            vRestoring = false

            vTentative = false


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Deliver, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
    }

    private fun transitionContentOfOpenAt15() {
                // SCE-MAP: record_list.scxml:914 :: open :: _transition_15
                if (pendingRecordListRecallPayload == null) {
                    return
                }

            vTaking = pendingRecordListRecallPayload!!.ops

            vTook = 0.toUInt()

            vDot = vDot.copy(wallTime = pendingRecordListRecallPayload!!.wallTime)

            vDot = vDot.copy(counter = pendingRecordListRecallPayload!!.counter)

            vDot = vDot.copy(nodeId = pendingRecordListRecallPayload!!.by)

            vRestoring = false

            vTentative = pendingRecordListRecallPayload!!.provisional && pendingRecordListRecallPayload!!.by == vNode


            if (vTentative) {

            vStanding = vStanding.copy(wallTime = pendingRecordListRecallPayload!!.wallTime)

            vStanding = vStanding.copy(counter = pendingRecordListRecallPayload!!.counter)

            vStanding = vStanding.copy(command = pendingRecordListRecallPayload!!.command)

            if (try { vStanding = vStanding.copy(id = com.sce.forge.runtime.SceChecked.at(vTaking, (0).toLong()).id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='standing.id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vStanding = vStanding.copy(field = com.sce.forge.runtime.SceChecked.at(vTaking, (0).toLong()).field); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='standing.field'>: an integer operation overflowed or failed"); true }) {
                return
            }


            var ifCondFailed42 = false
            if ((try { com.sce.forge.runtime.SceChecked.at(vTaking, (0).toLong()).kind == 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='taking[0].kind === 0'>: an integer operation overflowed or failed"); ifCondFailed42 = true; false })) {

            vStanding = vStanding.copy(kind = RecordListParentSceSynthInvokeListCommandEnum.EDIT)
            } else {

            vStanding = vStanding.copy(kind = RecordListParentSceSynthInvokeListCommandEnum.CLEAR)
            }
            if (ifCondFailed42) {
                return
            }

            if (if (vProvisional.size < 8) { vProvisional = vProvisional + (vStanding); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisional'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Deliver, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
    }

    private fun transitionContentOfOpenAt16() {
                // SCE-MAP: record_list.scxml:946 :: open :: _transition_16


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Take, EventMetadata.internal(sendData))
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

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcReceive__vv(vClock, vDot, vClock.wallTime, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfOpenAt17() {
                // SCE-MAP: record_list.scxml:966 :: open :: _transition_17
                if (pendingRecordListRestorePayload == null) {
                    return
                }

            vTaking = pendingRecordListRestorePayload!!.ops

            vTook = 0.toUInt()

            vRestoring = true

            vTentative = false


            for (t in vTaking) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Take, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }


            if (pendingRecordListRestorePayload!!.wallTime > vClock.wallTime || pendingRecordListRestorePayload!!.wallTime == vClock.wallTime && pendingRecordListRestorePayload!!.counter > vClock.counter) {

            vClock = vClock.copy(wallTime = pendingRecordListRestorePayload!!.wallTime)

            vClock = vClock.copy(counter = pendingRecordListRestorePayload!!.counter)
            }
    }

    private fun transitionContentOfOpenAt18() {
                // SCE-MAP: record_list.scxml:984 :: open :: _transition_18
                if (pendingRecordListRaisePayload == null) {
                    return
                }


            if (pendingRecordListRaisePayload!!.wallTime > vClock.wallTime || pendingRecordListRaisePayload!!.wallTime == vClock.wallTime && pendingRecordListRaisePayload!!.counter > vClock.counter) {

            vClock = vClock.copy(wallTime = pendingRecordListRaisePayload!!.wallTime)

            vClock = vClock.copy(counter = pendingRecordListRaisePayload!!.counter)
            }
    }

    private fun transitionContentOfOpenAt19() {
                // SCE-MAP: record_list.scxml:1001 :: open :: _transition_19

            if (try { vOp = com.sce.forge.runtime.SceChecked.at(vTaking, (vTook).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='op'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTook = com.sce.forge.runtime.SceChecked.add(vTook, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='took'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(element = vOp.id.toULong())

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), vOp.field.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vOp.kind == 0.toUByte()) {

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(vOp.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(id = vOp.id.toULong())

            vHeld = vHeld.copy(field = vOp.field.toULong())

            vHeld = vHeld.copy(value = vOp.value)

            vHeld = vHeld.copy(wallTime = vDot.wallTime)

            vHeld = vHeld.copy(counter = vDot.counter)

            vHeld = vHeld.copy(nodeId = vDot.nodeId)


            if (vRestoring) {

            vHeld = vHeld.copy(wallTime = vOp.wallTime)

            vHeld = vHeld.copy(counter = vOp.counter)

            vHeld = vHeld.copy(nodeId = vOp.nodeId)
            }

            vEntry = vEntry.copy(wallTime = vHeld.wallTime)

            vEntry = vEntry.copy(counter = vHeld.counter)

            vEntry = vEntry.copy(nodeId = vHeld.nodeId)

            vPresent = call_MvValueHolds__vv(vValues, vHeld)


            if (!vPresent) {

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueKeep__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (!vPresent && vAnnounce) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vOp.id).toLong())

            putParam(sendPayload, "field", (vOp.field).toLong())

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


            if (vTentative && vWasLive) {


            for (v in vValues) {


            if (v.id == vHeld.id && v.field == vHeld.field && v.wallTime == vHeld.wallTime && v.counter == vHeld.counter && v.nodeId == vHeld.nodeId) {

            vUndoing = vUndoing.copy(id = v.id)

            vUndoing = vUndoing.copy(field = v.field)

            vUndoing = vUndoing.copy(value = v.value)

            if (try { vUndoing = vUndoing.copy(text = com.sce.forge.runtime.SceChecked.bounded(v.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='undoing.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vUndoing = vUndoing.copy(wallTime = v.wallTime)

            vUndoing = vUndoing.copy(counter = v.counter)

            vUndoing = vUndoing.copy(nodeId = v.nodeId)
            }
            }

            vUndoing = vUndoing.copy(atWallTime = vDot.wallTime)

            vUndoing = vUndoing.copy(atCounter = vDot.counter)

            if (if (vUndo.size < 512) { vUndo = vUndo + (vUndoing); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undo'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }


            if (!vWasLive && (vUndo).size.toUInt() > 0.toUInt()) {

            vUndoKept = emptyList()


            for (u in vUndo) {


            if (u.id != vHeld.id || u.field != vHeld.field || u.wallTime != vHeld.wallTime || u.counter != vHeld.counter || u.nodeId != vHeld.nodeId) {

            if (if (vUndoKept.size < 512) { vUndoKept = vUndoKept + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undoKept'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }
            }

            vUndo = emptyList()


            for (u in vUndoKept) {

            if (if (vUndo.size < 512) { vUndo = vUndo + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undo'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }
            }

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueDrop__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vWasLive && vAnnounce) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vOp.id).toLong())

            putParam(sendPayload, "field", (vOp.field).toLong())

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
            }
    }

    private fun transitionContentOfOpenAt20() {
                // SCE-MAP: record_list.scxml:1103 :: open :: _transition_20


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> <param name='ops'> holds fewer elements than the 1 its field's type declares, so the event is not sent", "__send_22")
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

    private fun transitionContentOfOpenAt21() {
                // SCE-MAP: record_list.scxml:1117 :: open :: _transition_21
                if (pendingRecordListConfirmPayload == null) {
                    return
                }

            vProvisionalFound = false


            for (p in vProvisional) {


            if (p.wallTime == pendingRecordListConfirmPayload!!.wallTime && p.counter == pendingRecordListConfirmPayload!!.counter) {

            vProvisionalFound = true

            vStanding = p
            }
            }


            if (!vProvisionalFound) {

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vProvisionalFound) {

            vProvisionalKept = emptyList()


            for (p in vProvisional) {


            if (p.wallTime != vStanding.wallTime || p.counter != vStanding.counter) {

            if (if (vProvisionalKept.size < 8) { vProvisionalKept = vProvisionalKept + (p); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisionalKept'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }
            }

            vProvisional = emptyList()


            for (p in vProvisionalKept) {

            if (if (vProvisional.size < 8) { vProvisional = vProvisional + (p); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisional'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }

            vUndoKept = emptyList()


            for (u in vUndo) {


            if (u.atWallTime != vStanding.wallTime || u.atCounter != vStanding.counter) {

            if (if (vUndoKept.size < 512) { vUndoKept = vUndoKept + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undoKept'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }
            }

            vUndo = emptyList()


            for (u in vUndoKept) {

            if (if (vUndo.size < 512) { vUndo = vUndo + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undo'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }


            if (vAnnounce) {


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_23")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "wallTime", (vStanding.wallTime).toString())

            putParam(sendPayload, "counter", (vStanding.counter).toLong())

            putParam(sendPayload, "command", (vStanding.command).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Confirmed,
"record_list.confirmed",
                sendData,
0L,
                "__send_23",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_23")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_23")
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
            }
    }

    private fun transitionContentOfOpenAt22() {
                // SCE-MAP: record_list.scxml:1165 :: open :: _transition_22
                if (pendingRecordListWithdrawPayload == null) {
                    return
                }

            vProvisionalFound = false


            for ((sce_position_of_at, p) in vProvisional.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val at = sce_position_of_at.toUInt()


            if (!vProvisionalFound && p.wallTime == pendingRecordListWithdrawPayload!!.wallTime && p.counter == pendingRecordListWithdrawPayload!!.counter) {

            vProvisionalFound = true

            vWithdrawFrom = at
            }
            }


            if (!vProvisionalFound) {

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }


            if (vProvisionalFound) {


            for ((sce_position_of_at, p) in vProvisional.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val at = sce_position_of_at.toUInt()


            if (at >= vWithdrawFrom) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Unwind, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
            }
    }

    private fun transitionContentOfOpenAt23() {
                // SCE-MAP: record_list.scxml:1194 :: open :: _transition_23

            if (try { vStanding = com.sce.forge.runtime.SceChecked.at(vProvisional, (com.sce.forge.runtime.SceChecked.sub((vProvisional).size.toUInt(), 1.toUInt())).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='standing'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOne = emptyList()


            for (a in vAdds) {


            if (a.wallTime == vStanding.wallTime && a.counter == vStanding.counter && a.nodeId == vNode) {

            if (if (vOne.size < 1024) { vOne = vOne + (a); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }
            }
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetLive__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }


            for (a in vOne) {

            if (try { vHeld = vHeld.copy(id = com.sce.forge.runtime.SceChecked.div(a.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vHeld = vHeld.copy(field = com.sce.forge.runtime.SceChecked.rem(a.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.field'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(wallTime = a.wallTime)

            vHeld = vHeld.copy(counter = a.counter)

            vHeld = vHeld.copy(nodeId = a.nodeId)

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueDrop__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vAnnounce) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(com.sce.forge.runtime.SceChecked.div(a.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(com.sce.forge.runtime.SceChecked.rem(a.element, 65536.toULong())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

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
            }


            for (u in vUndo) {


            if (u.atWallTime == vStanding.wallTime && u.atCounter == vStanding.counter) {

            vEntry = vEntry.copy(element = u.id)

            if (try { vEntry = vEntry.copy(element = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(vEntry.element, 65536.toULong()), u.field)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='entry.element'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vEntry = vEntry.copy(wallTime = u.wallTime)

            vEntry = vEntry.copy(counter = u.counter)

            vEntry = vEntry.copy(nodeId = u.nodeId)

            vOne = emptyList()

            if (if (vOne.size < 1024) { vOne = vOne + (vEntry); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='one'>: the list already holds its capacity of 1024"); true }) {
                return
            }

            if (try { vAdds = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_OrsetUnion__vv(vAdds, vOne)), 1024); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='adds'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(id = u.id)

            vHeld = vHeld.copy(field = u.field)

            vHeld = vHeld.copy(value = u.value)

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(u.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = vHeld.copy(wallTime = u.wallTime)

            vHeld = vHeld.copy(counter = u.counter)

            vHeld = vHeld.copy(nodeId = u.nodeId)

            if (try { vValues = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_MvValueKeep__vv(vValues, vHeld)), 512); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='values'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vAnnounce) {

            if (try { vShareId = com.sce.forge.runtime.SceChecked.narrowToUInt(u.id); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareId'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vShareField = com.sce.forge.runtime.SceChecked.narrowToUShort(u.field); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='shareField'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vShareId).toLong())

            putParam(sendPayload, "field", (vShareField).toLong())

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
            }
            }

            vUndoKept = emptyList()


            for (u in vUndo) {


            if (u.atWallTime != vStanding.wallTime || u.atCounter != vStanding.counter) {

            if (if (vUndoKept.size < 512) { vUndoKept = vUndoKept + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undoKept'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }
            }

            vUndo = emptyList()


            for (u in vUndoKept) {

            if (if (vUndo.size < 512) { vUndo = vUndo + (u); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='undo'>: the list already holds its capacity of 512"); true }) {
                return
            }
            }

            vProvisionalKept = emptyList()


            for ((sce_position_of_at, p) in vProvisional.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val at = sce_position_of_at.toUInt()


            var ifCondFailed72 = false
            if ((try { com.sce.forge.runtime.SceChecked.add(at, 1.toUInt()) < (vProvisional).size.toUInt() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<if cond='at + 1 < len(provisional)'>: an integer operation overflowed or failed"); ifCondFailed72 = true; false })) {

            if (if (vProvisionalKept.size < 8) { vProvisionalKept = vProvisionalKept + (p); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisionalKept'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }
            if (ifCondFailed72) {
                return
            }
            }

            vProvisional = emptyList()


            for (p in vProvisionalKept) {

            if (if (vProvisional.size < 8) { vProvisional = vProvisional + (p); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='provisional'>: the list already holds its capacity of 8"); true }) {
                return
            }
            }


            if (vAnnounce) {


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_27")
                return@send true
            }
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (vStanding.id).toLong())

            putParam(sendPayload, "field", (vStanding.field).toLong())

            putParam(sendPayload, "kind", (vStanding.kind).declaredName)

            putParam(sendPayload, "reason", (RecordListParentSceSynthInvokeListRefusalEnum.PEER_REFUSED).declaredName)

            putParam(sendPayload, "command", (vStanding.command).toString())

            putParam(sendPayload, "withdrawn", true)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2.4 + C.1: a targetexpr is a target — the value is
            // routed as the same value written in `target` is, at once or after
            // the delay, by the table SendHelper.classifyTarget holds (C++
            // `SendHelper::classifyTarget`).
            when (sendToTarget(
                _rt,
RecordListParentSceSynthInvokeListEvent.RecordList.Refused,
"record_list.refused",
                sendData,
0L,
                "__send_27",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_27")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_27")
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
    }

    private fun transitionContentOfOpenAt24() {
                // SCE-MAP: record_list.scxml:1284 :: open :: _transition_24
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

            if (if (vMoved.size < 64) { vMoved = vMoved + (vEntry.element); false } else { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<sce:append target='moved'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
    }

    private fun transitionContentOfOpenAt25() {
                // SCE-MAP: record_list.scxml:1307 :: open :: _transition_25


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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_29")
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
                "__send_29",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_29")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_29")
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_30")
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
                "__send_30",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_30")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_30")
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
                raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr evaluated to nothing, or to a route the send does not declare, so there is no target to reach", "__send_31")
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
                "__send_31",
                RecordListParentSceSynthInvokeListEvent.Error.Communication,
            )) {
                com.sce.runtime.TargetSendOutcome.UNSUPPORTED -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<send> targetexpr produced a value that is not a target", "__send_31")
                    return@send true
                }
                com.sce.runtime.TargetSendOutcome.UNREACHABLE -> {
                    raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Communication, "<send> targetexpr names a session this processor cannot reach", "__send_31")
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

    private fun transitionContentOfOpenAt26() {
                // SCE-MAP: record_list.scxml:1365 :: open :: _transition_26

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentSceSynthInvokeListEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (vProbing) {

            vProbing = false

            vSpoiled = true

            vSpoiledFor = vProbe


            if (!vHolding) {


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            raiseInternal(RecordListParentSceSynthInvokeListEvent.RecordList.Close, EventMetadata.internal(sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            }
    }
}
