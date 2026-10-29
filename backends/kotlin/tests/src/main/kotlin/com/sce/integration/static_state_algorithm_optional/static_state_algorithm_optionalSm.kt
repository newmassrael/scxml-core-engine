// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_state_algorithm_optional.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_state_algorithm_optional.scxml:23 :: _machine

package com.sce.integration.static_state_algorithm_optional

import com.sce.runtime.*
import com.sce.generated.goal_day_sum.*
import com.sce.generated.goal_earliest.*
import com.sce.generated.goal_undated.*
import com.sce.generated.goal_until.*
import com.sce.generated.schema_goal.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticStateAlgorithmOptionalState : State {
    data object Tracking : StaticStateAlgorithmOptionalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticStateAlgorithmOptionalEvent : Event {
    data object Clear : StaticStateAlgorithmOptionalEvent
    sealed interface Error : StaticStateAlgorithmOptionalEvent {
        data object Execution : Error
    }
    sealed interface Goal : StaticStateAlgorithmOptionalEvent {
        data object Added : Goal
    }
    sealed interface Until : StaticStateAlgorithmOptionalEvent {
        data object At : Until
        data object None : Until
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticStateAlgorithmOptionalPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticStateAlgorithmOptionalGoalAddedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `goal.added`. Consumers inject it via the `raiseGoalAdded` seam
// on the machine — they never name this class directly.
data class StaticStateAlgorithmOptionalGoalAddedPayload(val id: UInt, val day: Long?, val weight: Int)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: a `record:Goal` datamodel value. */
data class StaticStateAlgorithmOptionalGoalRecord(val id: UInt, val day: Long?, val weight: Int) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("id" to SavedValues.of(id), "day" to SavedValues.optional(day) { SavedValues.of(it) }, "weight" to SavedValues.of(weight))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticStateAlgorithmOptionalGoalRecord = StaticStateAlgorithmOptionalGoalRecord(id = SavedValues.uint32(SavedValues.field(value, what, "id"), "$what.id"), day = SavedValues.optional(SavedValues.field(value, what, "day"), "$what.day") { e, w -> SavedValues.int64(e, w) }, weight = SavedValues.int32(SavedValues.field(value, what, "weight"), "$what.weight"))
    }
}
// ── SCE Accepted Subset §2.15: calls of an algorithm that takes or returns state ──
private fun call_DaySum__v(p0: List<StaticStateAlgorithmOptionalGoalRecord>, p1: Long): com.sce.forge.runtime.AlgorithmResult<Long> =
    when (val r = goalDaySum(p0.map { SchemaGoalPayload(id = it.id, day = it.day, weight = it.weight) }, p1)) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }
private fun call_Earliest__v(p0: List<StaticStateAlgorithmOptionalGoalRecord>): Long? {
    val r = goalEarliest(p0.map { SchemaGoalPayload(id = it.id, day = it.day, weight = it.weight) })
    return r
}
private fun call_Undated__v(p0: List<StaticStateAlgorithmOptionalGoalRecord>): com.sce.forge.runtime.AlgorithmResult<UInt> =
    when (val r = goalUndated(p0.map { SchemaGoalPayload(id = it.id, day = it.day, weight = it.weight) })) {
        is com.sce.forge.runtime.AlgorithmResult.Ok -> com.sce.forge.runtime.AlgorithmResult.Ok(r.value)
        is com.sce.forge.runtime.AlgorithmResult.Failed -> r
    }

// --- State Machine (W3C SCXML) ---

class StaticStateAlgorithmOptionalStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticStateAlgorithmOptionalState, StaticStateAlgorithmOptionalEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `goals` datamodel variable, published (`sce:direction="out"`). */
    private var vGoals: List<StaticStateAlgorithmOptionalGoalRecord> = emptyList()

    /** What the published `goals` holds. */
    val goals: List<StaticStateAlgorithmOptionalGoalRecord> get() = vGoals
    /** W3C SCXML 5.2: the `draft` datamodel variable, the machine's own. */
    private var vDraft: StaticStateAlgorithmOptionalGoalRecord = StaticStateAlgorithmOptionalGoalRecord(id = 0.toUInt(), day = null, weight = 0)
    /** W3C SCXML 5.2: the `today` datamodel variable, the machine's own. */
    private var vToday: Long = 100
    /** W3C SCXML 5.2: the `first` datamodel variable, published (`sce:direction="out"`). */
    private var vFirst: Long? = null

    /** What the published `first` holds. */
    val first: Long? get() = vFirst
    /** W3C SCXML 5.2: the `left` datamodel variable, published (`sce:direction="out"`). */
    private var vLeft: Long? = 0

    /** What the published `left` holds. */
    val left: Long? get() = vLeft
    /** W3C SCXML 5.2: the `undated` datamodel variable, published (`sce:direction="out"`). */
    private var vUndated: UInt = 0.toUInt()

    /** What the published `undated` holds. */
    val undated: UInt get() = vUndated
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    private var vTotal: Long = 0

    /** What the published `total` holds. */
    val total: Long get() = vTotal
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
        var today: Long? = null
        var first: Long? = null
        var left: Long? = null
        var undated: UInt? = null
        var total: Long? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.today?.let { vToday = it }
        params.first?.let { vFirst = it }
        params.left?.let { vLeft = it }
        params.undated?.let { vUndated = it }
        params.total?.let { vTotal = it }
        params.errors?.let { vErrors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val goals: List<StaticStateAlgorithmOptionalGoalRecord>,
        val first: Long?,
        val left: Long?,
        val undated: UInt,
        val total: Long,
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
        val configuration: Set<StaticStateAlgorithmOptionalState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        goals = vGoals,
        first = vFirst,
        left = vLeft,
        undated = vUndated,
        total = vTotal,
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
        first = "7307b7bfcadb8bdd01e279cefed31fec3445cce0265d74b56d005a0220d09ef9",
        revised = "2ac037b6dacd401bd691f3d511299403df4ebfeae17964fb4fc8733f14099c4c",
        format2First = "7307b7bfcadb8bdd01e279cefed31fec3445cce0265d74b56d005a0220d09ef9",
        format2Revised = "2ac037b6dacd401bd691f3d511299403df4ebfeae17964fb4fc8733f14099c4c",
        records = listOf(
            RecordLineage(
                event = "goal.added",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("goals", true),
                    RecordHolder("draft", false),
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
            "goals" to SavedValues.list(vGoals) { it.toSaved() },
            "draft" to vDraft.toSaved(),
            "today" to SavedValues.of(vToday),
            "first" to SavedValues.optional(vFirst) { SavedValues.of(it) },
            "left" to SavedValues.optional(vLeft) { SavedValues.of(it) },
            "undated" to SavedValues.of(vUndated),
            "total" to SavedValues.of(vTotal),
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
        val saved1 = SavedValues.list(savedGrown.variable("goals"), "goals", 4) { e, w -> StaticStateAlgorithmOptionalGoalRecord.fromSaved(e, w) }
        val saved2 = StaticStateAlgorithmOptionalGoalRecord.fromSaved(savedGrown.variable("draft"), "draft")
        val saved3 = SavedValues.int64(savedGrown.variable("today"), "today")
        val saved4 = SavedValues.optional(savedGrown.variable("first"), "first") { e, w -> SavedValues.int64(e, w) }
        val saved5 = SavedValues.optional(savedGrown.variable("left"), "left") { e, w -> SavedValues.int64(e, w) }
        val saved6 = SavedValues.uint32(savedGrown.variable("undated"), "undated")
        val saved7 = SavedValues.int64(savedGrown.variable("total"), "total")
        val saved8 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        vGoals = saved1
        vDraft = saved2
        vToday = saved3
        vFirst = saved4
        vLeft = saved5
        vUndated = saved6
        vTotal = saved7
        vErrors = saved8
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
        SavedValues.list(savedGrown.variable("goals"), "goals", 4) { e, w -> StaticStateAlgorithmOptionalGoalRecord.fromSaved(e, w) }
        StaticStateAlgorithmOptionalGoalRecord.fromSaved(savedGrown.variable("draft"), "draft")
        SavedValues.int64(savedGrown.variable("today"), "today")
        SavedValues.optional(savedGrown.variable("first"), "first") { e, w -> SavedValues.int64(e, w) }
        SavedValues.optional(savedGrown.variable("left"), "left") { e, w -> SavedValues.int64(e, w) }
        SavedValues.uint32(savedGrown.variable("undated"), "undated")
        SavedValues.int64(savedGrown.variable("total"), "total")
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingGoalAddedPayload: StaticStateAlgorithmOptionalGoalAddedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticStateAlgorithmOptionalEvent, metadata: EventMetadata) {
        pendingGoalAddedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticStateAlgorithmOptionalGoalAddedPayload -> pendingGoalAddedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticStateAlgorithmOptionalEvent.Goal.Added) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingGoalAddedPayload = StaticStateAlgorithmOptionalGoalAddedPayload(fields.uint32("id"), fields.optional("day") { _ -> fields.int64("day") }, fields.int32("weight"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `goal.added` — binds the event name and the payload field values in one call.
    fun raiseGoalAdded(id: UInt, day: Long?, weight: Int) {
        send(
            StaticStateAlgorithmOptionalEvent.Goal.Added,
            EventMetadata(
                type = "external",
                typedPayload = StaticStateAlgorithmOptionalGoalAddedPayload(id, day, weight),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "day" to day, "weight" to weight))
            )
        )
    }


    override val initialState: StaticStateAlgorithmOptionalState = StaticStateAlgorithmOptionalState.Tracking

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
    override val documentInitialTargets: List<EntryTarget<StaticStateAlgorithmOptionalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticStateAlgorithmOptionalState, HistoryId>> =
            listOf(StateTarget(StaticStateAlgorithmOptionalState.Tracking))

        // W3C SCXML 3.13: tracking's transition 0, as the microstep reads it.
        val transitionTrackingAt0 = EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>(
            StaticStateAlgorithmOptionalState.Tracking,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 1, as the microstep reads it.
        val transitionTrackingAt1 = EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>(
            StaticStateAlgorithmOptionalState.Tracking,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 2, as the microstep reads it.
        val transitionTrackingAt2 = EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>(
            StaticStateAlgorithmOptionalState.Tracking,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 3, as the microstep reads it.
        val transitionTrackingAt3 = EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>(
            StaticStateAlgorithmOptionalState.Tracking,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: tracking's transition 4, as the microstep reads it.
        val transitionTrackingAt4 = EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>(
            StaticStateAlgorithmOptionalState.Tracking,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticStateAlgorithmOptionalState? = when (stateId) {
        "tracking" -> StaticStateAlgorithmOptionalState.Tracking
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticStateAlgorithmOptionalState): String = when (state) {
        is StaticStateAlgorithmOptionalState.Tracking -> "tracking"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticStateAlgorithmOptionalState): Int = when (state) {
        is StaticStateAlgorithmOptionalState.Tracking -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticStateAlgorithmOptionalEvent? = when (name) {
        "clear" -> StaticStateAlgorithmOptionalEvent.Clear
        "error.execution" -> StaticStateAlgorithmOptionalEvent.Error.Execution
        "goal.added" -> StaticStateAlgorithmOptionalEvent.Goal.Added
        "until.at" -> StaticStateAlgorithmOptionalEvent.Until.At
        "until.none" -> StaticStateAlgorithmOptionalEvent.Until.None
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticStateAlgorithmOptionalEvent): String? = when (event) {
        is StaticStateAlgorithmOptionalEvent.Clear -> "clear"
        is StaticStateAlgorithmOptionalEvent.Error.Execution -> "error.execution"
        is StaticStateAlgorithmOptionalEvent.Goal.Added -> "goal.added"
        is StaticStateAlgorithmOptionalEvent.Until.At -> "until.at"
        is StaticStateAlgorithmOptionalEvent.Until.None -> "until.none"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticStateAlgorithmOptionalState,
        event: StaticStateAlgorithmOptionalEvent?
    ): EnabledTransition<StaticStateAlgorithmOptionalState, HistoryId>? = when (state) {
        is StaticStateAlgorithmOptionalState.Tracking -> when {
            event is StaticStateAlgorithmOptionalEvent.Goal.Added -> transitionTrackingAt0
            event is StaticStateAlgorithmOptionalEvent.Until.None -> transitionTrackingAt1
            event is StaticStateAlgorithmOptionalEvent.Until.At -> transitionTrackingAt2
            event is StaticStateAlgorithmOptionalEvent.Clear -> transitionTrackingAt3
            event is StaticStateAlgorithmOptionalEvent.Error.Execution -> transitionTrackingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_state_algorithm_optional.scxml:23 :: _machine
    override fun onEntry(state: StaticStateAlgorithmOptionalState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticStateAlgorithmOptionalState.Tracking -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:44 :: tracking :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_state_algorithm_optional.scxml:23 :: _machine
    override fun onExit(state: StaticStateAlgorithmOptionalState) {
        when (state) {
            is StaticStateAlgorithmOptionalState.Tracking -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:44 :: tracking :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_state_algorithm_optional.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticStateAlgorithmOptionalState, transitionIndex: Int) {
        when (source) {
        is StaticStateAlgorithmOptionalState.Tracking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:45 :: tracking :: _transition_0
                if (pendingGoalAddedPayload == null) {
                    return
                }

            vDraft = StaticStateAlgorithmOptionalGoalRecord(id = pendingGoalAddedPayload!!.id, day = pendingGoalAddedPayload!!.day, weight = pendingGoalAddedPayload!!.weight)

            if (if (vGoals.size < 4) { vGoals = vGoals + (vDraft); false } else { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<sce:append target='goals'>: the list already holds its capacity of 4"); true }) {
                return
            }

            vFirst = call_Earliest__v(vGoals)

            if (try { vLeft = com.sce.forge.runtime.SceChecked.take(goalUntil(vToday, vFirst)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='left'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vUndated = com.sce.forge.runtime.SceChecked.take(call_Undated__v(vGoals)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='undated'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTotal = com.sce.forge.runtime.SceChecked.take(call_DaySum__v(vGoals, 0)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:53 :: tracking :: _transition_1

            if (try { vLeft = com.sce.forge.runtime.SceChecked.take(goalUntil(vToday, null)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='left'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:56 :: tracking :: _transition_2

            if (try { vLeft = com.sce.forge.runtime.SceChecked.take(goalUntil(vToday, 150L)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='left'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:59 :: tracking :: _transition_3

            vGoals = emptyList()

            vFirst = call_Earliest__v(vGoals)

            if (try { vLeft = com.sce.forge.runtime.SceChecked.take(goalUntil(vToday, vFirst)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='left'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vUndated = com.sce.forge.runtime.SceChecked.take(call_Undated__v(vGoals)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='undated'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { vTotal = com.sce.forge.runtime.SceChecked.take(call_DaySum__v(vGoals, 0)); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_state_algorithm_optional.scxml:66 :: tracking :: _transition_4

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticStateAlgorithmOptionalEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
