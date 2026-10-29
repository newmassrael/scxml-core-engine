// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_record_call.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_record_call.scxml:13 :: _machine

package com.sce.integration.static_record_call

import com.sce.runtime.*
import com.sce.generated.ledger_amount.*
import com.sce.generated.ledger_larger.*
import com.sce.generated.schema_entry.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRecordCallState : State {
    data object Open : StaticRecordCallState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRecordCallEvent : Event {
    sealed interface Ledger : StaticRecordCallEvent {
        data object Entry : Ledger
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticRecordCallPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticRecordCallLedgerEntryPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `ledger.entry`. Consumers inject it via the `raiseLedgerEntry` seam
// on the machine — they never name this class directly.
data class StaticRecordCallLedgerEntryPayload(val id: UInt, val amount: Int)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Entry` datamodel value. */
data class StaticRecordCallEntryRecord(val id: UInt, val amount: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "amount" to SavedValues.of(amount))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticRecordCallEntryRecord = StaticRecordCallEntryRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), amount = SavedValues.int32(SavedValues.field(value, what, "amount"), "$what.amount"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_AmountOf__p(p0: StaticRecordCallLedgerEntryPayload): Int {
    val r = ledgerAmount(SchemaEntryPayload(id = p0.id, amount = p0.amount))
    return r
}
private fun call_AmountOf__v(p0: StaticRecordCallEntryRecord): Int {
    val r = ledgerAmount(SchemaEntryPayload(id = p0.id, amount = p0.amount))
    return r
}
private fun call_Larger__vp(p0: StaticRecordCallEntryRecord, p1: StaticRecordCallLedgerEntryPayload): StaticRecordCallEntryRecord {
    val r = ledgerLarger(SchemaEntryPayload(id = p0.id, amount = p0.amount), SchemaEntryPayload(id = p1.id, amount = p1.amount))
    return StaticRecordCallEntryRecord(id = r.id, amount = r.amount)
}

// --- State Machine (W3C SCXML) ---

class StaticRecordCallStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticRecordCallState, StaticRecordCallEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `best` datamodel variable, published (`sce:direction="out"`). */
    private var vBest: StaticRecordCallEntryRecord = StaticRecordCallEntryRecord(id = 0.toUInt(), amount = 0)

    /** What the published `best` holds. */
    val best: StaticRecordCallEntryRecord get() = vBest
    /** W3C SCXML 5.2: the `large` datamodel variable, published (`sce:direction="out"`). */
    private var vLarge: UInt = 0.toUInt()

    /** What the published `large` holds. */
    val large: UInt get() = vLarge
    /** W3C SCXML 5.2: the `debts` datamodel variable, published (`sce:direction="out"`). */
    private var vDebts: UInt = 0.toUInt()

    /** What the published `debts` holds. */
    val debts: UInt get() = vDebts
    /** W3C SCXML 5.2: the `peaks` datamodel variable, published (`sce:direction="out"`). */
    private var vPeaks: UInt = 0.toUInt()

    /** What the published `peaks` holds. */
    val peaks: UInt get() = vPeaks

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var large: UInt? = null
        var debts: UInt? = null
        var peaks: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.large?.let { vLarge = it }
        params.debts?.let { vDebts = it }
        params.peaks?.let { vPeaks = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val best: StaticRecordCallEntryRecord,
        val large: UInt,
        val debts: UInt,
        val peaks: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRecordCallState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        best = vBest,
        large = vLarge,
        debts = vDebts,
        peaks = vPeaks,
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
        first = "dc127a7411589216f56f7e89ef942578e92878ca89a119099a7221bc3092f957",
        revised = "8e7f755283cf3e3a2dc6d431941d678b939e964d88fffdd9bd9b317c46ed2832",
        format2First = "dc127a7411589216f56f7e89ef942578e92878ca89a119099a7221bc3092f957",
        format2Revised = "8e7f755283cf3e3a2dc6d431941d678b939e964d88fffdd9bd9b317c46ed2832",
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
                    RecordHolder("best", false),
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
            "best" to vBest.toSaved(),
            "large" to SavedValues.of(vLarge),
            "debts" to SavedValues.of(vDebts),
            "peaks" to SavedValues.of(vPeaks),
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
        val saved1 = StaticRecordCallEntryRecord.fromSaved(savedGrown.variable("best"), "best")
        val saved2 = SavedValues.uint32(savedGrown.variable("large"), "large")
        val saved3 = SavedValues.uint32(savedGrown.variable("debts"), "debts")
        val saved4 = SavedValues.uint32(savedGrown.variable("peaks"), "peaks")
        vBest = saved1
        vLarge = saved2
        vDebts = saved3
        vPeaks = saved4
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
        StaticRecordCallEntryRecord.fromSaved(savedGrown.variable("best"), "best")
        SavedValues.uint32(savedGrown.variable("large"), "large")
        SavedValues.uint32(savedGrown.variable("debts"), "debts")
        SavedValues.uint32(savedGrown.variable("peaks"), "peaks")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingLedgerEntryPayload: StaticRecordCallLedgerEntryPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticRecordCallEvent, metadata: EventMetadata) {
        pendingLedgerEntryPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticRecordCallLedgerEntryPayload -> pendingLedgerEntryPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticRecordCallEvent.Ledger.Entry) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingLedgerEntryPayload = StaticRecordCallLedgerEntryPayload(fields.uint32("id"), fields.int32("amount"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `ledger.entry` — binds the event name and the payload field values in one call.
    fun raiseLedgerEntry(id: UInt, amount: Int) {
        send(
            StaticRecordCallEvent.Ledger.Entry,
            EventMetadata(
                type = "external",
                typedPayload = StaticRecordCallLedgerEntryPayload(id, amount),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "amount" to amount))
            )
        )
    }


    override val initialState: StaticRecordCallState = StaticRecordCallState.Open

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
    override val documentInitialTargets: List<EntryTarget<StaticRecordCallState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRecordCallState, HistoryId>> =
            listOf(StateTarget(StaticRecordCallState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<StaticRecordCallState, HistoryId>(
            StaticRecordCallState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRecordCallState? = when (stateId) {
        "open" -> StaticRecordCallState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRecordCallState): String = when (state) {
        is StaticRecordCallState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRecordCallState): Int = when (state) {
        is StaticRecordCallState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRecordCallEvent? = when (name) {
        "ledger.entry" -> StaticRecordCallEvent.Ledger.Entry
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRecordCallEvent): String? = when (event) {
        is StaticRecordCallEvent.Ledger.Entry -> "ledger.entry"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRecordCallState,
        event: StaticRecordCallEvent?
    ): EnabledTransition<StaticRecordCallState, HistoryId>? = when (state) {
        is StaticRecordCallState.Open -> when {
            event is StaticRecordCallEvent.Ledger.Entry -> transitionOpenAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_record_call.scxml:13 :: _machine
    override fun onEntry(state: StaticRecordCallState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRecordCallState.Open -> {
                // SCE-MAP: static_record_call.scxml:27 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_record_call.scxml:13 :: _machine
    override fun onExit(state: StaticRecordCallState) {
        when (state) {
            is StaticRecordCallState.Open -> {
                // SCE-MAP: static_record_call.scxml:27 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_record_call.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticRecordCallState, transitionIndex: Int) {
        when (source) {
        is StaticRecordCallState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_record_call.scxml:28 :: open :: _transition_0
                if (pendingLedgerEntryPayload == null) {
                    return
                }

            vBest = call_Larger__vp(vBest, pendingLedgerEntryPayload!!)


            if (call_AmountOf__p(pendingLedgerEntryPayload!!) > 100) {

            if (try { vLarge = com.sce.forge.runtime.SceChecked.add(vLarge, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }


            if (call_AmountOf__p(pendingLedgerEntryPayload!!) < 0) {

            if (try { vDebts = com.sce.forge.runtime.SceChecked.add(vDebts, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }


            if (call_AmountOf__v(vBest) > 200) {

            if (try { vPeaks = com.sce.forge.runtime.SceChecked.add(vPeaks, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            }
            else -> {}
        }
        }
    }
}
