// SCE-GENERATED — DO NOT EDIT
// source-hash: 1fe728b521f52adf866bb0e0fb56e6c9d8272ffd7fa321459c53679f859508c9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_params__sce_synth_invoke__worker.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:3 :: _machine

package com.sce.integration.static_invoke_params

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeParamsSceSynthInvokeWorkerState : State {
    data object Leaf : StaticInvokeParamsSceSynthInvokeWorkerState
    data object Waiting : StaticInvokeParamsSceSynthInvokeWorkerState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeParamsSceSynthInvokeWorkerEvent : Event {

}
// --- State Machine (W3C SCXML) ---

class StaticInvokeParamsSceSynthInvokeWorkerStateMachine(
) : StateMachineEngine<StaticInvokeParamsSceSynthInvokeWorkerState, StaticInvokeParamsSceSynthInvokeWorkerEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `start` datamodel variable, the machine's own. */
    private var start: UInt = 0.toUInt()
    /** W3C SCXML 5.2: the `enabled` datamodel variable, the machine's own. */
    private var enabled: Boolean = false

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var start: UInt? = null
        var enabled: Boolean? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.start?.let { start = it }
        params.enabled?.let { enabled = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticInvokeParamsSceSynthInvokeWorkerState>,
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
    val savedShape: String = "e1e1861edcac3f5bdfe86401e82341a6b0e0d3386d6b98bfe5a4df301c6dcbfe"

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
            "start" to SavedValues.of(start),
            "enabled" to SavedValues.of(enabled),
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
        val saved1 = SavedValues.uint32(saved.variable("start"), "start")
        val saved2 = SavedValues.bool(saved.variable("enabled"), "enabled")
        start = saved1
        enabled = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticInvokeParamsSceSynthInvokeWorkerState = StaticInvokeParamsSceSynthInvokeWorkerState.Waiting

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
    override fun isFinalState(state: StaticInvokeParamsSceSynthInvokeWorkerState): Boolean = when (state) {
        is StaticInvokeParamsSceSynthInvokeWorkerState.Leaf -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeParamsSceSynthInvokeWorkerState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeParamsSceSynthInvokeWorkerState, HistoryId>> =
            listOf(StateTarget(StaticInvokeParamsSceSynthInvokeWorkerState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticInvokeParamsSceSynthInvokeWorkerState, HistoryId>(
            StaticInvokeParamsSceSynthInvokeWorkerState.Waiting,
            listOf(StateTarget(StaticInvokeParamsSceSynthInvokeWorkerState.Leaf)),
            0,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeParamsSceSynthInvokeWorkerState? = when (stateId) {
        "leaf" -> StaticInvokeParamsSceSynthInvokeWorkerState.Leaf
        "waiting" -> StaticInvokeParamsSceSynthInvokeWorkerState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeParamsSceSynthInvokeWorkerState): String = when (state) {
        is StaticInvokeParamsSceSynthInvokeWorkerState.Leaf -> "leaf"
        is StaticInvokeParamsSceSynthInvokeWorkerState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeParamsSceSynthInvokeWorkerState): Int = when (state) {
        is StaticInvokeParamsSceSynthInvokeWorkerState.Leaf -> 1
        is StaticInvokeParamsSceSynthInvokeWorkerState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeParamsSceSynthInvokeWorkerEvent? = when (name) {
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    // A child SM that inherits the has_parent_communication override while
    // declaring no events of its own leaves the sealed hierarchy with zero
    // implementors, so `StaticInvokeParamsSceSynthInvokeWorkerEvent` is uninhabited: no caller can
    // construct an argument and the body is unreachable. A `when` over an
    // uninhabited sealed subject is vacuously exhaustive, so any branch —
    // `else` included — is dead code the compiler rejects under -Werror.
    // Returning the null directly is the honest expression of "unreachable".
    override fun eventNameOf(event: StaticInvokeParamsSceSynthInvokeWorkerEvent): String? = null





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeParamsSceSynthInvokeWorkerState,
        event: StaticInvokeParamsSceSynthInvokeWorkerEvent?
    ): EnabledTransition<StaticInvokeParamsSceSynthInvokeWorkerState, HistoryId>? = when (state) {
        is StaticInvokeParamsSceSynthInvokeWorkerState.Waiting -> when {
            event == null && start == 7.toUInt() && enabled -> transitionWaitingAt0
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onEntry(state: StaticInvokeParamsSceSynthInvokeWorkerState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeParamsSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:12 :: leaf :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticInvokeParamsSceSynthInvokeWorkerState.Waiting -> {
                // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:9 :: waiting :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun onExit(state: StaticInvokeParamsSceSynthInvokeWorkerState) {
        when (state) {
            is StaticInvokeParamsSceSynthInvokeWorkerState.Leaf -> {
                // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:12 :: leaf :: _state_body
            }
            is StaticInvokeParamsSceSynthInvokeWorkerState.Waiting -> {
                // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:9 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_params__sce_synth_invoke__worker.scxml:3 :: _machine
    override fun executeTransitionContent(source: StaticInvokeParamsSceSynthInvokeWorkerState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
