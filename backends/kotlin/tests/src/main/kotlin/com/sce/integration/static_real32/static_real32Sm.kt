// SCE-GENERATED — DO NOT EDIT
// source-hash: 64fb223807f4c558ee24bbcd3b67b8ca6498504cd273de737c75cdc2b6d599ee

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_real32.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_real32.scxml:30 :: _machine

package com.sce.integration.static_real32

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticReal32State : State {
    data object Idle : StaticReal32State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticReal32Event : Event {
    data object Bump : StaticReal32Event
    data object Drift : StaticReal32Event
    data object Halve : StaticReal32Event
    data object Keep : StaticReal32Event
    data object Keepwide : StaticReal32Event
    data object Mix : StaticReal32Event
    data object Sum : StaticReal32Event
    data object Widen : StaticReal32Event
}
// --- State Machine (W3C SCXML) ---

class StaticReal32StateMachine(
) : StateMachineEngine<StaticReal32State, StaticReal32Event>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    var level: Float = 16777216.0f
        private set
    /** W3C SCXML 5.2: the `tenth` datamodel variable, the machine's own. */
    private var tenth: Float = 0.1f
    /** W3C SCXML 5.2: the `drift` datamodel variable, published (`sce:direction="out"`). */
    var drift: Float = 0.0f
        private set
    /** W3C SCXML 5.2: the `wide` datamodel variable, published (`sce:direction="out"`). */
    var wide: Double = 0.0
        private set
    /** W3C SCXML 5.2: the `samples` datamodel variable, published (`sce:direction="out"`). */
    var samples: List<Float> = emptyList()
        private set
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    var total: Float = 0.0f
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var level: Float? = null
        var tenth: Float? = null
        var drift: Float? = null
        var wide: Double? = null
        var total: Float? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.level?.let { level = it }
        params.tenth?.let { tenth = it }
        params.drift?.let { drift = it }
        params.wide?.let { wide = it }
        params.total?.let { total = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val level: Float,
        val drift: Float,
        val wide: Double,
        val samples: List<Float>,
        val total: Float,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticReal32State>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        level = level,
        drift = drift,
        wide = wide,
        samples = samples,
        total = total,
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
    val savedShape: String = "ce103af598245407b75ff596e5414dcff4a8677f8043802c3fdd2595e81c93cf"

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
            "level" to SavedValues.of(level),
            "tenth" to SavedValues.of(tenth),
            "drift" to SavedValues.of(drift),
            "wide" to SavedValues.of(wide),
            "samples" to SavedValues.list(samples) { SavedValues.of(it) },
            "total" to SavedValues.of(total),
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
        val saved1 = SavedValues.float32(saved.variable("level"), "level")
        val saved2 = SavedValues.float32(saved.variable("tenth"), "tenth")
        val saved3 = SavedValues.float32(saved.variable("drift"), "drift")
        val saved4 = SavedValues.float64(saved.variable("wide"), "wide")
        val saved5 = SavedValues.list(saved.variable("samples"), "samples", 3) { e, w -> SavedValues.float32(e, w) }
        val saved6 = SavedValues.float32(saved.variable("total"), "total")
        level = saved1
        tenth = saved2
        drift = saved3
        wide = saved4
        samples = saved5
        total = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticReal32State = StaticReal32State.Idle

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
    override val documentInitialTargets: List<EntryTarget<StaticReal32State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticReal32State, HistoryId>> =
            listOf(StateTarget(StaticReal32State.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 3, as the microstep reads it.
        val transitionIdleAt3 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 4, as the microstep reads it.
        val transitionIdleAt4 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 5, as the microstep reads it.
        val transitionIdleAt5 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 6, as the microstep reads it.
        val transitionIdleAt6 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: idle's transition 7, as the microstep reads it.
        val transitionIdleAt7 = EnabledTransition<StaticReal32State, HistoryId>(
            StaticReal32State.Idle,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticReal32State? = when (stateId) {
        "idle" -> StaticReal32State.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticReal32State): String = when (state) {
        is StaticReal32State.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticReal32State): Int = when (state) {
        is StaticReal32State.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticReal32Event? = when (name) {
        "bump" -> StaticReal32Event.Bump
        "drift" -> StaticReal32Event.Drift
        "halve" -> StaticReal32Event.Halve
        "keep" -> StaticReal32Event.Keep
        "keepwide" -> StaticReal32Event.Keepwide
        "mix" -> StaticReal32Event.Mix
        "sum" -> StaticReal32Event.Sum
        "widen" -> StaticReal32Event.Widen
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticReal32Event): String? = when (event) {
        is StaticReal32Event.Bump -> "bump"
        is StaticReal32Event.Drift -> "drift"
        is StaticReal32Event.Halve -> "halve"
        is StaticReal32Event.Keep -> "keep"
        is StaticReal32Event.Keepwide -> "keepwide"
        is StaticReal32Event.Mix -> "mix"
        is StaticReal32Event.Sum -> "sum"
        is StaticReal32Event.Widen -> "widen"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticReal32State,
        event: StaticReal32Event?
    ): EnabledTransition<StaticReal32State, HistoryId>? = when (state) {
        is StaticReal32State.Idle -> when {
            event is StaticReal32Event.Bump -> transitionIdleAt0
            event is StaticReal32Event.Halve -> transitionIdleAt1
            event is StaticReal32Event.Drift -> transitionIdleAt2
            event is StaticReal32Event.Widen -> transitionIdleAt3
            event is StaticReal32Event.Mix -> transitionIdleAt4
            event is StaticReal32Event.Keep -> transitionIdleAt5
            event is StaticReal32Event.Keepwide -> transitionIdleAt6
            event is StaticReal32Event.Sum -> transitionIdleAt7
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_real32.scxml:30 :: _machine
    override fun onEntry(state: StaticReal32State, isDefaultEntry: Boolean) {
        when (state) {
            is StaticReal32State.Idle -> {
                // SCE-MAP: static_real32.scxml:40 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_real32.scxml:30 :: _machine
    override fun onExit(state: StaticReal32State) {
        when (state) {
            is StaticReal32State.Idle -> {
                // SCE-MAP: static_real32.scxml:40 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_real32.scxml:30 :: _machine
    override fun executeTransitionContent(source: StaticReal32State, transitionIndex: Int) {
        when (source) {
        is StaticReal32State.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_real32.scxml:42 :: idle :: _transition_0

            level = level + 1.0f
            }
            1 -> {
                // SCE-MAP: static_real32.scxml:46 :: idle :: _transition_1

            level = level / 2.0f
            }
            2 -> {
                // SCE-MAP: static_real32.scxml:50 :: idle :: _transition_2

            drift = tenth + 0.2f
            }
            3 -> {
                // SCE-MAP: static_real32.scxml:54 :: idle :: _transition_3

            wide = tenth.toDouble()
            }
            4 -> {
                // SCE-MAP: static_real32.scxml:61 :: idle :: _transition_4

            wide = tenth.toDouble() + 0.2
            }
            5 -> {
                // SCE-MAP: static_real32.scxml:65 :: idle :: _transition_5

            if (if (samples.size < 3) { samples = samples + (tenth + 0.2f); false } else { true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_real32.scxml:70 :: idle :: _transition_6

            if (if (samples.size < 3) { samples = samples + (wide.toFloat()); false } else { true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_real32.scxml:74 :: idle :: _transition_7

            total = 0.0f


            for (v in samples) {

            total = total + v
            }
            }
            else -> {}
        }
        }
    }
}
