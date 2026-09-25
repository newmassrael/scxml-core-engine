// SCE-GENERATED — DO NOT EDIT
// source-hash: 9682ba42436be01ddeb48458c1e76a0d9251bd3f706e09da9977af19a7844382

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_host_call.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_host_call.scxml:13 :: _machine

package com.sce.integration.static_host_call

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticHostCallState : State {
    data object Idle : StaticHostCallState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticHostCallEvent : Event {
    data object Retry : StaticHostCallEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticHostCallActions {
    fun showAttempts(count: UInt, exhausted: Boolean)
}

/**
 * [StaticHostCallActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticHostCallActions : StaticHostCallActions {
    /** One recorded host call. */
    sealed interface Call {
        data class ShowAttempts(val count: UInt, val exhausted: Boolean) : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun showAttempts(count: UInt, exhausted: Boolean) {
        recorded += Call.ShowAttempts(count, exhausted)
    }
}

class StaticHostCallStateMachine(
    /**
     * W3C SCXML G.7: the host implementation every `<sce:action>` in this
     * document calls directly (`actions.<op>(…)`) instead of the script
     * engine.
     *
     * A constructor parameter rather than a setter, because the initial
     * state's `<onentry>` can already perform an act — a host installed
     * afterwards would arrive one act too late. It leads the parameter list
     * so it stays in the same position whether or not this machine also
     * takes a script engine.
     */
    private val actions: StaticHostCallActions,
) : StateMachineEngine<StaticHostCallState, StaticHostCallEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `attempts` datamodel variable, the machine's own. */
    private var attempts: UInt = 0.toUInt()

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticHostCallState>,
        val truncated: Boolean,
    )

    private val _snapshot = kotlinx.coroutines.flow.MutableStateFlow(
        Snapshot(emptySet(), false)
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
        _snapshot.value = Snapshot(activeConfiguration, truncated)
    }

    override val initialState: StaticHostCallState = StaticHostCallState.Idle

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticHostCallState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticHostCallState, HistoryId>> =
            listOf(StateTarget(StaticHostCallState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticHostCallState, HistoryId>(
            StaticHostCallState.Idle,
            listOf(StateTarget(StaticHostCallState.Idle)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticHostCallState? = when (stateId) {
        "idle" -> StaticHostCallState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticHostCallState): String = when (state) {
        is StaticHostCallState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticHostCallState): Int = when (state) {
        is StaticHostCallState.Idle -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticHostCallState,
        event: StaticHostCallEvent?
    ): EnabledTransition<StaticHostCallState, HistoryId>? = when (state) {
        is StaticHostCallState.Idle -> when {
            event is StaticHostCallEvent.Retry && attempts < 3.toUInt() -> transitionIdleAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_host_call.scxml:13 :: _machine
    override fun onEntry(state: StaticHostCallState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticHostCallState.Idle -> {
                // SCE-MAP: static_host_call.scxml:18 :: idle :: _state_body

            // W3C SCXML G.7: <sce:action name="showAttempts">
            actions.showAttempts(attempts, attempts >= 3.toUInt())
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_host_call.scxml:13 :: _machine
    override fun onExit(state: StaticHostCallState) {
        when (state) {
            is StaticHostCallState.Idle -> {
                // SCE-MAP: static_host_call.scxml:18 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_host_call.scxml:13 :: _machine
    override fun executeTransitionContent(source: StaticHostCallState, transitionIndex: Int) {
        when (source) {
        is StaticHostCallState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_host_call.scxml:25 :: idle :: _transition_0


            attempts = attempts + 1.toUInt()
            }
            else -> {}
        }
        }
    }
}
