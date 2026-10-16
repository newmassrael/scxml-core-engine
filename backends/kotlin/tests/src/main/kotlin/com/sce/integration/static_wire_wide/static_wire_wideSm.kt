// SCE-GENERATED — DO NOT EDIT
// source-hash: 5b579589f1ff33cd05a081b40dc01d4d0d0b2ef410a9718dd0ae11e84c00a952

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_wire_wide.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_wire_wide.scxml:28 :: _machine

package com.sce.integration.static_wire_wide

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticWireWideState : State {
    data object Relaying : StaticWireWideState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticWireWideEvent : Event {
    sealed interface Error : StaticWireWideEvent {
        data object Execution : Error
    }
    sealed interface Wide : StaticWireWideEvent {
        data object In : Wide
        data object Relayed : Wide
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticWireWidePayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticWireWideWideInPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `wide.in`. Consumers inject it via the `raiseWideIn` seam
// on the machine — they never name this class directly.
data class StaticWireWideWideInPayload(val big: ULong, val small: Long)

// StaticWireWideWideRelayedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `wide.relayed`. Consumers inject it via the `raiseWideRelayed` seam
// on the machine — they never name this class directly.
data class StaticWireWideWideRelayedPayload(val big: ULong, val small: Long)


// --- State Machine (W3C SCXML) ---

class StaticWireWideStateMachine(
) : StateMachineEngine<StaticWireWideState, StaticWireWideEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `low` datamodel variable, published (`sce:direction="out"`). */
    private var vLow: ULong = 0.toULong()

    /** What the published `low` holds. */
    val low: ULong get() = vLow
    /** W3C SCXML 5.2: the `smallLow` datamodel variable, published (`sce:direction="out"`). */
    private var vSmallLow: Long = 0

    /** What the published `smallLow` holds. */
    val smallLow: Long get() = vSmallLow
    /** W3C SCXML 5.2: the `large` datamodel variable, published (`sce:direction="out"`). */
    private var vLarge: Boolean = false

    /** What the published `large` holds. */
    val large: Boolean get() = vLarge
    /** W3C SCXML 5.2: the `negative` datamodel variable, published (`sce:direction="out"`). */
    private var vNegative: Boolean = false

    /** What the published `negative` holds. */
    val negative: Boolean get() = vNegative
    /** W3C SCXML 5.2: the `relays` datamodel variable, published (`sce:direction="out"`). */
    private var vRelays: UInt = 0.toUInt()

    /** What the published `relays` holds. */
    val relays: UInt get() = vRelays
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
        var low: ULong? = null
        var smallLow: Long? = null
        var large: Boolean? = null
        var negative: Boolean? = null
        var relays: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.low?.let { vLow = it }
        params.smallLow?.let { vSmallLow = it }
        params.large?.let { vLarge = it }
        params.negative?.let { vNegative = it }
        params.relays?.let { vRelays = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val low: ULong,
        val smallLow: Long,
        val large: Boolean,
        val negative: Boolean,
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
        val configuration: Set<StaticWireWideState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        low = vLow,
        smallLow = vSmallLow,
        large = vLarge,
        negative = vNegative,
        relays = vRelays,
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
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "e5c2a79f4a67f967a86bc7c09af85d1e1482468f8ce173db72d9c3e90c6c7fbf"

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
            "low" to SavedValues.of(vLow),
            "smallLow" to SavedValues.of(vSmallLow),
            "large" to SavedValues.of(vLarge),
            "negative" to SavedValues.of(vNegative),
            "relays" to SavedValues.of(vRelays),
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
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.uint64(saved.variable("low"), "low")
        val saved2 = SavedValues.int64(saved.variable("smallLow"), "smallLow")
        val saved3 = SavedValues.bool(saved.variable("large"), "large")
        val saved4 = SavedValues.bool(saved.variable("negative"), "negative")
        val saved5 = SavedValues.uint32(saved.variable("relays"), "relays")
        val saved6 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        vLow = saved1
        vSmallLow = saved2
        vLarge = saved3
        vNegative = saved4
        vRelays = saved5
        vRefusals = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingWideInPayload: StaticWireWideWideInPayload? = null
    private var pendingWideRelayedPayload: StaticWireWideWideRelayedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticWireWideEvent, metadata: EventMetadata) {
        pendingWideInPayload = null
        pendingWideRelayedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticWireWideWideInPayload -> pendingWideInPayload = tp
            is StaticWireWideWideRelayedPayload -> pendingWideRelayedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticWireWideEvent.Wide.In) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingWideInPayload = StaticWireWideWideInPayload(fields.uint64("big"), fields.int64("small"))
                } else if (event == StaticWireWideEvent.Wide.Relayed) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingWideRelayedPayload = StaticWireWideWideRelayedPayload(fields.uint64("big"), fields.int64("small"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `wide.in` — binds the event name and the payload field values in one call.
    fun raiseWideIn(big: ULong, small: Long) {
        send(
            StaticWireWideEvent.Wide.In,
            EventMetadata(
                type = "external",
                typedPayload = StaticWireWideWideInPayload(big, small),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("big" to big, "small" to small))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `wide.relayed` — binds the event name and the payload field values in one call.
    fun raiseWideRelayed(big: ULong, small: Long) {
        send(
            StaticWireWideEvent.Wide.Relayed,
            EventMetadata(
                type = "external",
                typedPayload = StaticWireWideWideRelayedPayload(big, small),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("big" to big, "small" to small))
            )
        )
    }


    override val initialState: StaticWireWideState = StaticWireWideState.Relaying

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
    override val documentInitialTargets: List<EntryTarget<StaticWireWideState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticWireWideState, HistoryId>> =
            listOf(StateTarget(StaticWireWideState.Relaying))

        // W3C SCXML 3.13: relaying's transition 0, as the microstep reads it.
        val transitionRelayingAt0 = EnabledTransition<StaticWireWideState, HistoryId>(
            StaticWireWideState.Relaying,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: relaying's transition 1, as the microstep reads it.
        val transitionRelayingAt1 = EnabledTransition<StaticWireWideState, HistoryId>(
            StaticWireWideState.Relaying,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: relaying's transition 2, as the microstep reads it.
        val transitionRelayingAt2 = EnabledTransition<StaticWireWideState, HistoryId>(
            StaticWireWideState.Relaying,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticWireWideState? = when (stateId) {
        "relaying" -> StaticWireWideState.Relaying
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticWireWideState): String = when (state) {
        is StaticWireWideState.Relaying -> "relaying"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticWireWideState): Int = when (state) {
        is StaticWireWideState.Relaying -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticWireWideEvent? = when (name) {
        "error.execution" -> StaticWireWideEvent.Error.Execution
        "wide.in" -> StaticWireWideEvent.Wide.In
        "wide.relayed" -> StaticWireWideEvent.Wide.Relayed
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticWireWideEvent): String? = when (event) {
        is StaticWireWideEvent.Error.Execution -> "error.execution"
        is StaticWireWideEvent.Wide.In -> "wide.in"
        is StaticWireWideEvent.Wide.Relayed -> "wide.relayed"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticWireWideState,
        event: StaticWireWideEvent?
    ): EnabledTransition<StaticWireWideState, HistoryId>? = when (state) {
        is StaticWireWideState.Relaying -> when {
            event is StaticWireWideEvent.Wide.In -> transitionRelayingAt0
            event is StaticWireWideEvent.Wide.Relayed -> transitionRelayingAt1
            event is StaticWireWideEvent.Error.Execution -> transitionRelayingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_wire_wide.scxml:28 :: _machine
    override fun onEntry(state: StaticWireWideState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticWireWideState.Relaying -> {
                // SCE-MAP: static_wire_wide.scxml:40 :: relaying :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_wire_wide.scxml:28 :: _machine
    override fun onExit(state: StaticWireWideState) {
        when (state) {
            is StaticWireWideState.Relaying -> {
                // SCE-MAP: static_wire_wide.scxml:40 :: relaying :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_wire_wide.scxml:28 :: _machine
    override fun executeTransitionContent(source: StaticWireWideState, transitionIndex: Int) {
        when (source) {
        is StaticWireWideState.Relaying -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_wire_wide.scxml:41 :: relaying :: _transition_0
                if (pendingWideInPayload == null) {
                    return
                }


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "big", (pendingWideInPayload!!.big).toString())

            try {
                putParam(sendPayload, "small", (com.sce.forge.runtime.SceChecked.sub(pendingWideInPayload!!.small, 1)).toString())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticWireWideEvent.Error.Execution, "<send> <param name='small'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticWireWideEvent.Wide.Relayed, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_wire_wide.scxml:47 :: relaying :: _transition_1
                if (pendingWideRelayedPayload == null) {
                    return
                }

            if (try { vLow = com.sce.forge.runtime.SceChecked.rem(pendingWideRelayedPayload!!.big, 1000.toULong()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWireWideEvent.Error.Execution, "<assign location='low'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vLarge = pendingWideRelayedPayload!!.big > 9007199254740992.toULong()

            if (try { vSmallLow = com.sce.forge.runtime.SceChecked.rem(pendingWideRelayedPayload!!.small, 1000); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWireWideEvent.Error.Execution, "<assign location='smallLow'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vNegative = pendingWideRelayedPayload!!.small < 0

            if (try { vRelays = com.sce.forge.runtime.SceChecked.add(vRelays, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWireWideEvent.Error.Execution, "<assign location='relays'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_wire_wide.scxml:54 :: relaying :: _transition_2

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWireWideEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
