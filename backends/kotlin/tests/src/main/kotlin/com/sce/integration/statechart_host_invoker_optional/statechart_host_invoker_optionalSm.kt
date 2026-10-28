// SCE-GENERATED — DO NOT EDIT
// source-hash: d2a16315bda3b311baf737203d9e9fa84cdbea31907669fbc8b60f205f09fef9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_host_invoker_optional.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_host_invoker_optional.scxml:36 :: _machine

package com.sce.integration.statechart_host_invoker_optional

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartHostInvokerOptionalState : State {
    data object Asking : StatechartHostInvokerOptionalState
    data object Idle : StatechartHostInvokerOptionalState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartHostInvokerOptionalEvent : Event {
    data object Ask : StatechartHostInvokerOptionalEvent
    sealed interface Done : StatechartHostInvokerOptionalEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Lookup : Invoke
        }
    }
    sealed interface Error : StatechartHostInvokerOptionalEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StatechartHostInvokerOptionalPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StatechartHostInvokerOptionalDoneInvokeLookupPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `done.invoke.lookup`. Consumers inject it via the `raiseDoneInvokeLookup` seam
// on the machine — they never name this class directly.
data class StatechartHostInvokerOptionalDoneInvokeLookupPayload(val rows: UByte, val at: Long?, val label: String?, val hit: Boolean?)


// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:result`: the record the host completes `<invoke id="lookup">` with (SCE Accepted Subset §2.12). */
data class StatechartHostInvokerOptionalLookupResult(
    val rows: UByte,
    val at: Long?,
    val label: String?,
    val hit: Boolean?,
) {
    /** The JSON `done.invoke.lookup` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("rows" to rows, "at" to at, "label" to label, "hit" to hit))
}

/**
 * The host side of this document's typed `<invoke type="x-sce-host">`s
 * (SCE Accepted Subset §2.12). Register it with `registerXSceHostInvoker`.
 */
interface StatechartHostInvokerOptionalXSceHostInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="lookup">`. [token] names this start;
     * a host that finishes later hands it back to `completeLookup` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startLookup(token: Long): StatechartHostInvokerOptionalLookupResult?

    /** §scxml-6.4: `<invoke id="lookup">`'s state exited while the start [token] names was still running. */
    fun cancelLookup(token: Long)
}


class StatechartHostInvokerOptionalStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StatechartHostInvokerOptionalState, StatechartHostInvokerOptionalEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `at` datamodel variable, the machine's own. */
    private var vAt: Long? = null
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var vLabel: String? = null
    /** W3C SCXML 5.2: the `hit` datamodel variable, the machine's own. */
    private var vHit: Boolean? = null
    /** W3C SCXML 5.2: the `answered` datamodel variable, published (`sce:direction="out"`). */
    private var vAnswered: UByte = 0.toUByte()

    /** What the published `answered` holds. */
    val answered: UByte get() = vAnswered
    /** W3C SCXML 5.2: the `rows_seen` datamodel variable, published (`sce:direction="out"`). */
    private var vRowsSeen: UByte = 0.toUByte()

    /** What the published `rows_seen` holds. */
    val rowsSeen: UByte get() = vRowsSeen
    /** W3C SCXML 5.2: the `at_present` datamodel variable, published (`sce:direction="out"`). */
    private var vAtPresent: Boolean = false

    /** What the published `at_present` holds. */
    val atPresent: Boolean get() = vAtPresent
    /** W3C SCXML 5.2: the `at_value` datamodel variable, published (`sce:direction="out"`). */
    private var vAtValue: Long = 0

    /** What the published `at_value` holds. */
    val atValue: Long get() = vAtValue
    /** W3C SCXML 5.2: the `label_present` datamodel variable, published (`sce:direction="out"`). */
    private var vLabelPresent: Boolean = false

    /** What the published `label_present` holds. */
    val labelPresent: Boolean get() = vLabelPresent
    /** W3C SCXML 5.2: the `label_value` datamodel variable, published (`sce:direction="out"`). */
    private var vLabelValue: String = ""

    /** What the published `label_value` holds. */
    val labelValue: String get() = vLabelValue
    /** W3C SCXML 5.2: the `hit_known` datamodel variable, published (`sce:direction="out"`). */
    private var vHitKnown: Boolean = false

    /** What the published `hit_known` holds. */
    val hitKnown: Boolean get() = vHitKnown
    /** W3C SCXML 5.2: the `hit_true` datamodel variable, published (`sce:direction="out"`). */
    private var vHitTrue: Boolean = false

    /** What the published `hit_true` holds. */
    val hitTrue: Boolean get() = vHitTrue

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var at: Long? = null
        var label: String? = null
        var hit: Boolean? = null
        var answered: UByte? = null
        var rowsSeen: UByte? = null
        var atPresent: Boolean? = null
        var atValue: Long? = null
        var labelPresent: Boolean? = null
        var labelValue: String? = null
        var hitKnown: Boolean? = null
        var hitTrue: Boolean? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.at?.let { vAt = it }
        params.label?.let { vLabel = it }
        params.hit?.let { vHit = it }
        params.answered?.let { vAnswered = it }
        params.rowsSeen?.let { vRowsSeen = it }
        params.atPresent?.let { vAtPresent = it }
        params.atValue?.let { vAtValue = it }
        params.labelPresent?.let { vLabelPresent = it }
        params.labelValue?.let { vLabelValue = it }
        params.hitKnown?.let { vHitKnown = it }
        params.hitTrue?.let { vHitTrue = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val answered: UByte,
        val rowsSeen: UByte,
        val atPresent: Boolean,
        val atValue: Long,
        val labelPresent: Boolean,
        val labelValue: String,
        val hitKnown: Boolean,
        val hitTrue: Boolean,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StatechartHostInvokerOptionalState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        answered = vAnswered,
        rowsSeen = vRowsSeen,
        atPresent = vAtPresent,
        atValue = vAtValue,
        labelPresent = vLabelPresent,
        labelValue = vLabelValue,
        hitKnown = vHitKnown,
        hitTrue = vHitTrue,
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
        first = "c17bcc6f983205f0e7b4f229d29e6652ab75ac153a4e27f397b78dba183d4fb4",
        revised = "0aeeff757f0470f02484f9f28d5b4ba1a105ea1f3e2c07ba2e9c27caadf0bce8",
        format2First = "c17bcc6f983205f0e7b4f229d29e6652ab75ac153a4e27f397b78dba183d4fb4",
        format2Revised = "0aeeff757f0470f02484f9f28d5b4ba1a105ea1f3e2c07ba2e9c27caadf0bce8",
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
            "at" to SavedValues.optional(vAt) { SavedValues.of(it) },
            "label" to SavedValues.optional(vLabel) { SavedValues.of(it) },
            "hit" to SavedValues.optional(vHit) { SavedValues.of(it) },
            "answered" to SavedValues.of(vAnswered),
            "rows_seen" to SavedValues.of(vRowsSeen),
            "at_present" to SavedValues.of(vAtPresent),
            "at_value" to SavedValues.of(vAtValue),
            "label_present" to SavedValues.of(vLabelPresent),
            "label_value" to SavedValues.of(vLabelValue),
            "hit_known" to SavedValues.of(vHitKnown),
            "hit_true" to SavedValues.of(vHitTrue),
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
        val saved1 = SavedValues.optional(savedGrown.variable("at"), "at") { e, w -> SavedValues.int64(e, w) }
        val saved2 = SavedValues.optional(savedGrown.variable("label"), "label") { e, w -> SavedValues.string(e, w, 16) }
        val saved3 = SavedValues.optional(savedGrown.variable("hit"), "hit") { e, w -> SavedValues.bool(e, w) }
        val saved4 = SavedValues.uint8(savedGrown.variable("answered"), "answered")
        val saved5 = SavedValues.uint8(savedGrown.variable("rows_seen"), "rows_seen")
        val saved6 = SavedValues.bool(savedGrown.variable("at_present"), "at_present")
        val saved7 = SavedValues.int64(savedGrown.variable("at_value"), "at_value")
        val saved8 = SavedValues.bool(savedGrown.variable("label_present"), "label_present")
        val saved9 = SavedValues.string(savedGrown.variable("label_value"), "label_value", 16)
        val saved10 = SavedValues.bool(savedGrown.variable("hit_known"), "hit_known")
        val saved11 = SavedValues.bool(savedGrown.variable("hit_true"), "hit_true")
        vAt = saved1
        vLabel = saved2
        vHit = saved3
        vAnswered = saved4
        vRowsSeen = saved5
        vAtPresent = saved6
        vAtValue = saved7
        vLabelPresent = saved8
        vLabelValue = saved9
        vHitKnown = saved10
        vHitTrue = saved11
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
        SavedValues.optional(savedGrown.variable("at"), "at") { e, w -> SavedValues.int64(e, w) }
        SavedValues.optional(savedGrown.variable("label"), "label") { e, w -> SavedValues.string(e, w, 16) }
        SavedValues.optional(savedGrown.variable("hit"), "hit") { e, w -> SavedValues.bool(e, w) }
        SavedValues.uint8(savedGrown.variable("answered"), "answered")
        SavedValues.uint8(savedGrown.variable("rows_seen"), "rows_seen")
        SavedValues.bool(savedGrown.variable("at_present"), "at_present")
        SavedValues.int64(savedGrown.variable("at_value"), "at_value")
        SavedValues.bool(savedGrown.variable("label_present"), "label_present")
        SavedValues.string(savedGrown.variable("label_value"), "label_value", 16)
        SavedValues.bool(savedGrown.variable("hit_known"), "hit_known")
        SavedValues.bool(savedGrown.variable("hit_true"), "hit_true")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StatechartHostInvokerOptionalState>> = listOf(
        Triple("x-sce-host", "lookup", StatechartHostInvokerOptionalState.Asking),
    )

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDoneInvokeLookupPayload: StatechartHostInvokerOptionalDoneInvokeLookupPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StatechartHostInvokerOptionalEvent, metadata: EventMetadata) {
        pendingDoneInvokeLookupPayload = null
        when (val tp = metadata.typedPayload) {
            is StatechartHostInvokerOptionalDoneInvokeLookupPayload -> pendingDoneInvokeLookupPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StatechartHostInvokerOptionalEvent.Done.Invoke.Lookup) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingDoneInvokeLookupPayload = StatechartHostInvokerOptionalDoneInvokeLookupPayload(fields.uint8("rows"), fields.optional("at") { _ -> fields.int64("at") }, fields.optional("label") { _ -> fields.string("label") }, fields.optional("hit") { _ -> fields.boolean("hit") })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.


    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="lookup">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeLookup(token: Long, result: StatechartHostInvokerOptionalLookupResult): Boolean =
        completeHostInvoke("x-sce-host", "lookup", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="x-sce-host"`.
     */
    fun registerXSceHostInvoker(invoker: StatechartHostInvokerOptionalXSceHostInvoker) {
        registerInvoker("x-sce-host") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "lookup" -> {
                        invoker.startLookup(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "lookup" -> {
                        invoker.cancelLookup(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StatechartHostInvokerOptionalState = StatechartHostInvokerOptionalState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StatechartHostInvokerOptionalState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartHostInvokerOptionalState, HistoryId>> =
            listOf(StateTarget(StatechartHostInvokerOptionalState.Idle))

        // W3C SCXML 3.13: asking's transition 0, as the microstep reads it.
        val transitionAskingAt0 = EnabledTransition<StatechartHostInvokerOptionalState, HistoryId>(
            StatechartHostInvokerOptionalState.Asking,
            listOf(StateTarget(StatechartHostInvokerOptionalState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartHostInvokerOptionalState, HistoryId>(
            StatechartHostInvokerOptionalState.Idle,
            listOf(StateTarget(StatechartHostInvokerOptionalState.Asking)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartHostInvokerOptionalState? = when (stateId) {
        "asking" -> StatechartHostInvokerOptionalState.Asking
        "idle" -> StatechartHostInvokerOptionalState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartHostInvokerOptionalState): String = when (state) {
        is StatechartHostInvokerOptionalState.Asking -> "asking"
        is StatechartHostInvokerOptionalState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartHostInvokerOptionalState): Int = when (state) {
        is StatechartHostInvokerOptionalState.Asking -> 1
        is StatechartHostInvokerOptionalState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartHostInvokerOptionalEvent? = when (name) {
        "ask" -> StatechartHostInvokerOptionalEvent.Ask
        "done.invoke" -> StatechartHostInvokerOptionalEvent.Done.Invoke.Self
        "done.invoke.lookup" -> StatechartHostInvokerOptionalEvent.Done.Invoke.Lookup
        "error.execution" -> StatechartHostInvokerOptionalEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartHostInvokerOptionalEvent): String? = when (event) {
        is StatechartHostInvokerOptionalEvent.Ask -> "ask"
        is StatechartHostInvokerOptionalEvent.Done.Invoke.Self -> "done.invoke"
        is StatechartHostInvokerOptionalEvent.Done.Invoke.Lookup -> "done.invoke.lookup"
        is StatechartHostInvokerOptionalEvent.Error.Execution -> "error.execution"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "lookup",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartHostInvokerOptionalState,
        event: StatechartHostInvokerOptionalEvent?
    ): EnabledTransition<StatechartHostInvokerOptionalState, HistoryId>? = when (state) {
        is StatechartHostInvokerOptionalState.Asking -> when {
            event is StatechartHostInvokerOptionalEvent.Done.Invoke.Lookup -> transitionAskingAt0
            else -> null
        }
        is StatechartHostInvokerOptionalState.Idle -> when {
            event is StatechartHostInvokerOptionalEvent.Ask -> transitionIdleAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_host_invoker_optional.scxml:36 :: _machine
    override fun onEntry(state: StatechartHostInvokerOptionalState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartHostInvokerOptionalState.Asking -> {
                // SCE-MAP: statechart_host_invoker_optional.scxml:58 :: asking :: _state_body
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
                    val generatedInvokeId = "asking.${System.identityHashCode(this)}.lookup"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "lookup",
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
                            raisePlatformError(StatechartHostInvokerOptionalEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartHostInvokerOptionalState.Idle -> {
                // SCE-MAP: statechart_host_invoker_optional.scxml:54 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_host_invoker_optional.scxml:36 :: _machine
    override fun onExit(state: StatechartHostInvokerOptionalState) {
        when (state) {
            is StatechartHostInvokerOptionalState.Asking -> {
                // SCE-MAP: statechart_host_invoker_optional.scxml:58 :: asking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "lookup")
            }
            is StatechartHostInvokerOptionalState.Idle -> {
                // SCE-MAP: statechart_host_invoker_optional.scxml:54 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_host_invoker_optional.scxml:36 :: _machine
    override fun executeTransitionContent(source: StatechartHostInvokerOptionalState, transitionIndex: Int) {
        when (source) {
        is StatechartHostInvokerOptionalState.Asking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_host_invoker_optional.scxml:60 :: asking :: _transition_0
                if (pendingDoneInvokeLookupPayload == null) {
                    return
                }

            vAt = pendingDoneInvokeLookupPayload!!.at

            if (try { vLabel = (pendingDoneInvokeLookupPayload!!.label)?.let { scePresent -> com.sce.forge.runtime.SceChecked.bounded(scePresent, 16) }; false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartHostInvokerOptionalEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }

            vHit = pendingDoneInvokeLookupPayload!!.hit

            vRowsSeen = pendingDoneInvokeLookupPayload!!.rows

            vAtPresent = vAt != null

            vAtValue = if (vAt != null) com.sce.forge.runtime.SceOptional.proven(vAt) else 0

            vLabelPresent = vLabel != null


            if (vLabel != null) {

            if (try { vLabelValue = com.sce.forge.runtime.SceChecked.bounded(com.sce.forge.runtime.SceOptional.proven(vLabel), 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartHostInvokerOptionalEvent.Error.Execution, "<assign location='label_value'>: an integer operation overflowed or failed"); true }) {
                return
            }
            } else {

            if (try { vLabelValue = com.sce.forge.runtime.SceChecked.bounded("", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartHostInvokerOptionalEvent.Error.Execution, "<assign location='label_value'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            vHitKnown = vHit != null

            vHitTrue = vHit == true

            if (try { vAnswered = com.sce.forge.runtime.SceChecked.add(vAnswered, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartHostInvokerOptionalEvent.Error.Execution, "<assign location='answered'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
