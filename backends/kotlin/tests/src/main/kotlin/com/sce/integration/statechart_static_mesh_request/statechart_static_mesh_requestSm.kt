// SCE-GENERATED — DO NOT EDIT
// source-hash: 223286088e3261c9c6ae4869649bfba8df4425fada9f5fea9c4df27a3c6d833b

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_static_mesh_request.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_static_mesh_request.scxml:45 :: _machine

package com.sce.integration.statechart_static_mesh_request

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartStaticMeshRequestState : State {
    data object Asking : StatechartStaticMeshRequestState
    data object Done : StatechartStaticMeshRequestState
    data object Idle : StatechartStaticMeshRequestState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartStaticMeshRequestEvent : Event {
    data object Bump : StatechartStaticMeshRequestEvent
    sealed interface Done : StatechartStaticMeshRequestEvent {
        sealed interface Invoke : Done {
            data object Self : Invoke
            data object Ask : Invoke
        }
    }
    sealed interface Error : StatechartStaticMeshRequestEvent {
        data object Execution : Error
        sealed interface Invoke : Error {
            data object Self : Invoke
            data object Ask : Invoke
        }
    }
    data object Ghost : StatechartStaticMeshRequestEvent
    data object Go : StatechartStaticMeshRequestEvent
}
// --- State Machine (W3C SCXML) ---

class StatechartStaticMeshRequestStateMachine(
) : StateMachineEngine<StatechartStaticMeshRequestState, StatechartStaticMeshRequestEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `load` datamodel variable, the machine's own. */
    private var load: UInt = 3.toUInt()
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var label: String = "idle"
    /** W3C SCXML 5.2: the `peer` datamodel variable, the machine's own. */
    private var peer: String = "#motor"
    /** W3C SCXML 5.2: the `answered` datamodel variable, published (`sce:direction="out"`). */
    var answered: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `failed` datamodel variable, published (`sce:direction="out"`). */
    var failed: UInt = 0.toUInt()
        private set
    /** W3C SCXML 5.2: the `refused` datamodel variable, published (`sce:direction="out"`). */
    var refused: UInt = 0.toUInt()
        private set

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var load: UInt? = null
        var label: String? = null
        var peer: String? = null
        var answered: UInt? = null
        var failed: UInt? = null
        var refused: UInt? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.load?.let { load = it }
        params.label?.let { label = it }
        params.peer?.let { peer = it }
        params.answered?.let { answered = it }
        params.failed?.let { failed = it }
        params.refused?.let { refused = it }
    }

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val answered: UInt,
        val failed: UInt,
        val refused: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StatechartStaticMeshRequestState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        answered = answered,
        failed = failed,
        refused = refused,
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
    val savedShape: String = "d4fc8b35c38b20ad10aa11be11d5f07569eb9d8721c7120f1a810d27608af7db"

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
            "load" to SavedValues.of(load),
            "label" to SavedValues.of(label),
            "peer" to SavedValues.of(peer),
            "answered" to SavedValues.of(answered),
            "failed" to SavedValues.of(failed),
            "refused" to SavedValues.of(refused),
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
        val saved1 = SavedValues.uint32(saved.variable("load"), "load")
        val saved2 = SavedValues.string(saved.variable("label"), "label", 16)
        val saved3 = SavedValues.string(saved.variable("peer"), "peer", 16)
        val saved4 = SavedValues.uint32(saved.variable("answered"), "answered")
        val saved5 = SavedValues.uint32(saved.variable("failed"), "failed")
        val saved6 = SavedValues.uint32(saved.variable("refused"), "refused")
        load = saved1
        label = saved2
        peer = saved3
        answered = saved4
        failed = saved5
        refused = saved6
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    // §scxml-6.4.1: a saved state names the `<invoke>`s a declared host invoker
    // is running, and a restore starts each again from the request it saved. What
    // it may name is what the document hands to a host.
    override val staticHostInvokes: List<Triple<String, String, StatechartStaticMeshRequestState>> = listOf(
        Triple("sce:mesh-rpc", "ask", StatechartStaticMeshRequestState.Asking),
    )

    override val initialState: StatechartStaticMeshRequestState = StatechartStaticMeshRequestState.Idle

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
    override fun isFinalState(state: StatechartStaticMeshRequestState): Boolean = when (state) {
        is StatechartStaticMeshRequestState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartStaticMeshRequestState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartStaticMeshRequestState, HistoryId>> =
            listOf(StateTarget(StatechartStaticMeshRequestState.Idle))

        // W3C SCXML 3.13: asking's transition 0, as the microstep reads it.
        val transitionAskingAt0 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Asking,
            listOf(StateTarget(StatechartStaticMeshRequestState.Done)),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 1, as the microstep reads it.
        val transitionAskingAt1 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Asking,
            listOf(StateTarget(StatechartStaticMeshRequestState.Done)),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: asking's transition 2, as the microstep reads it.
        val transitionAskingAt2 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Asking,
            listOf(StateTarget(StatechartStaticMeshRequestState.Done)),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Idle,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 2, as the microstep reads it.
        val transitionIdleAt2 = EnabledTransition<StatechartStaticMeshRequestState, HistoryId>(
            StatechartStaticMeshRequestState.Idle,
            listOf(StateTarget(StatechartStaticMeshRequestState.Asking)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartStaticMeshRequestState? = when (stateId) {
        "asking" -> StatechartStaticMeshRequestState.Asking
        "done" -> StatechartStaticMeshRequestState.Done
        "idle" -> StatechartStaticMeshRequestState.Idle
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartStaticMeshRequestState): String = when (state) {
        is StatechartStaticMeshRequestState.Asking -> "asking"
        is StatechartStaticMeshRequestState.Done -> "done"
        is StatechartStaticMeshRequestState.Idle -> "idle"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartStaticMeshRequestState): Int = when (state) {
        is StatechartStaticMeshRequestState.Asking -> 1
        is StatechartStaticMeshRequestState.Done -> 2
        is StatechartStaticMeshRequestState.Idle -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartStaticMeshRequestEvent? = when (name) {
        "bump" -> StatechartStaticMeshRequestEvent.Bump
        "done.invoke" -> StatechartStaticMeshRequestEvent.Done.Invoke.Self
        "done.invoke.ask" -> StatechartStaticMeshRequestEvent.Done.Invoke.Ask
        "error.execution" -> StatechartStaticMeshRequestEvent.Error.Execution
        "error.invoke" -> StatechartStaticMeshRequestEvent.Error.Invoke.Self
        "error.invoke.ask" -> StatechartStaticMeshRequestEvent.Error.Invoke.Ask
        "ghost" -> StatechartStaticMeshRequestEvent.Ghost
        "go" -> StatechartStaticMeshRequestEvent.Go
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartStaticMeshRequestEvent): String? = when (event) {
        is StatechartStaticMeshRequestEvent.Bump -> "bump"
        is StatechartStaticMeshRequestEvent.Done.Invoke.Self -> "done.invoke"
        is StatechartStaticMeshRequestEvent.Done.Invoke.Ask -> "done.invoke.ask"
        is StatechartStaticMeshRequestEvent.Error.Execution -> "error.execution"
        is StatechartStaticMeshRequestEvent.Error.Invoke.Self -> "error.invoke"
        is StatechartStaticMeshRequestEvent.Error.Invoke.Ask -> "error.invoke.ask"
        is StatechartStaticMeshRequestEvent.Ghost -> "ghost"
        is StatechartStaticMeshRequestEvent.Go -> "go"
    }

    // W3C SCXML 6.4: these invokes are run by the host, so their `done.invoke`
    // is accepted only through `completeHostInvoke`.
    override val hostInvokeIds: Set<String> = setOf(
        "ask",
    )





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartStaticMeshRequestState,
        event: StatechartStaticMeshRequestEvent?
    ): EnabledTransition<StatechartStaticMeshRequestState, HistoryId>? = when (state) {
        is StatechartStaticMeshRequestState.Asking -> when {
            event is StatechartStaticMeshRequestEvent.Done.Invoke.Ask -> transitionAskingAt0
            event is StatechartStaticMeshRequestEvent.Error.Invoke.Ask -> transitionAskingAt1
            event is StatechartStaticMeshRequestEvent.Error.Execution -> transitionAskingAt2
            else -> null
        }
        is StatechartStaticMeshRequestState.Idle -> when {
            event is StatechartStaticMeshRequestEvent.Bump -> transitionIdleAt0
            event is StatechartStaticMeshRequestEvent.Ghost -> transitionIdleAt1
            event is StatechartStaticMeshRequestEvent.Go -> transitionIdleAt2
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_static_mesh_request.scxml:45 :: _machine
    override fun onEntry(state: StatechartStaticMeshRequestState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartStaticMeshRequestState.Asking -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:68 :: asking :: _state_body
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
                    val generatedInvokeId = "asking.${System.identityHashCode(this)}.ask"
                    deferInvoke(state, generatedInvokeId) {
                        val hostInvokeSrc: String = peer
                        val hostInvokeParams = mutableMapOf<String, List<String>>()
                        // The same pairs as the data model holds them, for the
                        // request's eventData: one evaluation, read as text for
                        // params and typed for the JSON.
                        val hostInvokePayload = mutableMapOf<String, Any?>()
                        hostInvokeParams["_mesh_event"] =
                            (hostInvokeParams["_mesh_event"] ?: emptyList()) + "service.request.force"
                        hostInvokeParams["_mesh_deadline_ms"] =
                            (hostInvokeParams["_mesh_deadline_ms"] ?: emptyList()) + "250"
                        try {
                            val v: Any? = (com.sce.forge.runtime.SceChecked.mul(load, 2.toUInt())).toLong()
                            hostInvokeParams["force"] =
                                (hostInvokeParams["force"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "force", v)
                        } catch (_: com.sce.forge.runtime.AlgorithmFailure) {
                            // W3C SCXML 5.7.1: report the failure and omit the pair.
                            raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<invoke> <param name='force'> expr failed to evaluate")
                        }
                        run {
                            val v: Any? = label
                            hostInvokeParams["speed"] =
                                (hostInvokeParams["speed"] ?: emptyList()) + valueToWireString(v)
                            putParam(hostInvokePayload, "speed", v)
                        }
                        val started = performHostInvoke(
                            HostInvokeRequest(
                                processorType = "sce:mesh-rpc",
                                invokeId = "ask",
                                src = hostInvokeSrc,
                                params = hostInvokeParams,
                                eventData = if (hostInvokePayload.isEmpty()) "" else buildJsonFromParams(hostInvokePayload),
                                content = ""                            )
                        )
                        if (!started) {
                            // W3C SCXML 6.4.1: declared but no invoker
                            // registered. The document asked for a process to
                            // be run and none was, which is the same fact as
                            // an unsupported type — so the same event, rather
                            // than a silence that reads as started.
                            raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<invoke> names an invoker the host declared but never registered")
                        }
                    }
                }
            }
            is StatechartStaticMeshRequestState.Done -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:87 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartStaticMeshRequestState.Idle -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:57 :: idle :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_static_mesh_request.scxml:45 :: _machine
    override fun onExit(state: StatechartStaticMeshRequestState) {
        when (state) {
            is StatechartStaticMeshRequestState.Asking -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:68 :: asking :: _state_body
                // W3C SCXML 6.4: Cancel pending invokes for exited state (deferred but not yet executed)
                cancelPendingInvokesForState(state)
                // W3C SCXML 6.4: the host's invocation ends with the state
                // that started it. Unconditional here: the engine knows
                // whether this one ever started and stays silent when it did
                // not, so the emitted chain needs no bookkeeping of its own.
                cancelHostInvoke("sce:mesh-rpc", "ask")
            }
            is StatechartStaticMeshRequestState.Done -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:87 :: done :: _state_body
            }
            is StatechartStaticMeshRequestState.Idle -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:57 :: idle :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_static_mesh_request.scxml:45 :: _machine
    override fun executeTransitionContent(source: StatechartStaticMeshRequestState, transitionIndex: Int) {
        when (source) {
        is StatechartStaticMeshRequestState.Asking -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:76 :: asking :: _transition_0

            if (try { answered = com.sce.forge.runtime.SceChecked.add(answered, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='answered'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:79 :: asking :: _transition_1

            if (try { failed = com.sce.forge.runtime.SceChecked.add(failed, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='failed'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:82 :: asking :: _transition_2

            if (try { refused = com.sce.forge.runtime.SceChecked.add(refused, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='refused'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StatechartStaticMeshRequestState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:58 :: idle :: _transition_0

            if (try { load = com.sce.forge.runtime.SceChecked.add(load, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='load'>: an integer operation overflowed or failed"); true }) {
                return
            }

            if (try { label = com.sce.forge.runtime.SceChecked.bounded("busy", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='label'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: statechart_static_mesh_request.scxml:62 :: idle :: _transition_1

            if (try { peer = com.sce.forge.runtime.SceChecked.bounded("#ghost", 16); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticMeshRequestEvent.Error.Execution, "<assign location='peer'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        else -> {}
        }
    }
}
