// SCE-GENERATED — DO NOT EDIT
// source-hash: 090b32883712796c8da998647649f8ae8d6c2a646a1a50c5d49a92fbc3c7e438

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_block_ends_list.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_block_ends_list.scxml:18 :: _machine

package com.sce.integration.static_block_ends_list

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticBlockEndsListState : State {
    data object Waiting : StaticBlockEndsListState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticBlockEndsListEvent : Event {
    sealed interface Error : StaticBlockEndsListEvent {
        data object Execution : Error
    }
    data object Pick : StaticBlockEndsListEvent
}
// --- State Machine (W3C SCXML) ---

class StaticBlockEndsListStateMachine(
) : StateMachineEngine<StaticBlockEndsListState, StaticBlockEndsListEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `picked` datamodel variable, published (`sce:direction="out"`). */
    var picked: List<UByte> = emptyList()
        private set
    /** W3C SCXML 5.2: the `afterAppend` datamodel variable, published (`sce:direction="out"`). */
    var afterAppend: UByte = 0.toUByte()
        private set
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UByte = 0.toUByte()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var afterAppend: UByte? = null
        var errors: UByte? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.afterAppend?.let { afterAppend = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val picked: List<UByte>,
        val afterAppend: UByte,
        val errors: UByte,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticBlockEndsListState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        picked = picked,
        afterAppend = afterAppend,
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
    val savedShape: String = "670c183cda40bb46e81808b12dca21830ecda45892b88b766b37c8ca951c6b57"

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
            "afterAppend" to SavedValues.of(afterAppend),
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
        val saved1 = SavedValues.list(saved.variable("picked"), "picked", 2) { e, w -> SavedValues.uint8(e, w) }
        val saved2 = SavedValues.uint8(saved.variable("afterAppend"), "afterAppend")
        val saved3 = SavedValues.uint8(saved.variable("errors"), "errors")
        picked = saved1
        afterAppend = saved2
        errors = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticBlockEndsListState = StaticBlockEndsListState.Waiting

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
    override val documentInitialTargets: List<EntryTarget<StaticBlockEndsListState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticBlockEndsListState, HistoryId>> =
            listOf(StateTarget(StaticBlockEndsListState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticBlockEndsListState, HistoryId>(
            StaticBlockEndsListState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticBlockEndsListState, HistoryId>(
            StaticBlockEndsListState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticBlockEndsListState? = when (stateId) {
        "waiting" -> StaticBlockEndsListState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticBlockEndsListState): String = when (state) {
        is StaticBlockEndsListState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticBlockEndsListState): Int = when (state) {
        is StaticBlockEndsListState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticBlockEndsListEvent? = when (name) {
        "error.execution" -> StaticBlockEndsListEvent.Error.Execution
        "pick" -> StaticBlockEndsListEvent.Pick
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticBlockEndsListEvent): String? = when (event) {
        is StaticBlockEndsListEvent.Error.Execution -> "error.execution"
        is StaticBlockEndsListEvent.Pick -> "pick"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticBlockEndsListState,
        event: StaticBlockEndsListEvent?
    ): EnabledTransition<StaticBlockEndsListState, HistoryId>? = when (state) {
        is StaticBlockEndsListState.Waiting -> when {
            event is StaticBlockEndsListEvent.Pick -> transitionWaitingAt0
            event is StaticBlockEndsListEvent.Error.Execution -> transitionWaitingAt1
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_block_ends_list.scxml:18 :: _machine
    override fun onEntry(state: StaticBlockEndsListState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticBlockEndsListState.Waiting -> {
                // SCE-MAP: static_block_ends_list.scxml:25 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_block_ends_list.scxml:18 :: _machine
    override fun onExit(state: StaticBlockEndsListState) {
        when (state) {
            is StaticBlockEndsListState.Waiting -> {
                // SCE-MAP: static_block_ends_list.scxml:25 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_block_ends_list.scxml:18 :: _machine
    override fun executeTransitionContent(source: StaticBlockEndsListState, transitionIndex: Int) {
        when (source) {
        is StaticBlockEndsListState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_block_ends_list.scxml:26 :: waiting :: _transition_0

            if (if (picked.size < 2) { picked = picked + (7.toUByte()); false } else { raisePlatformError(StaticBlockEndsListEvent.Error.Execution, "<sce:append target='picked'>: the list already holds its capacity of 2"); true }) {
                return
            }

            if (try { afterAppend = com.sce.forge.runtime.SceChecked.add(afterAppend, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsListEvent.Error.Execution, "<assign location='afterAppend'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_block_ends_list.scxml:30 :: waiting :: _transition_1

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUByte()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticBlockEndsListEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
