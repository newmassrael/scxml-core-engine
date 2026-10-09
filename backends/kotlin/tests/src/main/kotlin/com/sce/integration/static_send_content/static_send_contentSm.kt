// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ad55f268a9fbf4c094293a60d20a17c7e9e6598a9a6fe5787c0e9f2898a7e28

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_content.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_content.scxml:14 :: _machine

package com.sce.integration.static_send_content

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendContentState : State {
    data object Viewing : StaticSendContentState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendContentEvent : Event {
    data object Bump : StaticSendContentEvent
    sealed interface Error : StaticSendContentEvent {
        data object Execution : Error
    }
    sealed interface Send : StaticSendContentEvent {
        data object Record : Send
    }
    sealed interface View : StaticSendContentEvent {
        data object Relayed : View
        data object Shown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticSendContentPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticSendContentViewRelayedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.relayed`. Consumers inject it via the `raiseViewRelayed` seam
// on the machine — they never name this class directly.
data class StaticSendContentViewRelayedPayload(val layout: StaticSendContentViewModeEnum, val zoom: UByte)

// StaticSendContentViewShownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.shown`. Consumers inject it via the `raiseViewShown` seam
// on the machine — they never name this class directly.
data class StaticSendContentViewShownPayload(val layout: StaticSendContentViewModeEnum, val zoom: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticSendContentViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSendContentViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
/** SCE Accepted Subset §2.15: a `record:View` datamodel value. */
data class StaticSendContentViewRecord(val layout: StaticSendContentViewModeEnum, val zoom: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("layout" to layout.toSaved(), "zoom" to SavedValues.of(zoom))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSendContentViewRecord = StaticSendContentViewRecord(layout = StaticSendContentViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), zoom = SavedValues.uint8(SavedValues.field(value, what, "zoom"), "$what.zoom"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSendContentStateMachine(
) : StateMachineEngine<StaticSendContentState, StaticSendContentEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticSendContentViewRecord = StaticSendContentViewRecord(layout = StaticSendContentViewModeEnum.DAY, zoom = 2.toUByte())
        private set
    /** W3C SCXML 5.2: the `received` datamodel variable, published (`sce:direction="out"`). */
    var received: StaticSendContentViewModeEnum = StaticSendContentViewModeEnum.MONTH
        private set
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    var level: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `relays` datamodel variable, published (`sce:direction="out"`). */
    var relays: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var level: UByte? = null
        var relays: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.level?.let { level = it }
        params.relays?.let { relays = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: StaticSendContentViewRecord,
        val received: StaticSendContentViewModeEnum,
        val level: UByte,
        val relays: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticSendContentState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
        received = received,
        level = level,
        relays = relays,
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
    val savedShape: String = "6f33e5b00a3a440af82bab37051ecfb5a5a93fab315376e997123933d7aa034e"

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
            "received" to received.toSaved(),
            "level" to SavedValues.of(level),
            "relays" to SavedValues.of(relays),
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
        val saved1 = StaticSendContentViewRecord.fromSaved(saved.variable("shown"), "shown")
        val saved2 = StaticSendContentViewModeEnum.fromSaved(saved.variable("received"), "received")
        val saved3 = SavedValues.uint8(saved.variable("level"), "level")
        val saved4 = SavedValues.uint32(saved.variable("relays"), "relays")
        shown = saved1
        received = saved2
        level = saved3
        relays = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewRelayedPayload: StaticSendContentViewRelayedPayload? = null
    private var pendingViewShownPayload: StaticSendContentViewShownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticSendContentEvent, metadata: EventMetadata) {
        pendingViewRelayedPayload = null
        pendingViewShownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticSendContentViewRelayedPayload -> pendingViewRelayedPayload = tp
            is StaticSendContentViewShownPayload -> pendingViewShownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticSendContentEvent.View.Relayed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewRelayedPayload = StaticSendContentViewRelayedPayload(fields.string("layout").let { name -> StaticSendContentViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                } else if (event == StaticSendContentEvent.View.Shown) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewShownPayload = StaticSendContentViewShownPayload(fields.string("layout").let { name -> StaticSendContentViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.relayed` — binds the event name and the payload field values in one call.
    fun raiseViewRelayed(layout: StaticSendContentViewModeEnum, zoom: UByte) {
        send(
            StaticSendContentEvent.View.Relayed,
            EventMetadata(
                type = "external",
                typedPayload = StaticSendContentViewRelayedPayload(layout, zoom),
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
    fun raiseViewShown(layout: StaticSendContentViewModeEnum, zoom: UByte) {
        send(
            StaticSendContentEvent.View.Shown,
            EventMetadata(
                type = "external",
                typedPayload = StaticSendContentViewShownPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }


    override val initialState: StaticSendContentState = StaticSendContentState.Viewing

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
    override val documentInitialTargets: List<EntryTarget<StaticSendContentState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendContentState, HistoryId>> =
            listOf(StateTarget(StaticSendContentState.Viewing))

        // W3C SCXML 3.13: viewing's transition 0, as the microstep reads it.
        val transitionViewingAt0 = EnabledTransition<StaticSendContentState, HistoryId>(
            StaticSendContentState.Viewing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 1, as the microstep reads it.
        val transitionViewingAt1 = EnabledTransition<StaticSendContentState, HistoryId>(
            StaticSendContentState.Viewing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 2, as the microstep reads it.
        val transitionViewingAt2 = EnabledTransition<StaticSendContentState, HistoryId>(
            StaticSendContentState.Viewing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 3, as the microstep reads it.
        val transitionViewingAt3 = EnabledTransition<StaticSendContentState, HistoryId>(
            StaticSendContentState.Viewing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendContentState? = when (stateId) {
        "viewing" -> StaticSendContentState.Viewing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendContentState): String = when (state) {
        is StaticSendContentState.Viewing -> "viewing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendContentState): Int = when (state) {
        is StaticSendContentState.Viewing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendContentEvent? = when (name) {
        "bump" -> StaticSendContentEvent.Bump
        "error.execution" -> StaticSendContentEvent.Error.Execution
        "send.record" -> StaticSendContentEvent.Send.Record
        "view.relayed" -> StaticSendContentEvent.View.Relayed
        "view.shown" -> StaticSendContentEvent.View.Shown
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendContentEvent): String? = when (event) {
        is StaticSendContentEvent.Bump -> "bump"
        is StaticSendContentEvent.Error.Execution -> "error.execution"
        is StaticSendContentEvent.Send.Record -> "send.record"
        is StaticSendContentEvent.View.Relayed -> "view.relayed"
        is StaticSendContentEvent.View.Shown -> "view.shown"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendContentState,
        event: StaticSendContentEvent?
    ): EnabledTransition<StaticSendContentState, HistoryId>? = when (state) {
        is StaticSendContentState.Viewing -> when {
            event is StaticSendContentEvent.Bump -> transitionViewingAt0
            event is StaticSendContentEvent.Send.Record -> transitionViewingAt1
            event is StaticSendContentEvent.View.Shown -> transitionViewingAt2
            event is StaticSendContentEvent.View.Relayed -> transitionViewingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_content.scxml:14 :: _machine
    override fun onEntry(state: StaticSendContentState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendContentState.Viewing -> {
                // SCE-MAP: static_send_content.scxml:28 :: viewing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_content.scxml:14 :: _machine
    override fun onExit(state: StaticSendContentState) {
        when (state) {
            is StaticSendContentState.Viewing -> {
                // SCE-MAP: static_send_content.scxml:28 :: viewing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_content.scxml:14 :: _machine
    override fun executeTransitionContent(source: StaticSendContentState, transitionIndex: Int) {
        when (source) {
        is StaticSendContentState.Viewing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_content.scxml:29 :: viewing :: _transition_0

            if (try { shown = shown.copy(zoom = com.sce.forge.runtime.SceChecked.add(shown.zoom, 1.toUByte())); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendContentEvent.Error.Execution, "<assign location='shown.zoom'>: an integer operation overflowed or failed"); true }) {
                return
            }

            shown = shown.copy(layout = StaticSendContentViewModeEnum.WEEK)
            }
            1 -> {
                // SCE-MAP: static_send_content.scxml:33 :: viewing :: _transition_1


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (shown.layout).declaredName)

            putParam(sendPayload, "zoom", (shown.zoom).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendContentEvent.View.Relayed, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_content.scxml:38 :: viewing :: _transition_2
                if (pendingViewShownPayload == null) {
                    return
                }


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (pendingViewShownPayload!!.layout).declaredName)

            putParam(sendPayload, "zoom", (pendingViewShownPayload!!.zoom).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendContentEvent.View.Relayed, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: static_send_content.scxml:43 :: viewing :: _transition_3
                if (pendingViewRelayedPayload == null) {
                    return
                }

            received = pendingViewRelayedPayload!!.layout

            level = pendingViewRelayedPayload!!.zoom

            if (try { relays = com.sce.forge.runtime.SceChecked.add(relays, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendContentEvent.Error.Execution, "<assign location='relays'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
