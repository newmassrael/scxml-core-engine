// SCE-GENERATED — DO NOT EDIT
// source-hash: 1fe728b521f52adf866bb0e0fb56e6c9d8272ffd7fa321459c53679f859508c9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_wire_enum.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_wire_enum.scxml:19 :: _machine

package com.sce.integration.static_wire_enum

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticWireEnumState : State {
    data object Viewing : StaticWireEnumState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticWireEnumEvent : Event {
    sealed interface Error : StaticWireEnumEvent {
        data object Execution : Error
    }
    data object Pick : StaticWireEnumEvent
    sealed interface Send : StaticWireEnumEvent {
        data object Chosen : Send
        data object Record : Send
        data object Variable : Send
    }
    sealed interface View : StaticWireEnumEvent {
        data object Shown : View
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticWireEnumPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticWireEnumViewShownPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `view.shown`. Consumers inject it via the `raiseViewShown` seam
// on the machine — they never name this class directly.
data class StaticWireEnumViewShownPayload(val layout: StaticWireEnumViewModeEnum, val zoom: UByte)


// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticWireEnumViewModeEnum(val declaredName: String) {
    MONTH("month"),
    WEEK("week"),
    DAY("day"),
    AGENDA_LIST("agenda_list");

    /** This value as a saved state writes it. */
    fun toSaved(): Any = declaredName

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticWireEnumViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }
    }
}
/** SCE Accepted Subset §2.15: a `record:View` datamodel value. */
data class StaticWireEnumViewRecord(val layout: StaticWireEnumViewModeEnum, val zoom: UByte) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("layout" to layout.toSaved(), "zoom" to SavedValues.of(zoom))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticWireEnumViewRecord = StaticWireEnumViewRecord(layout = StaticWireEnumViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), zoom = SavedValues.uint8(SavedValues.field(value, what, "zoom"), "$what.zoom"))
    }
}
// --- State Machine (W3C SCXML) ---

class StaticWireEnumStateMachine(
) : StateMachineEngine<StaticWireEnumState, StaticWireEnumEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `layout` datamodel variable, published (`sce:direction="out"`). */
    var layout: StaticWireEnumViewModeEnum = StaticWireEnumViewModeEnum.WEEK
        private set
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: StaticWireEnumViewRecord = StaticWireEnumViewRecord(layout = StaticWireEnumViewModeEnum.DAY, zoom = 2.toUByte())
        private set
    /** W3C SCXML 5.2: the `received` datamodel variable, published (`sce:direction="out"`). */
    var received: StaticWireEnumViewModeEnum = StaticWireEnumViewModeEnum.MONTH
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
        var deliveries: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.deliveries?.let { deliveries = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val layout: StaticWireEnumViewModeEnum,
        val shown: StaticWireEnumViewRecord,
        val received: StaticWireEnumViewModeEnum,
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
        val configuration: Set<StaticWireEnumState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        layout = layout,
        shown = shown,
        received = received,
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
    val savedShape: String = "c9a87f3c40076e0e661152c1c51fc9f5bab2cc48d50cc9aedc07d730e887d3ce"

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
            "shown" to shown.toSaved(),
            "received" to received.toSaved(),
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
        val saved1 = StaticWireEnumViewModeEnum.fromSaved(saved.variable("layout"), "layout")
        val saved2 = StaticWireEnumViewRecord.fromSaved(saved.variable("shown"), "shown")
        val saved3 = StaticWireEnumViewModeEnum.fromSaved(saved.variable("received"), "received")
        val saved4 = SavedValues.uint32(saved.variable("deliveries"), "deliveries")
        layout = saved1
        shown = saved2
        received = saved3
        deliveries = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingViewShownPayload: StaticWireEnumViewShownPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticWireEnumEvent, metadata: EventMetadata) {
        pendingViewShownPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticWireEnumViewShownPayload -> pendingViewShownPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticWireEnumEvent.View.Shown) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingViewShownPayload = StaticWireEnumViewShownPayload(fields.string("layout").let { name -> StaticWireEnumViewModeEnum.entries.firstOrNull { it.declaredName == name } ?: throw EventPayload.Refusal("'layout' ($name) is not a variant of ViewMode") }, fields.uint8("zoom"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `view.shown` — binds the event name and the payload field values in one call.
    fun raiseViewShown(layout: StaticWireEnumViewModeEnum, zoom: UByte) {
        send(
            StaticWireEnumEvent.View.Shown,
            EventMetadata(
                type = "external",
                typedPayload = StaticWireEnumViewShownPayload(layout, zoom),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("layout" to layout.declaredName, "zoom" to zoom))
            )
        )
    }


    override val initialState: StaticWireEnumState = StaticWireEnumState.Viewing

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
    override val documentInitialTargets: List<EntryTarget<StaticWireEnumState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticWireEnumState, HistoryId>> =
            listOf(StateTarget(StaticWireEnumState.Viewing))

        // W3C SCXML 3.13: viewing's transition 0, as the microstep reads it.
        val transitionViewingAt0 = EnabledTransition<StaticWireEnumState, HistoryId>(
            StaticWireEnumState.Viewing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 1, as the microstep reads it.
        val transitionViewingAt1 = EnabledTransition<StaticWireEnumState, HistoryId>(
            StaticWireEnumState.Viewing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 2, as the microstep reads it.
        val transitionViewingAt2 = EnabledTransition<StaticWireEnumState, HistoryId>(
            StaticWireEnumState.Viewing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 3, as the microstep reads it.
        val transitionViewingAt3 = EnabledTransition<StaticWireEnumState, HistoryId>(
            StaticWireEnumState.Viewing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: viewing's transition 4, as the microstep reads it.
        val transitionViewingAt4 = EnabledTransition<StaticWireEnumState, HistoryId>(
            StaticWireEnumState.Viewing,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticWireEnumState? = when (stateId) {
        "viewing" -> StaticWireEnumState.Viewing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticWireEnumState): String = when (state) {
        is StaticWireEnumState.Viewing -> "viewing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticWireEnumState): Int = when (state) {
        is StaticWireEnumState.Viewing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticWireEnumEvent? = when (name) {
        "error.execution" -> StaticWireEnumEvent.Error.Execution
        "pick" -> StaticWireEnumEvent.Pick
        "send.chosen" -> StaticWireEnumEvent.Send.Chosen
        "send.record" -> StaticWireEnumEvent.Send.Record
        "send.variable" -> StaticWireEnumEvent.Send.Variable
        "view.shown" -> StaticWireEnumEvent.View.Shown
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticWireEnumEvent): String? = when (event) {
        is StaticWireEnumEvent.Error.Execution -> "error.execution"
        is StaticWireEnumEvent.Pick -> "pick"
        is StaticWireEnumEvent.Send.Chosen -> "send.chosen"
        is StaticWireEnumEvent.Send.Record -> "send.record"
        is StaticWireEnumEvent.Send.Variable -> "send.variable"
        is StaticWireEnumEvent.View.Shown -> "view.shown"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticWireEnumState,
        event: StaticWireEnumEvent?
    ): EnabledTransition<StaticWireEnumState, HistoryId>? = when (state) {
        is StaticWireEnumState.Viewing -> when {
            event is StaticWireEnumEvent.Pick -> transitionViewingAt0
            event is StaticWireEnumEvent.Send.Variable -> transitionViewingAt1
            event is StaticWireEnumEvent.Send.Record -> transitionViewingAt2
            event is StaticWireEnumEvent.Send.Chosen -> transitionViewingAt3
            event is StaticWireEnumEvent.View.Shown -> transitionViewingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_wire_enum.scxml:19 :: _machine
    override fun onEntry(state: StaticWireEnumState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticWireEnumState.Viewing -> {
                // SCE-MAP: static_wire_enum.scxml:32 :: viewing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_wire_enum.scxml:19 :: _machine
    override fun onExit(state: StaticWireEnumState) {
        when (state) {
            is StaticWireEnumState.Viewing -> {
                // SCE-MAP: static_wire_enum.scxml:32 :: viewing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_wire_enum.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticWireEnumState, transitionIndex: Int) {
        when (source) {
        is StaticWireEnumState.Viewing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_wire_enum.scxml:33 :: viewing :: _transition_0

            layout = StaticWireEnumViewModeEnum.AGENDA_LIST
            }
            1 -> {
                // SCE-MAP: static_wire_enum.scxml:36 :: viewing :: _transition_1


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (layout).declaredName)

            putParam(sendPayload, "zoom", (3).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticWireEnumEvent.View.Shown, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_wire_enum.scxml:42 :: viewing :: _transition_2


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (shown.layout).declaredName)

            putParam(sendPayload, "zoom", (shown.zoom).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticWireEnumEvent.View.Shown, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            3 -> {
                // SCE-MAP: static_wire_enum.scxml:48 :: viewing :: _transition_3


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "layout", (if (layout == StaticWireEnumViewModeEnum.WEEK) StaticWireEnumViewModeEnum.DAY else layout).declaredName)

            putParam(sendPayload, "zoom", (4).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticWireEnumEvent.View.Shown, EventMetadata.external(sendId = "__send_2", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            4 -> {
                // SCE-MAP: static_wire_enum.scxml:54 :: viewing :: _transition_4
                if (pendingViewShownPayload == null) {
                    return
                }

            received = pendingViewShownPayload!!.layout

            if (try { deliveries = com.sce.forge.runtime.SceChecked.add(deliveries, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWireEnumEvent.Error.Execution, "<assign location='deliveries'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
