// SCE-GENERATED — DO NOT EDIT
// source-hash: e9c8544fc577b0f10a7b3d3f2c55aa52bb95a3f439b9ed10ddb1632871bd6aa8

// GENERATED CODE — DO NOT EDIT
// Source: sce-build/tests/fixtures/host_processor/statechart_delayed_host_send.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine

package com.sce.integration.statechart_delayed_host_send

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface StatechartDelayedHostSendState : State {
    data object Armed : StatechartDelayedHostSendState
    data object Cancelling : StatechartDelayedHostSendState
    data object CancelLost : StatechartDelayedHostSendState
    data object CancelPending : StatechartDelayedHostSendState
    data object Pass : StatechartDelayedHostSendState
    data object TooEarly : StatechartDelayedHostSendState
    data object Unserved : StatechartDelayedHostSendState
    data object Waiting : StatechartDelayedHostSendState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface StatechartDelayedHostSendEvent : Event {
    sealed interface Error : StatechartDelayedHostSendEvent {
        data object Execution : Error
    }
    data object Finish : StatechartDelayedHostSendEvent
    data object Probe : StatechartDelayedHostSendEvent
    data object Settle : StatechartDelayedHostSendEvent
    sealed interface Turn : StatechartDelayedHostSendEvent {
        data object Done : Turn
    }
    sealed interface Watch : StatechartDelayedHostSendEvent {
        data object Turn : Watch
    }
}
// --- State Machine (W3C SCXML) ---

class StatechartDelayedHostSendStateMachine(
) : StateMachineEngine<StatechartDelayedHostSendState, StatechartDelayedHostSendEvent>() {

    override val initialState: StatechartDelayedHostSendState = StatechartDelayedHostSendState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: StatechartDelayedHostSendState): Boolean = when (state) {
        is StatechartDelayedHostSendState.CancelLost, is StatechartDelayedHostSendState.Pass, is StatechartDelayedHostSendState.TooEarly, is StatechartDelayedHostSendState.Unserved -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<StatechartDelayedHostSendState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<StatechartDelayedHostSendState, HistoryId>> =
            listOf(StateTarget(StatechartDelayedHostSendState.Waiting))

        // W3C SCXML 3.13: armed's transition 0, as the microstep reads it.
        val transitionArmedAt0 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Armed,
            listOf(StateTarget(StatechartDelayedHostSendState.Cancelling)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: armed's transition 1, as the microstep reads it.
        val transitionArmedAt1 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Armed,
            listOf(StateTarget(StatechartDelayedHostSendState.Unserved)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 0, as the microstep reads it.
        val transitionCancellingAt0 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Cancelling,
            listOf(StateTarget(StatechartDelayedHostSendState.CancelPending)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelling's transition 1, as the microstep reads it.
        val transitionCancellingAt1 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Cancelling,
            listOf(StateTarget(StatechartDelayedHostSendState.CancelLost)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelPending's transition 0, as the microstep reads it.
        val transitionCancelPendingAt0 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.CancelPending,
            listOf(StateTarget(StatechartDelayedHostSendState.CancelLost)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: cancelPending's transition 1, as the microstep reads it.
        val transitionCancelPendingAt1 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.CancelPending,
            listOf(StateTarget(StatechartDelayedHostSendState.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Waiting,
            listOf(StateTarget(StatechartDelayedHostSendState.TooEarly)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<StatechartDelayedHostSendState, HistoryId>(
            StatechartDelayedHostSendState.Waiting,
            listOf(StateTarget(StatechartDelayedHostSendState.Armed)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): StatechartDelayedHostSendState? = when (stateId) {
        "armed" -> StatechartDelayedHostSendState.Armed
        "cancelling" -> StatechartDelayedHostSendState.Cancelling
        "cancelLost" -> StatechartDelayedHostSendState.CancelLost
        "cancelPending" -> StatechartDelayedHostSendState.CancelPending
        "pass" -> StatechartDelayedHostSendState.Pass
        "tooEarly" -> StatechartDelayedHostSendState.TooEarly
        "unserved" -> StatechartDelayedHostSendState.Unserved
        "waiting" -> StatechartDelayedHostSendState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: StatechartDelayedHostSendState): String = when (state) {
        is StatechartDelayedHostSendState.Armed -> "armed"
        is StatechartDelayedHostSendState.Cancelling -> "cancelling"
        is StatechartDelayedHostSendState.CancelLost -> "cancelLost"
        is StatechartDelayedHostSendState.CancelPending -> "cancelPending"
        is StatechartDelayedHostSendState.Pass -> "pass"
        is StatechartDelayedHostSendState.TooEarly -> "tooEarly"
        is StatechartDelayedHostSendState.Unserved -> "unserved"
        is StatechartDelayedHostSendState.Waiting -> "waiting"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: StatechartDelayedHostSendState): Int = when (state) {
        is StatechartDelayedHostSendState.Armed -> 1
        is StatechartDelayedHostSendState.Cancelling -> 2
        is StatechartDelayedHostSendState.CancelLost -> 6
        is StatechartDelayedHostSendState.CancelPending -> 3
        is StatechartDelayedHostSendState.Pass -> 4
        is StatechartDelayedHostSendState.TooEarly -> 5
        is StatechartDelayedHostSendState.Unserved -> 7
        is StatechartDelayedHostSendState.Waiting -> 0
    }

    // W3C SCXML 6.4: Resolve event name to Event object (cross-SM routing)
    override fun resolveEventByName(name: String): StatechartDelayedHostSendEvent? = when (name) {
        "error.execution" -> StatechartDelayedHostSendEvent.Error.Execution
        "finish" -> StatechartDelayedHostSendEvent.Finish
        "probe" -> StatechartDelayedHostSendEvent.Probe
        "settle" -> StatechartDelayedHostSendEvent.Settle
        "turn.done" -> StatechartDelayedHostSendEvent.Turn.Done
        "watch.turn" -> StatechartDelayedHostSendEvent.Watch.Turn
        else -> null
    }

    // W3C SCXML 6.4: Resolve Event object to event name string
    override fun eventNameOf(event: StatechartDelayedHostSendEvent): String? = when (event) {
        is StatechartDelayedHostSendEvent.Error.Execution -> "error.execution"
        is StatechartDelayedHostSendEvent.Finish -> "finish"
        is StatechartDelayedHostSendEvent.Probe -> "probe"
        is StatechartDelayedHostSendEvent.Settle -> "settle"
        is StatechartDelayedHostSendEvent.Turn.Done -> "turn.done"
        is StatechartDelayedHostSendEvent.Watch.Turn -> "watch.turn"
    }





    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: StatechartDelayedHostSendState,
        event: StatechartDelayedHostSendEvent?
    ): EnabledTransition<StatechartDelayedHostSendState, HistoryId>? = when (state) {
        is StatechartDelayedHostSendState.Armed -> when {
            event is StatechartDelayedHostSendEvent.Turn.Done -> transitionArmedAt0
            event is StatechartDelayedHostSendEvent.Error.Execution -> transitionArmedAt1
            else -> null
        }
        is StatechartDelayedHostSendState.Cancelling -> when {
            event is StatechartDelayedHostSendEvent.Settle -> transitionCancellingAt0
            event is StatechartDelayedHostSendEvent.Turn.Done -> transitionCancellingAt1
            else -> null
        }
        is StatechartDelayedHostSendState.CancelPending -> when {
            event is StatechartDelayedHostSendEvent.Turn.Done -> transitionCancelPendingAt0
            event is StatechartDelayedHostSendEvent.Finish -> transitionCancelPendingAt1
            else -> null
        }
        is StatechartDelayedHostSendState.Waiting -> when {
            event is StatechartDelayedHostSendEvent.Turn.Done -> transitionWaitingAt0
            event is StatechartDelayedHostSendEvent.Probe -> transitionWaitingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine
    override fun onEntry(state: StatechartDelayedHostSendState, isDefaultEntry: Boolean) {
        when (state) {
            is StatechartDelayedHostSendState.Armed -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:70 :: armed :: _state_body
            }
            is StatechartDelayedHostSendState.Cancelling -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:81 :: cancelling :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "watch.turn"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "h2"
                )
                val hostDelayMs = 200L
                scheduleHostSend("h2", hostDelayMs, hostRequest)
            }


            scheduleSend("__send_2", 100L, StatechartDelayedHostSendEvent.Settle)
            }
            is StatechartDelayedHostSendState.CancelLost -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:101 :: cancelLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.CancelPending -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:90 :: cancelPending :: _state_body


            cancelSend("h2")


            scheduleSend("__send_3", 200L, StatechartDelayedHostSendEvent.Finish)
            }
            is StatechartDelayedHostSendState.Pass -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:99 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.TooEarly -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:100 :: tooEarly :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.Unserved -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:102 :: unserved :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:59 :: waiting :: _state_body


            // W3C SCXML 6.2.5: "x-sce-host" is served by the host,
            // which declared it to this build. Dispatch rather than refuse —
            // and take the whole send, because a processor the host serves
            // owns delivery; falling through would also enqueue the event
            // locally and the document would see the act twice.
            run {
                val hostParams = mutableMapOf<String, List<String>>()
                val hostEventName = "watch.turn"
                val hostRequest = HostSendRequest(
                    processorType = "x-sce-host",
                    eventName = hostEventName,
                    target = "",
                    content = "",
                    params = hostParams,
                    sendId = "__send_0"
                )
                val hostDelayMs = 200L
                scheduleHostSend("__send_0", hostDelayMs, hostRequest)
            }


            scheduleSend("__send_1", 100L, StatechartDelayedHostSendEvent.Probe)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine
    override fun onExit(state: StatechartDelayedHostSendState) {
        when (state) {
            is StatechartDelayedHostSendState.Armed -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:70 :: armed :: _state_body
            }
            is StatechartDelayedHostSendState.Cancelling -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:81 :: cancelling :: _state_body
            }
            is StatechartDelayedHostSendState.CancelLost -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:101 :: cancelLost :: _state_body
            }
            is StatechartDelayedHostSendState.CancelPending -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:90 :: cancelPending :: _state_body
            }
            is StatechartDelayedHostSendState.Pass -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:99 :: pass :: _state_body
            }
            is StatechartDelayedHostSendState.TooEarly -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:100 :: tooEarly :: _state_body
            }
            is StatechartDelayedHostSendState.Unserved -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:102 :: unserved :: _state_body
            }
            is StatechartDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:59 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine
    override fun executeTransitionContent(source: StatechartDelayedHostSendState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
