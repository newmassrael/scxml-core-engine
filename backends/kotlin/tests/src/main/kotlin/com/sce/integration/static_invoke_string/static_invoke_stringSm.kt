// SCE-GENERATED — DO NOT EDIT
// source-hash: dd128732f0d0cc6bb147aae13f97e39edf9438b451af2c8f42fc5cd000534f68

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_string.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_string.scxml:23 :: _machine

package com.sce.integration.static_invoke_string

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeStringState : State {
    data object Running : StaticInvokeStringState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeStringEvent : Event {
    sealed interface Done : StaticInvokeStringEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Fits : Invoke
            data object Over : Invoke
            data object Wide : Invoke
        }
    }
    sealed interface Error : StaticInvokeStringEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeStringStateMachine(
) : StateMachineEngine<StaticInvokeStringState, StaticInvokeStringEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `lengthy` datamodel variable, the machine's own. */
    private var lengthy: String = "abcdefgh"
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    var completed: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `errors` datamodel variable, published (`sce:direction="out"`). */
    var errors: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var lengthy: String? = null
        var completed: UInt? = null
        var errors: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.lengthy?.let { lengthy = it }
        params.completed?.let { completed = it }
        params.errors?.let { errors = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val completed: UInt,
        val errors: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticInvokeStringState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        completed = completed,
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
    val savedShape: String = "e9a22043aaf9b92bc7cb8f09f41bafcbd0cab178ab3d26e9e39d7ab73b453106"

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
            "lengthy" to SavedValues.of(lengthy),
            "completed" to SavedValues.of(completed),
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
        val saved1 = SavedValues.string(saved.variable("lengthy"), "lengthy", 8)
        val saved2 = SavedValues.uint32(saved.variable("completed"), "completed")
        val saved3 = SavedValues.uint32(saved.variable("errors"), "errors")
        lengthy = saved1
        completed = saved2
        errors = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session, a hybrid one among
    // declared candidates, or one a declared host invoker serves.
    override val staticInvokes: List<Pair<String, StaticInvokeStringState>> = listOf(
        "fits" to StaticInvokeStringState.Running,
        "over" to StaticInvokeStringState.Running,
        "wide" to StaticInvokeStringState.Running,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeStringState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeStringState = StaticInvokeStringState.Running

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

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticInvokeStringState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeStringState, HistoryId>> =
            listOf(StateTarget(StaticInvokeStringState.Running))

        // W3C SCXML 3.13: running's transition 0, as the microstep reads it.
        val transitionRunningAt0 = EnabledTransition<StaticInvokeStringState, HistoryId>(
            StaticInvokeStringState.Running,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 1, as the microstep reads it.
        val transitionRunningAt1 = EnabledTransition<StaticInvokeStringState, HistoryId>(
            StaticInvokeStringState.Running,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 2, as the microstep reads it.
        val transitionRunningAt2 = EnabledTransition<StaticInvokeStringState, HistoryId>(
            StaticInvokeStringState.Running,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: running's transition 3, as the microstep reads it.
        val transitionRunningAt3 = EnabledTransition<StaticInvokeStringState, HistoryId>(
            StaticInvokeStringState.Running,
            emptyList(),
            3,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeStringState? = when (stateId) {
        "running" -> StaticInvokeStringState.Running
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeStringState): String = when (state) {
        is StaticInvokeStringState.Running -> "running"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeStringState): Int = when (state) {
        is StaticInvokeStringState.Running -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeStringEvent? = when (name) {
        "done.invoke" -> StaticInvokeStringEvent.Done.Invoke.Self
        "done.invoke.fits" -> StaticInvokeStringEvent.Done.Invoke.Fits
        "done.invoke.over" -> StaticInvokeStringEvent.Done.Invoke.Over
        "done.invoke.wide" -> StaticInvokeStringEvent.Done.Invoke.Wide
        "error.execution" -> StaticInvokeStringEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeStringEvent): String? = when (event) {
        is StaticInvokeStringEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeStringEvent.Done.Invoke.Fits -> "done.invoke.fits"
        is StaticInvokeStringEvent.Done.Invoke.Over -> "done.invoke.over"
        is StaticInvokeStringEvent.Done.Invoke.Wide -> "done.invoke.wide"
        is StaticInvokeStringEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeStringState,
        event: StaticInvokeStringEvent?
    ): EnabledTransition<StaticInvokeStringState, HistoryId>? = when (state) {
        is StaticInvokeStringState.Running -> when {
            event is StaticInvokeStringEvent.Done.Invoke.Fits -> transitionRunningAt0
            event is StaticInvokeStringEvent.Done.Invoke.Over -> transitionRunningAt1
            event is StaticInvokeStringEvent.Done.Invoke.Wide -> transitionRunningAt2
            event is StaticInvokeStringEvent.Error.Execution -> transitionRunningAt3
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_string.scxml:23 :: _machine
    override fun onEntry(state: StaticInvokeStringState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeStringState.Running -> {
                // SCE-MAP: static_invoke_string.scxml:30 :: running :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("fits", state)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("over", state)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("wide", state)
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
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeStringState) {
        when (invokeId) {
            "fits" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "running.${System.identityHashCode(this)}.fits"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeStringSceSynthInvokeFitsStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeStringSceSynthInvokeFitsStateMachine.InvokeParams()
                    try {
                        childSeed.title = com.sce.forge.runtime.SceChecked.bounded("wxyz", 4)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<invoke> <param name='title'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("fits", childSM, false, StaticInvokeStringEvent.Done.Invoke.Fits, "", generatedInvokeId)
                }
            }
            "over" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "running.${System.identityHashCode(this)}.over"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeStringSceSynthInvokeOverStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeStringSceSynthInvokeOverStateMachine.InvokeParams()
                    try {
                        childSeed.title = com.sce.forge.runtime.SceChecked.bounded(lengthy, 4)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<invoke> <param name='title'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("over", childSM, false, StaticInvokeStringEvent.Done.Invoke.Over, "", generatedInvokeId)
                }
            }
            "wide" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "running.${System.identityHashCode(this)}.wide"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeStringSceSynthInvokeWideStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeStringSceSynthInvokeWideStateMachine.InvokeParams()
                    try {
                        childSeed.title = com.sce.forge.runtime.SceChecked.bounded("é€", 4)
                    } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                        // W3C SCXML 5.7.1: report the failure and leave the value out.
                        raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<invoke> <param name='title'> expr failed to evaluate")
                    }
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("wide", childSM, false, StaticInvokeStringEvent.Done.Invoke.Wide, "", generatedInvokeId)
                }
            }
            else -> error("the document has no child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_string.scxml:23 :: _machine
    override fun onExit(state: StaticInvokeStringState) {
        when (state) {
            is StaticInvokeStringState.Running -> {
                // SCE-MAP: static_invoke_string.scxml:30 :: running :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("fits")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("over")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("wide")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_string.scxml:23 :: _machine
    override fun executeTransitionContent(source: StaticInvokeStringState, transitionIndex: Int) {
        when (source) {
        is StaticInvokeStringState.Running -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_string.scxml:76 :: running :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_invoke_string.scxml:79 :: running :: _transition_1

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 10.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_invoke_string.scxml:82 :: running :: _transition_2

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 100.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_invoke_string.scxml:85 :: running :: _transition_3

            if (try { errors = com.sce.forge.runtime.SceChecked.add(errors, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeStringEvent.Error.Execution, "<assign location='errors'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
