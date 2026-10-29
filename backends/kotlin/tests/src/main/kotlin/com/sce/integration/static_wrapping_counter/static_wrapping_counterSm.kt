// SCE-GENERATED — DO NOT EDIT
// source-hash: 1178ba59796ca1f7c46f8569fd4bfda85c2b97db65f51728f3615d990d97dd1c

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_wrapping_counter.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_wrapping_counter.scxml:17 :: _machine

package com.sce.integration.static_wrapping_counter

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticWrappingCounterState : State {
    data object Counting : StaticWrappingCounterState
    data object Near : StaticWrappingCounterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticWrappingCounterEvent : Event {
    data object Burn : StaticWrappingCounterEvent
    sealed interface Error : StaticWrappingCounterEvent {
        data object Execution : Error
    }
    data object Probe : StaticWrappingCounterEvent
    data object Tick : StaticWrappingCounterEvent
}
// --- State Machine (W3C SCXML) ---

class StaticWrappingCounterStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticWrappingCounterState, StaticWrappingCounterEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `ticks` datamodel variable, published (`sce:direction="out"`). */
    private var vTicks: UByte = 254.toUByte()

    /** What the published `ticks` holds. */
    val ticks: UByte get() = vTicks
    /** W3C SCXML 5.2: the `fuel` datamodel variable, published (`sce:direction="out"`). */
    private var vFuel: UByte = 2.toUByte()

    /** What the published `fuel` holds. */
    val fuel: UByte get() = vFuel
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
        var ticks: UByte? = null
        var fuel: UByte? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.ticks?.let { vTicks = it }
        params.fuel?.let { vFuel = it }
        params.refusals?.let { vRefusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val ticks: UByte,
        val fuel: UByte,
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
        val configuration: Set<StaticWrappingCounterState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        ticks = vTicks,
        fuel = vFuel,
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
        first = "c8950e4651c934d54e6a05f405686af7c0c7eea6eb2507b14af62473a4094251",
        revised = "adc0903083cc91825d88b7093752ede4ac5ce845a6e44cbd42b46c601014eed6",
        format2First = "c8950e4651c934d54e6a05f405686af7c0c7eea6eb2507b14af62473a4094251",
        format2Revised = "adc0903083cc91825d88b7093752ede4ac5ce845a6e44cbd42b46c601014eed6",
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
            "ticks" to SavedValues.of(vTicks),
            "fuel" to SavedValues.of(vFuel),
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
        val saved1 = SavedValues.uint8(savedGrown.variable("ticks"), "ticks")
        val saved2 = SavedValues.uint8(savedGrown.variable("fuel"), "fuel")
        val saved3 = SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
        vTicks = saved1
        vFuel = saved2
        vRefusals = saved3
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
        SavedValues.uint8(savedGrown.variable("ticks"), "ticks")
        SavedValues.uint8(savedGrown.variable("fuel"), "fuel")
        SavedValues.uint32(savedGrown.variable("refusals"), "refusals")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticWrappingCounterState = StaticWrappingCounterState.Counting

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticWrappingCounterState): Boolean = when (state) {
        is StaticWrappingCounterState.Near -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticWrappingCounterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticWrappingCounterState, HistoryId>> =
            listOf(StateTarget(StaticWrappingCounterState.Counting))

        // W3C SCXML 3.13: counting's transition 0, as the microstep reads it.
        val transitionCountingAt0 = EnabledTransition<StaticWrappingCounterState, HistoryId>(
            StaticWrappingCounterState.Counting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 1, as the microstep reads it.
        val transitionCountingAt1 = EnabledTransition<StaticWrappingCounterState, HistoryId>(
            StaticWrappingCounterState.Counting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 2, as the microstep reads it.
        val transitionCountingAt2 = EnabledTransition<StaticWrappingCounterState, HistoryId>(
            StaticWrappingCounterState.Counting,
            listOf(StateTarget(StaticWrappingCounterState.Near)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: counting's transition 3, as the microstep reads it.
        val transitionCountingAt3 = EnabledTransition<StaticWrappingCounterState, HistoryId>(
            StaticWrappingCounterState.Counting,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticWrappingCounterState? = when (stateId) {
        "counting" -> StaticWrappingCounterState.Counting
        "near" -> StaticWrappingCounterState.Near
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticWrappingCounterState): String = when (state) {
        is StaticWrappingCounterState.Counting -> "counting"
        is StaticWrappingCounterState.Near -> "near"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticWrappingCounterState): Int = when (state) {
        is StaticWrappingCounterState.Counting -> 0
        is StaticWrappingCounterState.Near -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticWrappingCounterEvent? = when (name) {
        "burn" -> StaticWrappingCounterEvent.Burn
        "error.execution" -> StaticWrappingCounterEvent.Error.Execution
        "probe" -> StaticWrappingCounterEvent.Probe
        "tick" -> StaticWrappingCounterEvent.Tick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticWrappingCounterEvent): String? = when (event) {
        is StaticWrappingCounterEvent.Burn -> "burn"
        is StaticWrappingCounterEvent.Error.Execution -> "error.execution"
        is StaticWrappingCounterEvent.Probe -> "probe"
        is StaticWrappingCounterEvent.Tick -> "tick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticWrappingCounterState,
        event: StaticWrappingCounterEvent?
    ): EnabledTransition<StaticWrappingCounterState, HistoryId>? = when (state) {
        is StaticWrappingCounterState.Counting -> when {
            event is StaticWrappingCounterEvent.Tick -> transitionCountingAt0
            event is StaticWrappingCounterEvent.Burn -> transitionCountingAt1
            event is StaticWrappingCounterEvent.Probe && com.sce.forge.runtime.SceWrapping.sub(vTicks, 250.toUByte()) < 10.toUByte() -> transitionCountingAt2
            event is StaticWrappingCounterEvent.Error.Execution -> transitionCountingAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_wrapping_counter.scxml:17 :: _machine
    override fun onEntry(state: StaticWrappingCounterState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticWrappingCounterState.Counting -> {
                // SCE-MAP: static_wrapping_counter.scxml:24 :: counting :: _state_body
            }
            is StaticWrappingCounterState.Near -> {
                // SCE-MAP: static_wrapping_counter.scxml:36 :: near :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_wrapping_counter.scxml:17 :: _machine
    override fun onExit(state: StaticWrappingCounterState) {
        when (state) {
            is StaticWrappingCounterState.Counting -> {
                // SCE-MAP: static_wrapping_counter.scxml:24 :: counting :: _state_body
            }
            is StaticWrappingCounterState.Near -> {
                // SCE-MAP: static_wrapping_counter.scxml:36 :: near :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_wrapping_counter.scxml:17 :: _machine
    override fun executeTransitionContent(source: StaticWrappingCounterState, transitionIndex: Int) {
        when (source) {
        is StaticWrappingCounterState.Counting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_wrapping_counter.scxml:25 :: counting :: _transition_0

            vTicks = com.sce.forge.runtime.SceWrapping.add(vTicks, 1.toUByte())
            }
            1 -> {
                // SCE-MAP: static_wrapping_counter.scxml:28 :: counting :: _transition_1

            vFuel = com.sce.forge.runtime.SceSaturating.sub(vFuel, 1.toUByte())
            }
            3 -> {
                // SCE-MAP: static_wrapping_counter.scxml:32 :: counting :: _transition_3

            if (try { vRefusals = com.sce.forge.runtime.SceChecked.add(vRefusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticWrappingCounterEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
