// SCE-GENERATED — DO NOT EDIT
// source-hash: e3981fcd9cff1f8f15c89f3192467f522d2adc9208df092cb08ac7ed74e348c9

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_child_host.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_child_host.scxml:24 :: _machine

package com.sce.integration.static_child_host

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticChildHostState : State {
    data object Finished : StaticChildHostState
    data object Idle : StaticChildHostState
    data object Working : StaticChildHostState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticChildHostEvent : Event {
    data object A : StaticChildHostEvent
    data object Again : StaticChildHostEvent
    data object B : StaticChildHostEvent
    data object Back : StaticChildHostEvent
    sealed interface Done : StaticChildHostEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Worker : Invoke
        }
    }
    sealed interface Error : StaticChildHostEvent {
        data object Execution : Error
    }
    data object Finish : StaticChildHostEvent
}
// --- State Machine (W3C SCXML) ---

// ── W3C SCXML G.7: `<sce:action>` host dispatch ───────────────────────
/**
 * W3C SCXML G.7: host operations dispatched by `<sce:action>`.
 * The host supplies the side effects while the statechart keeps each
 * operation symbolic. No runtime script engine is involved.
 */
interface StaticChildHostActions {
    fun actionsForWorker(): StaticChildHostSceSynthInvokeWorkerActions
}

/**
 * [StaticChildHostActions] that performs nothing and records every call in order —
 * the host a test drives the machine with. Read [calls] after the machine
 * has run; each call is compared by value.
 */
class RecordingStaticChildHostActions(
    /** Answers the host [StaticChildHostSceSynthInvokeWorkerActions] is asked for; the test keeps what it answers. */
    private val actionsForWorkerSource: () -> StaticChildHostSceSynthInvokeWorkerActions,
) : StaticChildHostActions {
    /** One recorded host call. */
    sealed interface Call {
        data object ActionsForWorker : Call
    }

    private val recorded = mutableListOf<Call>()

    /** Every call so far, oldest first. */
    val calls: List<Call>
        get() = recorded.toList()

    /** Forget the calls recorded so far. */
    fun clear() {
        recorded.clear()
    }

    override fun actionsForWorker(): StaticChildHostSceSynthInvokeWorkerActions {
        recorded += Call.ActionsForWorker
        return actionsForWorkerSource()
    }
}

class StaticChildHostStateMachine(
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
    private val actions: StaticChildHostActions,
) : StateMachineEngine<StaticChildHostState, StaticChildHostEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    var completed: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var completed: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.completed?.let { completed = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val completed: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticChildHostState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        completed = completed,
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
    val savedShape: String = "6065267cb1641307cbbf9f1bbd2c34f384a0b55362be9f25c4c9d54a5f2cf8e2"

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
            "completed" to SavedValues.of(completed),
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
        val saved1 = SavedValues.uint32(saved.variable("completed"), "completed")
        completed = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticChildHostState>> = listOf(
        "worker" to StaticChildHostState.Working,
    )

    override fun restartInvoke(invokeId: String, state: StaticChildHostState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticChildHostState = StaticChildHostState.Working

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // The generate manifest's `needs_parent`: what a root-start policy reads.
    override val needsParent: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StaticChildHostState): Boolean = when (state) {
        is StaticChildHostState.Finished -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticChildHostState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticChildHostState, HistoryId>> =
            listOf(StateTarget(StaticChildHostState.Working))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Idle,
            listOf(StateTarget(StaticChildHostState.Working)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Working,
            emptyList(),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Working,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Working,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 3, as the microstep reads it.
        val transitionWorkingAt3 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Working,
            listOf(StateTarget(StaticChildHostState.Idle)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 4, as the microstep reads it.
        val transitionWorkingAt4 = EnabledTransition<StaticChildHostState, HistoryId>(
            StaticChildHostState.Working,
            listOf(StateTarget(StaticChildHostState.Finished)),
            4,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticChildHostState? = when (stateId) {
        "finished" -> StaticChildHostState.Finished
        "idle" -> StaticChildHostState.Idle
        "working" -> StaticChildHostState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticChildHostState): String = when (state) {
        is StaticChildHostState.Finished -> "finished"
        is StaticChildHostState.Idle -> "idle"
        is StaticChildHostState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticChildHostState): Int = when (state) {
        is StaticChildHostState.Finished -> 2
        is StaticChildHostState.Idle -> 1
        is StaticChildHostState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticChildHostEvent? = when (name) {
        "a" -> StaticChildHostEvent.A
        "again" -> StaticChildHostEvent.Again
        "b" -> StaticChildHostEvent.B
        "back" -> StaticChildHostEvent.Back
        "done.invoke" -> StaticChildHostEvent.Done.Invoke.Self
        "done.invoke.worker" -> StaticChildHostEvent.Done.Invoke.Worker
        "error.execution" -> StaticChildHostEvent.Error.Execution
        "finish" -> StaticChildHostEvent.Finish
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticChildHostEvent): String? = when (event) {
        is StaticChildHostEvent.A -> "a"
        is StaticChildHostEvent.Again -> "again"
        is StaticChildHostEvent.B -> "b"
        is StaticChildHostEvent.Back -> "back"
        is StaticChildHostEvent.Done.Invoke.Self -> "done.invoke"
        is StaticChildHostEvent.Done.Invoke.Worker -> "done.invoke.worker"
        is StaticChildHostEvent.Error.Execution -> "error.execution"
        is StaticChildHostEvent.Finish -> "finish"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticChildHostState,
        event: StaticChildHostEvent?
    ): EnabledTransition<StaticChildHostState, HistoryId>? = when (state) {
        is StaticChildHostState.Idle -> when {
            event is StaticChildHostEvent.Back -> transitionIdleAt0
            else -> null
        }
        is StaticChildHostState.Working -> when {
            event is StaticChildHostEvent.A -> transitionWorkingAt0
            event is StaticChildHostEvent.B -> transitionWorkingAt1
            event is StaticChildHostEvent.Done.Invoke.Worker -> transitionWorkingAt2
            event is StaticChildHostEvent.Again -> transitionWorkingAt3
            event is StaticChildHostEvent.Finish -> transitionWorkingAt4
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_child_host.scxml:24 :: _machine
    override fun onEntry(state: StaticChildHostState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticChildHostState.Finished -> {
                // SCE-MAP: static_child_host.scxml:71 :: finished :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticChildHostState.Idle -> {
                // SCE-MAP: static_child_host.scxml:68 :: idle :: _state_body
            }
            is StaticChildHostState.Working -> {
                // SCE-MAP: static_child_host.scxml:29 :: working :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("worker", state)
            }
        }
    }

    // W3C SCXML 6.4: defer the start of the child session of the
    // `<invoke type="scxml">` (a static child, or a hybrid one that names its
    // child by `srcexpr`) `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    private fun deferStaticInvoke(invokeId: String, state: StaticChildHostState) {
        when (invokeId) {
            "worker" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "working.${System.identityHashCode(this)}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticChildHostSceSynthInvokeWorkerStateMachine(actions.actionsForWorker())

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, true, StaticChildHostEvent.Done.Invoke.Worker, "", generatedInvokeId)
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_child_host.scxml:24 :: _machine
    override fun onExit(state: StaticChildHostState) {
        when (state) {
            is StaticChildHostState.Finished -> {
                // SCE-MAP: static_child_host.scxml:71 :: finished :: _state_body
            }
            is StaticChildHostState.Idle -> {
                // SCE-MAP: static_child_host.scxml:68 :: idle :: _state_body
            }
            is StaticChildHostState.Working -> {
                // SCE-MAP: static_child_host.scxml:29 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_child_host.scxml:24 :: _machine
    override fun executeTransitionContent(source: StaticChildHostState, transitionIndex: Int) {
        when (source) {
        is StaticChildHostState.Working -> when (transitionIndex) {
            2 -> {
                // SCE-MAP: static_child_host.scxml:62 :: working :: _transition_2

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticChildHostEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
