// SCE-GENERATED — DO NOT EDIT
// source-hash: d321654623235a43e67203edd8f804817a9d25b08581dd212fe8c5386f0fb495

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_history.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_history.scxml:33 :: _machine

package com.sce.integration.static_history

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHistoryState : State {
    data object Burst : StaticHistoryState
    data object Crew : StaticHistoryState
    data object Cruise : StaticHistoryState
    data object Fast : StaticHistoryState
    data object L1 : StaticHistoryState
    data object L2 : StaticHistoryState
    data object Left : StaticHistoryState
    data object Paused : StaticHistoryState
    data object R1 : StaticHistoryState
    data object R2 : StaticHistoryState
    data object Right : StaticHistoryState
    data object Running : StaticHistoryState
    data object Slow : StaticHistoryState
    data object Working : StaticHistoryState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHistoryEvent : Event {
    data object Boost : StaticHistoryEvent
    data object Break : StaticHistoryEvent
    data object Faster : StaticHistoryEvent
    data object LeftNext : StaticHistoryEvent
    data object Pause : StaticHistoryEvent
    data object ResumeCrew : StaticHistoryEvent
    data object ResumeDeep : StaticHistoryEvent
    data object ResumeLast : StaticHistoryEvent
    data object RightNext : StaticHistoryEvent
    data object Work : StaticHistoryEvent
}
// --- State Machine (W3C SCXML) ---

class StaticHistoryStateMachine(
) : StateMachineEngine<StaticHistoryState, StaticHistoryEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `resumed` datamodel variable, published (`sce:direction="out"`). */
    var resumed: UByte = 0.toUByte()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var resumed: UByte? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.resumed?.let { resumed = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val resumed: UByte,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHistoryState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        resumed = resumed,
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
    val savedShape: String = "a694bcc2c6fbd3a4168fa70c9081522dc8979d50a9c363686a4968d6ea77308d"

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
            "resumed" to SavedValues.of(resumed),
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
        val saved1 = SavedValues.uint8(saved.variable("resumed"), "resumed")
        resumed = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticHistoryState = StaticHistoryState.Slow

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

    // W3C SCXML 3.3: State hierarchy parent mapping
    override fun parentOf(state: StaticHistoryState): StaticHistoryState? = when (state) {
        is StaticHistoryState.Burst -> StaticHistoryState.Fast
        is StaticHistoryState.Crew -> StaticHistoryState.Working
        is StaticHistoryState.Cruise -> StaticHistoryState.Fast
        is StaticHistoryState.Fast -> StaticHistoryState.Running
        is StaticHistoryState.L1 -> StaticHistoryState.Left
        is StaticHistoryState.L2 -> StaticHistoryState.Left
        is StaticHistoryState.Left -> StaticHistoryState.Crew
        is StaticHistoryState.R1 -> StaticHistoryState.Right
        is StaticHistoryState.R2 -> StaticHistoryState.Right
        is StaticHistoryState.Right -> StaticHistoryState.Crew
        is StaticHistoryState.Slow -> StaticHistoryState.Running
        else -> null
    }

    // W3C SCXML 3.3: a <state> with child states — exactly the states that
    // have an initial transition. A <parallel> is not compound.
    override fun isCompoundState(state: StaticHistoryState): Boolean = when (state) {
        is StaticHistoryState.Fast, is StaticHistoryState.Left, is StaticHistoryState.Right, is StaticHistoryState.Running, is StaticHistoryState.Working -> true
        else -> false
    }

    // W3C SCXML 3.4: Check if state is a parallel state
    override fun isParallelState(state: StaticHistoryState): Boolean = when (state) {
        is StaticHistoryState.Crew -> true
        else -> false
    }

    // §scxml-D-getChildStates: a state's <state>, <parallel> and <final>
    // children, in document order — for a <parallel>, its regions.
    override fun childStatesOf(state: StaticHistoryState): List<StaticHistoryState> =
        childStates[state] ?: emptyList()

    // W3C SCXML 3.3: a compound state's initial transition target, as written.
    override fun initialTargetsOf(state: StaticHistoryState): List<EntryTarget<StaticHistoryState, HistoryId>> =
        initialTargets[state] ?: emptyList()

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticHistoryState, HistoryId>>
        get() = documentInitialTargetList

    // W3C SCXML 3.10: the state a <history> is declared in.
    override fun historyParentOf(history: HistoryId): StaticHistoryState = historyParents.getValue(history)

    // W3C SCXML 3.10.2: a <history>'s default transition target, as written.
    override fun historyDefaultTargetsOf(history: HistoryId): List<EntryTarget<StaticHistoryState, HistoryId>> =
        historyDefaultTargets.getValue(history)

    // W3C SCXML 3.10: a state's <history> children, each with whether it is
    // deep — what the runtime records as the state is exited.
    override fun historiesOf(state: StaticHistoryState): List<Pair<HistoryId, Boolean>> =
        historiesByParent[state] ?: emptyList()

    // SCE Accepted Subset §2.15: the id a saved state keys a <history> by, and
    // the history an id names.
    override fun historyIdOf(history: HistoryId): String = when (history) {
        historyCrewAll -> "crew_all"
        historyDeepest -> "deepest"
        historyLast -> "last"
        else -> error("no <history> of this document is $history")
    }

    override fun resolveHistory(historyId: String): HistoryId? = when (historyId) {
        "crew_all" -> historyCrewAll
        "deepest" -> historyDeepest
        "last" -> historyLast
        else -> null
    }

    private companion object {
        /** W3C SCXML 3.10: the `crew_all` <history> (deep). */
        val historyCrewAll = HistoryId(0)

        /** W3C SCXML 3.10: the `deepest` <history> (deep). */
        val historyDeepest = HistoryId(1)

        /** W3C SCXML 3.10: the `last` <history> (shallow). */
        val historyLast = HistoryId(2)

        val childStates: Map<StaticHistoryState, List<StaticHistoryState>> = mapOf(
            StaticHistoryState.Crew to listOf(StaticHistoryState.Left, StaticHistoryState.Right),
            StaticHistoryState.Fast to listOf(StaticHistoryState.Cruise, StaticHistoryState.Burst),
            StaticHistoryState.Left to listOf(StaticHistoryState.L1, StaticHistoryState.L2),
            StaticHistoryState.Right to listOf(StaticHistoryState.R1, StaticHistoryState.R2),
            StaticHistoryState.Running to listOf(StaticHistoryState.Slow, StaticHistoryState.Fast),
            StaticHistoryState.Working to listOf(StaticHistoryState.Crew),
        )

        val initialTargets: Map<StaticHistoryState, List<EntryTarget<StaticHistoryState, HistoryId>>> = mapOf(
            StaticHistoryState.Fast to listOf(StateTarget(StaticHistoryState.Cruise)),
            StaticHistoryState.Left to listOf(StateTarget(StaticHistoryState.L1)),
            StaticHistoryState.Right to listOf(StateTarget(StaticHistoryState.R1)),
            StaticHistoryState.Running to listOf(StateTarget(StaticHistoryState.Slow)),
            StaticHistoryState.Working to listOf(StateTarget(StaticHistoryState.Crew)),
        )

        val documentInitialTargetList: List<EntryTarget<StaticHistoryState, HistoryId>> =
            listOf(StateTarget(StaticHistoryState.Running))

        val historyParents: Map<HistoryId, StaticHistoryState> = mapOf(
            historyCrewAll to StaticHistoryState.Working,
            historyDeepest to StaticHistoryState.Running,
            historyLast to StaticHistoryState.Running,
        )

        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<StaticHistoryState, HistoryId>>> = mapOf(
            historyCrewAll to listOf(StateTarget(StaticHistoryState.Crew)),
            historyDeepest to listOf(StateTarget(StaticHistoryState.Slow)),
            historyLast to listOf(StateTarget(StaticHistoryState.Slow)),
        )

        val historiesByParent: Map<StaticHistoryState, List<Pair<HistoryId, Boolean>>> = mapOf(
            StaticHistoryState.Running to listOf(historyDeepest to true, historyLast to false),
            StaticHistoryState.Working to listOf(historyCrewAll to true),
        )

        // W3C SCXML 3.13: cruise's transition 0, as the microstep reads it.
        val transitionCruiseAt0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Cruise,
            listOf(StateTarget(StaticHistoryState.Burst)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: l1's transition 0, as the microstep reads it.
        val transitionL1At0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.L1,
            listOf(StateTarget(StaticHistoryState.L2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 0, as the microstep reads it.
        val transitionPausedAt0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Paused,
            listOf(StateTarget(StaticHistoryState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 1, as the microstep reads it.
        val transitionPausedAt1 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Paused,
            listOf(HistoryTarget(historyLast)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 2, as the microstep reads it.
        val transitionPausedAt2 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Paused,
            listOf(HistoryTarget(historyDeepest)),
            2,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: paused's transition 3, as the microstep reads it.
        val transitionPausedAt3 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Paused,
            listOf(HistoryTarget(historyCrewAll)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: r1's transition 0, as the microstep reads it.
        val transitionR1At0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.R1,
            listOf(StateTarget(StaticHistoryState.R2)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 0, as the microstep reads it.
        val transitionRunningAt0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Running,
            listOf(StateTarget(StaticHistoryState.Paused)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: slow's transition 0, as the microstep reads it.
        val transitionSlowAt0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Slow,
            listOf(StateTarget(StaticHistoryState.Fast)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticHistoryState, HistoryId>(
            StaticHistoryState.Working,
            listOf(StateTarget(StaticHistoryState.Paused)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHistoryState? = when (stateId) {
        "burst" -> StaticHistoryState.Burst
        "crew" -> StaticHistoryState.Crew
        "cruise" -> StaticHistoryState.Cruise
        "fast" -> StaticHistoryState.Fast
        "l1" -> StaticHistoryState.L1
        "l2" -> StaticHistoryState.L2
        "left" -> StaticHistoryState.Left
        "paused" -> StaticHistoryState.Paused
        "r1" -> StaticHistoryState.R1
        "r2" -> StaticHistoryState.R2
        "right" -> StaticHistoryState.Right
        "running" -> StaticHistoryState.Running
        "slow" -> StaticHistoryState.Slow
        "working" -> StaticHistoryState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHistoryState): String = when (state) {
        is StaticHistoryState.Burst -> "burst"
        is StaticHistoryState.Crew -> "crew"
        is StaticHistoryState.Cruise -> "cruise"
        is StaticHistoryState.Fast -> "fast"
        is StaticHistoryState.L1 -> "l1"
        is StaticHistoryState.L2 -> "l2"
        is StaticHistoryState.Left -> "left"
        is StaticHistoryState.Paused -> "paused"
        is StaticHistoryState.R1 -> "r1"
        is StaticHistoryState.R2 -> "r2"
        is StaticHistoryState.Right -> "right"
        is StaticHistoryState.Running -> "running"
        is StaticHistoryState.Slow -> "slow"
        is StaticHistoryState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHistoryState): Int = when (state) {
        is StaticHistoryState.Burst -> 4
        is StaticHistoryState.Crew -> 6
        is StaticHistoryState.Cruise -> 3
        is StaticHistoryState.Fast -> 2
        is StaticHistoryState.L1 -> 8
        is StaticHistoryState.L2 -> 9
        is StaticHistoryState.Left -> 7
        is StaticHistoryState.Paused -> 13
        is StaticHistoryState.R1 -> 11
        is StaticHistoryState.R2 -> 12
        is StaticHistoryState.Right -> 10
        is StaticHistoryState.Running -> 0
        is StaticHistoryState.Slow -> 1
        is StaticHistoryState.Working -> 5
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticHistoryEvent? = when (name) {
        "boost" -> StaticHistoryEvent.Boost
        "break" -> StaticHistoryEvent.Break
        "faster" -> StaticHistoryEvent.Faster
        "left_next" -> StaticHistoryEvent.LeftNext
        "pause" -> StaticHistoryEvent.Pause
        "resume_crew" -> StaticHistoryEvent.ResumeCrew
        "resume_deep" -> StaticHistoryEvent.ResumeDeep
        "resume_last" -> StaticHistoryEvent.ResumeLast
        "right_next" -> StaticHistoryEvent.RightNext
        "work" -> StaticHistoryEvent.Work
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticHistoryEvent): String? = when (event) {
        is StaticHistoryEvent.Boost -> "boost"
        is StaticHistoryEvent.Break -> "break"
        is StaticHistoryEvent.Faster -> "faster"
        is StaticHistoryEvent.LeftNext -> "left_next"
        is StaticHistoryEvent.Pause -> "pause"
        is StaticHistoryEvent.ResumeCrew -> "resume_crew"
        is StaticHistoryEvent.ResumeDeep -> "resume_deep"
        is StaticHistoryEvent.ResumeLast -> "resume_last"
        is StaticHistoryEvent.RightNext -> "right_next"
        is StaticHistoryEvent.Work -> "work"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHistoryState,
        event: StaticHistoryEvent?
    ): EnabledTransition<StaticHistoryState, HistoryId>? = when (state) {
        is StaticHistoryState.Cruise -> when {
            event is StaticHistoryEvent.Boost -> transitionCruiseAt0
            else -> null
        }
        is StaticHistoryState.L1 -> when {
            event is StaticHistoryEvent.LeftNext -> transitionL1At0
            else -> null
        }
        is StaticHistoryState.Paused -> when {
            event is StaticHistoryEvent.Work -> transitionPausedAt0
            event is StaticHistoryEvent.ResumeLast -> transitionPausedAt1
            event is StaticHistoryEvent.ResumeDeep -> transitionPausedAt2
            event is StaticHistoryEvent.ResumeCrew -> transitionPausedAt3
            else -> null
        }
        is StaticHistoryState.R1 -> when {
            event is StaticHistoryEvent.RightNext -> transitionR1At0
            else -> null
        }
        is StaticHistoryState.Running -> when {
            event is StaticHistoryEvent.Pause -> transitionRunningAt0
            else -> null
        }
        is StaticHistoryState.Slow -> when {
            event is StaticHistoryEvent.Faster -> transitionSlowAt0
            else -> null
        }
        is StaticHistoryState.Working -> when {
            event is StaticHistoryEvent.Break -> transitionWorkingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun onEntry(state: StaticHistoryState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHistoryState.Burst -> {
                // SCE-MAP: static_history.scxml:52 :: burst :: _state_body
            }
            is StaticHistoryState.Crew -> {
                // SCE-MAP: static_history.scxml:60 :: crew :: _state_body
            }
            is StaticHistoryState.Cruise -> {
                // SCE-MAP: static_history.scxml:49 :: cruise :: _state_body
            }
            is StaticHistoryState.Fast -> {
                // SCE-MAP: static_history.scxml:48 :: fast :: _state_body
            }
            is StaticHistoryState.L1 -> {
                // SCE-MAP: static_history.scxml:62 :: l1 :: _state_body
            }
            is StaticHistoryState.L2 -> {
                // SCE-MAP: static_history.scxml:65 :: l2 :: _state_body
            }
            is StaticHistoryState.Left -> {
                // SCE-MAP: static_history.scxml:61 :: left :: _state_body
            }
            is StaticHistoryState.Paused -> {
                // SCE-MAP: static_history.scxml:76 :: paused :: _state_body
            }
            is StaticHistoryState.R1 -> {
                // SCE-MAP: static_history.scxml:68 :: r1 :: _state_body
            }
            is StaticHistoryState.R2 -> {
                // SCE-MAP: static_history.scxml:71 :: r2 :: _state_body
            }
            is StaticHistoryState.Right -> {
                // SCE-MAP: static_history.scxml:67 :: right :: _state_body
            }
            is StaticHistoryState.Running -> {
                // SCE-MAP: static_history.scxml:38 :: running :: _state_body
            }
            is StaticHistoryState.Slow -> {
                // SCE-MAP: static_history.scxml:45 :: slow :: _state_body
            }
            is StaticHistoryState.Working -> {
                // SCE-MAP: static_history.scxml:56 :: working :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun onExit(state: StaticHistoryState) {
        when (state) {
            is StaticHistoryState.Burst -> {
                // SCE-MAP: static_history.scxml:52 :: burst :: _state_body
            }
            is StaticHistoryState.Crew -> {
                // SCE-MAP: static_history.scxml:60 :: crew :: _state_body
            }
            is StaticHistoryState.Cruise -> {
                // SCE-MAP: static_history.scxml:49 :: cruise :: _state_body
            }
            is StaticHistoryState.Fast -> {
                // SCE-MAP: static_history.scxml:48 :: fast :: _state_body
            }
            is StaticHistoryState.L1 -> {
                // SCE-MAP: static_history.scxml:62 :: l1 :: _state_body
            }
            is StaticHistoryState.L2 -> {
                // SCE-MAP: static_history.scxml:65 :: l2 :: _state_body
            }
            is StaticHistoryState.Left -> {
                // SCE-MAP: static_history.scxml:61 :: left :: _state_body
            }
            is StaticHistoryState.Paused -> {
                // SCE-MAP: static_history.scxml:76 :: paused :: _state_body
            }
            is StaticHistoryState.R1 -> {
                // SCE-MAP: static_history.scxml:68 :: r1 :: _state_body
            }
            is StaticHistoryState.R2 -> {
                // SCE-MAP: static_history.scxml:71 :: r2 :: _state_body
            }
            is StaticHistoryState.Right -> {
                // SCE-MAP: static_history.scxml:67 :: right :: _state_body
            }
            is StaticHistoryState.Running -> {
                // SCE-MAP: static_history.scxml:38 :: running :: _state_body
            }
            is StaticHistoryState.Slow -> {
                // SCE-MAP: static_history.scxml:45 :: slow :: _state_body
            }
            is StaticHistoryState.Working -> {
                // SCE-MAP: static_history.scxml:56 :: working :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun executeTransitionContent(source: StaticHistoryState, transitionIndex: Int) {
        when (source) {
        is StaticHistoryState.Paused -> when (transitionIndex) {
            1 -> {
                // SCE-MAP: static_history.scxml:78 :: paused :: _transition_1

            if (try { resumed = com.sce.forge.runtime.SceChecked.add(resumed, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
