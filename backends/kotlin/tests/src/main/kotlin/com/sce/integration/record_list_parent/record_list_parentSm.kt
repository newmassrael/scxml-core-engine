// SCE-GENERATED — DO NOT EDIT
// source-hash: 438f3c44a0f49109d71c51cacca59b20d771e2147409672affa707c8d139f3c1

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/record_list_parent.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: record_list_parent.scxml:28 :: _machine

package com.sce.integration.record_list_parent

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface RecordListParentState : State {
    data object Holding : RecordListParentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface RecordListParentEvent : Event {
    sealed interface Done : RecordListParentEvent {
        data object Invoke : Done
    }
    sealed interface Error : RecordListParentEvent {
        data object Communication : Error
        data object Execution : Error
    }
    sealed interface RecordList : RecordListParentEvent {
        data object Edit : RecordList
        data object Join : RecordList
        data object Register : RecordList
        data object Supersede : RecordList
        data object Value : RecordList
        data object Write : RecordList
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `RecordListParentPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// RecordListParentRecordListEditPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.edit`. Consumers inject it via the `raiseRecordListEdit` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListEditPayload(val id: UInt, val field: UShort, val value: Long, val text: String, val now: Long)

// RecordListParentRecordListJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.join`. Consumers inject it via the `raiseRecordListJoin` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListJoinPayload(val node: ULong, val hub: Boolean)

// RecordListParentRecordListRegisterPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.register`. Consumers inject it via the `raiseRecordListRegister` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListRegisterPayload(val id: UInt, val field: UShort, val count: UInt, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)

// RecordListParentRecordListSupersedePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.supersede`. Consumers inject it via the `raiseRecordListSupersede` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListSupersedePayload(val id: UInt, val field: UShort, val wallTime: Long, val counter: UInt, val nodeId: ULong, val by: ULong)

// RecordListParentRecordListValuePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.value`. Consumers inject it via the `raiseRecordListValue` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListValuePayload(val id: UInt, val field: UShort, val ordinal: UInt, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)

// RecordListParentRecordListWritePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `record_list.write`. Consumers inject it via the `raiseRecordListWrite` seam
// on the machine — they never name this class directly.
data class RecordListParentRecordListWritePayload(val id: UInt, val field: UShort, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Told` datamodel value. */
data class RecordListParentToldRecord(val id: UInt, val field: UShort, val count: UInt, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "count" to SavedValues.of(count), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentToldRecord = RecordListParentToldRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), count = SavedValues.uint32(SavedValues.field(value, what, "count"), "$what.count"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:ToldValue` datamodel value. */
data class RecordListParentToldValueRecord(val id: UInt, val field: UShort, val ordinal: UInt, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "ordinal" to SavedValues.of(ordinal), "value" to SavedValues.of(value), "text" to SavedValues.of(text), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): RecordListParentToldValueRecord = RecordListParentToldValueRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint16(SavedValues.field(value, what, "field"), "$what.field"), ordinal = SavedValues.uint32(SavedValues.field(value, what, "ordinal"), "$what.ordinal"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 96), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
// --- State Machine (W3C SCXML) ---

class RecordListParentStateMachine(
) : StateMachineEngine<RecordListParentState, RecordListParentEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `told` datamodel variable, published (`sce:direction="out"`). */
    private var vTold: UInt = 0.toUInt()

    /** What the published `told` holds. */
    val told: UInt get() = vTold
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    private var vShown: RecordListParentToldRecord = RecordListParentToldRecord(id = 0.toUInt(), field = 0.toUShort(), count = 0.toUInt(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())

    /** What the published `shown` holds. */
    val shown: RecordListParentToldRecord get() = vShown
    /** W3C SCXML 5.2: the `others` datamodel variable, published (`sce:direction="out"`). */
    private var vOthers: List<RecordListParentToldValueRecord> = emptyList()

    /** What the published `others` holds. */
    val others: List<RecordListParentToldValueRecord> get() = vOthers
    /** W3C SCXML 5.2: the `other` datamodel variable, the machine's own. */
    private var vOther: RecordListParentToldValueRecord = RecordListParentToldValueRecord(id = 0.toUInt(), field = 0.toUShort(), ordinal = 0.toUInt(), value = 0, text = "", wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var told: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.told?.let { vTold = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val told: UInt,
        val shown: RecordListParentToldRecord,
        val others: List<RecordListParentToldValueRecord>,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<RecordListParentState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        told = vTold,
        shown = vShown,
        others = vOthers,
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
        first = "eb4e40412a797dcc82b22f18f86465cdd79276510acf8c264db23b9a59d6c36b",
        revised = "19cfd1639c9e5ead843e49cc179c4fba29708d80926fade1bee0dfb2d9fc1b07",
        records = listOf(
            RecordLineage(
                event = "record_list.register",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("shown", false),
                ),
            ),
            RecordLineage(
                event = "record_list.value",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("others", true),
                    RecordHolder("other", false),
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
            "told" to SavedValues.of(vTold),
            "shown" to vShown.toSaved(),
            "others" to SavedValues.list(vOthers) { it.toSaved() },
            "other" to vOther.toSaved(),
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
        val saved1 = SavedValues.uint32(savedGrown.variable("told"), "told")
        val saved2 = RecordListParentToldRecord.fromSaved(savedGrown.variable("shown"), "shown")
        val saved3 = SavedValues.list(savedGrown.variable("others"), "others", 8) { e, w -> RecordListParentToldValueRecord.fromSaved(e, w) }
        val saved4 = RecordListParentToldValueRecord.fromSaved(savedGrown.variable("other"), "other")
        vTold = saved1
        vShown = saved2
        vOthers = saved3
        vOther = saved4
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, RecordListParentState>> = listOf(
        "list" to RecordListParentState.Holding,
    )

    override fun restartInvoke(invokeId: String, state: RecordListParentState) = deferStaticInvoke(invokeId, state)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingRecordListEditPayload: RecordListParentRecordListEditPayload? = null
    private var pendingRecordListJoinPayload: RecordListParentRecordListJoinPayload? = null
    private var pendingRecordListRegisterPayload: RecordListParentRecordListRegisterPayload? = null
    private var pendingRecordListSupersedePayload: RecordListParentRecordListSupersedePayload? = null
    private var pendingRecordListValuePayload: RecordListParentRecordListValuePayload? = null
    private var pendingRecordListWritePayload: RecordListParentRecordListWritePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: RecordListParentEvent, metadata: EventMetadata) {
        pendingRecordListEditPayload = null
        pendingRecordListJoinPayload = null
        pendingRecordListRegisterPayload = null
        pendingRecordListSupersedePayload = null
        pendingRecordListValuePayload = null
        pendingRecordListWritePayload = null
        when (val tp = metadata.typedPayload) {
            is RecordListParentRecordListEditPayload -> pendingRecordListEditPayload = tp
            is RecordListParentRecordListJoinPayload -> pendingRecordListJoinPayload = tp
            is RecordListParentRecordListRegisterPayload -> pendingRecordListRegisterPayload = tp
            is RecordListParentRecordListSupersedePayload -> pendingRecordListSupersedePayload = tp
            is RecordListParentRecordListValuePayload -> pendingRecordListValuePayload = tp
            is RecordListParentRecordListWritePayload -> pendingRecordListWritePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == RecordListParentEvent.RecordList.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListEditPayload = RecordListParentRecordListEditPayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("now"))
                } else if (event == RecordListParentEvent.RecordList.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListJoinPayload = RecordListParentRecordListJoinPayload(fields.uint64("node"), fields.boolean("hub"))
                } else if (event == RecordListParentEvent.RecordList.Register) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListRegisterPayload = RecordListParentRecordListRegisterPayload(fields.uint32("id"), fields.uint16("field"), fields.uint32("count"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                } else if (event == RecordListParentEvent.RecordList.Supersede) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListSupersedePayload = RecordListParentRecordListSupersedePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"), fields.uint64("by"))
                } else if (event == RecordListParentEvent.RecordList.Value) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListValuePayload = RecordListParentRecordListValuePayload(fields.uint32("id"), fields.uint16("field"), fields.uint32("ordinal"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                } else if (event == RecordListParentEvent.RecordList.Write) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingRecordListWritePayload = RecordListParentRecordListWritePayload(fields.uint32("id"), fields.uint16("field"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.edit` — binds the event name and the payload field values in one call.
    fun raiseRecordListEdit(id: UInt, field: UShort, value: Long, text: String, now: Long) {
        send(
            RecordListParentEvent.RecordList.Edit,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListEditPayload(id, field, value, text, now),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "now" to now))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.join` — binds the event name and the payload field values in one call.
    fun raiseRecordListJoin(node: ULong, hub: Boolean) {
        send(
            RecordListParentEvent.RecordList.Join,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListJoinPayload(node, hub),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("node" to node, "hub" to hub))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.register` — binds the event name and the payload field values in one call.
    fun raiseRecordListRegister(id: UInt, field: UShort, count: UInt, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            RecordListParentEvent.RecordList.Register,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListRegisterPayload(id, field, count, value, text, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "count" to count, "value" to value, "text" to text, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.supersede` — binds the event name and the payload field values in one call.
    fun raiseRecordListSupersede(id: UInt, field: UShort, wallTime: Long, counter: UInt, nodeId: ULong, by: ULong) {
        send(
            RecordListParentEvent.RecordList.Supersede,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListSupersedePayload(id, field, wallTime, counter, nodeId, by),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId, "by" to by))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.value` — binds the event name and the payload field values in one call.
    fun raiseRecordListValue(id: UInt, field: UShort, ordinal: UInt, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            RecordListParentEvent.RecordList.Value,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListValuePayload(id, field, ordinal, value, text, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "ordinal" to ordinal, "value" to value, "text" to text, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `record_list.write` — binds the event name and the payload field values in one call.
    fun raiseRecordListWrite(id: UInt, field: UShort, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
        send(
            RecordListParentEvent.RecordList.Write,
            EventMetadata(
                type = "external",
                typedPayload = RecordListParentRecordListWritePayload(id, field, value, text, wallTime, counter, nodeId),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "field" to field, "value" to value, "text" to text, "wallTime" to wallTime, "counter" to counter, "nodeId" to nodeId))
            )
        )
    }


    override val initialState: RecordListParentState = RecordListParentState.Holding

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

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
    override val documentInitialTargets: List<EntryTarget<RecordListParentState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<RecordListParentState, HistoryId>> =
            listOf(StateTarget(RecordListParentState.Holding))

        // W3C SCXML 3.13: holding's transition 0, as the microstep reads it.
        val transitionHoldingAt0 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 1, as the microstep reads it.
        val transitionHoldingAt1 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 2, as the microstep reads it.
        val transitionHoldingAt2 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 3, as the microstep reads it.
        val transitionHoldingAt3 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 4, as the microstep reads it.
        val transitionHoldingAt4 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 5, as the microstep reads it.
        val transitionHoldingAt5 = EnabledTransition<RecordListParentState, HistoryId>(
            RecordListParentState.Holding,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): RecordListParentState? = when (stateId) {
        "holding" -> RecordListParentState.Holding
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: RecordListParentState): String = when (state) {
        is RecordListParentState.Holding -> "holding"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: RecordListParentState): Int = when (state) {
        is RecordListParentState.Holding -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): RecordListParentEvent? = when (name) {
        "done.invoke" -> RecordListParentEvent.Done.Invoke
        "error.communication" -> RecordListParentEvent.Error.Communication
        "error.execution" -> RecordListParentEvent.Error.Execution
        "record_list.edit" -> RecordListParentEvent.RecordList.Edit
        "record_list.join" -> RecordListParentEvent.RecordList.Join
        "record_list.register" -> RecordListParentEvent.RecordList.Register
        "record_list.supersede" -> RecordListParentEvent.RecordList.Supersede
        "record_list.value" -> RecordListParentEvent.RecordList.Value
        "record_list.write" -> RecordListParentEvent.RecordList.Write
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: RecordListParentEvent): String? = when (event) {
        is RecordListParentEvent.Done.Invoke -> "done.invoke"
        is RecordListParentEvent.Error.Communication -> "error.communication"
        is RecordListParentEvent.Error.Execution -> "error.execution"
        is RecordListParentEvent.RecordList.Edit -> "record_list.edit"
        is RecordListParentEvent.RecordList.Join -> "record_list.join"
        is RecordListParentEvent.RecordList.Register -> "record_list.register"
        is RecordListParentEvent.RecordList.Supersede -> "record_list.supersede"
        is RecordListParentEvent.RecordList.Value -> "record_list.value"
        is RecordListParentEvent.RecordList.Write -> "record_list.write"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: RecordListParentState,
        event: RecordListParentEvent?
    ): EnabledTransition<RecordListParentState, HistoryId>? = when (state) {
        is RecordListParentState.Holding -> when {
            event is RecordListParentEvent.RecordList.Join -> transitionHoldingAt0
            event is RecordListParentEvent.RecordList.Edit -> transitionHoldingAt1
            event is RecordListParentEvent.RecordList.Write -> transitionHoldingAt2
            event is RecordListParentEvent.RecordList.Supersede -> transitionHoldingAt3
            event is RecordListParentEvent.RecordList.Register -> transitionHoldingAt4
            event is RecordListParentEvent.RecordList.Value -> transitionHoldingAt5
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: record_list_parent.scxml:28 :: _machine
    override fun onEntry(state: RecordListParentState, isDefaultEntry: Boolean) {
        when (state) {
            is RecordListParentState.Holding -> {
                // SCE-MAP: record_list_parent.scxml:62 :: holding :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("list", state)
            }
        }
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    private fun deferStaticInvoke(invokeId: String, state: RecordListParentState) {
        when (invokeId) {
            "list" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "holding.${System.identityHashCode(this)}.list"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = RecordListParentSceSynthInvokeListStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = RecordListParentSceSynthInvokeListStateMachine.InvokeParams()
                    childSeed.announce = true
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("list", childSM, false, RecordListParentEvent.Done.Invoke, "", generatedInvokeId)
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: record_list_parent.scxml:28 :: _machine
    override fun onExit(state: RecordListParentState) {
        when (state) {
            is RecordListParentState.Holding -> {
                // SCE-MAP: record_list_parent.scxml:62 :: holding :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("list")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: record_list_parent.scxml:28 :: _machine
    override fun executeTransitionContent(source: RecordListParentState, transitionIndex: Int) {
        when (source) {
        is RecordListParentState.Holding -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: record_list_parent.scxml:68 :: holding :: _transition_0
                if (pendingRecordListJoinPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "node", (pendingRecordListJoinPayload!!.node).toString())

            putParam(sendPayload, "hub", pendingRecordListJoinPayload!!.hub)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("list", "record_list.join", sendData)) {
                raisePlatformError(RecordListParentEvent.Error.Communication, "<send target='#_list'> names an invocation that is not running", "__send_0")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: record_list_parent.scxml:74 :: holding :: _transition_1
                if (pendingRecordListEditPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListEditPayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListEditPayload!!.field).toLong())

            putParam(sendPayload, "value", (pendingRecordListEditPayload!!.value).toString())

            putParam(sendPayload, "text", pendingRecordListEditPayload!!.text)

            putParam(sendPayload, "now", (pendingRecordListEditPayload!!.now).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("list", "record_list.edit", sendData)) {
                raisePlatformError(RecordListParentEvent.Error.Communication, "<send target='#_list'> names an invocation that is not running", "__send_1")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: record_list_parent.scxml:83 :: holding :: _transition_2
                if (pendingRecordListWritePayload == null) {
                    return
                }


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
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("list", "record_list.write", sendData)) {
                raisePlatformError(RecordListParentEvent.Error.Communication, "<send target='#_list'> names an invocation that is not running", "__send_2")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: record_list_parent.scxml:94 :: holding :: _transition_3
                if (pendingRecordListSupersedePayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingRecordListSupersedePayload!!.id).toLong())

            putParam(sendPayload, "field", (pendingRecordListSupersedePayload!!.field).toLong())

            putParam(sendPayload, "wallTime", (pendingRecordListSupersedePayload!!.wallTime).toString())

            putParam(sendPayload, "counter", (pendingRecordListSupersedePayload!!.counter).toLong())

            putParam(sendPayload, "nodeId", (pendingRecordListSupersedePayload!!.nodeId).toString())

            putParam(sendPayload, "by", (pendingRecordListSupersedePayload!!.by).toString())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.4 (test192): Send event to invoked child. The build
            // refused a target naming no invocation above; one that is not
            // running is not there to reach (W3C SCXML C.1).
            if (!sendToChild("list", "record_list.supersede", sendData)) {
                raisePlatformError(RecordListParentEvent.Error.Communication, "<send target='#_list'> names an invocation that is not running", "__send_3")
                return@send true
            }
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            4 -> {
                // SCE-MAP: record_list_parent.scxml:107 :: holding :: _transition_4
                if (pendingRecordListRegisterPayload == null) {
                    return
                }

            vShown = vShown.copy(id = pendingRecordListRegisterPayload!!.id)

            vShown = vShown.copy(field = pendingRecordListRegisterPayload!!.field)

            vShown = vShown.copy(count = pendingRecordListRegisterPayload!!.count)

            vShown = vShown.copy(value = pendingRecordListRegisterPayload!!.value)

            if (try { vShown = vShown.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListRegisterPayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentEvent.Error.Execution, "<assign location='shown.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vShown = vShown.copy(wallTime = pendingRecordListRegisterPayload!!.wallTime)

            vShown = vShown.copy(counter = pendingRecordListRegisterPayload!!.counter)

            vShown = vShown.copy(nodeId = pendingRecordListRegisterPayload!!.nodeId)

            vOthers = emptyList()

            if (try { vTold = com.sce.forge.runtime.SceChecked.add(vTold, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentEvent.Error.Execution, "<assign location='told'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: record_list_parent.scxml:119 :: holding :: _transition_5
                if (pendingRecordListValuePayload == null) {
                    return
                }

            vOther = vOther.copy(id = pendingRecordListValuePayload!!.id)

            vOther = vOther.copy(field = pendingRecordListValuePayload!!.field)

            vOther = vOther.copy(ordinal = pendingRecordListValuePayload!!.ordinal)

            vOther = vOther.copy(value = pendingRecordListValuePayload!!.value)

            if (try { vOther = vOther.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingRecordListValuePayload!!.text, 96)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentEvent.Error.Execution, "<assign location='other.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vOther = vOther.copy(wallTime = pendingRecordListValuePayload!!.wallTime)

            vOther = vOther.copy(counter = pendingRecordListValuePayload!!.counter)

            vOther = vOther.copy(nodeId = pendingRecordListValuePayload!!.nodeId)

            if (if (vOthers.size < 8) { vOthers = vOthers + (vOther); false } else { raisePlatformError(RecordListParentEvent.Error.Execution, "<sce:append target='others'>: the list already holds its capacity of 8"); true }) {
                return
            }

            if (try { vTold = com.sce.forge.runtime.SceChecked.add(vTold, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(RecordListParentEvent.Error.Execution, "<assign location='told'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
