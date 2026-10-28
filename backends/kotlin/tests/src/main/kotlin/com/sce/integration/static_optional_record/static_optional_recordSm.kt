// SCE-GENERATED — DO NOT EDIT
// source-hash: 8b4828d778fc1da05444ac6a052eb44c820b7a80e1f4075fc52bc6e97e8a070a

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_optional_record.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_optional_record.scxml:25 :: _machine

package com.sce.integration.static_optional_record

import com.sce.runtime.*
import com.sce.generated.item_pick_first.*
import com.sce.generated.schema_item_pick.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticOptionalRecordState : State {
    data object Holding : StaticOptionalRecordState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticOptionalRecordEvent : Event {
    sealed interface Error : StaticOptionalRecordEvent {
        data object Execution : Error
    }
    sealed interface Item : StaticOptionalRecordEvent {
        data object Picked : Item
    }
    sealed interface Pick : StaticOptionalRecordEvent {
        data object Bump : Pick
        data object Clear : Pick
        data object Draft : Pick
        data object Find : Pick
        data object Read : Pick
        data object Second : Pick
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticOptionalRecordPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticOptionalRecordItemPickedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `item.picked`. Consumers inject it via the `raiseItemPicked` seam
// on the machine — they never name this class directly.
data class StaticOptionalRecordItemPickedPayload(val id: UInt, val title: String, val count: Int)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Item` datamodel value. */
data class StaticOptionalRecordItemRecord(val id: UInt, val title: String, val count: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "title" to SavedValues.of(title), "count" to SavedValues.of(count))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticOptionalRecordItemRecord = StaticOptionalRecordItemRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), title = SavedValues.string(SavedValues.field(value, what, "title"), "$what.title", 16), count = SavedValues.int32(SavedValues.field(value, what, "count"), "$what.count"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_FirstAtLeast__v(p0: List<StaticOptionalRecordItemRecord>, p1: Int): StaticOptionalRecordItemRecord? {
    val r = itemPickFirst(p0.map { SchemaItemPickPayload(id = it.id, title = it.title, count = it.count) }, p1)
    return r?.let { StaticOptionalRecordItemRecord(id = it.id, title = it.title, count = it.count) }
}

// --- State Machine (W3C SCXML) ---

class StaticOptionalRecordStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticOptionalRecordState, StaticOptionalRecordEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `pick` datamodel variable, published (`sce:direction="out"`). */
    private var vPick: StaticOptionalRecordItemRecord? = null

    /** What the published `pick` holds. */
    val pick: StaticOptionalRecordItemRecord? get() = vPick
    /** W3C SCXML 5.2: the `found` datamodel variable, published (`sce:direction="out"`). */
    private var vFound: StaticOptionalRecordItemRecord? = null

    /** What the published `found` holds. */
    val found: StaticOptionalRecordItemRecord? get() = vFound
    /** W3C SCXML 5.2: the `draft` datamodel variable, published (`sce:direction="out"`). */
    private var vDraft: StaticOptionalRecordItemRecord = StaticOptionalRecordItemRecord(id = 0.toUInt(), title = "draft", count = 0)

    /** What the published `draft` holds. */
    val draft: StaticOptionalRecordItemRecord get() = vDraft
    /** W3C SCXML 5.2: the `items` datamodel variable, published (`sce:direction="out"`). */
    private var vItems: List<StaticOptionalRecordItemRecord> = emptyList()

    /** What the published `items` holds. */
    val items: List<StaticOptionalRecordItemRecord> get() = vItems
    /** W3C SCXML 5.2: the `title` datamodel variable, published (`sce:direction="out"`). */
    private var vTitle: String = ""

    /** What the published `title` holds. */
    val title: String get() = vTitle
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    private var vCount: Int = 0

    /** What the published `count` holds. */
    val count: Int get() = vCount
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    private var vShown: Int = 0

    /** What the published `shown` holds. */
    val shown: Int get() = vShown
    /** W3C SCXML 5.2: the `known` datamodel variable, published (`sce:direction="out"`). */
    private var vKnown: Boolean = false

    /** What the published `known` holds. */
    val known: Boolean get() = vKnown
    /** W3C SCXML 5.2: the `absent` datamodel variable, published (`sce:direction="out"`). */
    private var vAbsent: Boolean = false

    /** What the published `absent` holds. */
    val absent: Boolean get() = vAbsent
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var title: String? = null
        var count: Int? = null
        var shown: Int? = null
        var known: Boolean? = null
        var absent: Boolean? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.title?.let { vTitle = it }
        params.count?.let { vCount = it }
        params.shown?.let { vShown = it }
        params.known?.let { vKnown = it }
        params.absent?.let { vAbsent = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val pick: StaticOptionalRecordItemRecord?,
        val found: StaticOptionalRecordItemRecord?,
        val draft: StaticOptionalRecordItemRecord,
        val items: List<StaticOptionalRecordItemRecord>,
        val title: String,
        val count: Int,
        val shown: Int,
        val known: Boolean,
        val absent: Boolean,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticOptionalRecordState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        pick = vPick,
        found = vFound,
        draft = vDraft,
        items = vItems,
        title = vTitle,
        count = vCount,
        shown = vShown,
        known = vKnown,
        absent = vAbsent,
        errors = vErrors,
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
        first = "4392a90d18937454f42b4496c0de4f16d5994e9117645033f72fdff9375d9bbf",
        revised = "6bfecb8074e65f88f572cb1e9ccfec46d785c2eea2908d71654fd318a64b213b",
        format2First = "4392a90d18937454f42b4496c0de4f16d5994e9117645033f72fdff9375d9bbf",
        format2Revised = "6bfecb8074e65f88f572cb1e9ccfec46d785c2eea2908d71654fd318a64b213b",
        records = listOf(
            RecordLineage(
                event = "item.picked",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    RecordHolder("pick", false),
                    RecordHolder("found", false),
                    RecordHolder("draft", false),
                    RecordHolder("items", true),
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
            "pick" to SavedValues.optional(vPick) { it.toSaved() },
            "found" to SavedValues.optional(vFound) { it.toSaved() },
            "draft" to vDraft.toSaved(),
            "items" to SavedValues.list(vItems) { it.toSaved() },
            "title" to SavedValues.of(vTitle),
            "count" to SavedValues.of(vCount),
            "shown" to SavedValues.of(vShown),
            "known" to SavedValues.of(vKnown),
            "absent" to SavedValues.of(vAbsent),
            "errors" to SavedValues.of(vErrors),
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
        val saved1 = SavedValues.optional(savedGrown.variable("pick"), "pick") { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("found"), "found") { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        val saved3 = StaticOptionalRecordItemRecord.fromSaved(savedGrown.variable("draft"), "draft")
        val saved4 = SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        val saved5 = SavedValues.string(savedGrown.variable("title"), "title", 16)
        val saved6 = SavedValues.int32(savedGrown.variable("count"), "count")
        val saved7 = SavedValues.int32(savedGrown.variable("shown"), "shown")
        val saved8 = SavedValues.bool(savedGrown.variable("known"), "known")
        val saved9 = SavedValues.bool(savedGrown.variable("absent"), "absent")
        val saved10 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        vPick = saved1
        vFound = saved2
        vDraft = saved3
        vItems = saved4
        vTitle = saved5
        vCount = saved6
        vShown = saved7
        vKnown = saved8
        vAbsent = saved9
        vErrors = saved10
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
        SavedValues.optional(savedGrown.variable("pick"), "pick") { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        SavedValues.optional(savedGrown.variable("found"), "found") { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        StaticOptionalRecordItemRecord.fromSaved(savedGrown.variable("draft"), "draft")
        SavedValues.list(savedGrown.variable("items"), "items", 3) { e, w -> StaticOptionalRecordItemRecord.fromSaved(e, w) }
        SavedValues.string(savedGrown.variable("title"), "title", 16)
        SavedValues.int32(savedGrown.variable("count"), "count")
        SavedValues.int32(savedGrown.variable("shown"), "shown")
        SavedValues.bool(savedGrown.variable("known"), "known")
        SavedValues.bool(savedGrown.variable("absent"), "absent")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingItemPickedPayload: StaticOptionalRecordItemPickedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticOptionalRecordEvent, metadata: EventMetadata) {
        pendingItemPickedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticOptionalRecordItemPickedPayload -> pendingItemPickedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticOptionalRecordEvent.Item.Picked) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingItemPickedPayload = StaticOptionalRecordItemPickedPayload(fields.uint32("id"), fields.string("title"), fields.int32("count"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `item.picked` — binds the event name and the payload field values in one call.
    fun raiseItemPicked(id: UInt, title: String, count: Int) {
        send(
            StaticOptionalRecordEvent.Item.Picked,
            EventMetadata(
                type = "external",
                typedPayload = StaticOptionalRecordItemPickedPayload(id, title, count),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "title" to title, "count" to count))
            )
        )
    }


    override val initialState: StaticOptionalRecordState = StaticOptionalRecordState.Holding

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
    override val documentInitialTargets: List<EntryTarget<StaticOptionalRecordState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticOptionalRecordState, HistoryId>> =
            listOf(StateTarget(StaticOptionalRecordState.Holding))

        // W3C SCXML 3.13: holding's transition 0, as the microstep reads it.
        val transitionHoldingAt0 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 1, as the microstep reads it.
        val transitionHoldingAt1 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 2, as the microstep reads it.
        val transitionHoldingAt2 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 3, as the microstep reads it.
        val transitionHoldingAt3 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 4, as the microstep reads it.
        val transitionHoldingAt4 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 5, as the microstep reads it.
        val transitionHoldingAt5 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 6, as the microstep reads it.
        val transitionHoldingAt6 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: holding's transition 7, as the microstep reads it.
        val transitionHoldingAt7 = EnabledTransition<StaticOptionalRecordState, HistoryId>(
            StaticOptionalRecordState.Holding,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticOptionalRecordState? = when (stateId) {
        "holding" -> StaticOptionalRecordState.Holding
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticOptionalRecordState): String = when (state) {
        is StaticOptionalRecordState.Holding -> "holding"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticOptionalRecordState): Int = when (state) {
        is StaticOptionalRecordState.Holding -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticOptionalRecordEvent? = when (name) {
        "error.execution" -> StaticOptionalRecordEvent.Error.Execution
        "item.picked" -> StaticOptionalRecordEvent.Item.Picked
        "pick.bump" -> StaticOptionalRecordEvent.Pick.Bump
        "pick.clear" -> StaticOptionalRecordEvent.Pick.Clear
        "pick.draft" -> StaticOptionalRecordEvent.Pick.Draft
        "pick.find" -> StaticOptionalRecordEvent.Pick.Find
        "pick.read" -> StaticOptionalRecordEvent.Pick.Read
        "pick.second" -> StaticOptionalRecordEvent.Pick.Second
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticOptionalRecordEvent): String? = when (event) {
        is StaticOptionalRecordEvent.Error.Execution -> "error.execution"
        is StaticOptionalRecordEvent.Item.Picked -> "item.picked"
        is StaticOptionalRecordEvent.Pick.Bump -> "pick.bump"
        is StaticOptionalRecordEvent.Pick.Clear -> "pick.clear"
        is StaticOptionalRecordEvent.Pick.Draft -> "pick.draft"
        is StaticOptionalRecordEvent.Pick.Find -> "pick.find"
        is StaticOptionalRecordEvent.Pick.Read -> "pick.read"
        is StaticOptionalRecordEvent.Pick.Second -> "pick.second"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticOptionalRecordState,
        event: StaticOptionalRecordEvent?
    ): EnabledTransition<StaticOptionalRecordState, HistoryId>? = when (state) {
        is StaticOptionalRecordState.Holding -> when {
            event is StaticOptionalRecordEvent.Item.Picked -> transitionHoldingAt0
            event is StaticOptionalRecordEvent.Pick.Draft -> transitionHoldingAt1
            event is StaticOptionalRecordEvent.Pick.Second -> transitionHoldingAt2
            event is StaticOptionalRecordEvent.Pick.Clear -> transitionHoldingAt3
            event is StaticOptionalRecordEvent.Pick.Read -> transitionHoldingAt4
            event is StaticOptionalRecordEvent.Pick.Bump -> transitionHoldingAt5
            event is StaticOptionalRecordEvent.Pick.Find -> transitionHoldingAt6
            event is StaticOptionalRecordEvent.Error.Execution -> transitionHoldingAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_optional_record.scxml:25 :: _machine
    override fun onEntry(state: StaticOptionalRecordState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticOptionalRecordState.Holding -> {
                // SCE-MAP: static_optional_record.scxml:45 :: holding :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_optional_record.scxml:25 :: _machine
    override fun onExit(state: StaticOptionalRecordState) {
        when (state) {
            is StaticOptionalRecordState.Holding -> {
                // SCE-MAP: static_optional_record.scxml:45 :: holding :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_optional_record.scxml:25 :: _machine
    override fun executeTransitionContent(source: StaticOptionalRecordState, transitionIndex: Int) {
        when (source) {
        is StaticOptionalRecordState.Holding -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_optional_record.scxml:48 :: holding :: _transition_0
                if (pendingItemPickedPayload == null) {
                    return
                }

            if (try { if (vItems.size < 3) { vItems = vItems + (StaticOptionalRecordItemRecord(id = pendingItemPickedPayload!!.id, title = com.sce.forge.runtime.SceChecked.bounded(pendingItemPickedPayload!!.title, 16), count = pendingItemPickedPayload!!.count)); false } else { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<sce:append target='items'>: the list already holds its capacity of 3"); true } } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<sce:append target='items'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vDraft = StaticOptionalRecordItemRecord(id = pendingItemPickedPayload!!.id, title = com.sce.forge.runtime.SceChecked.bounded(pendingItemPickedPayload!!.title, 16), count = pendingItemPickedPayload!!.count); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='draft'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vPick = StaticOptionalRecordItemRecord(id = pendingItemPickedPayload!!.id, title = com.sce.forge.runtime.SceChecked.bounded(pendingItemPickedPayload!!.title, 16), count = pendingItemPickedPayload!!.count); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_optional_record.scxml:54 :: holding :: _transition_1

            vPick = vDraft
            }
            2 -> {
                // SCE-MAP: static_optional_record.scxml:57 :: holding :: _transition_2

            if (try { vPick = com.sce.forge.runtime.SceChecked.at(vItems, (1).toLong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_optional_record.scxml:60 :: holding :: _transition_3

            vPick = null
            }
            4 -> {
                // SCE-MAP: static_optional_record.scxml:64 :: holding :: _transition_4


            if (vPick != null) {

            if (try { vTitle = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceOptional.proven(vPick).title, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vCount = com.sce.forge.runtime.SceOptional.proven(vPick).count
            } else {

            if (try { vTitle = com.sce.forge.runtime.SceChecked.bounded("none", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='title'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vCount = -1
            }

            vKnown = vPick != null && com.sce.forge.runtime.SceOptional.proven(vPick).count > 1

            vAbsent = vPick == null

            vShown = if (vPick != null) com.sce.forge.runtime.SceOptional.proven(vPick).count else 0
            }
            5 -> {
                // SCE-MAP: static_optional_record.scxml:77 :: holding :: _transition_5


            if (vPick != null) {

            if (try { vPick = com.sce.forge.runtime.SceOptional.proven(vPick).copy(count = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceOptional.proven(vPick).count, 1)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='pick.value.count'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vPick = com.sce.forge.runtime.SceOptional.proven(vPick).copy(title = com.sce.forge.runtime.SceChecked.bounded("bumped", 16)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='pick.value.title'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            6 -> {
                // SCE-MAP: static_optional_record.scxml:84 :: holding :: _transition_6

            vFound = call_FirstAtLeast__v(vItems, 5)
            }
            7 -> {
                // SCE-MAP: static_optional_record.scxml:87 :: holding :: _transition_7

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalRecordEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
