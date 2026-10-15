// SCE-GENERATED — DO NOT EDIT
// source-hash: 0e931b64d8b84ac074c11f557102b954390a5b6bc56ecf7eaee340a9062b12f3

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_state_algorithm.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_state_algorithm.scxml:20 :: _machine
@file:OptIn(ExperimentalUnsignedTypes::class)

package com.sce.integration.static_state_algorithm

import com.sce.runtime.*
import com.sce.generated.ledger_add.*
import com.sce.generated.ledger_amount.*
import com.sce.generated.ledger_bump.*
import com.sce.generated.ledger_ids.*
import com.sce.generated.ledger_total.*
import com.sce.generated.schema_entry.*
import com.sce.generated.schema_total.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticStateAlgorithmState : State {
    data object Open : StaticStateAlgorithmState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticStateAlgorithmEvent : Event {
    data object Bump : StaticStateAlgorithmEvent
    sealed interface Error : StaticStateAlgorithmEvent {
        data object Execution : Error
    }
    sealed interface Ledger : StaticStateAlgorithmEvent {
        data object Entry : Ledger
    }
    data object Track : StaticStateAlgorithmEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticStateAlgorithmPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticStateAlgorithmLedgerEntryPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `ledger.entry`. Consumers inject it via the `raiseLedgerEntry` seam
// on the machine — they never name this class directly.
data class StaticStateAlgorithmLedgerEntryPayload(val id: UInt, val amount: Int)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Entry` datamodel value. */
data class StaticStateAlgorithmEntryRecord(val id: UInt, val amount: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticStateAlgorithmEntryRecord = StaticStateAlgorithmEntryRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int32(SavedValues.field(value, what, "amount"), "$what.amount"))
    }
}
/** SCE Accepted Subset §2.15: a `record:Total` datamodel value. */
data class StaticStateAlgorithmTotalRecord(val count: UInt, val sum: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("count" to SavedValues.of(count), "sum" to SavedValues.of(sum))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticStateAlgorithmTotalRecord = StaticStateAlgorithmTotalRecord(count = SavedValues.uint32(SavedValues.field(value, what, "count"), "$what.count"), sum = SavedValues.int32(SavedValues.field(value, what, "sum"), "$what.sum"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_AddEntry__v(p0: List<StaticStateAlgorithmEntryRecord>, p1: UInt, p2: Int): com.sce.forge.runtime.AlgorithmResult<List<StaticStateAlgorithmEntryRecord>> =
    when (val r = ledgerAdd(p0.map { SchemaEntryPayload(id = it.id, amount = it.amount) }, p1, p2)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { StaticStateAlgorithmEntryRecord(id = it.id, amount = it.amount) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_AmountOf__p(p0: StaticStateAlgorithmLedgerEntryPayload): Int {
    val r = ledgerAmount(SchemaEntryPayload(id = p0.id, amount = p0.amount))
    return r
}
private fun call_Bump__v(p0: StaticStateAlgorithmTotalRecord, p1: Int): com.sce.forge.runtime.AlgorithmResult<StaticStateAlgorithmTotalRecord> =
    when (val r = ledgerBump(SchemaTotalPayload(count = p0.count, sum = p0.sum), p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(StaticStateAlgorithmTotalRecord(count = r.value.count, sum = r.value.sum))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_IdSum__v(p0: List<UInt>): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = ledgerIds(p0.toUIntArray())) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_TotalOf__v(p0: List<StaticStateAlgorithmEntryRecord>): com.sce.forge.runtime.AlgorithmResult<StaticStateAlgorithmTotalRecord> =
    when (val r = ledgerTotal(p0.map { SchemaEntryPayload(id = it.id, amount = it.amount) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(StaticStateAlgorithmTotalRecord(count = r.value.count, sum = r.value.sum))
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class StaticStateAlgorithmStateMachine(
) : StateMachineEngine<StaticStateAlgorithmState, StaticStateAlgorithmEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `entries` datamodel variable, published (`sce:direction="out"`). */
    private var vEntries: List<StaticStateAlgorithmEntryRecord> = emptyList()

    /** What the published `entries` holds. */
    val entries: List<StaticStateAlgorithmEntryRecord> get() = vEntries
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    private var vTotal: StaticStateAlgorithmTotalRecord = StaticStateAlgorithmTotalRecord(count = 0.toUInt(), sum = 0)

    /** What the published `total` holds. */
    val total: StaticStateAlgorithmTotalRecord get() = vTotal
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticStateAlgorithmEntryRecord = StaticStateAlgorithmEntryRecord(id = 0.toUInt(), amount = 0)

    /** What the published `last` holds. */
    val last: StaticStateAlgorithmEntryRecord get() = vLast
    /** W3C SCXML 5.2: the `ids` datamodel variable, published (`sce:direction="out"`). */
    private var vIds: List<UInt> = emptyList()

    /** What the published `ids` holds. */
    val ids: List<UInt> get() = vIds
    /** W3C SCXML 5.2: the `idsum` datamodel variable, published (`sce:direction="out"`). */
    private var vIdsum: UInt = 0.toUInt()

    /** What the published `idsum` holds. */
    val idsum: UInt get() = vIdsum
    /** W3C SCXML 5.2: the `large` datamodel variable, published (`sce:direction="out"`). */
    private var vLarge: UInt = 0.toUInt()

    /** What the published `large` holds. */
    val large: UInt get() = vLarge
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
        var idsum: UInt? = null
        var large: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.idsum?.let { vIdsum = it }
        params.large?.let { vLarge = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val entries: List<StaticStateAlgorithmEntryRecord>,
        val total: StaticStateAlgorithmTotalRecord,
        val last: StaticStateAlgorithmEntryRecord,
        val ids: List<UInt>,
        val idsum: UInt,
        val large: UInt,
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
        val configuration: Set<StaticStateAlgorithmState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        entries = vEntries,
        total = vTotal,
        last = vLast,
        ids = vIds,
        idsum = vIdsum,
        large = vLarge,
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
    val savedShape: String = "9974bd59dfcaa3327314694f10c51b9c6ca6182a91e88cc96fb36c167a4eb0da"

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
            "total" to vTotal.toSaved(),
            "last" to vLast.toSaved(),
            "ids" to SavedValues.list(vIds) { SavedValues.of(it) },
            "idsum" to SavedValues.of(vIdsum),
            "large" to SavedValues.of(vLarge),
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
        val saved1 = SavedValues.list(saved.variable("entries"), "entries", 8) { e, w -> StaticStateAlgorithmEntryRecord.fromSaved(e, w) }
        val saved2 = StaticStateAlgorithmTotalRecord.fromSaved(saved.variable("total"), "total")
        val saved3 = StaticStateAlgorithmEntryRecord.fromSaved(saved.variable("last"), "last")
        val saved4 = SavedValues.list(saved.variable("ids"), "ids", 8) { e, w -> SavedValues.uint32(e, w) }
        val saved5 = SavedValues.uint32(saved.variable("idsum"), "idsum")
        val saved6 = SavedValues.uint32(saved.variable("large"), "large")
        val saved7 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        vEntries = saved1
        vTotal = saved2
        vLast = saved3
        vIds = saved4
        vIdsum = saved5
        vLarge = saved6
        vRefusals = saved7
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingLedgerEntryPayload: StaticStateAlgorithmLedgerEntryPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticStateAlgorithmEvent, metadata: EventMetadata) {
        pendingLedgerEntryPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticStateAlgorithmLedgerEntryPayload -> pendingLedgerEntryPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticStateAlgorithmEvent.Ledger.Entry) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingLedgerEntryPayload = StaticStateAlgorithmLedgerEntryPayload(fields.uint32("id"), fields.int32("amount"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `ledger.entry` — binds the event name and the payload field values in one call.
    fun raiseLedgerEntry(id: UInt, amount: Int) {
        send(
            StaticStateAlgorithmEvent.Ledger.Entry,
            EventMetadata(
                type = "external",
                typedPayload = StaticStateAlgorithmLedgerEntryPayload(id, amount),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "amount" to amount))
            )
        )
    }


    override val initialState: StaticStateAlgorithmState = StaticStateAlgorithmState.Open

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
    override val documentInitialTargets: List<EntryTarget<StaticStateAlgorithmState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticStateAlgorithmState, HistoryId>> =
            listOf(StateTarget(StaticStateAlgorithmState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<StaticStateAlgorithmState, HistoryId>(
            StaticStateAlgorithmState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<StaticStateAlgorithmState, HistoryId>(
            StaticStateAlgorithmState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<StaticStateAlgorithmState, HistoryId>(
            StaticStateAlgorithmState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 3, as the microstep reads it.
        val transitionOpenAt3 = EnabledTransition<StaticStateAlgorithmState, HistoryId>(
            StaticStateAlgorithmState.Open,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticStateAlgorithmState? = when (stateId) {
        "open" -> StaticStateAlgorithmState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticStateAlgorithmState): String = when (state) {
        is StaticStateAlgorithmState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticStateAlgorithmState): Int = when (state) {
        is StaticStateAlgorithmState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticStateAlgorithmEvent? = when (name) {
        "bump" -> StaticStateAlgorithmEvent.Bump
        "error.execution" -> StaticStateAlgorithmEvent.Error.Execution
        "ledger.entry" -> StaticStateAlgorithmEvent.Ledger.Entry
        "track" -> StaticStateAlgorithmEvent.Track
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticStateAlgorithmEvent): String? = when (event) {
        is StaticStateAlgorithmEvent.Bump -> "bump"
        is StaticStateAlgorithmEvent.Error.Execution -> "error.execution"
        is StaticStateAlgorithmEvent.Ledger.Entry -> "ledger.entry"
        is StaticStateAlgorithmEvent.Track -> "track"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticStateAlgorithmState,
        event: StaticStateAlgorithmEvent?
    ): EnabledTransition<StaticStateAlgorithmState, HistoryId>? = when (state) {
        is StaticStateAlgorithmState.Open -> when {
            event is StaticStateAlgorithmEvent.Ledger.Entry -> transitionOpenAt0
            event is StaticStateAlgorithmEvent.Bump -> transitionOpenAt1
            event is StaticStateAlgorithmEvent.Track -> transitionOpenAt2
            event is StaticStateAlgorithmEvent.Error.Execution -> transitionOpenAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_state_algorithm.scxml:20 :: _machine
    override fun onEntry(state: StaticStateAlgorithmState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticStateAlgorithmState.Open -> {
                // SCE-MAP: static_state_algorithm.scxml:44 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_state_algorithm.scxml:20 :: _machine
    override fun onExit(state: StaticStateAlgorithmState) {
        when (state) {
            is StaticStateAlgorithmState.Open -> {
                // SCE-MAP: static_state_algorithm.scxml:44 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_state_algorithm.scxml:20 :: _machine
    override fun executeTransitionContent(source: StaticStateAlgorithmState, transitionIndex: Int) {
        when (source) {
        is StaticStateAlgorithmState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_state_algorithm.scxml:45 :: open :: _transition_0
                if (pendingLedgerEntryPayload == null) {
                    return
                }

            if (try { vEntries = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_AddEntry__v(vEntries, pendingLedgerEntryPayload!!.id, pendingLedgerEntryPayload!!.amount)), 8); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='entries'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTotal = com.sce.forge.runtime.SceChecked.take(call_TotalOf__v(vEntries)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLast = StaticStateAlgorithmEntryRecord(id = pendingLedgerEntryPayload!!.id, amount = pendingLedgerEntryPayload!!.amount)


            if (call_AmountOf__p(pendingLedgerEntryPayload!!) > 100) {

            if (try { vLarge = com.sce.forge.runtime.SceChecked.add(vLarge, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='large'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            1 -> {
                // SCE-MAP: static_state_algorithm.scxml:53 :: open :: _transition_1

            if (try { vTotal = com.sce.forge.runtime.SceChecked.take(call_Bump__v(vTotal, 1)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_state_algorithm.scxml:56 :: open :: _transition_2

            if (if (vIds.size < 8) { vIds = vIds + (vLast.id); false } else { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<sce:append target='ids'>: the list already holds its capacity of 8"); true }) {
                return
            }

            if (try { vIdsum = com.sce.forge.runtime.SceChecked.take(call_IdSum__v(vIds)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='idsum'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_state_algorithm.scxml:60 :: open :: _transition_3

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
