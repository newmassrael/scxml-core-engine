// SCE-GENERATED — DO NOT EDIT
// source-hash: 09d8fec62ddcc806d17c4e1da241b802dbe4f5317b3bd415a705b2ad99d82f3e

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_optional_widened.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_optional_widened.scxml:19 :: _machine

package com.sce.integration.static_optional_widened

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticOptionalWidenedState : State {
    data object Reading : StaticOptionalWidenedState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticOptionalWidenedEvent : Event {
    sealed interface Due : StaticOptionalWidenedEvent {
        data object Widened : Due
    }
    sealed interface Error : StaticOptionalWidenedEvent {
        data object Execution : Error
    }
}
// ── NL→IR Item C1 Path A: typed `_event.data` payload classes ─────────
// NL→IR Item C1 Path A (EventSchema MCU native lowering): typed
// `_event.data` payload classes for the EventSchema-imported events whose
// transition guards lowered to a native Kotlin comparison (no script engine).
// The Kotlin twin of the Rust `StaticOptionalWidenedPayload` enum / Go per-event payload
// structs: one data class per guarded event, carried through the queue in the
// type-erased `EventMetadata.typedPayload` and lifted into a nullable field.
// StaticOptionalWidenedDueWidenedPayload is the NL→IR Item C1 Path A typed `_event.data`
// payload for `due.widened`. Consumers inject it via the `raiseDueWidened` seam
// on the machine — they never name this class directly.
data class StaticOptionalWidenedDueWidenedPayload(val id: UInt, val due: Long?, val remind: Long?)


// --- State Machine (W3C SCXML) ---

class StaticOptionalWidenedStateMachine(
) : StateMachineEngine<StaticOptionalWidenedState, StaticOptionalWidenedEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `entry` datamodel variable, published (`sce:direction="out"`). */
    private var vEntry: UInt = 0.toUInt()

    /** What the published `entry` holds. */
    val entry: UInt get() = vEntry
    /** W3C SCXML 5.2: the `due` datamodel variable, published (`sce:direction="out"`). */
    private var vDue: Long? = null

    /** What the published `due` holds. */
    val due: Long? get() = vDue
    /** W3C SCXML 5.2: the `remind` datamodel variable, published (`sce:direction="out"`). */
    private var vRemind: Long? = null

    /** What the published `remind` holds. */
    val remind: Long? get() = vRemind
    /** W3C SCXML 5.2: the `taken` datamodel variable, published (`sce:direction="out"`). */
    private var vTaken: UInt = 0.toUInt()

    /** What the published `taken` holds. */
    val taken: UInt get() = vTaken
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
        var entry: UInt? = null
        var taken: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.entry?.let { vEntry = it }
        params.taken?.let { vTaken = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val entry: UInt,
        val due: Long?,
        val remind: Long?,
        val taken: UInt,
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
        val configuration: Set<StaticOptionalWidenedState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        entry = vEntry,
        due = vDue,
        remind = vRemind,
        taken = vTaken,
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
     * What a saved state of this document is bound to: a state saved from a
     * document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not, and one
     * saved at an earlier revision of a record schema is read with each field a
     * later revision added at its default.
     */
    val savedShape: SavedShape = SavedShape(
        first = "38e864a9ddfa2b05f7fccc665febabfee57f778c7e4b5c2ab35f7d7c92d210c7",
        revised = "3ccffe0f17d5d5f88c7cc7659f03f10cab579a3feefd728f043b8703b520e696",
        format2First = "38e864a9ddfa2b05f7fccc665febabfee57f778c7e4b5c2ab35f7d7c92d210c7",
        format2Revised = "3ccffe0f17d5d5f88c7cc7659f03f10cab579a3feefd728f043b8703b520e696",
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
            "entry" to SavedValues.of(vEntry),
            "due" to SavedValues.optional(vDue) { SavedValues.of(it) },
            "remind" to SavedValues.optional(vRemind) { SavedValues.of(it) },
            "taken" to SavedValues.of(vTaken),
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
        // The state as one of this machine: each record saved at an earlier
        // revision of its schema holds the defaults of the fields added since.
        val savedGrown = beginRestore(saved, savedShape)
        val saved1 = SavedValues.uint32(savedGrown.variable("entry"), "entry")
        val saved2 = SavedValues.optional(savedGrown.variable("due"), "due") { e, w -> SavedValues.int64(e, w) }
        val saved3 = SavedValues.optional(savedGrown.variable("remind"), "remind") { e, w -> SavedValues.int64(e, w) }
        val saved4 = SavedValues.uint32(savedGrown.variable("taken"), "taken")
        val saved5 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vEntry = saved1
        vDue = saved2
        vRemind = saved3
        vTaken = saved4
        vRefusals = saved5
        enterSaved(savedGrown, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // NL→IR Item C1 Path A: the current event's typed `_event.data` payload(s),
    // lifted from the dequeued event by populateTypedPayload and read by the
    // native transition guards. `null` between events / for untyped events.
    private var pendingDueWidenedPayload: StaticOptionalWidenedDueWidenedPayload? = null

    // NL→IR Item C1 Path A: bind the dequeued event's typed `_event.data` view
    // — from the type-erased carrier the inject seam fills, and otherwise by
    // lifting the fields out of `metadata.data`, which every other producer
    // fills. An event with neither resets the fields to null, so every typed
    // guard fails. A payload that cannot be read as this event's schema throws
    // EventPayload.Refusal, which the engine reports as error.execution. Twin
    // of the Go policy's PopulateEventMetadata + LiftTypedPayload / the C11 pop
    // loop's `sm->pending_payload = evt.payload`.
    override fun populateTypedPayload(event: StaticOptionalWidenedEvent, metadata: EventMetadata) {
        pendingDueWidenedPayload = null
        when (val tp = metadata.typedPayload) {
            is StaticOptionalWidenedDueWidenedPayload -> pendingDueWidenedPayload = tp
            else -> {
                // No typed carrier, so the producer was not the inject seam: read the
                // fields out of `data`, which every other producer fills.
                if (event == StaticOptionalWidenedEvent.Due.Widened) {
                    val fields = EventPayload.decode(metadata.data, "{\"remind\":null}")
                    pendingDueWidenedPayload = StaticOptionalWidenedDueWidenedPayload(fields.uint32("id"), fields.optional("due") { _ -> fields.int64("due") }, fields.optional("remind") { _ -> fields.int64("remind") })
                }
            }
        }
    }

    // NL→IR Item C1 Path A: per-event typed `_event.data` inject seams.
    // NL→IR Item C1 Path A typed `_event.data` inject seam for
    // `due.widened` — binds the event name and the payload field values in one call.
    fun raiseDueWidened(id: UInt, due: Long?, remind: Long?) {
        send(
            StaticOptionalWidenedEvent.Due.Widened,
            EventMetadata(
                type = "external",
                typedPayload = StaticOptionalWidenedDueWidenedPayload(id, due, remind),
                // Both carriers are filled: the typed one a native guard reads, and
                // `data`, which is what the script engine binds `_event.data` from.
                // Filling only the first left an `<assign expr="_event.data.x">` on
                // this event reading nothing, on every backend alike.
                data = EventPayload.encode(mapOf("id" to id, "due" to due, "remind" to remind))
            )
        )
    }


    override val initialState: StaticOptionalWidenedState = StaticOptionalWidenedState.Reading

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
    override val documentInitialTargets: List<EntryTarget<StaticOptionalWidenedState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticOptionalWidenedState, HistoryId>> =
            listOf(StateTarget(StaticOptionalWidenedState.Reading))

        // W3C SCXML 3.13: reading's transition 0, as the microstep reads it.
        val transitionReadingAt0 = EnabledTransition<StaticOptionalWidenedState, HistoryId>(
            StaticOptionalWidenedState.Reading,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: reading's transition 1, as the microstep reads it.
        val transitionReadingAt1 = EnabledTransition<StaticOptionalWidenedState, HistoryId>(
            StaticOptionalWidenedState.Reading,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticOptionalWidenedState? = when (stateId) {
        "reading" -> StaticOptionalWidenedState.Reading
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticOptionalWidenedState): String = when (state) {
        is StaticOptionalWidenedState.Reading -> "reading"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticOptionalWidenedState): Int = when (state) {
        is StaticOptionalWidenedState.Reading -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticOptionalWidenedEvent? = when (name) {
        "due.widened" -> StaticOptionalWidenedEvent.Due.Widened
        "error.execution" -> StaticOptionalWidenedEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticOptionalWidenedEvent): String? = when (event) {
        is StaticOptionalWidenedEvent.Due.Widened -> "due.widened"
        is StaticOptionalWidenedEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticOptionalWidenedState,
        event: StaticOptionalWidenedEvent?
    ): EnabledTransition<StaticOptionalWidenedState, HistoryId>? = when (state) {
        is StaticOptionalWidenedState.Reading -> when {
            event is StaticOptionalWidenedEvent.Due.Widened -> transitionReadingAt0
            event is StaticOptionalWidenedEvent.Error.Execution -> transitionReadingAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_optional_widened.scxml:19 :: _machine
    override fun onEntry(state: StaticOptionalWidenedState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticOptionalWidenedState.Reading -> {
                // SCE-MAP: static_optional_widened.scxml:29 :: reading :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_optional_widened.scxml:19 :: _machine
    override fun onExit(state: StaticOptionalWidenedState) {
        when (state) {
            is StaticOptionalWidenedState.Reading -> {
                // SCE-MAP: static_optional_widened.scxml:29 :: reading :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_optional_widened.scxml:19 :: _machine
    override fun executeTransitionContent(source: StaticOptionalWidenedState, transitionIndex: Int) {
        when (source) {
        is StaticOptionalWidenedState.Reading -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_optional_widened.scxml:30 :: reading :: _transition_0
                if (pendingDueWidenedPayload == null) {
                    return
                }

            vDue = pendingDueWidenedPayload!!.due

            vRemind = pendingDueWidenedPayload!!.remind

            vEntry = pendingDueWidenedPayload!!.id

            if (try { vTaken = com.sce.forge.runtime.SceChecked.add(vTaken, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalWidenedEvent.Error.Execution, "<assign location='taken'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_optional_widened.scxml:36 :: reading :: _transition_1

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOptionalWidenedEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
