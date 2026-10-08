// SCE-GENERATED — DO NOT EDIT
// source-hash: 596ac4b1afa5b66720218d6a44ed9e9093342daa23116ea6b40585ea85ed56ab

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_list_assign.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_list_assign.scxml:16 :: _machine
@file:OptIn(ExperimentalUnsignedTypes::class)

package com.sce.integration.static_list_assign

import com.sce.runtime.*
import com.sce.generated.day_run.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticListAssignState : State {
    data object Showing : StaticListAssignState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticListAssignEvent : Event {
    data object All : StaticListAssignEvent
    data object Empty : StaticListAssignEvent
    sealed interface Error : StaticListAssignEvent {
        data object Execution : Error
    }
    data object Late : StaticListAssignEvent
    data object Show : StaticListAssignEvent
}
// --- State Machine (W3C SCXML) ---

class StaticListAssignStateMachine(
) : StateMachineEngine<StaticListAssignState, StaticListAssignEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `shown` datamodel variable, published (`sce:direction="out"`). */
    var shown: List<UByte> = emptyList()
        private set
    /** W3C SCXML 5.2: the `first` datamodel variable, published (`sce:direction="out"`). */
    var first: UByte = 10.toUByte()
        private set
    /** W3C SCXML 5.2: the `size` datamodel variable, published (`sce:direction="out"`). */
    var size: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var first: UByte? = null
        var size: UInt? = null
        var refusals: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.first?.let { first = it }
        params.size?.let { size = it }
        params.refusals?.let { refusals = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val shown: List<UByte>,
        val first: UByte,
        val size: UInt,
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
        val configuration: Set<StaticListAssignState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        shown = shown,
        first = first,
        size = size,
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
    val savedShape: String = "dc459b0d58d7fbf8dc7bed7a1601a72dedee5848ae2dce1e9e650ef14e73ff74"

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
            "shown" to SavedValues.list(shown) { SavedValues.of(it) },
            "first" to SavedValues.of(first),
            "size" to SavedValues.of(size),
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
        val saved1 = SavedValues.list(saved.variable("shown"), "shown", 8) { e, w -> SavedValues.uint8(e, w) }
        val saved2 = SavedValues.uint8(saved.variable("first"), "first")
        val saved3 = SavedValues.uint32(saved.variable("size"), "size")
        val saved4 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        shown = saved1
        first = saved2
        size = saved3
        refusals = saved4
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticListAssignState = StaticListAssignState.Showing

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
    override val documentInitialTargets: List<EntryTarget<StaticListAssignState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticListAssignState, HistoryId>> =
            listOf(StateTarget(StaticListAssignState.Showing))

        // W3C SCXML 3.13: showing's transition 0, as the microstep reads it.
        val transitionShowingAt0 = EnabledTransition<StaticListAssignState, HistoryId>(
            StaticListAssignState.Showing,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 1, as the microstep reads it.
        val transitionShowingAt1 = EnabledTransition<StaticListAssignState, HistoryId>(
            StaticListAssignState.Showing,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 2, as the microstep reads it.
        val transitionShowingAt2 = EnabledTransition<StaticListAssignState, HistoryId>(
            StaticListAssignState.Showing,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 3, as the microstep reads it.
        val transitionShowingAt3 = EnabledTransition<StaticListAssignState, HistoryId>(
            StaticListAssignState.Showing,
            emptyList(),
            3,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: showing's transition 4, as the microstep reads it.
        val transitionShowingAt4 = EnabledTransition<StaticListAssignState, HistoryId>(
            StaticListAssignState.Showing,
            emptyList(),
            4,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticListAssignState? = when (stateId) {
        "showing" -> StaticListAssignState.Showing
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticListAssignState): String = when (state) {
        is StaticListAssignState.Showing -> "showing"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticListAssignState): Int = when (state) {
        is StaticListAssignState.Showing -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticListAssignEvent? = when (name) {
        "all" -> StaticListAssignEvent.All
        "empty" -> StaticListAssignEvent.Empty
        "error.execution" -> StaticListAssignEvent.Error.Execution
        "late" -> StaticListAssignEvent.Late
        "show" -> StaticListAssignEvent.Show
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticListAssignEvent): String? = when (event) {
        is StaticListAssignEvent.All -> "all"
        is StaticListAssignEvent.Empty -> "empty"
        is StaticListAssignEvent.Error.Execution -> "error.execution"
        is StaticListAssignEvent.Late -> "late"
        is StaticListAssignEvent.Show -> "show"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticListAssignState,
        event: StaticListAssignEvent?
    ): EnabledTransition<StaticListAssignState, HistoryId>? = when (state) {
        is StaticListAssignState.Showing -> when {
            event is StaticListAssignEvent.Show -> transitionShowingAt0
            event is StaticListAssignEvent.All -> transitionShowingAt1
            event is StaticListAssignEvent.Empty -> transitionShowingAt2
            event is StaticListAssignEvent.Late -> transitionShowingAt3
            event is StaticListAssignEvent.Error.Execution -> transitionShowingAt4
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_list_assign.scxml:16 :: _machine
    override fun onEntry(state: StaticListAssignState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticListAssignState.Showing -> {
                // SCE-MAP: static_list_assign.scxml:25 :: showing :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_list_assign.scxml:16 :: _machine
    override fun onExit(state: StaticListAssignState) {
        when (state) {
            is StaticListAssignState.Showing -> {
                // SCE-MAP: static_list_assign.scxml:25 :: showing :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_list_assign.scxml:16 :: _machine
    override fun executeTransitionContent(source: StaticListAssignState, transitionIndex: Int) {
        when (source) {
        is StaticListAssignState.Showing -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_list_assign.scxml:26 :: showing :: _transition_0

            if (try { shown = (com.sce.forge.runtime.SceChecked.take(dayRun(first, 3.toUByte()))).toList(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListAssignEvent.Error.Execution, "<assign location='shown'>: an integer operation overflowed or failed"); true }) {
                return
            }

            size = (shown).size.toUInt()
            }
            1 -> {
                // SCE-MAP: static_list_assign.scxml:30 :: showing :: _transition_1

            if (try { shown = (com.sce.forge.runtime.SceChecked.take(dayRun(first, 8.toUByte()))).toList(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListAssignEvent.Error.Execution, "<assign location='shown'>: an integer operation overflowed or failed"); true }) {
                return
            }

            size = (shown).size.toUInt()
            }
            2 -> {
                // SCE-MAP: static_list_assign.scxml:34 :: showing :: _transition_2

            if (try { shown = (com.sce.forge.runtime.SceChecked.take(dayRun(first, 0.toUByte()))).toList(); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListAssignEvent.Error.Execution, "<assign location='shown'>: an integer operation overflowed or failed"); true }) {
                return
            }

            size = (shown).size.toUInt()
            }
            3 -> {
                // SCE-MAP: static_list_assign.scxml:38 :: showing :: _transition_3

            first = 253.toUByte()
            }
            4 -> {
                // SCE-MAP: static_list_assign.scxml:41 :: showing :: _transition_4

            if (try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticListAssignEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
