// SCE-GENERATED — DO NOT EDIT
// source-hash: dd1a6b6c55ab533e307bc7e47f7d1f334126ccba017dba6d270609c22c55e26e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_whole_payload.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_whole_payload.scxml:17 :: _machine

package com.sce.integration.static_whole_payload

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticWholePayloadState : State {
    data object Viewing : StaticWholePayloadState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticWholePayloadEvent : Event {
    data object Peek : StaticWholePayloadEvent
    sealed interface View : StaticWholePayloadEvent {
        data object Shown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticWholePayloadPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticWholePayloadViewShownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.shown`. Consumers inject it via the `raiseViewShown` seam
// on the machine — they never name this class directly.
data class StaticWholePayloadViewShownPayload(val layout: StaticWholePayloadViewModeEnum, val zoom: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticWholePayloadViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticWholePayloadViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
/** SCE Accepted Subset §2.15: a `record:View` datamodel value. */
data class StaticWholePayloadViewRecord(val layout: StaticWholePayloadViewModeEnum, val zoom: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("layout" to layout.toSaved(), "zoom" to SavedValues.of(zoom))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticWholePayloadViewRecord = StaticWholePayloadViewRecord(layout = StaticWholePayloadViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), zoom = SavedValues.uint8(SavedValues.field(value, what, "zoom"), "$what.zoom"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticWholePayloadStateMachine(
) : StateMachineEngine<StaticWholePayloadState, StaticWholePayloadEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticWholePayloadViewRecord = StaticWholePayloadViewRecord(layout = StaticWholePayloadViewModeEnum.MONTH, zoom = 1.toUByte())
        private set
    /** W3C SCXML 5.2: the `seen` datamodel variable, published (`sce:direction="out"`). */
    var seen: List<StaticWholePayloadViewRecord> = emptyList()
        private set
    /** W3C SCXML 5.2: the `updates` datamodel variable, published (`sce:direction="out"`). */
    var updates: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `agendas` datamodel variable, published (`sce:direction="out"`). */
    var agendas: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `others` datamodel variable, published (`sce:direction="out"`). */
    var others: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var updates: UInt? = null
        var agendas: UInt? = null
        var others: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.updates?.let { updates = it }
        params.agendas?.let { agendas = it }
        params.others?.let { others = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticWholePayloadViewRecord,
        val seen: List<StaticWholePayloadViewRecord>,
        val updates: UInt,
        val agendas: UInt,
        val others: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticWholePayloadState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
        seen = seen,
        updates = updates,
        agendas = agendas,
        others = others,
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
    val savedShape: String = "b012d52eed2aa7c4c21026e8b9a1e1e55411a4dccd8a7ff7e6a30934ff248236"

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
            "shown" to shown.toSaved(),
            "seen" to SavedValues.list(seen) { it.toSaved() },
            "updates" to SavedValues.of(updates),
            "agendas" to SavedValues.of(agendas),
            "others" to SavedValues.of(others),
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
        val saved1 = StaticWholePayloadViewRecord.fromSaved(saved.variable("shown"), "shown")
        val saved2 = SavedValues.list(saved.variable("seen"), "seen", 4) { e, w -> StaticWholePayloadViewRecord.fromSaved(e, w) }
        val saved3 = SavedValues.uint32(saved.variable("updates"), "updates")
        val saved4 = SavedValues.uint32(saved.variable("agendas"), "agendas")
        val saved5 = SavedValues.uint32(saved.variable("others"), "others")
        shown = saved1
        seen = saved2
        updates = saved3
        agendas = saved4
        others = saved5
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewShownPayload: StaticWholePayloadViewShownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticWholePayloadEvent, metadata: EventMetadata) {
        pendingViewShownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticWholePayloadViewShownPayload -> pendingViewShownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticWholePayloadEvent.View.Shown) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewShownPayload = StaticWholePayloadViewShownPayload(fields.string("layout").let { name -> StaticWholePayloadViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.shown` — binds the event name and the payload field values in one call.
    fun raiseViewShown(layout: StaticWholePayloadViewModeEnum, zoom: UByte) {
        send(
            StaticWholePayloadEvent.View.Shown,
            EventMetadata(
                type = "external",
                typedPayload = StaticWholePayloadViewShownPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }


    override val initialState: StaticWholePayloadState = StaticWholePayloadState.Viewing

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
    override val documentInitialTargets: List<EntryTarget<StaticWholePayloadState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticWholePayloadState, HistoryId>> =
            listOf(StateTarget(StaticWholePayloadState.Viewing))

        // W3C SCXML 3.13: viewing's transition 0, as the microstep reads it.
        val transitionViewingAt0 = EnabledTransition<StaticWholePayloadState, HistoryId>(
            StaticWholePayloadState.Viewing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 1, as the microstep reads it.
        val transitionViewingAt1 = EnabledTransition<StaticWholePayloadState, HistoryId>(
            StaticWholePayloadState.Viewing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 2, as the microstep reads it.
        val transitionViewingAt2 = EnabledTransition<StaticWholePayloadState, HistoryId>(
            StaticWholePayloadState.Viewing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticWholePayloadState? = when (stateId) {
        "viewing" -> StaticWholePayloadState.Viewing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticWholePayloadState): String = when (state) {
        is StaticWholePayloadState.Viewing -> "viewing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticWholePayloadState): Int = when (state) {
        is StaticWholePayloadState.Viewing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticWholePayloadEvent? = when (name) {
        "peek" -> StaticWholePayloadEvent.Peek
        "view.shown" -> StaticWholePayloadEvent.View.Shown
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticWholePayloadEvent): String? = when (event) {
        is StaticWholePayloadEvent.Peek -> "peek"
        is StaticWholePayloadEvent.View.Shown -> "view.shown"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticWholePayloadState,
        event: StaticWholePayloadEvent?
    ): EnabledTransition<StaticWholePayloadState, HistoryId>? = when (state) {
        is StaticWholePayloadState.Viewing -> when {
            event is StaticWholePayloadEvent.View.Shown -> transitionViewingAt0
            event is StaticWholePayloadEvent.Peek && shown.layout == StaticWholePayloadViewModeEnum.AGENDA_LIST -> transitionViewingAt1
            event is StaticWholePayloadEvent.Peek -> transitionViewingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_whole_payload.scxml:17 :: _machine
    override fun onEntry(state: StaticWholePayloadState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticWholePayloadState.Viewing -> {
                // SCE-MAP: static_whole_payload.scxml:31 :: viewing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_whole_payload.scxml:17 :: _machine
    override fun onExit(state: StaticWholePayloadState) {
        when (state) {
            is StaticWholePayloadState.Viewing -> {
                // SCE-MAP: static_whole_payload.scxml:31 :: viewing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_whole_payload.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticWholePayloadState, transitionIndex: Int) {
        when (source) {
        is StaticWholePayloadState.Viewing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_whole_payload.scxml:32 :: viewing :: _transition_0
                if (pendingViewShownPayload == null) {
                    return
                }

            shown = StaticWholePayloadViewRecord(layout = pendingViewShownPayload!!.layout, zoom = pendingViewShownPayload!!.zoom)

            if (if (seen.size < 4) { seen = seen + (StaticWholePayloadViewRecord(layout = pendingViewShownPayload!!.layout, zoom = pendingViewShownPayload!!.zoom)); false } else { true }) {
                return
            }

            if (try { updates = com.sce.forge.runtime.SceChecked.add(updates, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_whole_payload.scxml:37 :: viewing :: _transition_1

            if (try { agendas = com.sce.forge.runtime.SceChecked.add(agendas, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_whole_payload.scxml:40 :: viewing :: _transition_2

            if (try { others = com.sce.forge.runtime.SceChecked.add(others, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
