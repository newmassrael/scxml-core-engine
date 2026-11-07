// SCE-GENERATED — DO NOT EDIT
// source-hash: 22249655389e33dabe1467cad0f6465e75f1c83f40cd0207380a1fc15324f922

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
    /**
     * docs/adr/0045: the id of this machine's session (`_sessionid`), which a
     * saved state names it by and a peer addresses it by (`#_scxml_<id>`), given
     * by the host that creates it: an XML Nmtoken, refused when it is not one
     * and never replaced. Drawn (a UUID version 4) when the host gives none. A
     * child is given the id its parent derives for it, and this is not read.
     */
    sessionId: String? = null,
) : StateMachineEngine<StaticRealState, StaticRealEvent>() {

    init {
        bindRootSessionId(sessionId)
    }

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    private var vLevel: Double = 0.5

    /** What the published `level` holds. */
    val level: Double get() = vLevel
    /** W3C SCXML 5.2: the `rate` datamodel variable, the machine's own. */
    private var vRate: Double = 1.5
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    private var vTotal: Double = 0.0

    /** What the published `total` holds. */
    val total: Double get() = vTotal
    /** W3C SCXML 5.2: the `tenth` datamodel variable, the machine's own. */
    private var vTenth: Double = 0.1
    /** W3C SCXML 5.2: the `drift` datamodel variable, published (`sce:direction="out"`). */
    private var vDrift: Double = 0.0

    /** What the published `drift` holds. */
    val drift: Double get() = vDrift
    /** W3C SCXML 5.2: the `samples` datamodel variable, published (`sce:direction="out"`). */
    private var vSamples: List<Double> = emptyList()

    /** What the published `samples` holds. */
    val samples: List<Double> get() = vSamples
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    private var vErrors: UInt = 0.toUInt()

    /** What the published `errors` holds. */
    val errors: UInt get() = vErrors

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
        acceptParamsPart0(params)
    }

    private fun acceptParamsPart0(params: InvokeParams) {
        params.level?.let { vLevel = it }
        params.rate?.let { vRate = it }
        params.total?.let { vTotal = it }
        params.tenth?.let { vTenth = it }
        params.drift?.let { vDrift = it }
        params.errors?.let { vErrors = it }
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
        level = vLevel,
        total = vTotal,
        drift = vDrift,
        samples = vSamples,
        errors = vErrors,
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
        first = "f44ab1df16ee701bda26935e1acf131aaf19f648ffc8db1be27381c1a76a4133",
        revised = "2587ba0a8cc82b3651360f544a67d759659dde6552f8fcdf2cc117fe60bbdf1f",
        format2First = "f44ab1df16ee701bda26935e1acf131aaf19f648ffc8db1be27381c1a76a4133",
        format2Revised = "2587ba0a8cc82b3651360f544a67d759659dde6552f8fcdf2cc117fe60bbdf1f",
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
        variables += "level" to SavedValues.of(vLevel)
        variables += "rate" to SavedValues.of(vRate)
        variables += "total" to SavedValues.of(vTotal)
        variables += "tenth" to SavedValues.of(vTenth)
        variables += "drift" to SavedValues.of(vDrift)
        variables += "samples" to SavedValues.list(vSamples) { SavedValues.of(it) }
        variables += "errors" to SavedValues.of(vErrors)
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
        val saved1 = SavedValues.float64(savedGrown.variable("level"), "level")
        val saved2 = SavedValues.float64(savedGrown.variable("rate"), "rate")
        val saved3 = SavedValues.float64(savedGrown.variable("total"), "total")
        val saved4 = SavedValues.float64(savedGrown.variable("tenth"), "tenth")
        val saved5 = SavedValues.float64(savedGrown.variable("drift"), "drift")
        val saved6 = SavedValues.list(savedGrown.variable("samples"), "samples", 3) { e, w -> SavedValues.float64(e, w) }
        val saved7 = SavedValues.uint32(savedGrown.variable("errors"), "errors")
        return {
            vLevel = saved1
            vRate = saved2
            vTotal = saved3
            vTenth = saved4
            vDrift = saved5
            vSamples = saved6
            vErrors = saved7
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
        SavedValues.float64(savedGrown.variable("level"), "level")
        SavedValues.float64(savedGrown.variable("rate"), "rate")
        SavedValues.float64(savedGrown.variable("total"), "total")
        SavedValues.float64(savedGrown.variable("tenth"), "tenth")
        SavedValues.float64(savedGrown.variable("drift"), "drift")
        SavedValues.list(savedGrown.variable("samples"), "samples", 3) { e, w -> SavedValues.float64(e, w) }
        SavedValues.uint32(savedGrown.variable("errors"), "errors")
    }

    override fun restoreAsSession(saved: SavedState, wallNowMs: Long) = restore(saved, wallNowMs)

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
    }

    private object TransitionsPart0 {

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
    override fun firstEnabledTransition(state: StaticRealState, event: StaticRealEvent?): EnabledTransition<StaticRealState, HistoryId>? = when (state) {
        is StaticRealState.High -> firstEnabledTransitionOfHigh(state, event)
        is StaticRealState.Low -> firstEnabledTransitionOfLow(state, event)
    }


    // high's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfHigh(state: StaticRealState, event: StaticRealEvent?): EnabledTransition<StaticRealState, HistoryId>? = when {
        event is StaticRealEvent.Shrink -> TransitionsPart0.transitionHighAt0
        event is StaticRealEvent.Drop && vLevel < 1.0 -> TransitionsPart0.transitionHighAt1
        else -> null
    }


    // low's own transitions, in document order.
    @Suppress("UNUSED_PARAMETER") private fun firstEnabledTransitionOfLow(state: StaticRealState, event: StaticRealEvent?): EnabledTransition<StaticRealState, HistoryId>? = when {
        event is StaticRealEvent.Grow -> TransitionsPart0.transitionLowAt0
        event is StaticRealEvent.Shrink -> TransitionsPart0.transitionLowAt1
        event is StaticRealEvent.Sum -> TransitionsPart0.transitionLowAt2
        event is StaticRealEvent.Drift -> TransitionsPart0.transitionLowAt3
        event is StaticRealEvent.Rise && vLevel > 7.0 -> TransitionsPart0.transitionLowAt4
        event is StaticRealEvent.Error.Execution -> TransitionsPart0.transitionLowAt5
        else -> null
    }





    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun onEntry(state: StaticRealState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticRealState.High -> onEntryOfHigh()
            is StaticRealState.Low -> onEntryOfLow()
        }
    }


    private fun onEntryOfHigh() {
                // SCE-MAP: static_real.scxml:57 :: high :: _state_body
    }

    private fun onEntryOfLow() {
                // SCE-MAP: static_real.scxml:29 :: low :: _state_body
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun onExit(state: StaticRealState) {
        when (state) {
            is StaticRealState.High -> onExitOfHigh()
            is StaticRealState.Low -> onExitOfLow()
        }
    }


    private fun onExitOfHigh() {
                // SCE-MAP: static_real.scxml:57 :: high :: _state_body
    }

    private fun onExitOfLow() {
                // SCE-MAP: static_real.scxml:29 :: low :: _state_body
    }



    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_real.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticRealState, transitionIndex: Int) {
        when (source) {
            is StaticRealState.High -> transitionContentOfHigh(transitionIndex)
            is StaticRealState.Low -> transitionContentOfLow(transitionIndex)
        }
    }


    // The content of `high`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfHigh(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfHighAt0()
            else -> {}
        }
    }


    private fun transitionContentOfHighAt0() {
                // SCE-MAP: static_real.scxml:58 :: high :: _transition_0

            vLevel = vLevel / 4.0
    }

    // The content of `low`'s own transitions, by the index the
    // selection answered with. A transition without content is never
    // dispatched here — the microstep asks only for transitions that carry
    // some — so the remaining indices are unreachable, which is what an empty
    // `else` says.
    private fun transitionContentOfLow(transitionIndex: Int) {
        when (transitionIndex) {
            0 -> transitionContentOfLowAt0()
            1 -> transitionContentOfLowAt1()
            2 -> transitionContentOfLowAt2()
            3 -> transitionContentOfLowAt3()
            5 -> transitionContentOfLowAt5()
            else -> {}
        }
    }


    private fun transitionContentOfLowAt0() {
                // SCE-MAP: static_real.scxml:32 :: low :: _transition_0

            vLevel = vLevel * 2.0 + vRate

            if (if (vSamples.size < 3) { vSamples = vSamples + (vLevel); false } else { raisePlatformError(StaticRealEvent.Error.Execution, "<sce:append target='samples'>: the list already holds its capacity of 3"); true }) {
                return
            }
    }

    private fun transitionContentOfLowAt1() {
                // SCE-MAP: static_real.scxml:37 :: low :: _transition_1

            vLevel = vLevel / 4.0
    }

    private fun transitionContentOfLowAt2() {
                // SCE-MAP: static_real.scxml:41 :: low :: _transition_2

            vTotal = 0.0


            for (v in vSamples) {

            vTotal = vTotal + v
            }
    }

    private fun transitionContentOfLowAt3() {
                // SCE-MAP: static_real.scxml:48 :: low :: _transition_3

            vDrift = vTenth + 0.2
    }

    private fun transitionContentOfLowAt5() {
                // SCE-MAP: static_real.scxml:53 :: low :: _transition_5

            if (try { vErrors = com.sce.forge.runtime.SceChecked.add(vErrors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticRealEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
    }
}
