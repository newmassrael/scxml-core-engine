// SCE-GENERATED — DO NOT EDIT
// source-hash: 8153420d7cf0af90d3fcf1a0988689d0a2d9a01bb414f5c89e80914a42bd4533

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_real.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_real.scxml:18 :: _machine

package com.sce.integration.static_real

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticRealState : State {
    data object High : StaticRealState
    data object Low : StaticRealState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticRealEvent : Event {
    data object Drift : StaticRealEvent
    data object Drop : StaticRealEvent
    sealed interface Error : StaticRealEvent {
        data object Execution : Error
    }
    data object Grow : StaticRealEvent
    data object Rise : StaticRealEvent
    data object Shrink : StaticRealEvent
    data object Sum : StaticRealEvent
}
// --- State Machine (W3C SCXML) ---

class StaticRealStateMachine(
) : StateMachineEngine<StaticRealState, StaticRealEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    var level: Double = 0.5
        private set
    /** W3C SCXML 5.2: the `rate` datamodel variable, the machine's own. */
    private var rate: Double = 1.5
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    var total: Double = 0.0
        private set
    /** W3C SCXML 5.2: the `tenth` datamodel variable, the machine's own. */
    private var tenth: Double = 0.1
    /** W3C SCXML 5.2: the `drift` datamodel variable, published (`sce:direction="out"`). */
    var drift: Double = 0.0
        private set
    /** W3C SCXML 5.2: the `samples` datamodel variable, published (`sce:direction="out"`). */
    var samples: List<Double> = emptyList()
        private set
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var level: Double? = null
        var rate: Double? = null
        var total: Double? = null
        var tenth: Double? = null
        var drift: Double? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.level?.let { level = it }
        params.rate?.let { rate = it }
        params.total?.let { total = it }
        params.tenth?.let { tenth = it }
        params.drift?.let { drift = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val level: Double,
        val total: Double,
        val drift: Double,
        val samples: List<Double>,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticRealState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        level = level,
        total = total,
        drift = drift,
        samples = samples,
        errors = errors,
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
    val savedShape: String = "f44ab1df16ee701bda26935e1acf131aaf19f648ffc8db1be27381c1a76a4133"

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
            "rate" to SavedValues.of(rate),
            "total" to SavedValues.of(total),
            "tenth" to SavedValues.of(tenth),
            "drift" to SavedValues.of(drift),
            "samples" to SavedValues.list(samples) { SavedValues.of(it) },
            "errors" to SavedValues.of(errors),
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
        val saved1 = SavedValues.float64(saved.variable("level"), "level")
        val saved2 = SavedValues.float64(saved.variable("rate"), "rate")
        val saved3 = SavedValues.float64(saved.variable("total"), "total")
        val saved4 = SavedValues.float64(saved.variable("tenth"), "tenth")
        val saved5 = SavedValues.float64(saved.variable("drift"), "drift")
        val saved6 = SavedValues.list(saved.variable("samples"), "samples", 3) { e, w -> SavedValues.float64(e, w) }
        val saved7 = SavedValues.uint32(saved.variable("errors"), "errors")
        level = saved1
        rate = saved2
        total = saved3
        tenth = saved4
        drift = saved5
        samples = saved6
        errors = saved7
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticRealState = StaticRealState.Low

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
    override val documentInitialTargets: List<EntryTarget<StaticRealState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticRealState, HistoryId>> =
            listOf(StateTarget(StaticRealState.Low))

        // W3C SCXML 3.13: high's transition 0, as the microstep reads it.
        val transitionHighAt0 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.High,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: high's transition 1, as the microstep reads it.
        val transitionHighAt1 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.High,
            listOf(StateTarget(StaticRealState.Low)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: low's transition 0, as the microstep reads it.
        val transitionLowAt0 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: low's transition 1, as the microstep reads it.
        val transitionLowAt1 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: low's transition 2, as the microstep reads it.
        val transitionLowAt2 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: low's transition 3, as the microstep reads it.
        val transitionLowAt3 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: low's transition 4, as the microstep reads it.
        val transitionLowAt4 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            listOf(StateTarget(StaticRealState.High)),
            4,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: low's transition 5, as the microstep reads it.
        val transitionLowAt5 = EnabledTransition<StaticRealState, HistoryId>(
            StaticRealState.Low,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticRealState? = when (stateId) {
        "high" -> StaticRealState.High
        "low" -> StaticRealState.Low
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticRealState): String = when (state) {
        is StaticRealState.High -> "high"
        is StaticRealState.Low -> "low"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticRealState): Int = when (state) {
        is StaticRealState.High -> 1
        is StaticRealState.Low -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticRealEvent? = when (name) {
        "drift" -> StaticRealEvent.Drift
        "drop" -> StaticRealEvent.Drop
        "error.execution" -> StaticRealEvent.Error.Execution
        "grow" -> StaticRealEvent.Grow
        "rise" -> StaticRealEvent.Rise
        "shrink" -> StaticRealEvent.Shrink
        "sum" -> StaticRealEvent.Sum
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticRealEvent): String? = when (event) {
        is StaticRealEvent.Drift -> "drift"
        is StaticRealEvent.Drop -> "drop"
        is StaticRealEvent.Error.Execution -> "error.execution"
        is StaticRealEvent.Grow -> "grow"
        is StaticRealEvent.Rise -> "rise"
        is StaticRealEvent.Shrink -> "shrink"
        is StaticRealEvent.Sum -> "sum"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticRealState,
        event: StaticRealEvent?
    ): EnabledTransition<StaticRealState, HistoryId>? = when (state) {
        is StaticRealState.High -> when {
            event is StaticRealEvent.Shrink -> transitionHighAt0
            event is StaticRealEvent.Drop && level < 1.0 -> transitionHighAt1
            else -> null
        }
        is StaticRealState.Low -> when {
            event is StaticRealEvent.Grow -> transitionLowAt0
            event is StaticRealEvent.Shrink -> transitionLowAt1
            event is StaticRealEvent.Sum -> transitionLowAt2
            event is StaticRealEvent.Drift -> transitionLowAt3
            event is StaticRealEvent.Rise && level > 7.0 -> transitionLowAt4
            event is StaticRealEvent.Error.Execution -> transitionLowAt5
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun onEntry(state: StaticRealState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRealState.High -> {
                // SCE-MAP: static_real.scxml:57 :: high :: _state_body
            }
            is StaticRealState.Low -> {
                // SCE-MAP: static_real.scxml:29 :: low :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun onExit(state: StaticRealState) {
        when (state) {
            is StaticRealState.High -> {
                // SCE-MAP: static_real.scxml:57 :: high :: _state_body
            }
            is StaticRealState.Low -> {
                // SCE-MAP: static_real.scxml:29 :: low :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticRealState, transitionIndex: Int) {
        when (source) {
        is StaticRealState.High -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_real.scxml:58 :: high :: _transition_0

            level = level / 4.0
            }
            else -> {}
        }
        is StaticRealState.Low -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_real.scxml:32 :: low :: _transition_0

            level = level * 2.0 + rate

            if (if (samples.size < 3) { samples = samples + (level); false } else { raisePlatformError(StaticRealEvent.Error.Execution, "<sce:append target='samples'>: the list already holds its capacity of 3"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_real.scxml:37 :: low :: _transition_1

            level = level / 4.0
            }
            2 -> {
                // SCE-MAP: static_real.scxml:41 :: low :: _transition_2

            total = 0.0


            for (v in samples) {

            total = total + v
            }
            }
            3 -> {
                // SCE-MAP: static_real.scxml:48 :: low :: _transition_3

            drift = tenth + 0.2
            }
            5 -> {
                // SCE-MAP: static_real.scxml:53 :: low :: _transition_5

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRealEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
