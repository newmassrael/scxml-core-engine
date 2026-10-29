// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_enum_number.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_enum_number.scxml:19 :: _machine

package com.sce.integration.static_enum_number

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticEnumNumberState : State {
    data object Board : StaticEnumNumberState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticEnumNumberEvent : Event {
    data object Check : StaticEnumNumberEvent
    sealed interface Error : StaticEnumNumberEvent {
        data object Execution : Error
    }
    sealed interface Item : StaticEnumNumberEvent {
        data object Moved : Item
    }
    sealed interface Status : StaticEnumNumberEvent {
        data object Number : Status
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticEnumNumberPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticEnumNumberItemMovedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `item.moved`. Consumers inject it via the `raiseItemMoved` seam
// on the machine — they never name this class directly.
data class StaticEnumNumberItemMovedPayload(val id: UInt, val status: StaticEnumNumberStatusEnum)

// StaticEnumNumberStatusNumberPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `status.number`. Consumers inject it via the `raiseStatusNumber` seam
// on the machine — they never name this class directly.
data class StaticEnumNumberStatusNumberPayload(val number: Long)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:Status` datamodel value. */
enum class StaticEnumNumberStatusEnum(val declaredName: String) {
    OPEN("open"),
    IN_PROGRESS("in_progress"),
    DONE("done");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        OPEN -> 2u.toUByte()
        IN_PROGRESS -> 5u.toUByte()
        DONE -> 7u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticEnumNumberStatusEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of Status")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticEnumNumberStatusEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticEnumNumberStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticEnumNumberState, StaticEnumNumberEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `number` datamodel variable, published (`sce:direction="out"`). */
    private var vNumber: Long = 0

    /** What the published `number` holds. */
    val number: Long get() = vNumber
    /** W3C SCXML 5.2: the `status` datamodel variable, published (`sce:direction="out"`). */
    private var vStatus: StaticEnumNumberStatusEnum = StaticEnumNumberStatusEnum.OPEN

    /** What the published `status` holds. */
    val status: StaticEnumNumberStatusEnum get() = vStatus
    /** W3C SCXML 5.2: the `checks` datamodel variable, published (`sce:direction="out"`). */
    private var vChecks: UInt = 0.toUInt()

    /** What the published `checks` holds. */
    val checks: UInt get() = vChecks
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
        var number: Long? = null
        var checks: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.number?.let { vNumber = it }
        params.checks?.let { vChecks = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val number: Long,
        val status: StaticEnumNumberStatusEnum,
        val checks: UInt,
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
        val configuration: Set<StaticEnumNumberState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        number = vNumber,
        status = vStatus,
        checks = vChecks,
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
        first = "0f8485a669758b7a5073da4026dd97bacd7efa02082fd04b6d3328f819136dc3",
        revised = "60b9a8563c43e410203cc6d326d4621c05c5e2e84f609a2325743588d9b77094",
        format2First = "63ac68ae088325edeec1675143c366e81754d7ab2eb2ad55743b258844e1215c",
        format2Revised = "fa6bfd376f19a7cee4db623937dbeb9e646051e24b0a7ce3792120145a761e8a",
        records = listOf(
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_item_status.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("status", null, false),
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
    fun save(wallNowMs: Long): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "number" to SavedValues.of(vNumber),
            "status" to vStatus.toSaved(),
            "checks" to SavedValues.of(vChecks),
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
        val saved1 = SavedValues.int64(savedGrown.variable("number"), "number")
        val saved2 = StaticEnumNumberStatusEnum.fromSaved(savedGrown.variable("status"), "status")
        val saved3 = SavedValues.uint32(savedGrown.variable("checks"), "checks")
        val saved4 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vNumber = saved1
        vStatus = saved2
        vChecks = saved3
        vRefusals = saved4
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
        SavedValues.int64(savedGrown.variable("number"), "number")
        StaticEnumNumberStatusEnum.fromSaved(savedGrown.variable("status"), "status")
        SavedValues.uint32(savedGrown.variable("checks"), "checks")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingItemMovedPayload: StaticEnumNumberItemMovedPayload? = null
    private var pendingStatusNumberPayload: StaticEnumNumberStatusNumberPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticEnumNumberEvent, metadata: EventMetadata) {
        pendingItemMovedPayload = null
        pendingStatusNumberPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticEnumNumberItemMovedPayload -> pendingItemMovedPayload = tp
            is StaticEnumNumberStatusNumberPayload -> pendingStatusNumberPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticEnumNumberEvent.Item.Moved) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingItemMovedPayload = StaticEnumNumberItemMovedPayload(fields.uint32("id"), fields.string("status").let { name -> StaticEnumNumberStatusEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'status' ($name) is not a variant of Status") })
                } else if (event == StaticEnumNumberEvent.Status.Number) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingStatusNumberPayload = StaticEnumNumberStatusNumberPayload(fields.int64("number"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `item.moved` — binds the event name and the payload field values in one call.
    fun raiseItemMoved(id: UInt, status: StaticEnumNumberStatusEnum) {
        send(
            StaticEnumNumberEvent.Item.Moved,
            EventMetadata(
                type = "external",
                typedPayload = StaticEnumNumberItemMovedPayload(id, status),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "status" to status.declaredName))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `status.number` — binds the event name and the payload field values in one call.
    fun raiseStatusNumber(number: Long) {
        send(
            StaticEnumNumberEvent.Status.Number,
            EventMetadata(
                type = "external",
                typedPayload = StaticEnumNumberStatusNumberPayload(number),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("number" to number))
            )
        )
    }


    override val initialState: StaticEnumNumberState = StaticEnumNumberState.Board

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
    override val documentInitialTargets: List<EntryTarget<StaticEnumNumberState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticEnumNumberState, HistoryId>> =
            listOf(StateTarget(StaticEnumNumberState.Board))

        // W3C SCXML 3.13: board's transition 0, as the microstep reads it.
        val transitionBoardAt0 = EnabledTransition<StaticEnumNumberState, HistoryId>(
            StaticEnumNumberState.Board,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 1, as the microstep reads it.
        val transitionBoardAt1 = EnabledTransition<StaticEnumNumberState, HistoryId>(
            StaticEnumNumberState.Board,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 2, as the microstep reads it.
        val transitionBoardAt2 = EnabledTransition<StaticEnumNumberState, HistoryId>(
            StaticEnumNumberState.Board,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: board's transition 3, as the microstep reads it.
        val transitionBoardAt3 = EnabledTransition<StaticEnumNumberState, HistoryId>(
            StaticEnumNumberState.Board,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticEnumNumberState? = when (stateId) {
        "board" -> StaticEnumNumberState.Board
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticEnumNumberState): String = when (state) {
        is StaticEnumNumberState.Board -> "board"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticEnumNumberState): Int = when (state) {
        is StaticEnumNumberState.Board -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticEnumNumberEvent? = when (name) {
        "check" -> StaticEnumNumberEvent.Check
        "error.execution" -> StaticEnumNumberEvent.Error.Execution
        "item.moved" -> StaticEnumNumberEvent.Item.Moved
        "status.number" -> StaticEnumNumberEvent.Status.Number
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticEnumNumberEvent): String? = when (event) {
        is StaticEnumNumberEvent.Check -> "check"
        is StaticEnumNumberEvent.Error.Execution -> "error.execution"
        is StaticEnumNumberEvent.Item.Moved -> "item.moved"
        is StaticEnumNumberEvent.Status.Number -> "status.number"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticEnumNumberState,
        event: StaticEnumNumberEvent?
    ): EnabledTransition<StaticEnumNumberState, HistoryId>? = when (state) {
        is StaticEnumNumberState.Board -> when {
            event is StaticEnumNumberEvent.Item.Moved -> transitionBoardAt0
            event is StaticEnumNumberEvent.Status.Number -> transitionBoardAt1
            event is StaticEnumNumberEvent.Check && (StaticEnumNumberStatusEnum::toUnderlying)(vStatus) == 5.toUByte() -> transitionBoardAt2
            event is StaticEnumNumberEvent.Error.Execution -> transitionBoardAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_enum_number.scxml:19 :: _machine
    override fun onEntry(state: StaticEnumNumberState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticEnumNumberState.Board -> {
                // SCE-MAP: static_enum_number.scxml:30 :: board :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_enum_number.scxml:19 :: _machine
    override fun onExit(state: StaticEnumNumberState) {
        when (state) {
            is StaticEnumNumberState.Board -> {
                // SCE-MAP: static_enum_number.scxml:30 :: board :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_enum_number.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticEnumNumberState, transitionIndex: Int) {
        when (source) {
        is StaticEnumNumberState.Board -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_enum_number.scxml:31 :: board :: _transition_0
                if (pendingItemMovedPayload == null) {
                    return
                }

            vNumber = (StaticEnumNumberStatusEnum::toUnderlying)(pendingItemMovedPayload!!.status).toLong()
            }
            1 -> {
                // SCE-MAP: static_enum_number.scxml:34 :: board :: _transition_1
                if (pendingStatusNumberPayload == null) {
                    return
                }

            if (try { vStatus = com.sce.forge.runtime.SceChecked.take(com.sce.forge.runtime.SceChecked.found(StaticEnumNumberStatusEnum.fromUnderlying(com.sce.forge.runtime.SceChecked.narrowToUByte(pendingStatusNumberPayload!!.number)))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticEnumNumberEvent.Error.Execution, "<assign location='status'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_enum_number.scxml:37 :: board :: _transition_2

            if (try { vChecks = com.sce.forge.runtime.SceChecked.add(vChecks, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticEnumNumberEvent.Error.Execution, "<assign location='checks'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_enum_number.scxml:40 :: board :: _transition_3

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticEnumNumberEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
