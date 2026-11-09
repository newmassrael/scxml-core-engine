// SCE-GENERATED — DO NOT EDIT
// source-hash: 39c188440acd2a30e1c96c90e825a582fc7011a2c558c89e70ea674bb5bda40e

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
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticHistoryState, StaticHistoryEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `resumed` datamodel variable, published (`sce:direction="out"`). */
    private var vResumed: UByte = 0.toUByte()

    /** What the published `resumed` holds. */
    val resumed: UByte get() = vResumed

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
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.resumed?.let { vResumed = it }
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
        resumed = vResumed,
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
        first = "a694bcc2c6fbd3a4168fa70c9081522dc8979d50a9c363686a4968d6ea77308d",
        revised = "c389dbae2c171ae0448bcc1888d41b21d85b54e0c576fba7a5c56fe11f31580b",
        format2First = "a694bcc2c6fbd3a4168fa70c9081522dc8979d50a9c363686a4968d6ea77308d",
        format2Revised = "c389dbae2c171ae0448bcc1888d41b21d85b54e0c576fba7a5c56fe11f31580b",
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
    fun save(wallNowMs: Long): SavedState {
        val variables = LinkedHashMap<String, Any?>()
        saveVariablesPart0(variables)
        return savedState(savedShape, variables, wallNowMs)
    }

    private fun saveVariablesPart0(variables: MutableMap<String, Any?>) {
        variables += "resumed" to SavedValues.of(vResumed)
    }

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
        val writeVariablesPart0 = restoreVariablesPart0(savedGrown)
        writeVariablesPart0()
        enterSaved(savedGrown, wallNowMs)
    }

    // Reads this part's variables from [savedGrown], refusing a value its
    // variable's type cannot hold, and answers what writes them: nothing is
    // written until every part has read.
    private fun restoreVariablesPart0(savedGrown: SavedState): () -> Unit {
        val saved1 = SavedValues.uint8(savedGrown.variable("resumed"), "resumed")
        return {
            vResumed = saved1
        }
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // docs/adr/0045: this machine as a child session of another's saved state —
    // saved with it, judged with it before anything is built, and restored in
    // place of initialized.
    override fun saveAsSession(wallNowMs: Long): SavedState = save(wallNowMs)

    override fun judgeAsSession(saved: SavedState) {
        val savedGrown = beginRestore(saved, savedShape)
        judgeVariablesPart0(savedGrown)
    }

    private fun judgeVariablesPart0(savedGrown: SavedState) {
        SavedValues.uint8(savedGrown.variable("resumed"), "resumed")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

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
        is StaticHistoryState.Fast -> true
        is StaticHistoryState.Left -> true
        is StaticHistoryState.Right -> true
        is StaticHistoryState.Running -> true
        is StaticHistoryState.Working -> true
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
        val historyCrewAll: HistoryId get() = HistoryId(0)

        /** W3C SCXML 3.10: the `deepest` <history> (deep). */
        val historyDeepest: HistoryId get() = HistoryId(1)

        /** W3C SCXML 3.10: the `last` <history> (shallow). */
        val historyLast: HistoryId get() = HistoryId(2)

        val childStates: Map<StaticHistoryState, List<StaticHistoryState>> = LinkedHashMap<StaticHistoryState, List<StaticHistoryState>>().also {
            childStatesPart0(it)
        }

        private fun childStatesPart0(table: MutableMap<StaticHistoryState, List<StaticHistoryState>>) {
            table[StaticHistoryState.Crew] = listOf(StaticHistoryState.Left, StaticHistoryState.Right)
            table[StaticHistoryState.Fast] = listOf(StaticHistoryState.Cruise, StaticHistoryState.Burst)
            table[StaticHistoryState.Left] = listOf(StaticHistoryState.L1, StaticHistoryState.L2)
            table[StaticHistoryState.Right] = listOf(StaticHistoryState.R1, StaticHistoryState.R2)
            table[StaticHistoryState.Running] = listOf(StaticHistoryState.Slow, StaticHistoryState.Fast)
            table[StaticHistoryState.Working] = listOf(StaticHistoryState.Crew)
        }


        val initialTargets: Map<StaticHistoryState, List<EntryTarget<StaticHistoryState, HistoryId>>> = LinkedHashMap<StaticHistoryState, List<EntryTarget<StaticHistoryState, HistoryId>>>().also {
            initialTargetsPart0(it)
        }

        private fun initialTargetsPart0(table: MutableMap<StaticHistoryState, List<EntryTarget<StaticHistoryState, HistoryId>>>) {
            table[StaticHistoryState.Fast] = listOf(StateTarget(StaticHistoryState.Cruise))
            table[StaticHistoryState.Left] = listOf(StateTarget(StaticHistoryState.L1))
            table[StaticHistoryState.Right] = listOf(StateTarget(StaticHistoryState.R1))
            table[StaticHistoryState.Running] = listOf(StateTarget(StaticHistoryState.Slow))
            table[StaticHistoryState.Working] = listOf(StateTarget(StaticHistoryState.Crew))
        }


        val documentInitialTargetList: List<EntryTarget<StaticHistoryState, HistoryId>> =
            listOf(StateTarget(StaticHistoryState.Running))

        val historyParents: Map<HistoryId, StaticHistoryState> = LinkedHashMap<HistoryId, StaticHistoryState>().also {
            historyParentsPart0(it)
        }

        private fun historyParentsPart0(table: MutableMap<HistoryId, StaticHistoryState>) {
            table[historyCrewAll] = StaticHistoryState.Working
            table[historyDeepest] = StaticHistoryState.Running
            table[historyLast] = StaticHistoryState.Running
        }


        val historyDefaultTargets: Map<HistoryId, List<EntryTarget<StaticHistoryState, HistoryId>>> = LinkedHashMap<HistoryId, List<EntryTarget<StaticHistoryState, HistoryId>>>().also {
            historyDefaultTargetsPart0(it)
        }

        private fun historyDefaultTargetsPart0(table: MutableMap<HistoryId, List<EntryTarget<StaticHistoryState, HistoryId>>>) {
            table[historyCrewAll] = listOf(StateTarget(StaticHistoryState.Crew))
            table[historyDeepest] = listOf(StateTarget(StaticHistoryState.Slow))
            table[historyLast] = listOf(StateTarget(StaticHistoryState.Slow))
        }


        val historiesByParent: Map<StaticHistoryState, List<Pair<HistoryId, Boolean>>> = LinkedHashMap<StaticHistoryState, List<Pair<HistoryId, Boolean>>>().also {
            historiesByParentPart0(it)
        }

        private fun historiesByParentPart0(table: MutableMap<StaticHistoryState, List<Pair<HistoryId, Boolean>>>) {
            table[StaticHistoryState.Running] = listOf(historyDeepest to true, historyLast to false)
            table[StaticHistoryState.Working] = listOf(historyCrewAll to true)
        }

    }

    private object TransitionsPart0 {

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
    override fun firstEnabledTransition(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when (state) {
        is StaticHistoryState.Cruise -> firstEnabledTransitionOfCruise(state, event)
        is StaticHistoryState.L1 -> firstEnabledTransitionOfL1(state, event)
        is StaticHistoryState.Paused -> firstEnabledTransitionOfPaused(state, event)
        is StaticHistoryState.R1 -> firstEnabledTransitionOfR1(state, event)
        is StaticHistoryState.Running -> firstEnabledTransitionOfRunning(state, event)
        is StaticHistoryState.Slow -> firstEnabledTransitionOfSlow(state, event)
        is StaticHistoryState.Working -> firstEnabledTransitionOfWorking(state, event)
        else -> null
    }


    // cruise's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfCruise(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.Boost -> TransitionsPart0.transitionCruiseAt0
        else -> null
    }


    // l1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfL1(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.LeftNext -> TransitionsPart0.transitionL1At0
        else -> null
    }


    // paused's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfPaused(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.Work -> TransitionsPart0.transitionPausedAt0
        event is StaticHistoryEvent.ResumeLast -> TransitionsPart0.transitionPausedAt1
        event is StaticHistoryEvent.ResumeDeep -> TransitionsPart0.transitionPausedAt2
        event is StaticHistoryEvent.ResumeCrew -> TransitionsPart0.transitionPausedAt3
        else -> null
    }


    // r1's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfR1(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.RightNext -> TransitionsPart0.transitionR1At0
        else -> null
    }


    // running's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfRunning(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.Pause -> TransitionsPart0.transitionRunningAt0
        else -> null
    }


    // slow's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfSlow(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.Faster -> TransitionsPart0.transitionSlowAt0
        else -> null
    }


    // working's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfWorking(state: StaticHistoryState, event: StaticHistoryEvent?): EnabledTransition<StaticHistoryState, HistoryId>? = when {
        event is StaticHistoryEvent.Break -> TransitionsPart0.transitionWorkingAt0
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun onEntry(state: StaticHistoryState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHistoryState.Burst -> onEntryOfBurst()
            is StaticHistoryState.Crew -> onEntryOfCrew()
            is StaticHistoryState.Cruise -> onEntryOfCruise()
            is StaticHistoryState.Fast -> onEntryOfFast()
            is StaticHistoryState.L1 -> onEntryOfL1()
            is StaticHistoryState.L2 -> onEntryOfL2()
            is StaticHistoryState.Left -> onEntryOfLeft()
            is StaticHistoryState.Paused -> onEntryOfPaused()
            is StaticHistoryState.R1 -> onEntryOfR1()
            is StaticHistoryState.R2 -> onEntryOfR2()
            is StaticHistoryState.Right -> onEntryOfRight()
            is StaticHistoryState.Running -> onEntryOfRunning()
            is StaticHistoryState.Slow -> onEntryOfSlow()
            is StaticHistoryState.Working -> onEntryOfWorking()
        }
    }


    private fun onEntryOfBurst() {
                // SCE-MAP: static_history.scxml:52 :: burst :: _state_body
    }

    private fun onEntryOfCrew() {
                // SCE-MAP: static_history.scxml:60 :: crew :: _state_body
    }

    private fun onEntryOfCruise() {
                // SCE-MAP: static_history.scxml:49 :: cruise :: _state_body
    }

    private fun onEntryOfFast() {
                // SCE-MAP: static_history.scxml:48 :: fast :: _state_body
    }

    private fun onEntryOfL1() {
                // SCE-MAP: static_history.scxml:62 :: l1 :: _state_body
    }

    private fun onEntryOfL2() {
                // SCE-MAP: static_history.scxml:65 :: l2 :: _state_body
    }

    private fun onEntryOfLeft() {
                // SCE-MAP: static_history.scxml:61 :: left :: _state_body
    }

    private fun onEntryOfPaused() {
                // SCE-MAP: static_history.scxml:76 :: paused :: _state_body
    }

    private fun onEntryOfR1() {
                // SCE-MAP: static_history.scxml:68 :: r1 :: _state_body
    }

    private fun onEntryOfR2() {
                // SCE-MAP: static_history.scxml:71 :: r2 :: _state_body
    }

    private fun onEntryOfRight() {
                // SCE-MAP: static_history.scxml:67 :: right :: _state_body
    }

    private fun onEntryOfRunning() {
                // SCE-MAP: static_history.scxml:38 :: running :: _state_body
    }

    private fun onEntryOfSlow() {
                // SCE-MAP: static_history.scxml:45 :: slow :: _state_body
    }

    private fun onEntryOfWorking() {
                // SCE-MAP: static_history.scxml:56 :: working :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun onExit(state: StaticHistoryState) {
        when (state) {
            is StaticHistoryState.Burst -> onExitOfBurst()
            is StaticHistoryState.Crew -> onExitOfCrew()
            is StaticHistoryState.Cruise -> onExitOfCruise()
            is StaticHistoryState.Fast -> onExitOfFast()
            is StaticHistoryState.L1 -> onExitOfL1()
            is StaticHistoryState.L2 -> onExitOfL2()
            is StaticHistoryState.Left -> onExitOfLeft()
            is StaticHistoryState.Paused -> onExitOfPaused()
            is StaticHistoryState.R1 -> onExitOfR1()
            is StaticHistoryState.R2 -> onExitOfR2()
            is StaticHistoryState.Right -> onExitOfRight()
            is StaticHistoryState.Running -> onExitOfRunning()
            is StaticHistoryState.Slow -> onExitOfSlow()
            is StaticHistoryState.Working -> onExitOfWorking()
        }
    }


    private fun onExitOfBurst() {
                // SCE-MAP: static_history.scxml:52 :: burst :: _state_body
    }

    private fun onExitOfCrew() {
                // SCE-MAP: static_history.scxml:60 :: crew :: _state_body
    }

    private fun onExitOfCruise() {
                // SCE-MAP: static_history.scxml:49 :: cruise :: _state_body
    }

    private fun onExitOfFast() {
                // SCE-MAP: static_history.scxml:48 :: fast :: _state_body
    }

    private fun onExitOfL1() {
                // SCE-MAP: static_history.scxml:62 :: l1 :: _state_body
    }

    private fun onExitOfL2() {
                // SCE-MAP: static_history.scxml:65 :: l2 :: _state_body
    }

    private fun onExitOfLeft() {
                // SCE-MAP: static_history.scxml:61 :: left :: _state_body
    }

    private fun onExitOfPaused() {
                // SCE-MAP: static_history.scxml:76 :: paused :: _state_body
    }

    private fun onExitOfR1() {
                // SCE-MAP: static_history.scxml:68 :: r1 :: _state_body
    }

    private fun onExitOfR2() {
                // SCE-MAP: static_history.scxml:71 :: r2 :: _state_body
    }

    private fun onExitOfRight() {
                // SCE-MAP: static_history.scxml:67 :: right :: _state_body
    }

    private fun onExitOfRunning() {
                // SCE-MAP: static_history.scxml:38 :: running :: _state_body
    }

    private fun onExitOfSlow() {
                // SCE-MAP: static_history.scxml:45 :: slow :: _state_body
    }

    private fun onExitOfWorking() {
                // SCE-MAP: static_history.scxml:56 :: working :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_history.scxml:33 :: _machine
    override fun executeTransitionContent(source: StaticHistoryState, transitionIndex: Int) {
        when (source) {
            is StaticHistoryState.Paused -> transitionContentOfPaused(transitionIndex)
            else -> {}
        }
    }


    // The content of `paused`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfPaused(transitionIndex: Int) {
        when (transitionIndex) {
            1 -> transitionContentOfPausedAt1()
            else -> {}
        }
    }


    private fun transitionContentOfPausedAt1() {
                // SCE-MAP: static_history.scxml:78 :: paused :: _transition_1

            if (try { vResumed = com.sce.forge.runtime.SceChecked.add(vResumed, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
    }
}
