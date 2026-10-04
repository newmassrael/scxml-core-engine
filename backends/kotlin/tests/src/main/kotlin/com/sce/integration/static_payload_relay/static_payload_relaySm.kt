// SCE-GENERATED — DO NOT EDIT
// source-hash: fb3302d1a39b9f33f23d5670c605e9c0dde32870608a39fa98fe9b54962bd426

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_payload_relay.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_payload_relay.scxml:18 :: _machine

package com.sce.integration.static_payload_relay

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticPayloadRelayState : State {
    data object Relaying : StaticPayloadRelayState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticPayloadRelayEvent : Event {
    sealed interface Error : StaticPayloadRelayEvent {
        data object Execution : Error
    }
    sealed interface View : StaticPayloadRelayEvent {
        data object Relayed : View
        data object Shown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticPayloadRelayPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticPayloadRelayViewRelayedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.relayed`. Consumers inject it via the `raiseViewRelayed` seam
// on the machine — they never name this class directly.
data class StaticPayloadRelayViewRelayedPayload(val layout: StaticPayloadRelayViewModeEnum, val zoom: UByte)

// StaticPayloadRelayViewShownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.shown`. Consumers inject it via the `raiseViewShown` seam
// on the machine — they never name this class directly.
data class StaticPayloadRelayViewShownPayload(val layout: StaticPayloadRelayViewModeEnum, val zoom: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticPayloadRelayViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticPayloadRelayViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticPayloadRelayStateMachine(
) : StateMachineEngine<StaticPayloadRelayState, StaticPayloadRelayEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `layout` datamodel variable, published (`sce:direction="out"`). */
    var layout: StaticPayloadRelayViewModeEnum = StaticPayloadRelayViewModeEnum.MONTH
        private set
    /** W3C SCXML 5.2: the `zoom` datamodel variable, published (`sce:direction="out"`). */
    var zoom: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `relays` datamodel variable, published (`sce:direction="out"`). */
    var relays: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var zoom: UByte? = null
        var relays: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.zoom?.let { zoom = it }
        params.relays?.let { relays = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val layout: StaticPayloadRelayViewModeEnum,
        val zoom: UByte,
        val relays: UInt,
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
        val configuration: Set<StaticPayloadRelayState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        layout = layout,
        zoom = zoom,
        relays = relays,
        refusals = refusals,
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
    val savedShape: String = "46a1d9b8bf0bd4bc55ac5b3b8080579e9e13beba2911ad0c38119420f6b7529a"

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
            "layout" to layout.toSaved(),
            "zoom" to SavedValues.of(zoom),
            "relays" to SavedValues.of(relays),
            "refusals" to SavedValues.of(refusals),
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
        val saved1 = StaticPayloadRelayViewModeEnum.fromSaved(saved.variable("layout"), "layout")
        val saved2 = SavedValues.uint8(saved.variable("zoom"), "zoom")
        val saved3 = SavedValues.uint32(saved.variable("relays"), "relays")
        val saved4 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        layout = saved1
        zoom = saved2
        relays = saved3
        refusals = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewRelayedPayload: StaticPayloadRelayViewRelayedPayload? = null
    private var pendingViewShownPayload: StaticPayloadRelayViewShownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticPayloadRelayEvent, metadata: EventMetadata) {
        pendingViewRelayedPayload = null
        pendingViewShownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticPayloadRelayViewRelayedPayload -> pendingViewRelayedPayload = tp
            is StaticPayloadRelayViewShownPayload -> pendingViewShownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticPayloadRelayEvent.View.Relayed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewRelayedPayload = StaticPayloadRelayViewRelayedPayload(fields.string("layout").let { name -> StaticPayloadRelayViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                } else if (event == StaticPayloadRelayEvent.View.Shown) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewShownPayload = StaticPayloadRelayViewShownPayload(fields.string("layout").let { name -> StaticPayloadRelayViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.relayed` — binds the event name and the payload field values in one call.
    fun raiseViewRelayed(layout: StaticPayloadRelayViewModeEnum, zoom: UByte) {
        send(
            StaticPayloadRelayEvent.View.Relayed,
            EventMetadata(
                type = "external",
                typedPayload = StaticPayloadRelayViewRelayedPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.shown` — binds the event name and the payload field values in one call.
    fun raiseViewShown(layout: StaticPayloadRelayViewModeEnum, zoom: UByte) {
        send(
            StaticPayloadRelayEvent.View.Shown,
            EventMetadata(
                type = "external",
                typedPayload = StaticPayloadRelayViewShownPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }


    override val initialState: StaticPayloadRelayState = StaticPayloadRelayState.Relaying

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
    override val documentInitialTargets: List<EntryTarget<StaticPayloadRelayState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticPayloadRelayState, HistoryId>> =
            listOf(StateTarget(StaticPayloadRelayState.Relaying))

        // W3C SCXML 3.13: relaying's transition 0, as the microstep reads it.
        val transitionRelayingAt0 = EnabledTransition<StaticPayloadRelayState, HistoryId>(
            StaticPayloadRelayState.Relaying,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: relaying's transition 1, as the microstep reads it.
        val transitionRelayingAt1 = EnabledTransition<StaticPayloadRelayState, HistoryId>(
            StaticPayloadRelayState.Relaying,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: relaying's transition 2, as the microstep reads it.
        val transitionRelayingAt2 = EnabledTransition<StaticPayloadRelayState, HistoryId>(
            StaticPayloadRelayState.Relaying,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticPayloadRelayState? = when (stateId) {
        "relaying" -> StaticPayloadRelayState.Relaying
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticPayloadRelayState): String = when (state) {
        is StaticPayloadRelayState.Relaying -> "relaying"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticPayloadRelayState): Int = when (state) {
        is StaticPayloadRelayState.Relaying -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticPayloadRelayEvent? = when (name) {
        "error.execution" -> StaticPayloadRelayEvent.Error.Execution
        "view.relayed" -> StaticPayloadRelayEvent.View.Relayed
        "view.shown" -> StaticPayloadRelayEvent.View.Shown
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticPayloadRelayEvent): String? = when (event) {
        is StaticPayloadRelayEvent.Error.Execution -> "error.execution"
        is StaticPayloadRelayEvent.View.Relayed -> "view.relayed"
        is StaticPayloadRelayEvent.View.Shown -> "view.shown"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticPayloadRelayState,
        event: StaticPayloadRelayEvent?
    ): EnabledTransition<StaticPayloadRelayState, HistoryId>? = when (state) {
        is StaticPayloadRelayState.Relaying -> when {
            event is StaticPayloadRelayEvent.View.Shown -> transitionRelayingAt0
            event is StaticPayloadRelayEvent.View.Relayed -> transitionRelayingAt1
            event is StaticPayloadRelayEvent.Error.Execution -> transitionRelayingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_payload_relay.scxml:18 :: _machine
    override fun onEntry(state: StaticPayloadRelayState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticPayloadRelayState.Relaying -> {
                // SCE-MAP: static_payload_relay.scxml:29 :: relaying :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_payload_relay.scxml:18 :: _machine
    override fun onExit(state: StaticPayloadRelayState) {
        when (state) {
            is StaticPayloadRelayState.Relaying -> {
                // SCE-MAP: static_payload_relay.scxml:29 :: relaying :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_payload_relay.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticPayloadRelayState, transitionIndex: Int) {
        when (source) {
        is StaticPayloadRelayState.Relaying -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_payload_relay.scxml:30 :: relaying :: _transition_0
                if (pendingViewShownPayload == null) {
                    return
                }


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (pendingViewShownPayload!!.layout).declaredName)

            try {
                putParam(sendPayload, "zoom", (com.sce.forge.runtime.SceChecked.add(pendingViewShownPayload!!.zoom, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticPayloadRelayEvent.Error.Execution, "<send> <param name='zoom'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticPayloadRelayEvent.View.Relayed, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_payload_relay.scxml:36 :: relaying :: _transition_1
                if (pendingViewRelayedPayload == null) {
                    return
                }

            zoom = pendingViewRelayedPayload!!.zoom

            layout = pendingViewRelayedPayload!!.layout

            if (try { relays = com.sce.forge.runtime.SceChecked.add(relays, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadRelayEvent.Error.Execution, "<assign location='relays'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_payload_relay.scxml:41 :: relaying :: _transition_2

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticPayloadRelayEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
