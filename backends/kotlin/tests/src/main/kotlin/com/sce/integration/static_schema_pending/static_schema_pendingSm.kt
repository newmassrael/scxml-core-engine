// SCE-GENERATED — DO NOT EDIT
// source-hash: 047dc621dfd01b6348575becd26162c58fa20fa7680f076526766f91e97d2c5f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_schema_pending.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_schema_pending.scxml:17 :: _machine

package com.sce.integration.static_schema_pending

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSchemaPendingState : State {
    data object Waiting : StaticSchemaPendingState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSchemaPendingEvent : Event {
    data object Arm : StaticSchemaPendingEvent
    sealed interface Error : StaticSchemaPendingEvent {
        data object Execution : Error
    }
    sealed interface Signal : StaticSchemaPendingEvent {
        data object Raised : Signal
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticSchemaPendingPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticSchemaPendingSignalRaisedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `signal.raised`. Consumers inject it via the `raiseSignalRaised` seam
// on the machine — they never name this class directly.
data class StaticSchemaPendingSignalRaisedPayload(val level: StaticSchemaPendingSignalLevelEnum, val count: UShort, val urgent: Boolean)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:SignalLevel` datamodel value. */
enum class StaticSchemaPendingSignalLevelEnum(val declaredName: String) {
    LOW("low"),
    HIGH("high"),
    CRITICAL("critical");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        LOW -> 0u.toUByte()
        HIGH -> 1u.toUByte()
        CRITICAL -> 2u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSchemaPendingSignalLevelEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of SignalLevel")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticSchemaPendingSignalLevelEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Signal` datamodel value. */
data class StaticSchemaPendingSignalRecord(val level: StaticSchemaPendingSignalLevelEnum, val count: UShort, val urgent: Boolean) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("level" to level.toSaved(), "count" to SavedValues.of(count), "urgent" to SavedValues.of(urgent))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSchemaPendingSignalRecord = StaticSchemaPendingSignalRecord(level = StaticSchemaPendingSignalLevelEnum.fromSaved(SavedValues.field(value, what, "level"), "$what.level"), count = SavedValues.uint16(SavedValues.field(value, what, "count"), "$what.count"), urgent = SavedValues.bool(SavedValues.field(value, what, "urgent"), "$what.urgent"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSchemaPendingStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticSchemaPendingState, StaticSchemaPendingEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticSchemaPendingSignalRecord = StaticSchemaPendingSignalRecord(level = StaticSchemaPendingSignalLevelEnum.LOW, count = 0.toUShort(), urgent = false)

    /** What the published `last` holds. */
    val last: StaticSchemaPendingSignalRecord get() = vLast
    /** W3C SCXML 5.2: the `taken` datamodel variable, published (`sce:direction="out"`). */
    private var vTaken: UInt = 0.toUInt()

    /** What the published `taken` holds. */
    val taken: UInt get() = vTaken
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
        var taken: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.taken?.let { vTaken = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val last: StaticSchemaPendingSignalRecord,
        val taken: UInt,
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
        val configuration: Set<StaticSchemaPendingState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        last = vLast,
        taken = vTaken,
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
        first = "f7e359f42783c9931d2c1dbf44f80fee16a403f0e56b70bbc26ff6874c901834",
        revised = "2619b075d4aa2597ecf884cba0f3b6d95715e5dbdca9bb22dc2d20be2f4dead2",
        format2First = "c0a487fdd7e3380c9c4044d0fa1211b81a23cbe90c581ff9490a619a0447a749",
        format2Revised = "3fbeae2b4200145eef7af1a2e393f315a5466a7f5ff65b7bcabbb7ed2e75c9fa",
        records = listOf(
            RecordLineage(
                event = "signal.raised",
                revision = 2,
                revisions = listOf("2b64de54f91bff7c63333f77d7362c701e702ab2379252eaf60d39931148ad66"),
                added = listOf(
                    AddedField("urgent", 2, "true"),
                ),
                widened = listOf(
                ),
                holders = listOf(
                    RecordHolder("last", false),
                ),
            ),
        ),
        enums = listOf(
            EnumLineage(
                key = "enum:enum_signal_level.scxml",
                revision = 2,
                revisions = listOf("48003474fab3bfc2bc59b262dfe3b3f85acd0f80271e6f72432dd8bf1319603c"),
                added = listOf(
                    AddedVariant("critical", 2),
                ),
                holders = listOf(
                    EnumHolder("last", "level", false),
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
            "last" to vLast.toSaved(),
            "taken" to SavedValues.of(vTaken),
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
        val saved1 = StaticSchemaPendingSignalRecord.fromSaved(savedGrown.variable("last"), "last")
        val saved2 = SavedValues.uint32(savedGrown.variable("taken"), "taken")
        val saved3 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vLast = saved1
        vTaken = saved2
        vRefusals = saved3
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
        StaticSchemaPendingSignalRecord.fromSaved(savedGrown.variable("last"), "last")
        SavedValues.uint32(savedGrown.variable("taken"), "taken")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingSignalRaisedPayload: StaticSchemaPendingSignalRaisedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticSchemaPendingEvent, metadata: EventMetadata) {
        pendingSignalRaisedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticSchemaPendingSignalRaisedPayload -> pendingSignalRaisedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticSchemaPendingEvent.Signal.Raised) {
                    val fields = EventPayload.decode(metadata.data, "{\"urgent\":true}")
                    pendingSignalRaisedPayload = StaticSchemaPendingSignalRaisedPayload(fields.string("level").let { name -> StaticSchemaPendingSignalLevelEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'level' ($name) is not a variant of SignalLevel") }, fields.uint16("count"), fields.boolean("urgent"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `signal.raised` — binds the event name and the payload field values in one call.
    fun raiseSignalRaised(level: StaticSchemaPendingSignalLevelEnum, count: UShort, urgent: Boolean) {
        send(
            StaticSchemaPendingEvent.Signal.Raised,
            EventMetadata(
                type = "external",
                typedPayload = StaticSchemaPendingSignalRaisedPayload(level, count, urgent),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("level" to level.declaredName, "count" to count, "urgent" to urgent))
            )
        )
    }


    override val initialState: StaticSchemaPendingState = StaticSchemaPendingState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StaticSchemaPendingState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSchemaPendingState, HistoryId>> =
            listOf(StateTarget(StaticSchemaPendingState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticSchemaPendingState, HistoryId>(
            StaticSchemaPendingState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticSchemaPendingState, HistoryId>(
            StaticSchemaPendingState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StaticSchemaPendingState, HistoryId>(
            StaticSchemaPendingState.Waiting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSchemaPendingState? = when (stateId) {
        "waiting" -> StaticSchemaPendingState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSchemaPendingState): String = when (state) {
        is StaticSchemaPendingState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSchemaPendingState): Int = when (state) {
        is StaticSchemaPendingState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSchemaPendingEvent? = when (name) {
        "arm" -> StaticSchemaPendingEvent.Arm
        "error.execution" -> StaticSchemaPendingEvent.Error.Execution
        "signal.raised" -> StaticSchemaPendingEvent.Signal.Raised
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSchemaPendingEvent): String? = when (event) {
        is StaticSchemaPendingEvent.Arm -> "arm"
        is StaticSchemaPendingEvent.Error.Execution -> "error.execution"
        is StaticSchemaPendingEvent.Signal.Raised -> "signal.raised"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSchemaPendingState,
        event: StaticSchemaPendingEvent?
    ): EnabledTransition<StaticSchemaPendingState, HistoryId>? = when (state) {
        is StaticSchemaPendingState.Waiting -> when {
            event is StaticSchemaPendingEvent.Arm -> transitionWaitingAt0
            event is StaticSchemaPendingEvent.Signal.Raised -> transitionWaitingAt1
            event is StaticSchemaPendingEvent.Error.Execution -> transitionWaitingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_schema_pending.scxml:17 :: _machine
    override fun onEntry(state: StaticSchemaPendingState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSchemaPendingState.Waiting -> {
                // SCE-MAP: static_schema_pending.scxml:30 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_schema_pending.scxml:17 :: _machine
    override fun onExit(state: StaticSchemaPendingState) {
        when (state) {
            is StaticSchemaPendingState.Waiting -> {
                // SCE-MAP: static_schema_pending.scxml:30 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_schema_pending.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticSchemaPendingState, transitionIndex: Int) {
        when (source) {
        is StaticSchemaPendingState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_schema_pending.scxml:31 :: waiting :: _transition_0


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "level", (StaticSchemaPendingSignalLevelEnum.HIGH).declaredName)

            putParam(sendPayload, "count", (7).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: Delayed send
            scheduleSend("later", 1000L, StaticSchemaPendingEvent.Signal.Raised, EventMetadata.external(sendId = "later", origin = sessionId, data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_schema_pending.scxml:37 :: waiting :: _transition_1
                if (pendingSignalRaisedPayload == null) {
                    return
                }

            vLast = StaticSchemaPendingSignalRecord(level = pendingSignalRaisedPayload!!.level, count = pendingSignalRaisedPayload!!.count, urgent = pendingSignalRaisedPayload!!.urgent)

            if (try { vTaken = com.sce.forge.runtime.SceChecked.add(vTaken, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSchemaPendingEvent.Error.Execution, "<assign location='taken'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_schema_pending.scxml:41 :: waiting :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSchemaPendingEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
