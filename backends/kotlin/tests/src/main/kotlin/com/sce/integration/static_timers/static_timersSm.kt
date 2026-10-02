// SCE-GENERATED — DO NOT EDIT
// source-hash: cbe6524cc6d7a04c90586ae8e0fecdcc124ec753fe76818a098fd6c81c01547f

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/static_datamodel/static_timers.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: static_timers.scxml:22 :: _machine

package com.sce.integration.static_timers

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StaticTimersState : State {
    data object Done : StaticTimersState
    data object Waiting : StaticTimersState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StaticTimersEvent : Event {
    data object Beat : StaticTimersEvent
    data object Echo : StaticTimersEvent
    sealed interface Error : StaticTimersEvent {
        data object Execution : Error
    }
    data object Inner : StaticTimersEvent
    data object Stop : StaticTimersEvent
    data object Timeout : StaticTimersEvent
}
// --- State Machine (W3C SCXML) ---

class StaticTimersStateMachine(
) : StateMachineEngine<StaticTimersState, StaticTimersEvent>() {

    // ── SCE Accepted Subset §2.15: the datamodel="sce-static" variables ─────
    /** W3C SCXML 5.2: the `trace` datamodel variable, published (`sce:direction="out"`). */
    var trace: UInt = 0.toUInt()
        private set

    /** The published variables as one immutable value, in declaration order. */
    data class Data(
        val trace: UInt,
    )

    /**
     * What a host observes: the full active configuration — every active
     * state, each region of a `<parallel>` included — and the published
     * variables, taken together at a macrostep boundary. `truncated` is
     * `true` when that macrostep was stopped at the microstep ceiling, so the
     * configuration is not a stable one.
     */
    data class Snapshot(
        val configuration: Set<StaticTimersState>,
        val data: Data,
        val truncated: Boolean,
    )

    private fun currentData(): Data = Data(
        trace = trace,
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
    val savedShape: String = "4ccb8b35ee15410c2fa524072e0d2cc992b50980b7db00f5ea7bd0635c1a5b68"

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
            "trace" to SavedValues.of(trace),
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
        val saved1 = SavedValues.uint32(saved.variable("trace"), "trace")
        trace = saved1
        enterSaved(saved, wallNowMs)
    }

    /** [restore] at the host's wall clock now. */
    fun restore(saved: SavedState) = restore(saved, SavedState.wallClockMs())

    override val initialState: StaticTimersState = StaticTimersState.Waiting

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
    override fun isFinalState(state: StaticTimersState): Boolean = when (state) {
        is StaticTimersState.Done -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StaticTimersState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StaticTimersState, HistoryId>> =
            listOf(StateTarget(StaticTimersState.Waiting))

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StaticTimersState, HistoryId>(
            StaticTimersState.Waiting,
            emptyList(),
            0,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StaticTimersState, HistoryId>(
            StaticTimersState.Waiting,
            emptyList(),
            1,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 2, as the microstep reads it.
        val transitionWaitingAt2 = EnabledTransition<StaticTimersState, HistoryId>(
            StaticTimersState.Waiting,
            emptyList(),
            2,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 3, as the microstep reads it.
        val transitionWaitingAt3 = EnabledTransition<StaticTimersState, HistoryId>(
            StaticTimersState.Waiting,
            listOf(StateTarget(StaticTimersState.Done)),
            3,
            hasActions = true,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 4, as the microstep reads it.
        val transitionWaitingAt4 = EnabledTransition<StaticTimersState, HistoryId>(
            StaticTimersState.Waiting,
            emptyList(),
            4,
            hasActions = true,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StaticTimersState? = when (stateId) {
        "done" -> StaticTimersState.Done
        "waiting" -> StaticTimersState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StaticTimersState): String = when (state) {
        is StaticTimersState.Done -> "done"
        is StaticTimersState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StaticTimersState): Int = when (state) {
        is StaticTimersState.Done -> 1
        is StaticTimersState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StaticTimersEvent? = when (name) {
        "beat" -> StaticTimersEvent.Beat
        "echo" -> StaticTimersEvent.Echo
        "error.execution" -> StaticTimersEvent.Error.Execution
        "inner" -> StaticTimersEvent.Inner
        "stop" -> StaticTimersEvent.Stop
        "timeout" -> StaticTimersEvent.Timeout
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StaticTimersEvent): String? = when (event) {
        is StaticTimersEvent.Beat -> "beat"
        is StaticTimersEvent.Echo -> "echo"
        is StaticTimersEvent.Error.Execution -> "error.execution"
        is StaticTimersEvent.Inner -> "inner"
        is StaticTimersEvent.Stop -> "stop"
        is StaticTimersEvent.Timeout -> "timeout"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StaticTimersState,
        event: StaticTimersEvent?
    ): EnabledTransition<StaticTimersState, HistoryId>? = when (state) {
        is StaticTimersState.Waiting -> when {
            event is StaticTimersEvent.Inner -> transitionWaitingAt0
            event is StaticTimersEvent.Beat -> transitionWaitingAt1
            event is StaticTimersEvent.Echo -> transitionWaitingAt2
            event is StaticTimersEvent.Timeout -> transitionWaitingAt3
            event is StaticTimersEvent.Stop -> transitionWaitingAt4
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: static_timers.scxml:22 :: _machine
    override fun onEntry(state: StaticTimersState, isDefaultEntry: Boolean) {
        when (state) {
            is StaticTimersState.Done -> {
                // SCE-MAP: static_timers.scxml:50 :: done :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StaticTimersState.Waiting -> {
                // SCE-MAP: static_timers.scxml:27 :: waiting :: _state_body
                // W3C SCXML 3.8: Onentry block 1/1
                run {


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("timer", 5000L, StaticTimersEvent.Timeout, EventMetadata.external(sendId = "timer", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_0", 2000L, StaticTimersEvent.Beat, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 6.2: Delayed send
            scheduleSend("__send_1", 2000L, StaticTimersEvent.Echo, EventMetadata.external(sendId = "__send_1", origin = scriptSessionId ?: "", data = sendData))
            false
            }) {
                // W3C SCXML 4.9: an error raised while this element was
                // processed ends the block.
                return@run
            } // end of run send@ (W3C SCXML 6.2: a discarded message)


            if (run send@{
            val sendData = ""
            // W3C SCXML 5.10: an internal send carries `_event.data` just as an
            // external one does.
            // W3C SCXML 6.2.4: a delay postpones the send, it does not change
            // where it goes — the event joins the internal queue when due.
            scheduleInternalSend("__send_2", 1000L, StaticTimersEvent.Inner, EventMetadata.internal(sendData))
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
    // SCE-MAP: static_timers.scxml:22 :: _machine
    override fun onExit(state: StaticTimersState) {
        when (state) {
            is StaticTimersState.Done -> {
                // SCE-MAP: static_timers.scxml:50 :: done :: _state_body
            }
            is StaticTimersState.Waiting -> {
                // SCE-MAP: static_timers.scxml:27 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: static_timers.scxml:22 :: _machine
    override fun executeTransitionContent(source: StaticTimersState, transitionIndex: Int) {
        when (source) {
        is StaticTimersState.Waiting -> when (transitionIndex) {
            0 -> {
                // SCE-MAP: static_timers.scxml:34 :: waiting :: _transition_0

            if (try { trace = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(trace, 10.toUInt()), 1.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTimersEvent.Error.Execution, "<assign location='trace'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            1 -> {
                // SCE-MAP: static_timers.scxml:37 :: waiting :: _transition_1

            if (try { trace = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(trace, 10.toUInt()), 2.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTimersEvent.Error.Execution, "<assign location='trace'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            2 -> {
                // SCE-MAP: static_timers.scxml:40 :: waiting :: _transition_2

            if (try { trace = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(trace, 10.toUInt()), 4.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTimersEvent.Error.Execution, "<assign location='trace'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            3 -> {
                // SCE-MAP: static_timers.scxml:43 :: waiting :: _transition_3

            if (try { trace = com.sce.forge.runtime.SceChecked.add(com.sce.forge.runtime.SceChecked.mul(trace, 10.toUInt()), 3.toUInt()); false } catch (_: com.sce.forge.runtime.AlgorithmFailure) { raisePlatformError(StaticTimersEvent.Error.Execution, "<assign location='trace'>: an integer operation overflowed or failed"); true }) {
                return
            }
            }
            4 -> {
                // SCE-MAP: static_timers.scxml:46 :: waiting :: _transition_4


            cancelSend("timer")
            }
            else -> {}
        }
        else -> {}
        }
    }
}
