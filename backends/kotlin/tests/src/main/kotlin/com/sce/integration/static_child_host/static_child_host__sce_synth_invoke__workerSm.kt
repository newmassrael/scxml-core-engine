// SCE-GENERATED — DO NOT EDIT
// source-hash: 7ad55f268a9fbf4c094293a60d20a17c7e9e6598a9a6fe5787c0e9f2898a7e28

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_child_host__sce_synth_invoke__worker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:3 :: _machine

package com.sce.integration.static_child_host

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticChildHostSceSynthInvokeWorkerState : State {
    data object First : StaticChildHostSceSynthInvokeWorkerState
    data object Leaf : StaticChildHostSceSynthInvokeWorkerState
    data object Second : StaticChildHostSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticChildHostSceSynthInvokeWorkerEvent : Event {
    data object A : StaticChildHostSceSynthInvokeWorkerEvent
    data object B : StaticChildHostSceSynthInvokeWorkerEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticChildHostSceSynthInvokeWorkerActions {
    fun finished(steps: UInt)
    fun started()
}

/**
 * [StaticChildHostSceSynthInvokeWorkerActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticChildHostSceSynthInvokeWorkerActions : StaticChildHostSceSynthInvokeWorkerActions {
    /** One recorded host call. */
    sealed interface Call {
        data class Finished(val steps: UInt) : Call
        data object Started : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun finished(steps: UInt) {
        recorded += Call.Finished(steps)
    }
    override fun started() {
        recorded += Call.Started
    }
}

class StaticChildHostSceSynthInvokeWorkerStateMachine(
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
    private val actions: StaticChildHostSceSynthInvokeWorkerActions,
) : StateMachineEngine<StaticChildHostSceSynthInvokeWorkerState, StaticChildHostSceSynthInvokeWorkerEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `steps` datamodel variable, the machine's own. */
    private var steps: UInt = 0.toUInt()

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var steps: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.steps?.let { steps = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticChildHostSceSynthInvokeWorkerState>,
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

    // ── SCE Accepted Subset §2.15: saving this machine, restoring it ─────────

    /**
     * The shape a saved state of this document is bound to: a state saved
     * from a document that renamed, re-typed or moved a state or a variable is
     * refused, one saved before a guard or an action changed is not.
     */
    val savedShape: String = "ab673a2340c38fa1f96d8b689fa7e99f02da1f54896b58beb82fe1768df75d4f"

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
            "steps" to SavedValues.of(steps),
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
        val saved1 = SavedValues.uint32(saved.variable("steps"), "steps")
        steps = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticChildHostSceSynthInvokeWorkerState = StaticChildHostSceSynthInvokeWorkerState.First

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

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticChildHostSceSynthInvokeWorkerState): Boolean = when (state) {
        is StaticChildHostSceSynthInvokeWorkerState.Leaf -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticChildHostSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticChildHostSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(StaticChildHostSceSynthInvokeWorkerState.First))

        // W3C SCXML 3.13: first's transition 0, as the microstep reads it.
        val transitionFirstAt0 = EnabledTransition<StaticChildHostSceSynthInvokeWorkerState, HistoryId>(
            StaticChildHostSceSynthInvokeWorkerState.First,
            listOf(StateTarget(StaticChildHostSceSynthInvokeWorkerState.Second)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: second's transition 0, as the microstep reads it.
        val transitionSecondAt0 = EnabledTransition<StaticChildHostSceSynthInvokeWorkerState, HistoryId>(
            StaticChildHostSceSynthInvokeWorkerState.Second,
            listOf(StateTarget(StaticChildHostSceSynthInvokeWorkerState.Leaf)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticChildHostSceSynthInvokeWorkerState? = when (stateId) {
        "first" -> StaticChildHostSceSynthInvokeWorkerState.First
        "leaf" -> StaticChildHostSceSynthInvokeWorkerState.Leaf
        "second" -> StaticChildHostSceSynthInvokeWorkerState.Second
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticChildHostSceSynthInvokeWorkerState): String = when (state) {
        is StaticChildHostSceSynthInvokeWorkerState.First -> "first"
        is StaticChildHostSceSynthInvokeWorkerState.Leaf -> "leaf"
        is StaticChildHostSceSynthInvokeWorkerState.Second -> "second"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticChildHostSceSynthInvokeWorkerState): Int = when (state) {
        is StaticChildHostSceSynthInvokeWorkerState.First -> 0
        is StaticChildHostSceSynthInvokeWorkerState.Leaf -> 2
        is StaticChildHostSceSynthInvokeWorkerState.Second -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticChildHostSceSynthInvokeWorkerEvent? = when (name) {
        "a" -> StaticChildHostSceSynthInvokeWorkerEvent.A
        "b" -> StaticChildHostSceSynthInvokeWorkerEvent.B
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticChildHostSceSynthInvokeWorkerEvent): String? = when (event) {
        is StaticChildHostSceSynthInvokeWorkerEvent.A -> "a"
        is StaticChildHostSceSynthInvokeWorkerEvent.B -> "b"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticChildHostSceSynthInvokeWorkerState,
        event: StaticChildHostSceSynthInvokeWorkerEvent?
    ): EnabledTransition<StaticChildHostSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is StaticChildHostSceSynthInvokeWorkerState.First -> when {
            event is StaticChildHostSceSynthInvokeWorkerEvent.A -> transitionFirstAt0
            else -> null
        }
        is StaticChildHostSceSynthInvokeWorkerState.Second -> when {
            event is StaticChildHostSceSynthInvokeWorkerEvent.B -> transitionSecondAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onEntry(state: StaticChildHostSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticChildHostSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:8 :: first :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            // W3C SCXML G.7: <sce:action name="started">
            actions.started()
                }
            }
            is StaticChildHostSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:24 :: leaf :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticChildHostSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:16 :: second :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onExit(state: StaticChildHostSceSynthInvokeWorkerState) {
        when (state) {
            is StaticChildHostSceSynthInvokeWorkerState.First -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:8 :: first :: _state_body
            }
            is StaticChildHostSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:24 :: leaf :: _state_body
            }
            is StaticChildHostSceSynthInvokeWorkerState.Second -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:16 :: second :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticChildHostSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
        is StaticChildHostSceSynthInvokeWorkerState.First -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:12 :: first :: _transition_0

            if (try { steps = com.sce.forge.runtime.SceChecked.add(steps, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }
            }
            else -> {}
        }
        is StaticChildHostSceSynthInvokeWorkerState.Second -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_child_host__sce_synth_invoke__worker.scxml:17 :: second :: _transition_0

            if (try { steps = com.sce.forge.runtime.SceChecked.add(steps, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { true }) {
                return
            }

            // W3C SCXML G.7: <sce:action name="finished">
            actions.finished(steps)
            }
            else -> {}
        }
        else -> {}
        }
    }
}
