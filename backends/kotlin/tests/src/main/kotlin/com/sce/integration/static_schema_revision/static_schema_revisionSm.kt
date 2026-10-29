// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_schema_revision.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_schema_revision.scxml:19 :: _machine

package com.sce.integration.static_schema_revision

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSchemaRevisionState : State {
    data object Reading : StaticSchemaRevisionState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSchemaRevisionEvent : Event {
    sealed interface Error : StaticSchemaRevisionEvent {
        data object Execution : Error
    }
    sealed interface Reading : StaticSchemaRevisionEvent {
        data object Revised : Reading
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticSchemaRevisionPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticSchemaRevisionReadingRevisedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `reading.revised`. Consumers inject it via the `raiseReadingRevised` seam
// on the machine — they never name this class directly.
data class StaticSchemaRevisionReadingRevisedPayload(val level: UByte, val layout: StaticSchemaRevisionViewModeEnum, val urgent: Boolean)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticSchemaRevisionViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    /** The number the enum document declares for this value. */
    fun toUnderlying(): UByte = when (this) {
        MONTH -> 0u.toUByte()
        WEEK -> 1u.toUByte()
        DAY -> 2u.toUByte()
        AGENDA_LIST -> 3u.toUByte()
    }

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSchemaRevisionViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticSchemaRevisionViewModeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Revised` datamodel value. */
data class StaticSchemaRevisionRevisedRecord(val level: UByte, val layout: StaticSchemaRevisionViewModeEnum, val urgent: Boolean) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("level" to SavedValues.of(level), "layout" to layout.toSaved(), "urgent" to SavedValues.of(urgent))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSchemaRevisionRevisedRecord = StaticSchemaRevisionRevisedRecord(level = SavedValues.uint8(SavedValues.field(value, what, "level"), "$what.level"), layout = StaticSchemaRevisionViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), urgent = SavedValues.bool(SavedValues.field(value, what, "urgent"), "$what.urgent"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSchemaRevisionStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticSchemaRevisionState, StaticSchemaRevisionEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, published (`sce:direction="out"`). */
    private var vLast: StaticSchemaRevisionRevisedRecord = StaticSchemaRevisionRevisedRecord(level = 0.toUByte(), layout = StaticSchemaRevisionViewModeEnum.MONTH, urgent = false)

    /** What the published `last` holds. */
    val last: StaticSchemaRevisionRevisedRecord get() = vLast
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
        val last: StaticSchemaRevisionRevisedRecord,
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
        val configuration: Set<StaticSchemaRevisionState>,
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
        first = "11dbc3cc8b1f883cc9da011d2d5c3968ac0eeb404a0f494deede91552cd6623d",
        revised = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5",
        format2First = "11dbc3cc8b1f883cc9da011d2d5c3968ac0eeb404a0f494deede91552cd6623d",
        format2Revised = "cb771df5efc33bf380bfd5a267ca027b408f8cd064bbc7eec5ca0266e23d06f5",
        records = listOf(
            RecordLineage(
                event = "reading.revised",
                revision = 3,
                revisions = listOf("d9ba6a71207a3f219929c8fda298d7a35c76cb7950cc287ae2fa8a8b8ff45ad7", "e511eddacf1d4e220c85f78c95988fe26a2a1319cf119cb085f1a84f4f197f8d"),
                added = listOf(
                    AddedField("layout", 2, "\"week\""),
                    AddedField("urgent", 3, "true"),
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
                key = "enum:enum_view_mode.scxml",
                revision = 1,
                revisions = listOf(),
                added = listOf(
                ),
                holders = listOf(
                    EnumHolder("last", "layout", false),
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
        val saved1 = StaticSchemaRevisionRevisedRecord.fromSaved(savedGrown.variable("last"), "last")
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
        StaticSchemaRevisionRevisedRecord.fromSaved(savedGrown.variable("last"), "last")
        SavedValues.uint32(savedGrown.variable("taken"), "taken")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingReadingRevisedPayload: StaticSchemaRevisionReadingRevisedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticSchemaRevisionEvent, metadata: EventMetadata) {
        pendingReadingRevisedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticSchemaRevisionReadingRevisedPayload -> pendingReadingRevisedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticSchemaRevisionEvent.Reading.Revised) {
                    val fields = EventPayload.decode(metadata.data, "{\"layout\":\"week\",\"urgent\":true}")
                    pendingReadingRevisedPayload = StaticSchemaRevisionReadingRevisedPayload(fields.uint8("level"), fields.string("layout").let { name -> StaticSchemaRevisionViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.boolean("urgent"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `reading.revised` — binds the event name and the payload field values in one call.
    fun raiseReadingRevised(level: UByte, layout: StaticSchemaRevisionViewModeEnum, urgent: Boolean) {
        send(
            StaticSchemaRevisionEvent.Reading.Revised,
            EventMetadata(
                type = "external",
                typedPayload = StaticSchemaRevisionReadingRevisedPayload(level, layout, urgent),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("level" to level, "layout" to layout.declaredName, "urgent" to urgent))
            )
        )
    }


    override val initialState: StaticSchemaRevisionState = StaticSchemaRevisionState.Reading

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
    override val documentInitialTargets: List<EntryTarget<StaticSchemaRevisionState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSchemaRevisionState, HistoryId>> =
            listOf(StateTarget(StaticSchemaRevisionState.Reading))

        // W3C SCXML 3.13: reading's transition 0, as the microstep reads it.
        val transitionReadingAt0 = EnabledTransition<StaticSchemaRevisionState, HistoryId>(
            StaticSchemaRevisionState.Reading,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 1, as the microstep reads it.
        val transitionReadingAt1 = EnabledTransition<StaticSchemaRevisionState, HistoryId>(
            StaticSchemaRevisionState.Reading,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSchemaRevisionState? = when (stateId) {
        "reading" -> StaticSchemaRevisionState.Reading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSchemaRevisionState): String = when (state) {
        is StaticSchemaRevisionState.Reading -> "reading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSchemaRevisionState): Int = when (state) {
        is StaticSchemaRevisionState.Reading -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSchemaRevisionEvent? = when (name) {
        "error.execution" -> StaticSchemaRevisionEvent.Error.Execution
        "reading.revised" -> StaticSchemaRevisionEvent.Reading.Revised
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSchemaRevisionEvent): String? = when (event) {
        is StaticSchemaRevisionEvent.Error.Execution -> "error.execution"
        is StaticSchemaRevisionEvent.Reading.Revised -> "reading.revised"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSchemaRevisionState,
        event: StaticSchemaRevisionEvent?
    ): EnabledTransition<StaticSchemaRevisionState, HistoryId>? = when (state) {
        is StaticSchemaRevisionState.Reading -> when {
            event is StaticSchemaRevisionEvent.Reading.Revised -> transitionReadingAt0
            event is StaticSchemaRevisionEvent.Error.Execution -> transitionReadingAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_schema_revision.scxml:19 :: _machine
    override fun onEntry(state: StaticSchemaRevisionState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSchemaRevisionState.Reading -> {
                // SCE-MAP: static_schema_revision.scxml:32 :: reading :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_schema_revision.scxml:19 :: _machine
    override fun onExit(state: StaticSchemaRevisionState) {
        when (state) {
            is StaticSchemaRevisionState.Reading -> {
                // SCE-MAP: static_schema_revision.scxml:32 :: reading :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_schema_revision.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticSchemaRevisionState, transitionIndex: Int) {
        when (source) {
        is StaticSchemaRevisionState.Reading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_schema_revision.scxml:33 :: reading :: _transition_0
                if (pendingReadingRevisedPayload == null) {
                    return
                }

            vLast = StaticSchemaRevisionRevisedRecord(level = pendingReadingRevisedPayload!!.level, layout = pendingReadingRevisedPayload!!.layout, urgent = pendingReadingRevisedPayload!!.urgent)

            if (try { vTaken = com.sce.forge.runtime.SceChecked.add(vTaken, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSchemaRevisionEvent.Error.Execution, "<assign location='taken'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_schema_revision.scxml:37 :: reading :: _transition_1

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSchemaRevisionEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
