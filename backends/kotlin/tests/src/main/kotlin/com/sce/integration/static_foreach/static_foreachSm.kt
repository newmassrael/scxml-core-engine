// SCE-GENERATED — DO NOT EDIT
// source-hash: 9493bf23538ed8d47286821b6e59498d494b9a1841184ab306e76017e761f5de

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_foreach.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_foreach.scxml:13 :: _machine

package com.sce.integration.static_foreach

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticForeachState : State {
    data object Working : StaticForeachState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticForeachEvent : Event {
    data object A : StaticForeachEvent
    data object B : StaticForeachEvent
    data object C : StaticForeachEvent
    data object Clear : StaticForeachEvent
    data object Count : StaticForeachEvent
    data object Cross : StaticForeachEvent
    data object Double : StaticForeachEvent
    sealed interface Error : StaticForeachEvent {
        data object Execution : Error
    }
    data object Squeeze : StaticForeachEvent
    data object Sum : StaticForeachEvent
    data object Weigh : StaticForeachEvent
}
// --- State Machine (W3C SCXML) ---

class StaticForeachStateMachine(
) : StateMachineEngine<StaticForeachState, StaticForeachEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `picked` datamodel variable, published (`sce:direction="out"`). */
    var picked: List<UByte> = emptyList()
        private set
    /** W3C SCXML 5.2: the `total` datamodel variable, published (`sce:direction="out"`). */
    var total: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `weighted` datamodel variable, published (`sce:direction="out"`). */
    var weighted: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `small` datamodel variable, published (`sce:direction="out"`). */
    var small: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `crossings` datamodel variable, published (`sce:direction="out"`). */
    var crossings: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `visited` datamodel variable, published (`sce:direction="out"`). */
    var visited: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `finished` datamodel variable, published (`sce:direction="out"`). */
    var finished: UInt = 0.toUInt()
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
        var total: UInt? = null
        var weighted: UInt? = null
        var small: UByte? = null
        var crossings: UInt? = null
        var visited: UInt? = null
        var finished: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.total?.let { total = it }
        params.weighted?.let { weighted = it }
        params.small?.let { small = it }
        params.crossings?.let { crossings = it }
        params.visited?.let { visited = it }
        params.finished?.let { finished = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: List<UByte>,
        val total: UInt,
        val weighted: UInt,
        val small: UByte,
        val crossings: UInt,
        val visited: UInt,
        val finished: UInt,
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
        val configuration: Set<StaticForeachState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        picked = picked,
        total = total,
        weighted = weighted,
        small = small,
        crossings = crossings,
        visited = visited,
        finished = finished,
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
    val savedShape: String = "9a550dcd196beac21c3a3402db563b982c37d0f7ef094afdc6f5cd8d7c71375f"

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
            "picked" to SavedValues.list(picked) { SavedValues.of(it) },
            "total" to SavedValues.of(total),
            "weighted" to SavedValues.of(weighted),
            "small" to SavedValues.of(small),
            "crossings" to SavedValues.of(crossings),
            "visited" to SavedValues.of(visited),
            "finished" to SavedValues.of(finished),
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
        val saved1 = SavedValues.list(saved.variable("picked"), "picked", 4) { e, w -> SavedValues.uint8(e, w) }
        val saved2 = SavedValues.uint32(saved.variable("total"), "total")
        val saved3 = SavedValues.uint32(saved.variable("weighted"), "weighted")
        val saved4 = SavedValues.uint8(saved.variable("small"), "small")
        val saved5 = SavedValues.uint32(saved.variable("crossings"), "crossings")
        val saved6 = SavedValues.uint32(saved.variable("visited"), "visited")
        val saved7 = SavedValues.uint32(saved.variable("finished"), "finished")
        val saved8 = SavedValues.uint32(saved.variable("errors"), "errors")
        picked = saved1
        total = saved2
        weighted = saved3
        small = saved4
        crossings = saved5
        visited = saved6
        finished = saved7
        errors = saved8
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticForeachState = StaticForeachState.Working

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
    override val documentInitialTargets: List<EntryTarget<StaticForeachState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticForeachState, HistoryId>> =
            listOf(StateTarget(StaticForeachState.Working))

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 3, as the microstep reads it.
        val transitionWorkingAt3 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 4, as the microstep reads it.
        val transitionWorkingAt4 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 5, as the microstep reads it.
        val transitionWorkingAt5 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            5,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 6, as the microstep reads it.
        val transitionWorkingAt6 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            6,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 7, as the microstep reads it.
        val transitionWorkingAt7 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            7,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 8, as the microstep reads it.
        val transitionWorkingAt8 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            8,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 9, as the microstep reads it.
        val transitionWorkingAt9 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            9,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: working's transition 10, as the microstep reads it.
        val transitionWorkingAt10 = EnabledTransition<StaticForeachState, HistoryId>(
            StaticForeachState.Working,
            emptyList(),
            10,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticForeachState? = when (stateId) {
        "working" -> StaticForeachState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticForeachState): String = when (state) {
        is StaticForeachState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticForeachState): Int = when (state) {
        is StaticForeachState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticForeachEvent? = when (name) {
        "a" -> StaticForeachEvent.A
        "b" -> StaticForeachEvent.B
        "c" -> StaticForeachEvent.C
        "clear" -> StaticForeachEvent.Clear
        "count" -> StaticForeachEvent.Count
        "cross" -> StaticForeachEvent.Cross
        "double" -> StaticForeachEvent.Double
        "error.execution" -> StaticForeachEvent.Error.Execution
        "squeeze" -> StaticForeachEvent.Squeeze
        "sum" -> StaticForeachEvent.Sum
        "weigh" -> StaticForeachEvent.Weigh
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticForeachEvent): String? = when (event) {
        is StaticForeachEvent.A -> "a"
        is StaticForeachEvent.B -> "b"
        is StaticForeachEvent.C -> "c"
        is StaticForeachEvent.Clear -> "clear"
        is StaticForeachEvent.Count -> "count"
        is StaticForeachEvent.Cross -> "cross"
        is StaticForeachEvent.Double -> "double"
        is StaticForeachEvent.Error.Execution -> "error.execution"
        is StaticForeachEvent.Squeeze -> "squeeze"
        is StaticForeachEvent.Sum -> "sum"
        is StaticForeachEvent.Weigh -> "weigh"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticForeachState,
        event: StaticForeachEvent?
    ): EnabledTransition<StaticForeachState, HistoryId>? = when (state) {
        is StaticForeachState.Working -> when {
            event is StaticForeachEvent.A -> transitionWorkingAt0
            event is StaticForeachEvent.B -> transitionWorkingAt1
            event is StaticForeachEvent.C -> transitionWorkingAt2
            event is StaticForeachEvent.Clear -> transitionWorkingAt3
            event is StaticForeachEvent.Sum -> transitionWorkingAt4
            event is StaticForeachEvent.Weigh -> transitionWorkingAt5
            event is StaticForeachEvent.Squeeze -> transitionWorkingAt6
            event is StaticForeachEvent.Double -> transitionWorkingAt7
            event is StaticForeachEvent.Cross -> transitionWorkingAt8
            event is StaticForeachEvent.Count -> transitionWorkingAt9
            event is StaticForeachEvent.Error.Execution -> transitionWorkingAt10
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_foreach.scxml:13 :: _machine
    override fun onEntry(state: StaticForeachState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticForeachState.Working -> {
                // SCE-MAP: static_foreach.scxml:25 :: working :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_foreach.scxml:13 :: _machine
    override fun onExit(state: StaticForeachState) {
        when (state) {
            is StaticForeachState.Working -> {
                // SCE-MAP: static_foreach.scxml:25 :: working :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_foreach.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticForeachState, transitionIndex: Int) {
        when (source) {
        is StaticForeachState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_foreach.scxml:26 :: working :: _transition_0

            if (if (picked.size < 4) { picked = picked + (100.toUByte()); false } else { raisePlatformError(StaticForeachEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 4"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_foreach.scxml:27 :: working :: _transition_1

            if (if (picked.size < 4) { picked = picked + (200.toUByte()); false } else { raisePlatformError(StaticForeachEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 4"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_foreach.scxml:28 :: working :: _transition_2

            if (if (picked.size < 4) { picked = picked + (5.toUByte()); false } else { raisePlatformError(StaticForeachEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 4"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_foreach.scxml:29 :: working :: _transition_3

            picked = emptyList()
            }
            4 -> {
                // SCE-MAP: static_foreach.scxml:31 :: working :: _transition_4

            total = 0.toUInt()


            for (v in picked) {

            if (try { total = com.sce.forge.runtime.SceChecked.add(total, v.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='total'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { finished = com.sce.forge.runtime.SceChecked.add(finished, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='finished'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            5 -> {
                // SCE-MAP: static_foreach.scxml:39 :: working :: _transition_5

            weighted = 0.toUInt()


            for ((sce_position_of_i, v) in picked.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()

            if (try { weighted = com.sce.forge.runtime.SceChecked.add(weighted, com.sce.forge.runtime.SceChecked.mul(v.toUInt(), com.sce.forge.runtime.SceChecked.add(i, 1.toUInt()))); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='weighted'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { finished = com.sce.forge.runtime.SceChecked.add(finished, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='finished'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            6 -> {
                // SCE-MAP: static_foreach.scxml:49 :: working :: _transition_6

            small = 0.toUByte()


            for (v in picked) {

            if (try { small = com.sce.forge.runtime.SceChecked.add(small, v); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='small'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }

            if (try { finished = com.sce.forge.runtime.SceChecked.add(finished, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='finished'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            7 -> {
                // SCE-MAP: static_foreach.scxml:58 :: working :: _transition_7


            for (v in picked) {

            if (if (picked.size < 4) { picked = picked + (v); false } else { raisePlatformError(StaticForeachEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 4"); true }) {
                return
            }
            }

            if (try { finished = com.sce.forge.runtime.SceChecked.add(finished, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='finished'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            8 -> {
                // SCE-MAP: static_foreach.scxml:65 :: working :: _transition_8

            crossings = 0.toUInt()


            for ((sce_position_of_i, v) in picked.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()


            for (w in picked) {

            if (try { crossings = com.sce.forge.runtime.SceChecked.add(crossings, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='crossings'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            }
            9 -> {
                // SCE-MAP: static_foreach.scxml:74 :: working :: _transition_9

            visited = 0.toUInt()


            for ((sce_position_of_i, v) in picked.withIndex()) {
                @Suppress("UNUSED_VARIABLE") val i = sce_position_of_i.toUInt()

            if (try { visited = com.sce.forge.runtime.SceChecked.add(visited, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='visited'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            }
            10 -> {
                // SCE-MAP: static_foreach.scxml:80 :: working :: _transition_10

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticForeachEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
