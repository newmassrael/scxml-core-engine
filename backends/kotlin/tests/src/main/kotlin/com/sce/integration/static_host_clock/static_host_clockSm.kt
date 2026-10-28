// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_clock.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_clock.scxml:24 :: _machine

package com.sce.integration.static_host_clock

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostClockState : State {
    data object Idle : StaticHostClockState
    data object Locating : StaticHostClockState
    data object Reading : StaticHostClockState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostClockEvent : Event {
    sealed interface Done : StaticHostClockEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Now : Invoke
            data object Zone : Invoke
        }
    }
    sealed interface Error : StaticHostClockEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Now : Invoke
            data object Zone : Invoke
        }
    }
    sealed interface Host : StaticHostClockEvent {
        sealed interface Clock : Host {
            sealed interface Now : Clock {
                data object Request : Now
            }
            sealed interface Zone : Clock {
                data object Request : Zone
            }
        }
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticHostClockPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticHostClockDoneInvokeNowPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.now`. Consumers inject it via the `raiseDoneInvokeNow` seam
// on the machine — they never name this class directly.
data class StaticHostClockDoneInvokeNowPayload(val error: String, val utc_ms: Long, val monotonic_ms: Long)

// StaticHostClockDoneInvokeZonePayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.zone`. Consumers inject it via the `raiseDoneInvokeZone` seam
// on the machine — they never name this class directly.
data class StaticHostClockDoneInvokeZonePayload(val error: String, val zone_id: String, val utc_offset_s: Int)


// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:result`: the record the host completes `<invoke id="now">` with (SCE Accepted Subset §2.12). */
data class StaticHostClockNowResult(
    val error: String,
    val utc_ms: Long,
    val monotonic_ms: Long,
) {
    /** The JSON `done.invoke.now` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "utc_ms" to utc_ms, "monotonic_ms" to monotonic_ms))
}

/** `sce:result`: the record the host completes `<invoke id="zone">` with (SCE Accepted Subset §2.12). */
data class StaticHostClockZoneResult(
    val error: String,
    val zone_id: String,
    val utc_offset_s: Int,
) {
    /** The JSON `done.invoke.zone` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "zone_id" to zone_id, "utc_offset_s" to utc_offset_s))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-clock-now">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostClockNowInvoker`.
 */
interface StaticHostClockSceHostClockNowInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="now">`. [token] names this start;
     * a host that finishes later hands it back to `completeNow` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startNow(token: Long): StaticHostClockNowResult?

    /** §scxml-6.4: `<invoke id="now">`'s state exited while the start [token] names was still running. */
    fun cancelNow(token: Long)
}

/**
 * The host side of this document's typed `<invoke type="sce-host-clock-zone">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostClockZoneInvoker`.
 */
interface StaticHostClockSceHostClockZoneInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="zone">`. [token] names this start;
     * a host that finishes later hands it back to `completeZone` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startZone(token: Long): StaticHostClockZoneResult?

    /** §scxml-6.4: `<invoke id="zone">`'s state exited while the start [token] names was still running. */
    fun cancelZone(token: Long)
}


class StaticHostClockStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticHostClockState, StaticHostClockEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `outcome` datamodel variable, published (`sce:direction="out"`). */
    private var vOutcome: String = ""

    /** What the published `outcome` holds. */
    val outcome: String get() = vOutcome
    /** W3C SCXML 5.2: the `utc_ms` datamodel variable, published (`sce:direction="out"`). */
    private var vUtcMs: Long = 0

    /** What the published `utc_ms` holds. */
    val utcMs: Long get() = vUtcMs
    /** W3C SCXML 5.2: the `monotonic_ms` datamodel variable, published (`sce:direction="out"`). */
    private var vMonotonicMs: Long = 0

    /** What the published `monotonic_ms` holds. */
    val monotonicMs: Long get() = vMonotonicMs
    /** W3C SCXML 5.2: the `zone_id` datamodel variable, published (`sce:direction="out"`). */
    private var vZoneId: String = ""

    /** What the published `zone_id` holds. */
    val zoneId: String get() = vZoneId
    /** W3C SCXML 5.2: the `utc_offset_s` datamodel variable, published (`sce:direction="out"`). */
    private var vUtcOffsetS: Int = 0

    /** What the published `utc_offset_s` holds. */
    val utcOffsetS: Int get() = vUtcOffsetS

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var outcome: String? = null
        var utcMs: Long? = null
        var monotonicMs: Long? = null
        var zoneId: String? = null
        var utcOffsetS: Int? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.outcome?.let { vOutcome = it }
        params.utcMs?.let { vUtcMs = it }
        params.monotonicMs?.let { vMonotonicMs = it }
        params.zoneId?.let { vZoneId = it }
        params.utcOffsetS?.let { vUtcOffsetS = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val outcome: String,
        val utcMs: Long,
        val monotonicMs: Long,
        val zoneId: String,
        val utcOffsetS: Int,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostClockState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        outcome = vOutcome,
        utcMs = vUtcMs,
        monotonicMs = vMonotonicMs,
        zoneId = vZoneId,
        utcOffsetS = vUtcOffsetS,
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
        first = "141dd63c49b9a5468a2b25316c88f8e51335fc52b99a414f3690a97bddae4723",
        revised = "9e2c0126d926e2df92012105a0f1d3d2582fa4ceb0b69a155973d005b28fb879",
        format2First = "141dd63c49b9a5468a2b25316c88f8e51335fc52b99a414f3690a97bddae4723",
        format2Revised = "9e2c0126d926e2df92012105a0f1d3d2582fa4ceb0b69a155973d005b28fb879",
        records = listOf(
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
            "outcome" to SavedValues.of(vOutcome),
            "utc_ms" to SavedValues.of(vUtcMs),
            "monotonic_ms" to SavedValues.of(vMonotonicMs),
            "zone_id" to SavedValues.of(vZoneId),
            "utc_offset_s" to SavedValues.of(vUtcOffsetS),
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
        val saved1 = SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        val saved2 = SavedValues.int64(savedGrown.variable("utc_ms"), "utc_ms")
        val saved3 = SavedValues.int64(savedGrown.variable("monotonic_ms"), "monotonic_ms")
        val saved4 = SavedValues.string(savedGrown.variable("zone_id"), "zone_id", 64)
        val saved5 = SavedValues.int32(savedGrown.variable("utc_offset_s"), "utc_offset_s")
        vOutcome = saved1
        vUtcMs = saved2
        vMonotonicMs = saved3
        vZoneId = saved4
        vUtcOffsetS = saved5
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
        SavedValues.string(savedGrown.variable("outcome"), "outcome", 24)
        SavedValues.int64(savedGrown.variable("utc_ms"), "utc_ms")
        SavedValues.int64(savedGrown.variable("monotonic_ms"), "monotonic_ms")
        SavedValues.string(savedGrown.variable("zone_id"), "zone_id", 64)
        SavedValues.int32(savedGrown.variable("utc_offset_s"), "utc_offset_s")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticHostClockState>> = listOf(
        Triple("sce-host-clock-now", "now", StaticHostClockState.Reading),
        Triple("sce-host-clock-zone", "zone", StaticHostClockState.Locating),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeNowPayload: StaticHostClockDoneInvokeNowPayload? = null
    private var pendingDoneInvokeZonePayload: StaticHostClockDoneInvokeZonePayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticHostClockEvent, metadata: EventMetadata) {
        pendingDoneInvokeNowPayload = null
        pendingDoneInvokeZonePayload = null
        when (val tp = metadata.typedPayload) {
            is StaticHostClockDoneInvokeNowPayload -> pendingDoneInvokeNowPayload = tp
            is StaticHostClockDoneInvokeZonePayload -> pendingDoneInvokeZonePayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticHostClockEvent.Done.Invoke.Now) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeNowPayload = StaticHostClockDoneInvokeNowPayload(fields.string("error"), fields.int64("utc_ms"), fields.int64("monotonic_ms"))
                } else if (event == StaticHostClockEvent.Done.Invoke.Zone) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeZonePayload = StaticHostClockDoneInvokeZonePayload(fields.string("error"), fields.string("zone_id"), fields.int32("utc_offset_s"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="now">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeNow(token: Long, result: StaticHostClockNowResult): Boolean =
        completeHostInvoke("sce-host-clock-now", "now", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-clock-now"`.
     */
    fun registerSceHostClockNowInvoker(invoker: StaticHostClockSceHostClockNowInvoker) {
        registerInvoker("sce-host-clock-now") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "now" -> {
                        invoker.startNow(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "now" -> {
                        invoker.cancelNow(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }

    /**
     * Complete `<invoke id="zone">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeZone(token: Long, result: StaticHostClockZoneResult): Boolean =
        completeHostInvoke("sce-host-clock-zone", "zone", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-clock-zone"`.
     */
    fun registerSceHostClockZoneInvoker(invoker: StaticHostClockSceHostClockZoneInvoker) {
        registerInvoker("sce-host-clock-zone") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "zone" -> {
                        invoker.startZone(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "zone" -> {
                        invoker.cancelZone(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticHostClockState = StaticHostClockState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticHostClockState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostClockState, HistoryId>> =
            listOf(StateTarget(StaticHostClockState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Idle,
            listOf(StateTarget(StaticHostClockState.Reading)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Idle,
            listOf(StateTarget(StaticHostClockState.Locating)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 0, as the microstep reads it.
        val transitionLocatingAt0 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Locating,
            listOf(StateTarget(StaticHostClockState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 1, as the microstep reads it.
        val transitionLocatingAt1 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Locating,
            listOf(StateTarget(StaticHostClockState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 2, as the microstep reads it.
        val transitionLocatingAt2 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Locating,
            listOf(StateTarget(StaticHostClockState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: locating's transition 3, as the microstep reads it.
        val transitionLocatingAt3 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Locating,
            listOf(StateTarget(StaticHostClockState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: reading's transition 0, as the microstep reads it.
        val transitionReadingAt0 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Reading,
            listOf(StateTarget(StaticHostClockState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: reading's transition 1, as the microstep reads it.
        val transitionReadingAt1 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Reading,
            listOf(StateTarget(StaticHostClockState.Idle)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: reading's transition 2, as the microstep reads it.
        val transitionReadingAt2 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Reading,
            listOf(StateTarget(StaticHostClockState.Idle)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: reading's transition 3, as the microstep reads it.
        val transitionReadingAt3 = EnabledTransition<StaticHostClockState, HistoryId>(
            StaticHostClockState.Reading,
            listOf(StateTarget(StaticHostClockState.Idle)),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostClockState? = when (stateId) {
        "idle" -> StaticHostClockState.Idle
        "locating" -> StaticHostClockState.Locating
        "reading" -> StaticHostClockState.Reading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostClockState): String = when (state) {
        is StaticHostClockState.Idle -> "idle"
        is StaticHostClockState.Locating -> "locating"
        is StaticHostClockState.Reading -> "reading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostClockState): Int = when (state) {
        is StaticHostClockState.Idle -> 0
        is StaticHostClockState.Locating -> 2
        is StaticHostClockState.Reading -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHostClockEvent? = when (name) {
        "done.invoke" -> StaticHostClockEvent.Done.Invoke.Self
        "done.invoke.now" -> StaticHostClockEvent.Done.Invoke.Now
        "done.invoke.zone" -> StaticHostClockEvent.Done.Invoke.Zone
        "error.execution" -> StaticHostClockEvent.Error.Execution
        "error.invoke.now" -> StaticHostClockEvent.Error.Invoke.Now
        "error.invoke.zone" -> StaticHostClockEvent.Error.Invoke.Zone
        "host.clock.now.request" -> StaticHostClockEvent.Host.Clock.Now.Request
        "host.clock.zone.request" -> StaticHostClockEvent.Host.Clock.Zone.Request
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHostClockEvent): String? = when (event) {
        is StaticHostClockEvent.Done.Invoke.Self -> "done.invoke"
        is StaticHostClockEvent.Done.Invoke.Now -> "done.invoke.now"
        is StaticHostClockEvent.Done.Invoke.Zone -> "done.invoke.zone"
        is StaticHostClockEvent.Error.Execution -> "error.execution"
        is StaticHostClockEvent.Error.Invoke.Now -> "error.invoke.now"
        is StaticHostClockEvent.Error.Invoke.Zone -> "error.invoke.zone"
        is StaticHostClockEvent.Host.Clock.Now.Request -> "host.clock.now.request"
        is StaticHostClockEvent.Host.Clock.Zone.Request -> "host.clock.zone.request"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "now",
        "zone",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostClockState,
        event: StaticHostClockEvent?
    ): EnabledTransition<StaticHostClockState, HistoryId>? = when (state) {
        is StaticHostClockState.Idle -> when {
            event is StaticHostClockEvent.Host.Clock.Now.Request -> transitionIdleAt0
            event is StaticHostClockEvent.Host.Clock.Zone.Request -> transitionIdleAt1
            else -> null
        }
        is StaticHostClockState.Locating -> when {
            event is StaticHostClockEvent.Done.Invoke.Zone && pendingDoneInvokeZonePayload != null && (pendingDoneInvokeZonePayload!!.error == "") -> transitionLocatingAt0
            event is StaticHostClockEvent.Done.Invoke.Zone -> transitionLocatingAt1
            event is StaticHostClockEvent.Error.Invoke.Zone -> transitionLocatingAt2
            event is StaticHostClockEvent.Error.Execution -> transitionLocatingAt3
            else -> null
        }
        is StaticHostClockState.Reading -> when {
            event is StaticHostClockEvent.Done.Invoke.Now && pendingDoneInvokeNowPayload != null && (pendingDoneInvokeNowPayload!!.error == "") -> transitionReadingAt0
            event is StaticHostClockEvent.Done.Invoke.Now -> transitionReadingAt1
            event is StaticHostClockEvent.Error.Invoke.Now -> transitionReadingAt2
            event is StaticHostClockEvent.Error.Execution -> transitionReadingAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_clock.scxml:24 :: _machine
    override fun onEntry(state: StaticHostClockState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostClockState.Idle -> {
                // SCE-MAP: static_host_clock.scxml:36 :: idle :: _state_body
            }
            is StaticHostClockState.Locating -> {
                // SCE-MAP: static_host_clock.scxml:59 :: locating :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "locating.${System.identityHashCode(this)}.zone"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-clock-zone",
                                invokeId = "zone",
                                src = "",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StaticHostClockEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StaticHostClockState.Reading -> {
                // SCE-MAP: static_host_clock.scxml:41 :: reading :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "reading.${System.identityHashCode(this)}.now"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-clock-now",
                                invokeId = "now",
                                src = "",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StaticHostClockEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_clock.scxml:24 :: _machine
    override fun onExit(state: StaticHostClockState) {
        when (state) {
            is StaticHostClockState.Idle -> {
                // SCE-MAP: static_host_clock.scxml:36 :: idle :: _state_body
            }
            is StaticHostClockState.Locating -> {
                // SCE-MAP: static_host_clock.scxml:59 :: locating :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-clock-zone", "zone")
            }
            is StaticHostClockState.Reading -> {
                // SCE-MAP: static_host_clock.scxml:41 :: reading :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-clock-now", "now")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_clock.scxml:24 :: _machine
    override fun executeTransitionContent(source: StaticHostClockState, transitionIndex: Int) {
        when (source) {
        is StaticHostClockState.Locating -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_clock.scxml:61 :: locating :: _transition_0
                if (pendingDoneInvokeZonePayload == null) {
                    return
                }

            if (try { vZoneId = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeZonePayload!!.zone_id, 64); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='zone_id'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vUtcOffsetS = pendingDoneInvokeZonePayload!!.utc_offset_s

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_clock.scxml:66 :: locating :: _transition_1
                if (pendingDoneInvokeZonePayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeZonePayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_clock.scxml:69 :: locating :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_clock.scxml:72 :: locating :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticHostClockState.Reading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_clock.scxml:43 :: reading :: _transition_0
                if (pendingDoneInvokeNowPayload == null) {
                    return
                }

            vUtcMs = pendingDoneInvokeNowPayload!!.utc_ms

            vMonotonicMs = pendingDoneInvokeNowPayload!!.monotonic_ms

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("done", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_host_clock.scxml:48 :: reading :: _transition_1
                if (pendingDoneInvokeNowPayload == null) {
                    return
                }

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded(pendingDoneInvokeNowPayload!!.error, 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_host_clock.scxml:51 :: reading :: _transition_2

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("deadline", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_host_clock.scxml:54 :: reading :: _transition_3

            if (try { vOutcome = com.sce.forge.runtime.SceChecked.bounded("refused", 24); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticHostClockEvent.Error.Execution, "<assign location='outcome'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
