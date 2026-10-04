// SCE-GENERATED — DO NOT EDIT
// source-hash: dd1a6b6c55ab533e307bc7e47f7d1f334126ccba017dba6d270609c22c55e26e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_send_params.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_send_params.scxml:23 :: _machine

package com.sce.integration.static_send_params

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticSendParamsState : State {
    data object Idle : StaticSendParamsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticSendParamsEvent : Event {
    data object Bad : StaticSendParamsEvent
    data object Echo : StaticSendParamsEvent
    sealed interface Error : StaticSendParamsEvent {
        data object Execution : Error
    }
    data object Go : StaticSendParamsEvent
    data object Partial : StaticSendParamsEvent
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticSendParamsPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticSendParamsEchoPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `echo`. Consumers inject it via the `raiseEcho` seam
// on the machine — they never name this class directly.
data class StaticSendParamsEchoPayload(val total: UInt, val ok: Boolean, val tag: String)

// StaticSendParamsPartialPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `partial`. Consumers inject it via the `raisePartial` seam
// on the machine — they never name this class directly.
data class StaticSendParamsPartialPayload(val total: UInt, val part: UByte)


// --- State Machine (W3C SCXML) ---

class StaticSendParamsStateMachine(
) : StateMachineEngine<StaticSendParamsState, StaticSendParamsEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, the machine's own. */
    private var count: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `small` datamodel variable, the machine's own. */
    private var small: UByte = 250.toUByte()
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var label: String = "tally"
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    var total: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `ok` datamodel variable, published (`sce:direction="out"`). */
    var ok: Boolean = false
        private set
    /** W3C SCXML 5.2: the `tag` datamodel variable, published (`sce:direction="out"`). */
    var tag: String = ""
        private set
    /** W3C SCXML 5.2: the `partialTotal` datamodel variable, published (`sce:direction="out"`). */
    var partialTotal: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `partialPart` datamodel variable, the machine's own. */
    private var partialPart: UByte = 0.toUByte()
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var small: UByte? = null
        var label: String? = null
        var total: UInt? = null
        var ok: Boolean? = null
        var tag: String? = null
        var partialTotal: UInt? = null
        var partialPart: UByte? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { count = it }
        params.small?.let { small = it }
        params.label?.let { label = it }
        params.total?.let { total = it }
        params.ok?.let { ok = it }
        params.tag?.let { tag = it }
        params.partialTotal?.let { partialTotal = it }
        params.partialPart?.let { partialPart = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val total: UInt,
        val ok: Boolean,
        val tag: String,
        val partialTotal: UInt,
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
        val configuration: Set<StaticSendParamsState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        total = total,
        ok = ok,
        tag = tag,
        partialTotal = partialTotal,
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
    val savedShape: String = "7e5e95cee8de997396fe13304f2b1d4514e4e762405c5db762384b1df906a243"

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
            "count" to SavedValues.of(count),
            "small" to SavedValues.of(small),
            "label" to SavedValues.of(label),
            "total" to SavedValues.of(total),
            "ok" to SavedValues.of(ok),
            "tag" to SavedValues.of(tag),
            "partialTotal" to SavedValues.of(partialTotal),
            "partialPart" to SavedValues.of(partialPart),
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
        val saved1 = SavedValues.uint32(saved.variable("count"), "count")
        val saved2 = SavedValues.uint8(saved.variable("small"), "small")
        val saved3 = SavedValues.string(saved.variable("label"), "label", 16)
        val saved4 = SavedValues.uint32(saved.variable("total"), "total")
        val saved5 = SavedValues.bool(saved.variable("ok"), "ok")
        val saved6 = SavedValues.string(saved.variable("tag"), "tag", 16)
        val saved7 = SavedValues.uint32(saved.variable("partialTotal"), "partialTotal")
        val saved8 = SavedValues.uint8(saved.variable("partialPart"), "partialPart")
        val saved9 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        count = saved1
        small = saved2
        label = saved3
        total = saved4
        ok = saved5
        tag = saved6
        partialTotal = saved7
        partialPart = saved8
        refusals = saved9
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingEchoPayload: StaticSendParamsEchoPayload? = null
    private var pendingPartialPayload: StaticSendParamsPartialPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticSendParamsEvent, metadata: EventMetadata) {
        pendingEchoPayload = null
        pendingPartialPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticSendParamsEchoPayload -> pendingEchoPayload = tp
            is StaticSendParamsPartialPayload -> pendingPartialPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticSendParamsEvent.Echo) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingEchoPayload = StaticSendParamsEchoPayload(fields.uint32("total"), fields.boolean("ok"), fields.string("tag"))
                } else if (event == StaticSendParamsEvent.Partial) {
                    val fields = EventPayload.decode(metadata.data)
                    pendingPartialPayload = StaticSendParamsPartialPayload(fields.uint32("total"), fields.uint8("part"))
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `echo` — binds the event name and the payload field values in one call.
    fun raiseEcho(total: UInt, ok: Boolean, tag: String) {
        send(
            StaticSendParamsEvent.Echo,
            EventMetadata(
                type = "external",
                typedPayload = StaticSendParamsEchoPayload(total, ok, tag),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("total" to total, "ok" to ok, "tag" to tag))
            )
        )
    }
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `partial` — binds the event name and the payload field values in one call.
    fun raisePartial(total: UInt, part: UByte) {
        send(
            StaticSendParamsEvent.Partial,
            EventMetadata(
                type = "external",
                typedPayload = StaticSendParamsPartialPayload(total, part),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("total" to total, "part" to part))
            )
        )
    }


    override val initialState: StaticSendParamsState = StaticSendParamsState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticSendParamsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticSendParamsState, HistoryId>> =
            listOf(StateTarget(StaticSendParamsState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticSendParamsState, HistoryId>(
            StaticSendParamsState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticSendParamsState, HistoryId>(
            StaticSendParamsState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticSendParamsState, HistoryId>(
            StaticSendParamsState.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticSendParamsState, HistoryId>(
            StaticSendParamsState.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticSendParamsState, HistoryId>(
            StaticSendParamsState.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticSendParamsState? = when (stateId) {
        "idle" -> StaticSendParamsState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticSendParamsState): String = when (state) {
        is StaticSendParamsState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticSendParamsState): Int = when (state) {
        is StaticSendParamsState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticSendParamsEvent? = when (name) {
        "bad" -> StaticSendParamsEvent.Bad
        "echo" -> StaticSendParamsEvent.Echo
        "error.execution" -> StaticSendParamsEvent.Error.Execution
        "go" -> StaticSendParamsEvent.Go
        "partial" -> StaticSendParamsEvent.Partial
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticSendParamsEvent): String? = when (event) {
        is StaticSendParamsEvent.Bad -> "bad"
        is StaticSendParamsEvent.Echo -> "echo"
        is StaticSendParamsEvent.Error.Execution -> "error.execution"
        is StaticSendParamsEvent.Go -> "go"
        is StaticSendParamsEvent.Partial -> "partial"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticSendParamsState,
        event: StaticSendParamsEvent?
    ): EnabledTransition<StaticSendParamsState, HistoryId>? = when (state) {
        is StaticSendParamsState.Idle -> when {
            event is StaticSendParamsEvent.Go -> transitionIdleAt0
            event is StaticSendParamsEvent.Bad -> transitionIdleAt1
            event is StaticSendParamsEvent.Echo -> transitionIdleAt2
            event is StaticSendParamsEvent.Partial -> transitionIdleAt3
            event is StaticSendParamsEvent.Error.Execution -> transitionIdleAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_send_params.scxml:23 :: _machine
    override fun onEntry(state: StaticSendParamsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticSendParamsState.Idle -> {
                // SCE-MAP: static_send_params.scxml:38 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_send_params.scxml:23 :: _machine
    override fun onExit(state: StaticSendParamsState) {
        when (state) {
            is StaticSendParamsState.Idle -> {
                // SCE-MAP: static_send_params.scxml:38 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_send_params.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticSendParamsState, transitionIndex: Int) {
        when (source) {
        is StaticSendParamsState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_send_params.scxml:39 :: idle :: _transition_0


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "total", (com.sce.forge.runtime.SceChecked.mul(count, 2.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendParamsEvent.Error.Execution, "<send> <param name='total'> could not be read")
                paramFailed = true
            }

            putParam(sendPayload, "ok", count > 2.toUInt())

            putParam(sendPayload, "tag", label)

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendParamsEvent.Echo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            1 -> {
                // SCE-MAP: static_send_params.scxml:46 :: idle :: _transition_1


            if (run send@{
            var paramFailed = false
            val sendPayload = mutableMapOf<String, Any?>()
            try {
                putParam(sendPayload, "total", (com.sce.forge.runtime.SceChecked.mul(count, 2.toUInt())).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendParamsEvent.Error.Execution, "<send> <param name='total'> could not be read")
                paramFailed = true
            }

            try {
                putParam(sendPayload, "part", (com.sce.forge.runtime.SceChecked.add(small, small)).toLong())
            } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                raisePlatformError(StaticSendParamsEvent.Error.Execution, "<send> <param name='part'> could not be read")
                paramFailed = true
            }

            val sendData = buildJsonFromParams(sendPayload)
            // W3C SCXML 6.2: send to this session's external queue
            send(StaticSendParamsEvent.Partial, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            paramFailed
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
            }
            2 -> {
                // SCE-MAP: static_send_params.scxml:52 :: idle :: _transition_2
                if (pendingEchoPayload == null) {
                    return
                }

            total = pendingEchoPayload!!.total

            ok = pendingEchoPayload!!.ok

            if (try { tag = com.sce.forge.runtime.SceChecked.bounded(pendingEchoPayload!!.tag, 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendParamsEvent.Error.Execution, "<assign location='tag'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_send_params.scxml:57 :: idle :: _transition_3
                if (pendingPartialPayload == null) {
                    return
                }

            partialPart = pendingPartialPayload!!.part

            partialTotal = pendingPartialPayload!!.total
            }
            4 -> {
                // SCE-MAP: static_send_params.scxml:61 :: idle :: _transition_4

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticSendParamsEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
