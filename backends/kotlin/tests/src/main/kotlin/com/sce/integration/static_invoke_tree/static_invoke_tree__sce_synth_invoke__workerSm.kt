// SCE-GENERATED — DO NOT EDIT
// source-hash: 75c0c87af90d82dec15f516a721dd486d47e1280589aefe06f15efdef5f4f850

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_tree__sce_synth_invoke__worker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:3 :: _machine

package com.sce.integration.static_invoke_tree

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeTreeSceSynthInvokeWorkerState : State {
    data object First : StaticInvokeTreeSceSynthInvokeWorkerState
    data object Second : StaticInvokeTreeSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeTreeSceSynthInvokeWorkerEvent : Event {
    data object Answer : StaticInvokeTreeSceSynthInvokeWorkerEvent
    sealed interface Done : StaticInvokeTreeSceSynthInvokeWorkerEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Clock : Invoke
        }
    }
    sealed interface Error : StaticInvokeTreeSceSynthInvokeWorkerEvent {
        data object Communication : Error
        data object Execution : Error
    }
    data object Go : StaticInvokeTreeSceSynthInvokeWorkerEvent
    data object Nudge : StaticInvokeTreeSceSynthInvokeWorkerEvent
    data object Question : StaticInvokeTreeSceSynthInvokeWorkerEvent
    data object Report : StaticInvokeTreeSceSynthInvokeWorkerEvent
    data object Stamped : StaticInvokeTreeSceSynthInvokeWorkerEvent
    data object Swap : StaticInvokeTreeSceSynthInvokeWorkerEvent
}
// ── SCE Accepted Subset §2.15: sce-static enum and record variable classes ─────
/** SCE Accepted Subset §2.15: an `enum:ViewMode` datamodel value. */
enum class StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum(val declaredName: String) {
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
        fun fromSaved(value: Any?, what: String): StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum {
            val declared = SavedValues.string(value, what)
            return entries.firstOrNull { it.declaredName == declared }
                ?: throw StateRefusal("'$what' ($declared) is not a variant of ViewMode")
        }

        /** The variant that declares [raw], or null for a number no variant declares. */
        fun fromUnderlying(raw: UByte): StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum? = entries.firstOrNull { it.toUnderlying() == raw }
    }
}
/** SCE Accepted Subset §2.15: a `record:Revised` datamodel value. */
data class StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord(val level: UByte, val layout: StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum, val urgent: Boolean) {
    /** This value as a saved state writes it. */
    fun toSaved(): Any = linkedMapOf("level" to SavedValues.of(level), "layout" to layout.toSaved(), "urgent" to SavedValues.of(urgent))

    companion object {
        /** The value a saved state holds, refused unless it is one. */
        fun fromSaved(value: Any?, what: String): StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord = StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord(level = SavedValues.uint8(SavedValues.field(value, what, "level"), "$what.level"), layout = StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum.fromSaved(SavedValues.field(value, what, "layout"), "$what.layout"), urgent = SavedValues.bool(SavedValues.field(value, what, "urgent"), "$what.urgent"))
    }
}
// --- State Machine (W3C SCXML) ---

// ── SCE Accepted Subset 2.12: typed host-run invoke interface ─────────
/** `sce:result`: the record the host completes `<invoke id="clock">` with (SCE Accepted Subset §2.12). */
data class StaticInvokeTreeSceSynthInvokeWorkerClockResult(
    val error: String,
    val utc_ms: Long,
    val monotonic_ms: Long,
) {
    /** The JSON `done.invoke.clock` carries this record as. */
    fun wire(): String = EventPayload.encode(mapOf("error" to error, "utc_ms" to utc_ms, "monotonic_ms" to monotonic_ms))
}

/**
 * The host side of this document's typed `<invoke type="sce-host-clock-now">`s
 * (SCE Accepted Subset §2.12). Register it with `registerSceHostClockNowInvoker`.
 */
interface StaticInvokeTreeSceSynthInvokeWorkerSceHostClockNowInvoker {
    /**
     * §scxml-6.4: begin `<invoke id="clock">`. [token] names this start;
     * a host that finishes later hands it back to `completeClock` (or
     * `completeHostInvoke`). Non-null completes the invocation now.
     */
    fun startClock(token: Long): StaticInvokeTreeSceSynthInvokeWorkerClockResult?

    /** §scxml-6.4: `<invoke id="clock">`'s state exited while the start [token] names was still running. */
    fun cancelClock(token: Long)
}


class StaticInvokeTreeSceSynthInvokeWorkerStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticInvokeTreeSceSynthInvokeWorkerState, StaticInvokeTreeSceSynthInvokeWorkerEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `last` datamodel variable, the machine's own. */
    private var vLast: StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord = StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord(level = 0.toUByte(), layout = StaticInvokeTreeSceSynthInvokeWorkerViewModeEnum.MONTH, urgent = false)
    /** W3C SCXML 5.2: the `pick` datamodel variable, the machine's own. */
    private var vPick: String = "static_hybrid_watcher.scxml"
    /** W3C SCXML 5.2: the `base` datamodel variable, the machine's own. */
    private var vBase: UInt = 7.toUInt()
    /** W3C SCXML 5.2: the `steps` datamodel variable, the machine's own. */
    private var vSteps: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `helper_id` datamodel variable, the machine's own. */
    private var vHelperId: String = ""

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var pick: String? = null
        var base: UInt? = null
        var steps: UInt? = null
        var helperId: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.pick?.let { vPick = it }
        params.base?.let { vBase = it }
        params.steps?.let { vSteps = it }
        params.helperId?.let { vHelperId = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticInvokeTreeSceSynthInvokeWorkerState>,
        val truncated: Boolean,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), false)
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
        _snapshot.value = Snapshot(activeConfiguration, truncated)
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
        first = "850830555bb055d5c90bde694afb8be5eb62c215679dc890c5119668be6599fd",
        revised = "20db9275db41c57b3b1ac7f4cd55f3dd13d2d342523321fb4a47097cb59547cd",
        format2First = "850830555bb055d5c90bde694afb8be5eb62c215679dc890c5119668be6599fd",
        format2Revised = "20db9275db41c57b3b1ac7f4cd55f3dd13d2d342523321fb4a47097cb59547cd",
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
            "pick" to SavedValues.of(vPick),
            "base" to SavedValues.of(vBase),
            "steps" to SavedValues.of(vSteps),
            "helper_id" to SavedValues.of(vHelperId),
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
        val saved1 = StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord.fromSaved(savedGrown.variable("last"), "last")
        val saved2 = SavedValues.string(savedGrown.variable("pick"), "pick", 48)
        val saved3 = SavedValues.uint32(savedGrown.variable("base"), "base")
        val saved4 = SavedValues.uint32(savedGrown.variable("steps"), "steps")
        val saved5 = SavedValues.string(savedGrown.variable("helper_id"), "helper_id", 64)
        vLast = saved1
        vPick = saved2
        vBase = saved3
        vSteps = saved4
        vHelperId = saved5
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
        StaticInvokeTreeSceSynthInvokeWorkerRevisedRecord.fromSaved(savedGrown.variable("last"), "last")
        SavedValues.string(savedGrown.variable("pick"), "pick", 48)
        SavedValues.uint32(savedGrown.variable("base"), "base")
        SavedValues.uint32(savedGrown.variable("steps"), "steps")
        SavedValues.string(savedGrown.variable("helper_id"), "helper_id", 64)
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeTreeSceSynthInvokeWorkerState>> = listOf(
        "_invoke_0" to StaticInvokeTreeSceSynthInvokeWorkerState.Second,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeTreeSceSynthInvokeWorkerState) = deferStaticInvoke(invokeId, state)

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StaticInvokeTreeSceSynthInvokeWorkerState>> = listOf(
        Triple("sce-host-clock-now", "clock", StaticInvokeTreeSceSynthInvokeWorkerState.Second),
    )

    // SCE Accepted Subset 2.12: registration and typed completion for this
    // document's typed host-run invokes.

    /**
     * Complete `<invoke id="clock">`'s start [token] with its record —
     * `completeHostInvoke` with the record's JSON, so a stale or unknown token is
     * refused the same way (`false`).
     */
    fun completeClock(token: Long, result: StaticInvokeTreeSceSynthInvokeWorkerClockResult): Boolean =
        completeHostInvoke("sce-host-clock-now", "clock", token, result.wire())

    /**
     * Register [invoker] as the handler for `type="sce-host-clock-now"`.
     */
    fun registerSceHostClockNowInvoker(invoker: StaticInvokeTreeSceSynthInvokeWorkerSceHostClockNowInvoker) {
        registerInvoker("sce-host-clock-now") { event ->
            val start = event.start
            val cancel = event.cancel
            if (start != null) {
                when (start.invokeId) {
                    "clock" -> {
                        invoker.startClock(start.token)?.let { HostInvokeResponse(doneData = it.wire()) }
                    }
                    else -> null
                }
            } else if (cancel != null) {
                when (cancel.invokeId) {
                    "clock" -> {
                        invoker.cancelClock(cancel.token)
                        null
                    }
                    else -> null
                }
            } else {
                null
            }
        }
    }


    override val initialState: StaticInvokeTreeSceSynthInvokeWorkerState = StaticInvokeTreeSceSynthInvokeWorkerState.First

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(StaticInvokeTreeSceSynthInvokeWorkerState.First))

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.First,
            listOf(StateTarget(StaticInvokeTreeSceSynthInvokeWorkerState.Second)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: first's transition 1, as the microstep reads it.
        val transitionFirstAt1 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.First,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.Second,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: second's transition 1, as the microstep reads it.
        val transitionSecondAt1 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.Second,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: second's transition 2, as the microstep reads it.
        val transitionSecondAt2 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.Second,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: second's transition 3, as the microstep reads it.
        val transitionSecondAt3 = EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeTreeSceSynthInvokeWorkerState.Second,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeTreeSceSynthInvokeWorkerState? = when (stateId) {
        "first" -> StaticInvokeTreeSceSynthInvokeWorkerState.First
        "second" -> StaticInvokeTreeSceSynthInvokeWorkerState.Second
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeTreeSceSynthInvokeWorkerState): String = when (state) {
        is StaticInvokeTreeSceSynthInvokeWorkerState.First -> "first"
        is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> "second"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeTreeSceSynthInvokeWorkerState): Int = when (state) {
        is StaticInvokeTreeSceSynthInvokeWorkerState.First -> 0
        is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeTreeSceSynthInvokeWorkerEvent? = when (name) {
        "answer" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Answer
        "done.invoke" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Self
        "done.invoke.clock" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Clock
        "error.communication" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication
        "error.execution" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution
        "go" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Go
        "nudge" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Nudge
        "question" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Question
        "report" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Report
        "stamped" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Stamped
        "swap" -> StaticInvokeTreeSceSynthInvokeWorkerEvent.Swap
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeTreeSceSynthInvokeWorkerEvent): String? = when (event) {
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Answer -> "answer"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Clock -> "done.invoke.clock"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication -> "error.communication"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution -> "error.execution"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Go -> "go"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Nudge -> "nudge"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Question -> "question"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Report -> "report"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Stamped -> "stamped"
        is StaticInvokeTreeSceSynthInvokeWorkerEvent.Swap -> "swap"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "clock",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeTreeSceSynthInvokeWorkerState,
        event: StaticInvokeTreeSceSynthInvokeWorkerEvent?
    ): EnabledTransition<StaticInvokeTreeSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is StaticInvokeTreeSceSynthInvokeWorkerState.First -> when {
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Go -> transitionFirstAt0
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Question -> transitionFirstAt1
            else -> null
        }
        is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> when {
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Clock -> transitionSecondAt0
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Swap -> transitionSecondAt1
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Nudge -> transitionSecondAt2
            event is StaticInvokeTreeSceSynthInvokeWorkerEvent.Question -> transitionSecondAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onEntry(state: StaticInvokeTreeSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeTreeSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:21 :: first :: _state_body
            }
            is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:30 :: second :: _state_body
                // W3C SCXML 6.4: Defer the hybrid invoke until macrostep end. `deferStaticInvoke`
                // starts it — for entering the state and for a restore alike, so the two
                // cannot start it differently.
                deferStaticInvoke("_invoke_0", state)
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
                    val generatedInvokeId = "second.${System.identityHashCode(this)}.clock"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce-host-clock-now",
                                invokeId = "clock",
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
                            raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeTreeSceSynthInvokeWorkerState) {
        when (invokeId) {
            "_invoke_0" -> {
                // W3C SCXML 6.4: Hybrid invoke — runtime expression evaluation + dynamic child
                // C++ parity: StateMachine::createFromSCXMLString() / FileLoadingHelper::loadScxmlFile()
                run {
                    // docs/adr/0045: the platform part of the id is the number of
                    // this start, so the id comes from the tree alone.
                    val generatedInvokeId = "second.${nextInvokeStart("_invoke_0")}._invoke_0"
                    // docs/adr/0045: under `datamodel="sce-static"` the id is written to
                    // the string variable `idlocation` names.
                    vHelperId = generatedInvokeId
                    deferInvoke(state, generatedInvokeId) {
                        try {
                            // §scxml-6.4: the `srcexpr` names the document to start.
                            val filePath: String = vPick
                            // §scxml-6.4 + SCE_ACCEPTED_SUBSET.md §2.13: the value
                            // chooses among the declared candidates, matched on the
                            // document stem so `file:x.scxml` and `./x.scxml` name
                            // one child. `startInvoke` takes a star-projected
                            // machine, so only the construction differs here.
                            // docs/adr/0045: each candidate takes its id where it
                            // is built, before it is handed an argument.
                            val __sceSelected = DocumentStem.of(filePath)
                            val childSM = when (__sceSelected) {
                                "static_hybrid_watcher" -> adoptChild(StaticHybridWatcherStateMachine(), generatedInvokeId).also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridWatcherStateMachine.InvokeParams()
                                    childSeed.start = vBase
                                    child.acceptParams(childSeed)

                                }
                                "static_hybrid_holder" -> adoptChild(StaticHybridHolderStateMachine(), generatedInvokeId).also { child ->
                                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                                    val childSeed = StaticHybridHolderStateMachine.InvokeParams()
                                    childSeed.start = vBase
                                    child.acceptParams(childSeed)

                                }
                                else -> {
                                    // The failure the Interpreter reports when a
                                    // document will not load: nothing to create.
                                    raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<invoke srcexpr> evaluated to a document it did not declare")
                                    return@deferInvoke
                                }
                            }

                            startInvoke("_invoke_0", childSM, false, StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Self, "", generatedInvokeId, __sceSelected, __sceSelected, generatedInvokeId)
                        } catch (_: Exception) {
                            // W3C SCXML 6.4: the child could not be started. Evaluation
                            // failure no longer reaches here — it has its own raise above,
                            // under the one wording every emitter uses for that fact.
                            raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<invoke> could not start a child")
                        }
                    }
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // docs/adr/0045, section 3, steps 1 and 3: the machine of a running child a
    // saved state lists, built as the `<invoke>` that started it builds it and
    // with what a start does with the child once built. Nothing is evaluated here:
    // no `<param>`, no `srcexpr`, no `idlocation` — the child resumes where it
    // stood, and a hybrid child is the candidate its saved `document` names,
    // whatever the `srcexpr` reads now.
    override fun buildSavedChild(child: SavedChild): ResumedChild<StaticInvokeTreeSceSynthInvokeWorkerEvent> = when (child.id) {
        "_invoke_0" -> when (child.document) {
            "static_hybrid_watcher" -> ResumedChild(
                StaticHybridWatcherStateMachine(),
                false, StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Self, "", "static_hybrid_watcher", null,
            )
            "static_hybrid_holder" -> ResumedChild(
                StaticHybridHolderStateMachine(),
                false, StaticInvokeTreeSceSynthInvokeWorkerEvent.Done.Invoke.Self, "", "static_hybrid_holder", null,
            )
            else -> throw StateRefusal("invokes names '${child.id}' running '${child.document}', a document that `<invoke>` does not start")
        }
        else -> throw StateRefusal("the document has no child session '${child.id}'")
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onExit(state: StaticInvokeTreeSceSynthInvokeWorkerState) {
        when (state) {
            is StaticInvokeTreeSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:21 :: first :: _state_body
            }
            is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:30 :: second :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("_invoke_0")
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce-host-clock-now", "clock")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticInvokeTreeSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
        is StaticInvokeTreeSceSynthInvokeWorkerState.First -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:22 :: first :: _transition_0

            if (try { vSteps = com.sce.forge.runtime.SceChecked.add(vSteps, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<assign location='steps'>: an integer operation overflowed or failed"); true }) {
                return
            }


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test187): Delayed send to parent — cancelled when child stops
            scheduleParentSend("report", 5000L, "report", sendData, StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:26 :: first :: _transition_1


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_0")
                return@send true
            }
            toParent("answer", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        is StaticInvokeTreeSceSynthInvokeWorkerState.Second -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:36 :: second :: _transition_0


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_1")
                return@send true
            }
            toParent("stamped", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:39 :: second :: _transition_1

            if (try { vPick = com.sce.forge.runtime.SceChecked.bounded("static_hybrid_holder.scxml", 48); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<assign location='pick'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:42 :: second :: _transition_2

            if (try { vSteps = com.sce.forge.runtime.SceChecked.add(vSteps, 10.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Execution, "<assign location='steps'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_invoke_tree__sce_synth_invoke__worker.scxml:45 :: second :: _transition_3


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.4 (test191): Send event to parent via invoke callback.
            // W3C SCXML C.1: a session its host started, not an `<invoke>`, has
            // no parent — error.communication, nothing delivered, the block ended.
            val toParent = onSendToParent
            if (toParent == null) {
                raisePlatformError(StaticInvokeTreeSceSynthInvokeWorkerEvent.Error.Communication, "<send target='#_parent'> has no parent session to reach", "__send_2")
                return@send true
            }
            toParent("answer", sendData)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            else -> {}
        }
        }
    }
}
