// SCE-GENERATED — DO NOT EDIT
// source-hash: 1b119c0255d1c36e11019d5921d0f6feae4c282ce26237deacc1273630d484c2

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_overflow.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_overflow.scxml:15 :: _machine

package com.sce.integration.static_overflow

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticOverflowState : State {
    data object Probed : StaticOverflowState
    data object Waiting : StaticOverflowState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticOverflowEvent : Event {
    sealed interface Error : StaticOverflowEvent {
        data object Execution : Error
    }
    data object Probe : StaticOverflowEvent
    data object Up : StaticOverflowEvent
}
// --- State Machine (W3C SCXML) ---

class StaticOverflowStateMachine(
) : StateMachineEngine<StaticOverflowState, StaticOverflowEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `level` datamodel variable, published (`sce:direction="out"`). */
    var level: UByte = 250.toUByte()
        private set
    /** W3C SCXML 5.2: the `refusals` datamodel variable, published (`sce:direction="out"`). */
    var refusals: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val level: UByte,
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
        val configuration: Set<StaticOverflowState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        level = level,
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
    val savedShape: String = "36f152960236f7d9e2f8944e15cfae5bbe7d8b406bb28cf433b9deb7f761512e"

    /**
     * This machine's whole state at the macrostep boundary it stands at —
     * every variable, the machine's own included, and where it stands — as
     * the `sce-saved-state` document every backend reads ([SavedState.toJson]).
     *
     * @throws StateRefusal for a machine that is not running, or whose last
     *   macrostep stopped at the microstep ceiling.
     */
    fun save(): SavedState = savedState(
        savedShape,
        linkedMapOf(
            "level" to SavedValues.of(level),
            "refusals" to SavedValues.of(refusals),
        ),
    )

    /**
     * Stand this machine where [saved] left one, in place of [initialize]: no
     * `<onentry>` runs and no `<data>` is evaluated, since the saved run
     * already did both. Every value is read before any is written, so a
     * refused restore leaves the machine as it was.
     *
     * @throws StateRefusal for a machine that has already started, a state
     *   saved from a document of another shape, a configuration that is not
     *   one of this document, or a value its variable's type cannot hold.
     */
    fun restore(saved: SavedState) {
        beginRestore(saved, savedShape)
        val saved1 = SavedValues.uint8(saved.variable("level"), "level")
        val saved2 = SavedValues.uint32(saved.variable("refusals"), "refusals")
        level = saved1
        refusals = saved2
        enterSaved(saved)
    }

    override val initialState: StaticOverflowState = StaticOverflowState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticOverflowState): Boolean = when (state) {
        is StaticOverflowState.Probed -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticOverflowState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticOverflowState, HistoryId>> =
            listOf(StateTarget(StaticOverflowState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticOverflowState, HistoryId>(
            StaticOverflowState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = true,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticOverflowState, HistoryId>(
            StaticOverflowState.Waiting,
            listOf(StateTarget(StaticOverflowState.Probed)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StaticOverflowState, HistoryId>(
            StaticOverflowState.Waiting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = true,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticOverflowState? = when (stateId) {
        "probed" -> StaticOverflowState.Probed
        "waiting" -> StaticOverflowState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticOverflowState): String = when (state) {
        is StaticOverflowState.Probed -> "probed"
        is StaticOverflowState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticOverflowState): Int = when (state) {
        is StaticOverflowState.Probed -> 1
        is StaticOverflowState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticOverflowEvent? = when (name) {
        "error.execution" -> StaticOverflowEvent.Error.Execution
        "probe" -> StaticOverflowEvent.Probe
        "up" -> StaticOverflowEvent.Up
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticOverflowEvent): String? = when (event) {
        is StaticOverflowEvent.Error.Execution -> "error.execution"
        is StaticOverflowEvent.Probe -> "probe"
        is StaticOverflowEvent.Up -> "up"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticOverflowState,
        event: StaticOverflowEvent?
    ): EnabledTransition<StaticOverflowState, HistoryId>? = when (state) {
        is StaticOverflowState.Waiting -> when {
            event is StaticOverflowEvent.Up -> transitionWaitingAt0
            event is StaticOverflowEvent.Probe && (try { com.sce.forge.runtime.SceChecked.add(level, 10.toUByte()) > 0.toUByte() } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOverflowEvent.Error.Execution, "<transition cond='level + 10 > 0'>: an integer operation overflowed or failed"); false }) -> transitionWaitingAt1
            event is StaticOverflowEvent.Error.Execution -> transitionWaitingAt2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_overflow.scxml:15 :: _machine
    override fun onEntry(state: StaticOverflowState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticOverflowState.Probed -> {
                // SCE-MAP: static_overflow.scxml:30 :: probed :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticOverflowState.Waiting -> {
                // SCE-MAP: static_overflow.scxml:21 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_overflow.scxml:15 :: _machine
    override fun onExit(state: StaticOverflowState) {
        when (state) {
            is StaticOverflowState.Probed -> {
                // SCE-MAP: static_overflow.scxml:30 :: probed :: _state_body
            }
            is StaticOverflowState.Waiting -> {
                // SCE-MAP: static_overflow.scxml:21 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_overflow.scxml:15 :: _machine
    override fun executeTransitionContent(source: StaticOverflowState, transitionIndex: Int) {
        when (source) {
        is StaticOverflowState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_overflow.scxml:22 :: waiting :: _transition_0

            try { level = com.sce.forge.runtime.SceChecked.add(level, 3.toUByte()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOverflowEvent.Error.Execution, "<assign location='level'>: an integer operation overflowed or failed") }
            }
            2 -> {
                // SCE-MAP: static_overflow.scxml:26 :: waiting :: _transition_2

            try { refusals = com.sce.forge.runtime.SceChecked.add(refusals, 1.toUInt()) } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticOverflowEvent.Error.Execution, "<assign location='refusals'>: an integer operation overflowed or failed") }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
