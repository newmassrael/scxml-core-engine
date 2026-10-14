// SCE-GENERATED — DO NOT EDIT
// source-hash: a92737f3147f50ca2cf6319055f29d7afe2c17766723037ae46a81e73e7a66df

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/todo_peer.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: todo_peer.scxml:25 :: _machine

package com.sce.integration.todo_peer

import com.sce.runtime.*
import com.sce.generated.hlc_receive.*
import com.sce.generated.hlc_send.*
import com.sce.generated.hlc_timestamp.*
import com.sce.generated.lww_cell.*
import com.sce.generated.lww_write.*


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
data class TodoPeerTodoEditPayload(val id: ULong, val field: ULong, val value: Long, val text: String, val now: Long)

// TodoPeerTodoJoinPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.join`. Consumers inject it via the `raiseTodoJoin` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoJoinPayload(val node: ULong, val hub: Boolean)

// TodoPeerTodoWritePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `todo.write`. Consumers inject it via the `raiseTodoWrite` seam
// on the machine — they never name this class directly.
data class TodoPeerTodoWritePayload(val id: ULong, val field: ULong, val value: Long, val text: String, val wallTime: Long, val counter: UInt, val nodeId: ULong)


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
/** SCE Accepted Subset §2.15: a `record:LwwCell` datamodel value. */
data class TodoPeerLwwCellRecord(val row: ULong, val col: ULong, val value: Long, val wallTime: Long, val counter: UInt, val nodeId: ULong) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("row" to SavedValues.of(row), "col" to SavedValues.of(col), "value" to SavedValues.of(value), "wallTime" to SavedValues.of(wallTime), "counter" to SavedValues.of(counter), "nodeId" to SavedValues.of(nodeId))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerLwwCellRecord = TodoPeerLwwCellRecord(row = SavedValues.uint64(SavedValues.field(value, what, "row"), "$what.row"), col = SavedValues.uint64(SavedValues.field(value, what, "col"), "$what.col"), value = SavedValues.int64(SavedValues.field(value, what, "value"), "$what.value"), wallTime = SavedValues.int64(SavedValues.field(value, what, "wallTime"), "$what.wallTime"), counter = SavedValues.uint32(SavedValues.field(value, what, "counter"), "$what.counter"), nodeId = SavedValues.uint64(SavedValues.field(value, what, "nodeId"), "$what.nodeId"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Text` datamodel value. */
data class TodoPeerTextRecord(val id: ULong, val field: ULong, val text: String) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "field" to SavedValues.of(field), "text" to SavedValues.of(text))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): TodoPeerTextRecord = TodoPeerTextRecord(id = SavedValues.uint64(SavedValues.field(value, what, "id"), "$what.id"), field = SavedValues.uint64(SavedValues.field(value, what, "field"), "$what.field"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 32))
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
private fun call_LwwWrite__vv(p0: List<TodoPeerLwwCellRecord>, p1: TodoPeerLwwCellRecord): com.sce.forge.runtime.AlgorithmResult<List<TodoPeerLwwCellRecord>> =
    when (val r = lwwWrite(p0.map { LwwCellPayload(row = it.row, col = it.col, value = it.value, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) }, LwwCellPayload(row = p1.row, col = p1.col, value = p1.value, wallTime = p1.wallTime, counter = p1.counter, nodeId = p1.nodeId))) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { TodoPeerLwwCellRecord(row = it.row, col = it.col, value = it.value, wallTime = it.wallTime, counter = it.counter, nodeId = it.nodeId) })
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
    /** W3C SCXML 5.2: the `cells` datamodel variable, published (`sce:direction="out"`). */
    private var vCells: List<TodoPeerLwwCellRecord> = emptyList()

    /** What the published `cells` holds. */
    val cells: List<TodoPeerLwwCellRecord> get() = vCells
    /** W3C SCXML 5.2: the `write` datamodel variable, the machine's own. */
    private var vWrite: TodoPeerLwwCellRecord = TodoPeerLwwCellRecord(row = 0.toULong(), col = 0.toULong(), value = 0, wallTime = 0, counter = 0.toUInt(), nodeId = 0.toULong())
    /** W3C SCXML 5.2: the `texts` datamodel variable, published (`sce:direction="out"`). */
    private var vTexts: List<TodoPeerTextRecord> = emptyList()

    /** What the published `texts` holds. */
    val texts: List<TodoPeerTextRecord> get() = vTexts
    /** W3C SCXML 5.2: the `rebuilt` datamodel variable, the machine's own. */
    private var vRebuilt: List<TodoPeerTextRecord> = emptyList()
    /** W3C SCXML 5.2: the `held` datamodel variable, the machine's own. */
    private var vHeld: TodoPeerTextRecord = TodoPeerTextRecord(id = 0.toULong(), field = 0.toULong(), text = "")
    /** W3C SCXML 5.2: the `won` datamodel variable, the machine's own. */
    private var vWon: Boolean = false
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
        var won: Boolean? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.node?.let { vNode = it }
        params.hub?.let { vHub = it }
        params.won?.let { vWon = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val node: ULong,
        val hub: Boolean,
        val clock: TodoPeerHlcRecord,
        val cells: List<TodoPeerLwwCellRecord>,
        val texts: List<TodoPeerTextRecord>,
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
        cells = vCells,
        texts = vTexts,
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
    val savedShape: String = "473a98f7617a12feaca9142c1ab1383961d04cc1addc75811b2bec2352c16308"

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
            "write" to vWrite.toSaved(),
            "texts" to SavedValues.list(vTexts) { it.toSaved() },
            "rebuilt" to SavedValues.list(vRebuilt) { it.toSaved() },
            "held" to vHeld.toSaved(),
            "won" to SavedValues.of(vWon),
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
        val saved5 = SavedValues.list(saved.variable("cells"), "cells", 256) { e, w -> TodoPeerLwwCellRecord.fromSaved(e, w) }
        val saved6 = TodoPeerLwwCellRecord.fromSaved(saved.variable("write"), "write")
        val saved7 = SavedValues.list(saved.variable("texts"), "texts", 64) { e, w -> TodoPeerTextRecord.fromSaved(e, w) }
        val saved8 = SavedValues.list(saved.variable("rebuilt"), "rebuilt", 64) { e, w -> TodoPeerTextRecord.fromSaved(e, w) }
        val saved9 = TodoPeerTextRecord.fromSaved(saved.variable("held"), "held")
        val saved10 = SavedValues.bool(saved.variable("won"), "won")
        val saved11 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        vNode = saved1
        vHub = saved2
        vClock = saved3
        vStamp = saved4
        vCells = saved5
        vWrite = saved6
        vTexts = saved7
        vRebuilt = saved8
        vHeld = saved9
        vWon = saved10
        vRefusals = saved11
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTodoEditPayload: TodoPeerTodoEditPayload? = null
    private var pendingTodoJoinPayload: TodoPeerTodoJoinPayload? = null
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
        pendingTodoWritePayload = null
        when (val tp = metadata.typedPayload) {
            is TodoPeerTodoEditPayload -> pendingTodoEditPayload = tp
            is TodoPeerTodoJoinPayload -> pendingTodoJoinPayload = tp
            is TodoPeerTodoWritePayload -> pendingTodoWritePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == TodoPeerEvent.Todo.Edit) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoEditPayload = TodoPeerTodoEditPayload(fields.uint64("id"), fields.uint64("field"), fields.int64("value"), fields.string("text"), fields.int64("now"))
                } else if (event == TodoPeerEvent.Todo.Join) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoJoinPayload = TodoPeerTodoJoinPayload(fields.uint64("node"), fields.boolean("hub"))
                } else if (event == TodoPeerEvent.Todo.Write) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTodoWritePayload = TodoPeerTodoWritePayload(fields.uint64("id"), fields.uint64("field"), fields.int64("value"), fields.string("text"), fields.int64("wallTime"), fields.uint32("counter"), fields.uint64("nodeId"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `todo.edit` — binds the event name and the payload field values in one call.
    fun raiseTodoEdit(id: ULong, field: ULong, value: Long, text: String, now: Long) {
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
    // `todo.write` — binds the event name and the payload field values in one call.
    fun raiseTodoWrite(id: ULong, field: ULong, value: Long, text: String, wallTime: Long, counter: UInt, nodeId: ULong) {
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
        "todo.write" -> TodoPeerEvent.Todo.Write
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: TodoPeerEvent): String? = when (event) {
        is TodoPeerEvent.Error.Execution -> "error.execution"
        is TodoPeerEvent.Todo.Edit -> "todo.edit"
        is TodoPeerEvent.Todo.Join -> "todo.join"
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
            event is TodoPeerEvent.Error.Execution -> transitionOpenAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: todo_peer.scxml:25 :: _machine
    override fun onEntry(state: TodoPeerState, isDefaultEntry: Boolean) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:68 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: todo_peer.scxml:25 :: _machine
    override fun onExit(state: TodoPeerState) {
        when (state) {
            is TodoPeerState.Open -> {
                // SCE-MAP: todo_peer.scxml:68 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: todo_peer.scxml:25 :: _machine
    override fun executeTransitionContent(source: TodoPeerState, transitionIndex: Int) {
        when (source) {
        is TodoPeerState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: todo_peer.scxml:69 :: open :: _transition_0
                if (pendingTodoJoinPayload == null) {
                    return
                }

            vNode = pendingTodoJoinPayload!!.node

            vHub = pendingTodoJoinPayload!!.hub
            }
            1 -> {
                // SCE-MAP: todo_peer.scxml:75 :: open :: _transition_1
                if (pendingTodoEditPayload == null) {
                    return
                }

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingTodoEditPayload!!.text, 32)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vClock = com.sce.forge.runtime.SceChecked.take(call_HlcSend__v(vClock, pendingTodoEditPayload!!.now, vNode)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='clock'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toString())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toString())

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
            putParam(sendPayload, "id", (pendingTodoEditPayload!!.id).toString())

            putParam(sendPayload, "field", (pendingTodoEditPayload!!.field).toString())

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
            2 -> {
                // SCE-MAP: todo_peer.scxml:102 :: open :: _transition_2
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

            vWrite = vWrite.copy(row = pendingTodoWritePayload!!.id)

            vWrite = vWrite.copy(col = pendingTodoWritePayload!!.field)

            vWrite = vWrite.copy(value = pendingTodoWritePayload!!.value)

            vWrite = vWrite.copy(wallTime = pendingTodoWritePayload!!.wallTime)

            vWrite = vWrite.copy(counter = pendingTodoWritePayload!!.counter)

            vWrite = vWrite.copy(nodeId = pendingTodoWritePayload!!.nodeId)

            if (try { vCells = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_LwwWrite__vv(vCells, vWrite)), 256); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='cells'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vWon = false


            for (c in vCells) {


            if (c.row == pendingTodoWritePayload!!.id && c.col == pendingTodoWritePayload!!.field && c.wallTime == pendingTodoWritePayload!!.wallTime && c.counter == pendingTodoWritePayload!!.counter && c.nodeId == pendingTodoWritePayload!!.nodeId) {

            vWon = true
            }
            }


            if (vWon && pendingTodoWritePayload!!.field == 1.toULong()) {

            vRebuilt = emptyList()


            for (t in vTexts) {


            if (t.id != pendingTodoWritePayload!!.id || t.field != pendingTodoWritePayload!!.field) {

            if (if (vRebuilt.size < 64) { vRebuilt = vRebuilt + (t); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }

            vHeld = vHeld.copy(id = pendingTodoWritePayload!!.id)

            vHeld = vHeld.copy(field = pendingTodoWritePayload!!.field)

            if (try { vHeld = vHeld.copy(text = com.sce.forge.runtime.SceChecked.bounded(pendingTodoWritePayload!!.text, 32)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='held.text'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (if (vRebuilt.size < 64) { vRebuilt = vRebuilt + (vHeld); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='rebuilt'>: the list already holds its capacity of 64"); true }) {
                return
            }

            vTexts = emptyList()


            for (r in vRebuilt) {

            if (if (vTexts.size < 64) { vTexts = vTexts + (r); false } else { raisePlatformError(TodoPeerEvent.Error.Execution, "<sce:append target='texts'>: the list already holds its capacity of 64"); true }) {
                return
            }
            }
            }


            if (vHub && pendingTodoWritePayload!!.nodeId != 1.toULong()) {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "id", (pendingTodoWritePayload!!.id).toString())

            putParam(sendPayload, "field", (pendingTodoWritePayload!!.field).toString())

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
                raisePlatformError(TodoPeerEvent.Error.Execution, "<send type='sce:mesh'> names a processor the host declared but never registered", "__send_2")
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
            putParam(sendPayload, "id", (pendingTodoWritePayload!!.id).toString())

            putParam(sendPayload, "field", (pendingTodoWritePayload!!.field).toString())

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
            }
            3 -> {
                // SCE-MAP: todo_peer.scxml:169 :: open :: _transition_3

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(TodoPeerEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
