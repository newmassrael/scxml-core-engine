// SCE-GENERATED — DO NOT EDIT
// source-hash: 1f6bc95c36ab86616cc9027b13ff1ebc63474613294e4c113db9208e7d168c84

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_static_host_invoke.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_static_host_invoke.scxml:42 :: _machine

package com.sce.integration.statechart_static_host_invoke

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartStaticHostInvokeState : State {
    data object After : StatechartStaticHostInvokeState
    data object Failed : StatechartStaticHostInvokeState
    data object Idle : StatechartStaticHostInvokeState
    data object Working : StatechartStaticHostInvokeState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartStaticHostInvokeEvent : Event {
    data object Bump : StatechartStaticHostInvokeEvent
    sealed interface Done : StatechartStaticHostInvokeEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object H : Invoke
        }
    }
    sealed interface Error : StatechartStaticHostInvokeEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object H : Invoke
        }
    }
    data object Leave : StatechartStaticHostInvokeEvent
    data object Start : StatechartStaticHostInvokeEvent
}
// --- State Machine (W3C SCXML) ---

class StatechartStaticHostInvokeStateMachine(
) : StateMachineEngine<StatechartStaticHostInvokeState, StatechartStaticHostInvokeEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `job` datamodel variable, the machine's own. */
    private var job: UInt = 7.toUInt()
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var label: String = "report"
    /** W3C SCXML 5.2: the `seen` datamodel variable, published (`sce:direction="out"`). */
    var seen: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var job: UInt? = null
        var label: String? = null
        var seen: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.job?.let { job = it }
        params.label?.let { label = it }
        params.seen?.let { seen = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val seen: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StatechartStaticHostInvokeState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        seen = seen,
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
    val savedShape: String = "6c5500351260aea6df554c7d384da2c3d9c85ac7094c6f1401a93a4610aa72e4"

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
            "job" to SavedValues.of(job),
            "label" to SavedValues.of(label),
            "seen" to SavedValues.of(seen),
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
        val saved1 = SavedValues.uint32(saved.variable("job"), "job")
        val saved2 = SavedValues.string(saved.variable("label"), "label", 16)
        val saved3 = SavedValues.uint32(saved.variable("seen"), "seen")
        job = saved1
        label = saved2
        seen = saved3
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StatechartStaticHostInvokeState>> = listOf(
        Triple("x-sce-host", "h", StatechartStaticHostInvokeState.Working),
    )

    override val initialState: StatechartStaticHostInvokeState = StatechartStaticHostInvokeState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StatechartStaticHostInvokeState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartStaticHostInvokeState, HistoryId>> =
            listOf(StateTarget(StatechartStaticHostInvokeState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Idle,
            listOf(StateTarget(StatechartStaticHostInvokeState.Working)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 0, as the microstep reads it.
        val transitionWorkingAt0 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Working,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 1, as the microstep reads it.
        val transitionWorkingAt1 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Working,
            listOf(StateTarget(StatechartStaticHostInvokeState.After)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 2, as the microstep reads it.
        val transitionWorkingAt2 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Working,
            listOf(StateTarget(StatechartStaticHostInvokeState.Failed)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: working's transition 3, as the microstep reads it.
        val transitionWorkingAt3 = EnabledTransition<StatechartStaticHostInvokeState, HistoryId>(
            StatechartStaticHostInvokeState.Working,
            listOf(StateTarget(StatechartStaticHostInvokeState.Idle)),
            3,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartStaticHostInvokeState? = when (stateId) {
        "after" -> StatechartStaticHostInvokeState.After
        "failed" -> StatechartStaticHostInvokeState.Failed
        "idle" -> StatechartStaticHostInvokeState.Idle
        "working" -> StatechartStaticHostInvokeState.Working
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartStaticHostInvokeState): String = when (state) {
        is StatechartStaticHostInvokeState.After -> "after"
        is StatechartStaticHostInvokeState.Failed -> "failed"
        is StatechartStaticHostInvokeState.Idle -> "idle"
        is StatechartStaticHostInvokeState.Working -> "working"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartStaticHostInvokeState): Int = when (state) {
        is StatechartStaticHostInvokeState.After -> 2
        is StatechartStaticHostInvokeState.Failed -> 3
        is StatechartStaticHostInvokeState.Idle -> 0
        is StatechartStaticHostInvokeState.Working -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartStaticHostInvokeEvent? = when (name) {
        "bump" -> StatechartStaticHostInvokeEvent.Bump
        "done.invoke" -> StatechartStaticHostInvokeEvent.Done.Invoke.Self
        "done.invoke.h" -> StatechartStaticHostInvokeEvent.Done.Invoke.H
        "error.execution" -> StatechartStaticHostInvokeEvent.Error.Execution
        "error.invoke" -> StatechartStaticHostInvokeEvent.Error.Invoke.Self
        "error.invoke.h" -> StatechartStaticHostInvokeEvent.Error.Invoke.H
        "leave" -> StatechartStaticHostInvokeEvent.Leave
        "start" -> StatechartStaticHostInvokeEvent.Start
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartStaticHostInvokeEvent): String? = when (event) {
        is StatechartStaticHostInvokeEvent.Bump -> "bump"
        is StatechartStaticHostInvokeEvent.Done.Invoke.Self -> "done.invoke"
        is StatechartStaticHostInvokeEvent.Done.Invoke.H -> "done.invoke.h"
        is StatechartStaticHostInvokeEvent.Error.Execution -> "error.execution"
        is StatechartStaticHostInvokeEvent.Error.Invoke.Self -> "error.invoke"
        is StatechartStaticHostInvokeEvent.Error.Invoke.H -> "error.invoke.h"
        is StatechartStaticHostInvokeEvent.Leave -> "leave"
        is StatechartStaticHostInvokeEvent.Start -> "start"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "h",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartStaticHostInvokeState,
        event: StatechartStaticHostInvokeEvent?
    ): EnabledTransition<StatechartStaticHostInvokeState, HistoryId>? = when (state) {
        is StatechartStaticHostInvokeState.Idle -> when {
            event is StatechartStaticHostInvokeEvent.Bump -> transitionIdleAt0
            event is StatechartStaticHostInvokeEvent.Start -> transitionIdleAt1
            else -> null
        }
        is StatechartStaticHostInvokeState.Working -> when {
            event is StatechartStaticHostInvokeEvent.Bump -> transitionWorkingAt0
            event is StatechartStaticHostInvokeEvent.Done.Invoke.H -> transitionWorkingAt1
            event is StatechartStaticHostInvokeEvent.Error.Invoke.H -> transitionWorkingAt2
            event is StatechartStaticHostInvokeEvent.Leave -> transitionWorkingAt3
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_static_host_invoke.scxml:42 :: _machine
    override fun onEntry(state: StatechartStaticHostInvokeState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartStaticHostInvokeState.After -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:77 :: after :: _state_body
            }
            is StatechartStaticHostInvokeState.Failed -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:78 :: failed :: _state_body
            }
            is StatechartStaticHostInvokeState.Idle -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:51 :: idle :: _state_body
            }
            is StatechartStaticHostInvokeState.Working -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:58 :: working :: _state_body
                // W3C SCXML 6.4.1: the host declared this `type`, so the
                // deferred closure STARTS the invocation rather than refusing
                // it. Deferred like its sibling so §scxml-6.4 ordering holds —
                // an invoke runs at macrostep end — and so a state that exits
                // first never starts it at all.
                //
                // The id handed to the host is the DOCUMENT's, not the
                // per-instance one the pending queue carries:
                // `done.invoke.<id>` is the name the author wrote a transition
                // for, so it is the name the host must answer on.
                run {
                    val generatedInvokeId = "working.${System.identityHashCode(this)}.h"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        run {
                            val v: Any? = (5000).toLong()
                            hostInvokeParams["_sce_deadline_ms"] =
                                (hostInvokeParams["_sce_deadline_ms"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "_sce_deadline_ms", v)
                        }
                        run {
                            val v: Any? = label
                            hostInvokeParams["label"] =
                                (hostInvokeParams["label"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "label", v)
                        }
                        run {
                            val v: Any? = (job).toLong()
                            hostInvokeParams["job"] =
                                (hostInvokeParams["job"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "job", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "x-sce-host",
                                invokeId = "h",
                                src = "job://report",
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = "payload"                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartStaticHostInvokeEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_static_host_invoke.scxml:42 :: _machine
    override fun onExit(state: StatechartStaticHostInvokeState) {
        when (state) {
            is StatechartStaticHostInvokeState.After -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:77 :: after :: _state_body
            }
            is StatechartStaticHostInvokeState.Failed -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:78 :: failed :: _state_body
            }
            is StatechartStaticHostInvokeState.Idle -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:51 :: idle :: _state_body
            }
            is StatechartStaticHostInvokeState.Working -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:58 :: working :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("x-sce-host", "h")
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_static_host_invoke.scxml:42 :: _machine
    override fun executeTransitionContent(source: StatechartStaticHostInvokeState, transitionIndex: Int) {
        when (source) {
        is StatechartStaticHostInvokeState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:52 :: idle :: _transition_0

            if (try { job = com.sce.forge.runtime.SceChecked.add(job, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostInvokeEvent.Error.Execution, "<assign location='job'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StatechartStaticHostInvokeState.Working -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:65 :: working :: _transition_0

            if (try { job = com.sce.forge.runtime.SceChecked.add(job, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostInvokeEvent.Error.Execution, "<assign location='job'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:68 :: working :: _transition_1

            if (try { seen = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(seen, 10.toUInt()), 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostInvokeEvent.Error.Execution, "<assign location='seen'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: statechart_static_host_invoke.scxml:71 :: working :: _transition_2

            if (try { seen = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(seen, 10.toUInt()), 2.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticHostInvokeEvent.Error.Execution, "<assign location='seen'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
