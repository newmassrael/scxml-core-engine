// SCE-GENERATED — DO NOT EDIT
// source-hash: 19ecd19be2f9404a14b82c2f9ab7ddb05905712d4c77e74c22ff87f6e0e6ce33

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StaticCounterState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: StaticCounterState): Int = when (state) {
        is StaticCounterState.Counting -> 0
        is StaticCounterState.Done -> 1
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StaticCounterState,
        event: StaticCounterEvent
    ): TransitionResult<StaticCounterState> = when (state) {
        is StaticCounterState.Counting -> processCounting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processCounting(
        event: StaticCounterEvent
    ): TransitionResult<StaticCounterState> = when {
        event is StaticCounterEvent.Tick && count < 10.toUInt() && isStateActive("counting") -> TransitionResult.Internal(0)
        event is StaticCounterEvent.Go && ready -> TransitionResult.External(StaticCounterState.Done, StaticCounterState.Counting, 1)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_counter.scxml:13 :: _machine
    override fun onEntry(state: StaticCounterState, pathChild: StaticCounterState?) {
        when (state) {
            is StaticCounterState.Counting -> {
                // SCE-MAP: static_counter.scxml:20 :: counting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("counting")) return

            println("count: " + count)
            }
            is StaticCounterState.Done -> {
                // SCE-MAP: static_counter.scxml:32 :: done :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("done")) return
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
                activeStateIds.remove("counting")
            }
            is StaticCounterState.Done -> {
                // SCE-MAP: static_counter.scxml:32 :: done :: _state_body
                activeStateIds.remove("done")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: static_counter.scxml:13 :: _machine
    override fun executeTransitionActions(
        source: StaticCounterState,
        event: StaticCounterEvent?,
        transitionIndex: Int
    ) {
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
