// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_text_record.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_text_record.scxml:19 :: _machine

package com.sce.integration.static_text_record

import com.sce.runtime.*
import com.sce.generated.text_slot.*
import com.sce.generated.text_slot_holds.*
import com.sce.generated.text_slot_latest.*
import com.sce.generated.text_slot_retitled.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticTextRecordState : State {
    data object Open : StaticTextRecordState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticTextRecordEvent : Event {
    sealed interface Error : StaticTextRecordEvent {
        data object Execution : Error
    }
    data object Retitle : StaticTextRecordEvent
    sealed interface Text : StaticTextRecordEvent {
        data object Slot : Text
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticTextRecordPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticTextRecordTextSlotPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `text.slot`. Consumers inject it via the `raiseTextSlot` seam
// on the machine — they never name this class directly.
data class StaticTextRecordTextSlotPayload(val slot: UInt, val version: Long, val text: String)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Slot` datamodel value. */
data class StaticTextRecordSlotRecord(val slot: UInt, val version: Long, val text: String) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("slot" to SavedValues.of(slot), "version" to SavedValues.of(version), "text" to SavedValues.of(text))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticTextRecordSlotRecord = StaticTextRecordSlotRecord(slot = SavedValues.uint32(SavedValues.field(value, what, "slot"), "$what.slot"), version = SavedValues.int64(SavedValues.field(value, what, "version"), "$what.version"), text = SavedValues.string(SavedValues.field(value, what, "text"), "$what.text", 32))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_Holds__vv(p0: List<StaticTextRecordSlotRecord>, p1: StaticTextRecordSlotRecord): Boolean {
    val r = textSlotHolds(p0.map { TextSlotPayload(slot = it.slot, version = it.version, text = it.text) }, TextSlotPayload(slot = p1.slot, version = p1.version, text = p1.text))
    return r
}
private fun call_Latest__vv(p0: StaticTextRecordSlotRecord, p1: StaticTextRecordSlotRecord): StaticTextRecordSlotRecord {
    val r = textSlotLatest(TextSlotPayload(slot = p0.slot, version = p0.version, text = p0.text), TextSlotPayload(slot = p1.slot, version = p1.version, text = p1.text))
    return StaticTextRecordSlotRecord(slot = r.slot, version = r.version, text = r.text)
}
private fun call_Retitled__v(p0: List<StaticTextRecordSlotRecord>): com.sce.forge.runtime.AlgorithmResult<List<StaticTextRecordSlotRecord>> =
    when (val r = textSlotRetitled(p0.map { TextSlotPayload(slot = it.slot, version = it.version, text = it.text) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value.map { StaticTextRecordSlotRecord(slot = it.slot, version = it.version, text = it.text) })
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class StaticTextRecordStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticTextRecordState, StaticTextRecordEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `slots` datamodel variable, published (`sce:direction="out"`). */
    private var vSlots: List<StaticTextRecordSlotRecord> = emptyList()

    /** What the published `slots` holds. */
    val slots: List<StaticTextRecordSlotRecord> get() = vSlots
    /** W3C SCXML 5.2: the `held` datamodel variable, published (`sce:direction="out"`). */
    private var vHeld: StaticTextRecordSlotRecord = StaticTextRecordSlotRecord(slot = 1.toUInt(), version = 0, text = "start")

    /** What the published `held` holds. */
    val held: StaticTextRecordSlotRecord get() = vHeld
    /** W3C SCXML 5.2: the `incoming` datamodel variable, published (`sce:direction="out"`). */
    private var vIncoming: StaticTextRecordSlotRecord = StaticTextRecordSlotRecord(slot = 1.toUInt(), version = 0, text = "")

    /** What the published `incoming` holds. */
    val incoming: StaticTextRecordSlotRecord get() = vIncoming
    /** W3C SCXML 5.2: the `known` datamodel variable, published (`sce:direction="out"`). */
    private var vKnown: UInt = 0.toUInt()

    /** What the published `known` holds. */
    val known: UInt get() = vKnown
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
        var known: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.known?.let { vKnown = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val slots: List<StaticTextRecordSlotRecord>,
        val held: StaticTextRecordSlotRecord,
        val incoming: StaticTextRecordSlotRecord,
        val known: UInt,
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
        val configuration: Set<StaticTextRecordState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        slots = vSlots,
        held = vHeld,
        incoming = vIncoming,
        known = vKnown,
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
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "58fff1e777041e7697c25e2df69b4fe0c77d445be391d30ae03c3f285ae9076d",
        revised = "2cde5acaac7e84bf9fcd80e0d7634ff9754198115b76527014df8fd4758406bf",
        format2First = "58fff1e777041e7697c25e2df69b4fe0c77d445be391d30ae03c3f285ae9076d",
        format2Revised = "2cde5acaac7e84bf9fcd80e0d7634ff9754198115b76527014df8fd4758406bf",
        records = listOf(
            RecordLineage(
                event = "text.slot",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("slots", true),
                    RecordHolder("held", false),
                    RecordHolder("incoming", false),
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
            "slots" to SavedValues.list(vSlots) { it.toSaved() },
            "held" to vHeld.toSaved(),
            "incoming" to vIncoming.toSaved(),
            "known" to SavedValues.of(vKnown),
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
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val saved1 = SavedValues.list(savedGrown.variable("slots"), "slots", 16) { e, w -> StaticTextRecordSlotRecord.fromSaved(e, w) }
        val saved2 = StaticTextRecordSlotRecord.fromSaved(savedGrown.variable("held"), "held")
        val saved3 = StaticTextRecordSlotRecord.fromSaved(savedGrown.variable("incoming"), "incoming")
        val saved4 = SavedValues.uint32(savedGrown.variable("known"), "known")
        val saved5 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vSlots = saved1
        vHeld = saved2
        vIncoming = saved3
        vKnown = saved4
        vRefusals = saved5
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
        SavedValues.list(savedGrown.variable("slots"), "slots", 16) { e, w -> StaticTextRecordSlotRecord.fromSaved(e, w) }
        StaticTextRecordSlotRecord.fromSaved(savedGrown.variable("held"), "held")
        StaticTextRecordSlotRecord.fromSaved(savedGrown.variable("incoming"), "incoming")
        SavedValues.uint32(savedGrown.variable("known"), "known")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingTextSlotPayload: StaticTextRecordTextSlotPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticTextRecordEvent, metadata: EventMetadata) {
        pendingTextSlotPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticTextRecordTextSlotPayload -> pendingTextSlotPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticTextRecordEvent.Text.Slot) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingTextSlotPayload = StaticTextRecordTextSlotPayload(fields.uint32("slot"), fields.int64("version"), fields.string("text"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `text.slot` — binds the event name and the payload field values in one call.
    fun raiseTextSlot(slot: UInt, version: Long, text: String) {
        send(
            StaticTextRecordEvent.Text.Slot,
            EventMetadata(
                type = "external",
                typedPayload = StaticTextRecordTextSlotPayload(slot, version, text),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("slot" to slot, "version" to version, "text" to text))
            )
        )
    }


    override val initialState: StaticTextRecordState = StaticTextRecordState.Open

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
    override val documentInitialTargets: List<EntryTarget<StaticTextRecordState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticTextRecordState, HistoryId>> =
            listOf(StateTarget(StaticTextRecordState.Open))

        // W3C SCXML 3.13: open's transition 0, as the microstep reads it.
        val transitionOpenAt0 = EnabledTransition<StaticTextRecordState, HistoryId>(
            StaticTextRecordState.Open,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 1, as the microstep reads it.
        val transitionOpenAt1 = EnabledTransition<StaticTextRecordState, HistoryId>(
            StaticTextRecordState.Open,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: open's transition 2, as the microstep reads it.
        val transitionOpenAt2 = EnabledTransition<StaticTextRecordState, HistoryId>(
            StaticTextRecordState.Open,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticTextRecordState? = when (stateId) {
        "open" -> StaticTextRecordState.Open
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticTextRecordState): String = when (state) {
        is StaticTextRecordState.Open -> "open"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticTextRecordState): Int = when (state) {
        is StaticTextRecordState.Open -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticTextRecordEvent? = when (name) {
        "error.execution" -> StaticTextRecordEvent.Error.Execution
        "retitle" -> StaticTextRecordEvent.Retitle
        "text.slot" -> StaticTextRecordEvent.Text.Slot
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticTextRecordEvent): String? = when (event) {
        is StaticTextRecordEvent.Error.Execution -> "error.execution"
        is StaticTextRecordEvent.Retitle -> "retitle"
        is StaticTextRecordEvent.Text.Slot -> "text.slot"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticTextRecordState,
        event: StaticTextRecordEvent?
    ): EnabledTransition<StaticTextRecordState, HistoryId>? = when (state) {
        is StaticTextRecordState.Open -> when {
            event is StaticTextRecordEvent.Text.Slot -> transitionOpenAt0
            event is StaticTextRecordEvent.Retitle -> transitionOpenAt1
            event is StaticTextRecordEvent.Error.Execution -> transitionOpenAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_text_record.scxml:19 :: _machine
    override fun onEntry(state: StaticTextRecordState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticTextRecordState.Open -> {
                // SCE-MAP: static_text_record.scxml:40 :: open :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_text_record.scxml:19 :: _machine
    override fun onExit(state: StaticTextRecordState) {
        when (state) {
            is StaticTextRecordState.Open -> {
                // SCE-MAP: static_text_record.scxml:40 :: open :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_text_record.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticTextRecordState, transitionIndex: Int) {
        when (source) {
        is StaticTextRecordState.Open -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_text_record.scxml:41 :: open :: _transition_0
                if (pendingTextSlotPayload == null) {
                    return
                }

            if (try { vIncoming = StaticTextRecordSlotRecord(slot = pendingTextSlotPayload!!.slot, version = pendingTextSlotPayload!!.version, text = com.sce.forge.runtime.SceChecked.bounded(pendingTextSlotPayload!!.text, 32)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTextRecordEvent.Error.Execution, "<assign location='incoming'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHeld = call_Latest__vv(vHeld, vIncoming)


            if (call_Holds__vv(vSlots, vIncoming)) {

            if (try { vKnown = com.sce.forge.runtime.SceChecked.add(vKnown, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTextRecordEvent.Error.Execution, "<assign location='known'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (if (vSlots.size < 16) { vSlots = vSlots + (vIncoming); false } else { raisePlatformError(StaticTextRecordEvent.Error.Execution, "<sce:append target='slots'>: the list already holds its capacity of 16"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_text_record.scxml:49 :: open :: _transition_1

            if (try { vSlots = com.sce.forge.runtime.SceChecked.within(com.sce.forge.runtime.SceChecked.take(call_Retitled__v(vSlots)), 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTextRecordEvent.Error.Execution, "<assign location='slots'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_text_record.scxml:52 :: open :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTextRecordEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
