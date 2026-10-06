// SCE-GENERATED — DO NOT EDIT
// source-hash: af6eb1cd310564a79f397036ef4dce92d3d2c3e97c92d00497068c34348ff27d

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke.scxml:22 :: _machine

package com.sce.integration.static_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeState : State {
    data object Finished : StaticInvokeState
    data object Idle : StaticInvokeState
    data object Working : StaticInvokeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeEvent : Event {
    data object A : StaticInvokeEvent
    data object Abort : StaticInvokeEvent
    data object B : StaticInvokeEvent
    sealed interface Done : StaticInvokeEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Worker : Invoke
        }
    }
    sealed interface Error : StaticInvokeEvent {
        data object Execution : Error
    }
    data object Finish : StaticInvokeEvent
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeStateMachine(
) : StateMachineEngine<StaticInvokeState, StaticInvokeEvent>() {

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
        val configuration: Set<StaticInvokeState>,
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
    override val staticInvokes: List<Pair<String, StaticInvokeState>> = listOf(
        "worker" to StaticInvokeState.Working,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeState = StaticInvokeState.Working

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
    override fun isFinalState(state: StaticInvokeState): Boolean = when (state) {
        is StaticInvokeState.Finished -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeState, HistoryId>> =
            listOf(StateTarget(StaticInvokeState.Working))

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticInvokeState, HistoryId>(
            StaticInvokeState.Working,
            emptyList(),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StaticInvokeState, HistoryId>(
            StaticInvokeState.Working,
            emptyList(),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StaticInvokeState, HistoryId>(
            StaticInvokeState.Working,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 3, as the microstep reads it.
        val transitionWorkingAt3 = EnabledTransition<StaticInvokeState, HistoryId>(
            StaticInvokeState.Working,
            listOf(StateTarget(StaticInvokeState.Finished)),
            3,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 4, as the microstep reads it.
        val transitionWorkingAt4 = EnabledTransition<StaticInvokeState, HistoryId>(
            StaticInvokeState.Working,
            listOf(StateTarget(StaticInvokeState.Idle)),
            4,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeState? = when (stateId) {
        "finished" -> StaticInvokeState.Finished
        "idle" -> StaticInvokeState.Idle
        "working" -> StaticInvokeState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeState): String = when (state) {
        is StaticInvokeState.Finished -> "finished"
        is StaticInvokeState.Idle -> "idle"
        is StaticInvokeState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeState): Int = when (state) {
        is StaticInvokeState.Finished -> 2
        is StaticInvokeState.Idle -> 1
        is StaticInvokeState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeEvent? = when (name) {
        "a" -> StaticInvokeEvent.A
        "abort" -> StaticInvokeEvent.Abort
        "b" -> StaticInvokeEvent.B
        "done.invoke" -> StaticInvokeEvent.Done.Invoke.Self
        "done.invoke.worker" -> StaticInvokeEvent.Done.Invoke.Worker
        "error.execution" -> StaticInvokeEvent.Error.Execution
        "finish" -> StaticInvokeEvent.Finish
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeEvent): String? = when (event) {
        is StaticInvokeEvent.A -> "a"
        is StaticInvokeEvent.Abort -> "abort"
        is StaticInvokeEvent.B -> "b"
        is StaticInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeEvent.Done.Invoke.Worker -> "done.invoke.worker"
        is StaticInvokeEvent.Error.Execution -> "error.execution"
        is StaticInvokeEvent.Finish -> "finish"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeState,
        event: StaticInvokeEvent?
    ): EnabledTransition<StaticInvokeState, HistoryId>? = when (state) {
        is StaticInvokeState.Working -> when {
            event is StaticInvokeEvent.A -> transitionWorkingAt0
            event is StaticInvokeEvent.B -> transitionWorkingAt1
            event is StaticInvokeEvent.Done.Invoke.Worker -> transitionWorkingAt2
            event is StaticInvokeEvent.Finish -> transitionWorkingAt3
            event is StaticInvokeEvent.Abort -> transitionWorkingAt4
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke.scxml:22 :: _machine
    override fun onEntry(state: StaticInvokeState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeState.Finished -> {
                // SCE-MAP: static_invoke.scxml:53 :: finished :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticInvokeState.Idle -> {
                // SCE-MAP: static_invoke.scxml:52 :: idle :: _state_body
            }
            is StaticInvokeState.Working -> {
                // SCE-MAP: static_invoke.scxml:27 :: working :: _state_body
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
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeState) {
        when (invokeId) {
            "worker" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "working.${System.identityHashCode(this)}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeSceSynthInvokeWorkerStateMachine()

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, true, StaticInvokeEvent.Done.Invoke.Worker, "", generatedInvokeId)
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke.scxml:22 :: _machine
    override fun onExit(state: StaticInvokeState) {
        when (state) {
            is StaticInvokeState.Finished -> {
                // SCE-MAP: static_invoke.scxml:53 :: finished :: _state_body
            }
            is StaticInvokeState.Idle -> {
                // SCE-MAP: static_invoke.scxml:52 :: idle :: _state_body
            }
            is StaticInvokeState.Working -> {
                // SCE-MAP: static_invoke.scxml:27 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke.scxml:22 :: _machine
    override fun executeTransitionContent(source: StaticInvokeState, transitionIndex: Int) {
        when (source) {
        is StaticInvokeState.Working -> when (transitionIndex) {
            2 -> {
                // SCE-MAP: static_invoke.scxml:46 :: working :: _transition_2

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
