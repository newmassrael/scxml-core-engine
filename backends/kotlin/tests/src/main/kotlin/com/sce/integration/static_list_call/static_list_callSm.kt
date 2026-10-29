// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_list_call.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_list_call.scxml:13 :: _machine
@file:OptIn(ExperimentalUnsignedTypes::class)

package com.sce.integration.static_list_call

import com.sce.runtime.*
import com.sce.generated.ids_above.*
import com.sce.generated.ledger_has.*
import com.sce.generated.schema_entry.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticListCallState : State {
    data object Open : StaticListCallState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticListCallEvent : Event {
    sealed interface Ledger : StaticListCallEvent {
        data object Entry : Ledger
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticListCallPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticListCallLedgerEntryPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `ledger.entry`. Consumers inject it via the `raiseLedgerEntry` seam
// on the machine — they never name this class directly.
data class StaticListCallLedgerEntryPayload(val id: UInt, val amount: Int)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Entry` datamodel value. */
data class StaticListCallEntryRecord(val id: UInt, val amount: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticListCallEntryRecord = StaticListCallEntryRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int32(SavedValues.field(value, what, "amount"), "$what.amount"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_Above__v(p0: List<UInt>, p1: UInt): Boolean {
    val r = idsAbove(p0.toUIntArray(), p1)
    return r
}
private fun call_Has__v(p0: List<StaticListCallEntryRecord>, p1: UInt): Boolean {
    val r = ledgerHas(p0.map { SchemaEntryPayload(id = it.id, amount = it.amount) }, p1)
    return r
}

// --- State Machine (W3C SCXML) ---

class StaticListCallStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticListCallState, StaticListCallEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `entries` datamodel variable, published (`sce:direction="out"`). */
    private var vEntries: List<StaticListCallEntryRecord> = emptyList()

    /** What the published `entries` holds. */
    val entries: List<StaticListCallEntryRecord> get() = vEntries
    /** W3C SCXML 5.2: the `ids` datamodel variable, published (`sce:direction="out"`). */
    private var vIds: List<UInt> = emptyList()

    /** What the published `ids` holds. */
    val ids: List<UInt> get() = vIds
    /** W3C SCXML 5.2: the `line` datamodel variable, published (`sce:direction="out"`). */
    private var vLine: StaticListCallEntryRecord = StaticListCallEntryRecord(id = 0.toUInt(), amount = 0)

    /** What the published `line` holds. */
    val line: StaticListCallEntryRecord get() = vLine
    /** W3C SCXML 5.2: the `repeats` datamodel variable, published (`sce:direction="out"`). */
    private var vRepeats: UInt = 0.toUInt()

    /** What the published `repeats` holds. */
    val repeats: UInt get() = vRepeats
    /** W3C SCXML 5.2: the `high` datamodel variable, published (`sce:direction="out"`). */
    private var vHigh: UInt = 0.toUInt()

    /** What the published `high` holds. */
    val high: UInt get() = vHigh

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var repeats: UInt? = null
        var high: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.repeats?.let { vRepeats = it }
        params.high?.let { vHigh = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val entries: List<StaticListCallEntryRecord>,
        val ids: List<UInt>,
        val line: StaticListCallEntryRecord,
        val repeats: UInt,
        val high: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticListCallState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        entries = vEntries,
        ids = vIds,
        line = vLine,
        repeats = vRepeats,
        high = vHigh,
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
        first = "cc3c6a68d49a4f3f4a0a60a12e419e42e4fe45aa3cf7178b225123dabc1bdec0",
        revised = "fab3769be78802f970514e036b890a74516228c9f439de1d7feb81ae829a23b7",
        format2First = "cc3c6a68d49a4f3f4a0a60a12e419e42e4fe45aa3cf7178b225123dabc1bdec0",
        format2Revised = "fab3769be78802f970514e036b890a74516228c9f439de1d7feb81ae829a23b7",
        records = listOf(
            RecordLineage(
                event = "ledger.entry",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("entries", true),
                    RecordHolder("line", false),
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
            "entries" to SavedValues.list(vEntries) { it.toSaved() },
            "ids" to SavedValues.list(vIds) { SavedValues.of(it) },
            "line" to vLine.toSaved(),
            "repeats" to SavedValues.of(vRepeats),
            "high" to SavedValues.of(vHigh),
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
        val saved1 = SavedValues.list(savedGrown.variable("entries"), "entries", 8) { e, w -> StaticListCallEntryRecord.fromSaved(e, w) }
        val saved2 = SavedValues.list(savedGrown.variable("ids"), "ids", 8) { e, w -> SavedValues.uint32(e, w) }
        val saved3 = StaticListCallEntryRecord.fromSaved(savedGrown.variable("line"), "line")
        val saved4 = SavedValues.uint32(savedGrown.variable("repeats"), "repeats")
        val saved5 = SavedValues.uint32(savedGrown.variable("high"), "high")
        vEntries = saved1
        vIds = saved2
        vLine = saved3
        vRepeats = saved4
        vHigh = saved5
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        SavedValues.list(savedGrown.variable("entries"), "entries", 8) { e, w -> StaticListCallEntryRecord.fromSaved(e, w) }
        SavedValues.list(savedGrown.variable("ids"), "ids", 8) { e, w -> SavedValues.uint32(e, w) }
        StaticListCallEntryRecord.fromSaved(savedGrown.variable("line"), "line")
        SavedValues.uint32(savedGrown.variable("repeats"), "repeats")
        SavedValues.uint32(savedGrown.variable("high"), "high")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingLedgerEntryPayload: StaticListCallLedgerEntryPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticListCallEvent, metadata: EventMetadata) {
        pendingLedgerEntryPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticListCallLedgerEntryPayload -> pendingLedgerEntryPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticListCallEvent.Ledger.Entry) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingLedgerEntryPayload = StaticListCallLedgerEntryPayload(fields.uint32("id"), fields.int32("amount"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `ledger.entry` — binds the event name and the payload field values in one call.
    fun raiseLedgerEntry(id: UInt, amount: Int) {
        send(
            StaticListCallEvent.Ledger.Entry,
            EventMetadata(
                type = "external",
                typedPayload = StaticListCallLedgerEntryPayload(id, amount),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "amount" to amount))
            )
        )
    }


    override val initialState: StaticListCallState = StaticListCallState.Open

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
    override val documentInitialTargets: List<EntryTarget<StaticListCallState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticListCallState, HistoryId>> =
            listOf(StateTarget(StaticListCallState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<StaticListCallState, HistoryId>(
            StaticListCallState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticListCallState? = when (stateId) {
        "open" -> StaticListCallState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticListCallState): String = when (state) {
        is StaticListCallState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticListCallState): Int = when (state) {
        is StaticListCallState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticListCallEvent? = when (name) {
        "ledger.entry" -> StaticListCallEvent.Ledger.Entry
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticListCallEvent): String? = when (event) {
        is StaticListCallEvent.Ledger.Entry -> "ledger.entry"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticListCallState,
        event: StaticListCallEvent?
    ): EnabledTransition<StaticListCallState, HistoryId>? = when (state) {
        is StaticListCallState.Open -> when {
            event is StaticListCallEvent.Ledger.Entry -> transitionOpenAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_list_call.scxml:13 :: _machine
    override fun onEntry(state: StaticListCallState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticListCallState.Open -> {
                // SCE-MAP: static_list_call.scxml:28 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_list_call.scxml:13 :: _machine
    override fun onExit(state: StaticListCallState) {
        when (state) {
            is StaticListCallState.Open -> {
                // SCE-MAP: static_list_call.scxml:28 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_list_call.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticListCallState, transitionIndex: Int) {
        when (source) {
        is StaticListCallState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_list_call.scxml:29 :: open :: _transition_0
                if (pendingLedgerEntryPayload == null) {
                    return
                }


            if (call_Has__v(vEntries, pendingLedgerEntryPayload!!.id)) {

            if (try { vRepeats = com.sce.forge.runtime.SceChecked.add(vRepeats, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }

            vLine = StaticListCallEntryRecord(id = pendingLedgerEntryPayload!!.id, amount = pendingLedgerEntryPayload!!.amount)

            if (if (vEntries.size < 8) { vEntries = vEntries + (vLine); false } else { true }) {
                return
            }

            if (if (vIds.size < 8) { vIds = vIds + (pendingLedgerEntryPayload!!.id); false } else { true }) {
                return
            }


            if (call_Above__v(vIds, 100.toUInt())) {

            if (try { vHigh = com.sce.forge.runtime.SceChecked.add(vHigh, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            }
            else -> {}
        }
        }
    }
}
