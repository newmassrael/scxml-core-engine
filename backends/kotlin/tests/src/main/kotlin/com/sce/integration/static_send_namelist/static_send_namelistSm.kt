// SCE-GENERATED — DO NOT EDIT
// source-hash: 8831cdf9a24d5319be675dfb105f20981ee799a3ff543da9abd5bce677e481a1

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_namelist.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_namelist.scxml:13 :: _machine

package com.sce.integration.static_send_namelist

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendNamelistState : State {
    data object Viewing : StaticSendNamelistState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendNamelistEvent : Event {
    sealed interface Error : StaticSendNamelistEvent {
        data object Execution : Error
    }
    data object Pick : StaticSendNamelistEvent
    sealed interface Send : StaticSendNamelistEvent {
        data object Mixed : Send
        data object Names : Send
    }
    sealed interface View : StaticSendNamelistEvent {
        data object Shown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticSendNamelistPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticSendNamelistViewShownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.shown`. Consumers inject it via the `raiseViewShown` seam
// on the machine — they never name this class directly.
data class StaticSendNamelistViewShownPayload(val layout: StaticSendNamelistViewModeEnum, val zoom: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticSendNamelistViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticSendNamelistViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
// --- State Machine (W3C SCXML) ---

class StaticSendNamelistStateMachine(
) : StateMachineEngine<StaticSendNamelistState, StaticSendNamelistEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `layout` datamodel variable, published (`sce:direction="out"`). */
    var layout: StaticSendNamelistViewModeEnum = StaticSendNamelistViewModeEnum.WEEK
        private set
    /** W3C SCXML 5.2: the `zoom` datamodel variable, published (`sce:direction="out"`). */
    var zoom: UByte = 3.toUByte()
        private set
    /** W3C SCXML 5.2: the `received` datamodel variable, published (`sce:direction="out"`). */
    var received: StaticSendNamelistViewModeEnum = StaticSendNamelistViewModeEnum.MONTH
        private set
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    var level: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `deliveries` datamodel variable, published (`sce:direction="out"`). */
    var deliveries: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var zoom: UByte? = null
        var level: UByte? = null
        var deliveries: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.zoom?.let { zoom = it }
        params.level?.let { level = it }
        params.deliveries?.let { deliveries = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val layout: StaticSendNamelistViewModeEnum,
        val zoom: UByte,
        val received: StaticSendNamelistViewModeEnum,
        val level: UByte,
        val deliveries: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticSendNamelistState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        layout = layout,
        zoom = zoom,
        received = received,
        level = level,
        deliveries = deliveries,
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
    val savedShape: String = "4c5fcd09ccb548a79be78a0a5ea36c3c9f17a5dfb4706bc3b43fd85462ba40d9"

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
            "received" to received.toSaved(),
            "level" to SavedValues.of(level),
            "deliveries" to SavedValues.of(deliveries),
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
        val saved1 = StaticSendNamelistViewModeEnum.fromSaved(saved.variable("layout"), "layout")
        val saved2 = SavedValues.uint8(saved.variable("zoom"), "zoom")
        val saved3 = StaticSendNamelistViewModeEnum.fromSaved(saved.variable("received"), "received")
        val saved4 = SavedValues.uint8(saved.variable("level"), "level")
        val saved5 = SavedValues.uint32(saved.variable("deliveries"), "deliveries")
        layout = saved1
        zoom = saved2
        received = saved3
        level = saved4
        deliveries = saved5
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewShownPayload: StaticSendNamelistViewShownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticSendNamelistEvent, metadata: EventMetadata) {
        pendingViewShownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticSendNamelistViewShownPayload -> pendingViewShownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticSendNamelistEvent.View.Shown) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewShownPayload = StaticSendNamelistViewShownPayload(fields.string("layout").let { name -> StaticSendNamelistViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.shown` — binds the event name and the payload field values in one call.
    fun raiseViewShown(layout: StaticSendNamelistViewModeEnum, zoom: UByte) {
        send(
            StaticSendNamelistEvent.View.Shown,
            EventMetadata(
                type = "external",
                typedPayload = StaticSendNamelistViewShownPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }


    override val initialState: StaticSendNamelistState = StaticSendNamelistState.Viewing

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
    override val documentInitialTargets: List<EntryTarget<StaticSendNamelistState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendNamelistState, HistoryId>> =
            listOf(StateTarget(StaticSendNamelistState.Viewing))

        // W3C SCXML 3.13: viewing's transition 0, as the microstep reads it.
        val transitionViewingAt0 = EnabledTransition<StaticSendNamelistState, HistoryId>(
            StaticSendNamelistState.Viewing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 1, as the microstep reads it.
        val transitionViewingAt1 = EnabledTransition<StaticSendNamelistState, HistoryId>(
            StaticSendNamelistState.Viewing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 2, as the microstep reads it.
        val transitionViewingAt2 = EnabledTransition<StaticSendNamelistState, HistoryId>(
            StaticSendNamelistState.Viewing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 3, as the microstep reads it.
        val transitionViewingAt3 = EnabledTransition<StaticSendNamelistState, HistoryId>(
            StaticSendNamelistState.Viewing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendNamelistState? = when (stateId) {
        "viewing" -> StaticSendNamelistState.Viewing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendNamelistState): String = when (state) {
        is StaticSendNamelistState.Viewing -> "viewing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendNamelistState): Int = when (state) {
        is StaticSendNamelistState.Viewing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendNamelistEvent? = when (name) {
        "error.execution" -> StaticSendNamelistEvent.Error.Execution
        "pick" -> StaticSendNamelistEvent.Pick
        "send.mixed" -> StaticSendNamelistEvent.Send.Mixed
        "send.names" -> StaticSendNamelistEvent.Send.Names
        "view.shown" -> StaticSendNamelistEvent.View.Shown
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendNamelistEvent): String? = when (event) {
        is StaticSendNamelistEvent.Error.Execution -> "error.execution"
        is StaticSendNamelistEvent.Pick -> "pick"
        is StaticSendNamelistEvent.Send.Mixed -> "send.mixed"
        is StaticSendNamelistEvent.Send.Names -> "send.names"
        is StaticSendNamelistEvent.View.Shown -> "view.shown"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendNamelistState,
        event: StaticSendNamelistEvent?
    ): EnabledTransition<StaticSendNamelistState, HistoryId>? = when (state) {
        is StaticSendNamelistState.Viewing -> when {
            event is StaticSendNamelistEvent.Pick -> transitionViewingAt0
            event is StaticSendNamelistEvent.Send.Names -> transitionViewingAt1
            event is StaticSendNamelistEvent.Send.Mixed -> transitionViewingAt2
            event is StaticSendNamelistEvent.View.Shown -> transitionViewingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_namelist.scxml:13 :: _machine
    override fun onEntry(state: StaticSendNamelistState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendNamelistState.Viewing -> {
                // SCE-MAP: static_send_namelist.scxml:24 :: viewing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_namelist.scxml:13 :: _machine
    override fun onExit(state: StaticSendNamelistState) {
        when (state) {
            is StaticSendNamelistState.Viewing -> {
                // SCE-MAP: static_send_namelist.scxml:24 :: viewing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_namelist.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticSendNamelistState, transitionIndex: Int) {
        when (source) {
        is StaticSendNamelistState.Viewing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_namelist.scxml:25 :: viewing :: _transition_0

            layout = StaticSendNamelistViewModeEnum.AGENDA_LIST

            zoom = 9.toUByte()
            }
            1 -> {
                // SCE-MAP: static_send_namelist.scxml:29 :: viewing :: _transition_1


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (layout).declaredName)

            putParam(sendPayload, "zoom", (zoom).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendNamelistEvent.View.Shown, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_namelist.scxml:32 :: viewing :: _transition_2


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "zoom", (com.sce.forge.runtime.SceChecked.add(zoom, 1.toUByte())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendNamelistEvent.Error.Execution, "<send> <param name='zoom'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "layout", (layout).declaredName)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendNamelistEvent.View.Shown, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: static_send_namelist.scxml:37 :: viewing :: _transition_3
                if (pendingViewShownPayload == null) {
                    return
                }

            received = pendingViewShownPayload!!.layout

            level = pendingViewShownPayload!!.zoom

            if (try { deliveries = com.sce.forge.runtime.SceChecked.add(deliveries, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendNamelistEvent.Error.Execution, "<assign location='deliveries'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
