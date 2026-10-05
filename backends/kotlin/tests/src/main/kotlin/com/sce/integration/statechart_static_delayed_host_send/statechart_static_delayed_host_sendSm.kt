// SCE-GENERATED — DO NOT EDIT
// source-hash: 244875d17e69c3f5e7e3331d8a43e20ae13a35dfa6077c3f4b8715a65a5926ae

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_static_delayed_host_send.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_static_delayed_host_send.scxml:38 :: _machine

package com.sce.integration.statechart_static_delayed_host_send

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartStaticDelayedHostSendState : State {
    data object Idle : StatechartStaticDelayedHostSendState
    data object Waiting : StatechartStaticDelayedHostSendState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartStaticDelayedHostSendEvent : Event {
    data object Audit : StatechartStaticDelayedHostSendEvent
    data object Bump : StatechartStaticDelayedHostSendEvent
    data object Cancel : StatechartStaticDelayedHostSendEvent
    sealed interface Error : StatechartStaticDelayedHostSendEvent {
        data object Execution : Error
    }
    data object Start : StatechartStaticDelayedHostSendEvent
    data object Stop : StatechartStaticDelayedHostSendEvent
}
// --- State Machine (W3C SCXML) ---

class StatechartStaticDelayedHostSendStateMachine(
) : StateMachineEngine<StatechartStaticDelayedHostSendState, StatechartStaticDelayedHostSendEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `job` datamodel variable, the machine's own. */
    private var job: UInt = 7.toUInt()
    /** W3C SCXML 5.2: the `label` datamodel variable, the machine's own. */
    private var label: String = "report"

    /**
     * §scxml-6.4.1: the values a parent's `<param>` and `namelist` give this
     * machine's variables before it starts. A variable left `null` keeps the
     * value its `<data>` gave it.
     */
    class InvokeParams {
        var job: UInt? = null
        var label: String? = null
    }

    /** Give this machine the values [params] carries, in place of the ones its `<data>` gave. Called before [initialize]. */
    fun acceptParams(params: InvokeParams) {
        params.job?.let { job = it }
        params.label?.let { label = it }
    }

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StatechartStaticDelayedHostSendState>,
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
    val savedShape: String = "3dd18182ce8f84df8c1ae5222a201ee2019781e2717fef02bdf86519c43df534"

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
        job = saved1
        label = saved2
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StatechartStaticDelayedHostSendState = StatechartStaticDelayedHostSendState.Idle

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
    override val documentInitialTargets: List<EntryTarget<StatechartStaticDelayedHostSendState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartStaticDelayedHostSendState, HistoryId>> =
            listOf(StateTarget(StatechartStaticDelayedHostSendState.Idle))

        // W3C SCXML 3.13: idle's transition 0, as the microstep reads it.
        val transitionIdleAt0 = EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>(
            StatechartStaticDelayedHostSendState.Idle,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: idle's transition 1, as the microstep reads it.
        val transitionIdleAt1 = EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>(
            StatechartStaticDelayedHostSendState.Idle,
            listOf(StateTarget(StatechartStaticDelayedHostSendState.Waiting)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>(
            StatechartStaticDelayedHostSendState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>(
            StatechartStaticDelayedHostSendState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>(
            StatechartStaticDelayedHostSendState.Waiting,
            listOf(StateTarget(StatechartStaticDelayedHostSendState.Idle)),
            2,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartStaticDelayedHostSendState? = when (stateId) {
        "idle" -> StatechartStaticDelayedHostSendState.Idle
        "waiting" -> StatechartStaticDelayedHostSendState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartStaticDelayedHostSendState): String = when (state) {
        is StatechartStaticDelayedHostSendState.Idle -> "idle"
        is StatechartStaticDelayedHostSendState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartStaticDelayedHostSendState): Int = when (state) {
        is StatechartStaticDelayedHostSendState.Idle -> 0
        is StatechartStaticDelayedHostSendState.Waiting -> 1
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartStaticDelayedHostSendEvent? = when (name) {
        "audit" -> StatechartStaticDelayedHostSendEvent.Audit
        "bump" -> StatechartStaticDelayedHostSendEvent.Bump
        "cancel" -> StatechartStaticDelayedHostSendEvent.Cancel
        "error.execution" -> StatechartStaticDelayedHostSendEvent.Error.Execution
        "start" -> StatechartStaticDelayedHostSendEvent.Start
        "stop" -> StatechartStaticDelayedHostSendEvent.Stop
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartStaticDelayedHostSendEvent): String? = when (event) {
        is StatechartStaticDelayedHostSendEvent.Audit -> "audit"
        is StatechartStaticDelayedHostSendEvent.Bump -> "bump"
        is StatechartStaticDelayedHostSendEvent.Cancel -> "cancel"
        is StatechartStaticDelayedHostSendEvent.Error.Execution -> "error.execution"
        is StatechartStaticDelayedHostSendEvent.Start -> "start"
        is StatechartStaticDelayedHostSendEvent.Stop -> "stop"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartStaticDelayedHostSendState,
        event: StatechartStaticDelayedHostSendEvent?
    ): EnabledTransition<StatechartStaticDelayedHostSendState, HistoryId>? = when (state) {
        is StatechartStaticDelayedHostSendState.Idle -> when {
            event is StatechartStaticDelayedHostSendEvent.Bump -> transitionIdleAt0
            event is StatechartStaticDelayedHostSendEvent.Start -> transitionIdleAt1
            else -> null
        }
        is StatechartStaticDelayedHostSendState.Waiting -> when {
            event is StatechartStaticDelayedHostSendEvent.Bump -> transitionWaitingAt0
            event is StatechartStaticDelayedHostSendEvent.Cancel -> transitionWaitingAt1
            event is StatechartStaticDelayedHostSendEvent.Stop -> transitionWaitingAt2
            else -> null
        }
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_static_delayed_host_send.scxml:38 :: _machine
    override fun onEntry(state: StatechartStaticDelayedHostSendState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartStaticDelayedHostSendState.Idle -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:46 :: idle :: _state_body
            }
            is StatechartStaticDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:53 :: waiting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendPayload = mutableMapOf<String, Any?>()
            putParam(sendPayload, "label", label)

            putParam(sendPayload, "job", (job).toLong())

            val sendData = buildJsonFromParams(sendPayload)
            // The same pairs as the text a form carries (W3C SCXML C.2), so no
            // `<param>` is evaluated twice.
            val sendWireParams = sendPayload.mapValues { (_, v) ->
                if (v is RepeatedParam) v.values.map { valueToWireString(it) } else listOf(valueToWireString(v))
            }
            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            val hostRequest = HostSendRequest(
                processorType = "x-sce-host",
                eventName = "audit",
                target = "job://report",
                content = "",
                params = sendWireParams,
                sendId = "a",
                eventData = sendData,
                // SCE_MESH.md §mesh-10.7: the invokeid of the event being
                // processed now, carried back out as a W3C child's send to its
                // parent carries it (§scxml-6.4.1).
                invokeId = currentEventMetadata.invokeId
            )
            // W3C SCXML 6.2.4: a `delay` is a property of the SEND, not of the
            // processor it named. The engine performs the act from its
            // scheduler drain at the deadline, including the W3C SCXML 6.2
            // report for an act nobody performed. W3C SCXML 6.3: it lands in
            // the delayed-send queue under the send id, so a `<cancel>`
            // reaches it and the host never sees the act.
            scheduleHostSend("a", 500L, hostRequest)
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)
                }
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_static_delayed_host_send.scxml:38 :: _machine
    override fun onExit(state: StatechartStaticDelayedHostSendState) {
        when (state) {
            is StatechartStaticDelayedHostSendState.Idle -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:46 :: idle :: _state_body
            }
            is StatechartStaticDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:53 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_static_delayed_host_send.scxml:38 :: _machine
    override fun executeTransitionContent(source: StatechartStaticDelayedHostSendState, transitionIndex: Int) {
        when (source) {
        is StatechartStaticDelayedHostSendState.Idle -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:47 :: idle :: _transition_0

            if (try { job = com.sce.forge.runtime.SceChecked.add(job, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticDelayedHostSendEvent.Error.Execution, "<assign location='job'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            else -> {}
        }
        is StatechartStaticDelayedHostSendState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:60 :: waiting :: _transition_0

            if (try { job = com.sce.forge.runtime.SceChecked.add(job, 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StatechartStaticDelayedHostSendEvent.Error.Execution, "<assign location='job'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: statechart_static_delayed_host_send.scxml:63 :: waiting :: _transition_1


            cancelSend("a")
            }
            else -> {}
        }
        }
    }
}
