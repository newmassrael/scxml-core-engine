// SCE-GENERATED — DO NOT EDIT
// source-hash: 9682ba42436be01ddeb48458c1e76a0d9251bd3f706e09da9977af19a7844382

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
) : StateMachineEngine<StaticCounterState, StaticCounterEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `count` datamodel variable, published (`sce:direction="out"`). */
    var count: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `ready` datamodel variable, published (`sce:direction="out"`). */
    var ready: Boolean = false
        private set
    /** W3C SCXML 5.2: the `step` datamodel variable, the machine's own. */
    private var step: UInt = 1.toUInt()

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
        count = count,
        ready = ready,
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

    override val initialState: StaticCounterState = StaticCounterState.Counting

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
            event is StaticCounterEvent.Tick && count < 10.toUInt() && isStateActive("counting") -> transitionCountingAt0
            event is StaticCounterEvent.Go && ready -> transitionCountingAt1
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

            println("count: " + count)
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


            count = count + step


            if (count == 5.toUInt()) {


            ready = true
            } else if (count > 7.toUInt()) {


            ready = false
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
