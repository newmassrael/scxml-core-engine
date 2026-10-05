// SCE-GENERATED — DO NOT EDIT
// source-hash: 860af7978d3b862216839cf8b169c14ab2f1b057aa2d784af873fa0958afaca8

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_invoke_params.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_invoke_params.scxml:33 :: _machine

package com.sce.integration.static_invoke_params

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticInvokeParamsState : State {
    data object Plain : StaticInvokeParamsState
    data object Working : StaticInvokeParamsState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticInvokeParamsEvent : Event {
    data object Bump : StaticInvokeParamsEvent
    sealed interface Done : StaticInvokeParamsEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Control : Invoke
            data object Watcher : Invoke
            data object Worker : Invoke
        }
    }
    sealed interface Error : StaticInvokeParamsEvent {
        data object Execution : Error
    }
}
// --- State Machine (W3C SCXML) ---

class StaticInvokeParamsStateMachine(
) : StateMachineEngine<StaticInvokeParamsState, StaticInvokeParamsEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `base` datamodel variable, the machine's own. */
    private var base: UInt = 4.toUInt()
    /** W3C SCXML 5.2: the `enabled` datamodel variable, the machine's own. */
    private var enabled: Boolean = true
    /** W3C SCXML 5.2: the `completed` datamodel variable, published (`sce:direction="out"`). */
    var completed: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var base: UInt? = null
        var enabled: Boolean? = null
        var completed: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.base?.let { base = it }
        params.enabled?.let { enabled = it }
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
        val configuration: Set<StaticInvokeParamsState>,
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
    val savedShape: String = "f9ab3ba81430cdbce450cf0b6fad5b6cee7263ce0ffd9390d2bba6fd2bba3847"

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
            "base" to SavedValues.of(base),
            "enabled" to SavedValues.of(enabled),
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
        val saved1 = SavedValues.uint32(saved.variable("base"), "base")
        val saved2 = SavedValues.bool(saved.variable("enabled"), "enabled")
        val saved3 = SavedValues.uint32(saved.variable("completed"), "completed")
        base = saved1
        enabled = saved2
        completed = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4: a saved state names the `<invoke>`s whose child is running,
    // and a restore starts each again from its beginning. This document saves
    // only when every invoke is a static child session.
    override val staticInvokes: List<Pair<String, StaticInvokeParamsState>> = listOf(
        "worker" to StaticInvokeParamsState.Working,
        "control" to StaticInvokeParamsState.Plain,
        "watcher" to StaticInvokeParamsState.Plain,
    )

    override fun restartInvoke(invokeId: String, state: StaticInvokeParamsState) = deferStaticInvoke(invokeId, state)

    override val initialState: StaticInvokeParamsState = StaticInvokeParamsState.Working

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
    override val documentInitialTargets: List<EntryTarget<StaticInvokeParamsState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticInvokeParamsState, HistoryId>> =
            listOf(StateTarget(StaticInvokeParamsState.Working))

        // W3C SCXML 3.13: plain's transition 0, as the microstep reads it.
        val transitionPlainAt0 = EnabledTransition<StaticInvokeParamsState, HistoryId>(
            StaticInvokeParamsState.Plain,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: plain's transition 1, as the microstep reads it.
        val transitionPlainAt1 = EnabledTransition<StaticInvokeParamsState, HistoryId>(
            StaticInvokeParamsState.Plain,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: plain's transition 2, as the microstep reads it.
        val transitionPlainAt2 = EnabledTransition<StaticInvokeParamsState, HistoryId>(
            StaticInvokeParamsState.Plain,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StaticInvokeParamsState, HistoryId>(
            StaticInvokeParamsState.Working,
            listOf(StateTarget(StaticInvokeParamsState.Plain)),
            0,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticInvokeParamsState? = when (stateId) {
        "plain" -> StaticInvokeParamsState.Plain
        "working" -> StaticInvokeParamsState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticInvokeParamsState): String = when (state) {
        is StaticInvokeParamsState.Plain -> "plain"
        is StaticInvokeParamsState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticInvokeParamsState): Int = when (state) {
        is StaticInvokeParamsState.Plain -> 1
        is StaticInvokeParamsState.Working -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticInvokeParamsEvent? = when (name) {
        "bump" -> StaticInvokeParamsEvent.Bump
        "done.invoke" -> StaticInvokeParamsEvent.Done.Invoke.Self
        "done.invoke.control" -> StaticInvokeParamsEvent.Done.Invoke.Control
        "done.invoke.watcher" -> StaticInvokeParamsEvent.Done.Invoke.Watcher
        "done.invoke.worker" -> StaticInvokeParamsEvent.Done.Invoke.Worker
        "error.execution" -> StaticInvokeParamsEvent.Error.Execution
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticInvokeParamsEvent): String? = when (event) {
        is StaticInvokeParamsEvent.Bump -> "bump"
        is StaticInvokeParamsEvent.Done.Invoke.Self -> "done.invoke"
        is StaticInvokeParamsEvent.Done.Invoke.Control -> "done.invoke.control"
        is StaticInvokeParamsEvent.Done.Invoke.Watcher -> "done.invoke.watcher"
        is StaticInvokeParamsEvent.Done.Invoke.Worker -> "done.invoke.worker"
        is StaticInvokeParamsEvent.Error.Execution -> "error.execution"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticInvokeParamsState,
        event: StaticInvokeParamsEvent?
    ): EnabledTransition<StaticInvokeParamsState, HistoryId>? = when (state) {
        is StaticInvokeParamsState.Plain -> when {
            event is StaticInvokeParamsEvent.Done.Invoke.Control -> transitionPlainAt0
            event is StaticInvokeParamsEvent.Bump -> transitionPlainAt1
            event is StaticInvokeParamsEvent.Done.Invoke.Watcher -> transitionPlainAt2
            else -> null
        }
        is StaticInvokeParamsState.Working -> when {
            event is StaticInvokeParamsEvent.Done.Invoke.Worker -> transitionWorkingAt0
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_invoke_params.scxml:33 :: _machine
    override fun onEntry(state: StaticInvokeParamsState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticInvokeParamsState.Plain -> {
                // SCE-MAP: static_invoke_params.scxml:64 :: plain :: _state_body
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("control", state)
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("watcher", state)
            }
            is StaticInvokeParamsState.Working -> {
                // SCE-MAP: static_invoke_params.scxml:40 :: working :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {

            if (try { base = com.sce.forge.runtime.SceChecked.add(base, 3.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeParamsEvent.Error.Execution, "<assign location='base'>: an integer operation overflowed or failed"); true }) {
                return@run
            }
                }
                // W3C SCXML 6.4: Defer invoked child state machine until macrostep end
                deferStaticInvoke("worker", state)
            }
        }
    }

    // W3C SCXML 6.4: defer the start of the static child session of the
    // `<invoke type="scxml">` `invokeId`, held by `state`, to the macrostep's
    // end. One body for the two things that start a child: entering the state
    // (`onEntry` above) and a restore, which starts again each running
    // invocation a saved state lists (`restartInvoke`). The child is not saved,
    // so what a restore needs is exactly what entering the state does, and two
    // spellings of it would be two places a change to one is forgotten in the
    // other. A state that exits before the macrostep ends cancels the entry
    // (`cancelPendingInvokesForState`) in either case.
    private fun deferStaticInvoke(invokeId: String, state: StaticInvokeParamsState) {
        when (invokeId) {
            "control" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "plain.${System.identityHashCode(this)}.control"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeParamsSceSynthInvokeControlStateMachine()

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("control", childSM, false, StaticInvokeParamsEvent.Done.Invoke.Control, "", generatedInvokeId)
                }
            }
            "watcher" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "plain.${System.identityHashCode(this)}.watcher"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeParamsSceSynthInvokeWatcherStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeParamsSceSynthInvokeWatcherStateMachine.InvokeParams()
                    childSeed.start = base
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("watcher", childSM, false, StaticInvokeParamsEvent.Done.Invoke.Watcher, "", generatedInvokeId)
                }
            }
            "worker" -> run {
                // W3C SCXML 3.12.1: Generate invoke ID in "stateid.platformid.index" format
                val generatedInvokeId = "working.${System.identityHashCode(this)}.worker"
                deferInvoke(state, generatedInvokeId) {

                    val childSM = StaticInvokeParamsSceSynthInvokeWorkerStateMachine()
                    // W3C SCXML 6.4.1: hand the child the values this invoke names
                    val childSeed = StaticInvokeParamsSceSynthInvokeWorkerStateMachine.InvokeParams()
                    childSeed.start = base
                    childSeed.enabled = enabled
                    childSM.acceptParams(childSeed)

                    // W3C SCXML 6.4: Static ID for done.invoke/cancel, generated ID for child events
                    startInvoke("worker", childSM, false, StaticInvokeParamsEvent.Done.Invoke.Worker, "", generatedInvokeId)
                }
            }
            else -> error("the document has no static child session '$invokeId' (codegen invariant)")
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: static_invoke_params.scxml:33 :: _machine
    override fun onExit(state: StaticInvokeParamsState) {
        when (state) {
            is StaticInvokeParamsState.Plain -> {
                // SCE-MAP: static_invoke_params.scxml:64 :: plain :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("control")
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("watcher")
            }
            is StaticInvokeParamsState.Working -> {
                // SCE-MAP: static_invoke_params.scxml:40 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: Cancel active invoked child on state exit
                cancelInvoke("worker")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_invoke_params.scxml:33 :: _machine
    override fun executeTransitionContent(source: StaticInvokeParamsState, transitionIndex: Int) {
        when (source) {
        is StaticInvokeParamsState.Plain -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_params.scxml:80 :: plain :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 100.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeParamsEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_invoke_params.scxml:98 :: plain :: _transition_1

            if (try { base = com.sce.forge.runtime.SceChecked.add(base, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeParamsEvent.Error.Execution, "<assign location='base'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_invoke_params.scxml:101 :: plain :: _transition_2

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 10.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeParamsEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StaticInvokeParamsState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_invoke_params.scxml:60 :: working :: _transition_0

            if (try { completed = com.sce.forge.runtime.SceChecked.add(completed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticInvokeParamsEvent.Error.Execution, "<assign location='completed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        }
    }
}
