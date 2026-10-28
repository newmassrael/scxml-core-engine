// SCE-GENERATED — DO NOT EDIT
// source-hash: d11fb6db69c27fed25f47a2f22a5bf7fb78da524d08f0c299e9cc553d32a6375

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_counter.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_counter.scxml:13 :: _machine

package com.sce.integration.static_counter

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticCounterState : State {
    data object Counting : StaticCounterState
    data object Done : StaticCounterState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticCounterEvent : Event {
    data object Go : StaticCounterEvent
    data object Tick : StaticCounterEvent
}
// --- State Machine (W3C SCXML) ---

class StaticCounterStateMachine(
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticCounterState, StaticCounterEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    private var vCount: UInt = 0.toUInt()

    /** What the published `count` holds. */
    val count: UInt get() = vCount
    /** W3C SCXML 5.2: the `ready` datamodel variable, published (`sce:direction="out"`). */
    private var vReady: Boolean = false

    /** What the published `ready` holds. */
    val ready: Boolean get() = vReady
    /** W3C SCXML 5.2: the `step` datamodel variable, the machine's own. */
    private var vStep: UInt = 1.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var count: UInt? = null
        var ready: Boolean? = null
        var step: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.count?.let { vCount = it }
        params.ready?.let { vReady = it }
        params.step?.let { vStep = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val count: UInt,
        val ready: Boolean,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticCounterState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        count = vCount,
        ready = vReady,
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
        first = "b1e55a5fb4149b2224d189f552488790e62394135415d7e93c3423ce7d4f4da6",
        revised = "9e81e07793879a193c502fb56f49ebf7e9ef54bf8e9258775a47df18e340aac0",
        format2First = "b1e55a5fb4149b2224d189f552488790e62394135415d7e93c3423ce7d4f4da6",
        format2Revised = "9e81e07793879a193c502fb56f49ebf7e9ef54bf8e9258775a47df18e340aac0",
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
            "count" to SavedValues.of(vCount),
            "ready" to SavedValues.of(vReady),
            "step" to SavedValues.of(vStep),
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
        val saved1 = SavedValues.uint32(savedGrown.variable("count"), "count")
        val saved2 = SavedValues.bool(savedGrown.variable("ready"), "ready")
        val saved3 = SavedValues.uint32(savedGrown.variable("step"), "step")
        vCount = saved1
        vReady = saved2
        vStep = saved3
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
        SavedValues.uint32(savedGrown.variable("count"), "count")
        SavedValues.bool(savedGrown.variable("ready"), "ready")
        SavedValues.uint32(savedGrown.variable("step"), "step")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

    override val initialState: StaticCounterState = StaticCounterState.Counting

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
    override fun isFinalState(state: StaticCounterState): Boolean = when (state) {
        is StaticCounterState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticCounterState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticCounterState, HistoryId>> =
            listOf(StateTarget(StaticCounterState.Counting))

        // W3C SCXML 3.13: counting's transition 0, as the microstep reads it.
        val transitionCountingAt0 = EnabledTransition<StaticCounterState, HistoryId>(
            StaticCounterState.Counting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: counting's transition 1, as the microstep reads it.
        val transitionCountingAt1 = EnabledTransition<StaticCounterState, HistoryId>(
            StaticCounterState.Counting,
            listOf(StateTarget(StaticCounterState.Done)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticCounterState? = when (stateId) {
        "counting" -> StaticCounterState.Counting
        "done" -> StaticCounterState.Done
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticCounterState): String = when (state) {
        is StaticCounterState.Counting -> "counting"
        is StaticCounterState.Done -> "done"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticCounterState): Int = when (state) {
        is StaticCounterState.Counting -> 0
        is StaticCounterState.Done -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticCounterEvent? = when (name) {
        "go" -> StaticCounterEvent.Go
        "tick" -> StaticCounterEvent.Tick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticCounterEvent): String? = when (event) {
        is StaticCounterEvent.Go -> "go"
        is StaticCounterEvent.Tick -> "tick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticCounterState,
        event: StaticCounterEvent?
    ): EnabledTransition<StaticCounterState, HistoryId>? = when (state) {
        is StaticCounterState.Counting -> when {
            event is StaticCounterEvent.Tick && vCount < 10.toUInt() && isStateActive("counting") -> transitionCountingAt0
            event is StaticCounterEvent.Go && vReady -> transitionCountingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_counter.scxml:13 :: _machine
    override fun onEntry(state: StaticCounterState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticCounterState.Counting -> {
                // SCE-MAP: static_counter.scxml:20 :: counting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            println("count: " + vCount)
                }
            }
            is StaticCounterState.Done -> {
                // SCE-MAP: static_counter.scxml:32 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_counter.scxml:13 :: _machine
    override fun onExit(state: StaticCounterState) {
        when (state) {
            is StaticCounterState.Counting -> {
                // SCE-MAP: static_counter.scxml:20 :: counting :: _state_body
            }
            is StaticCounterState.Done -> {
                // SCE-MAP: static_counter.scxml:32 :: done :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_counter.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticCounterState, transitionIndex: Int) {
        when (source) {
        is StaticCounterState.Counting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_counter.scxml:22 :: counting :: _transition_0

            if (try { vCount = com.sce.forge.runtime.SceChecked.add(vCount, vStep); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }


            if (vCount == 5.toUInt()) {

            vReady = true
            } else if (vCount > 7.toUInt()) {

            vReady = false
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
