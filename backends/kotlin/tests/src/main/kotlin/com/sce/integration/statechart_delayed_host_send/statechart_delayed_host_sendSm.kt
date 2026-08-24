// SCE-GENERATED — DO NOT EDIT
// source-hash: 484e7440f07c529b155abfa6f79282de908af5e2fc4314e70bd834573adce55b
// template-hash: 580f12cd61336d7449660775c4fcc4f615ee3c32bffa0e9792363e260aed93e2
// generated-at: 0

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

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: StatechartDelayedHostSendState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
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




    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: StatechartDelayedHostSendState,
        event: StatechartDelayedHostSendEvent
    ): TransitionResult<StatechartDelayedHostSendState> = when (state) {
        is StatechartDelayedHostSendState.Armed -> processArmed(event)
        is StatechartDelayedHostSendState.Cancelling -> processCancelling(event)
        is StatechartDelayedHostSendState.CancelPending -> processCancelPending(event)
        is StatechartDelayedHostSendState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processArmed(
        event: StatechartDelayedHostSendEvent
    ): TransitionResult<StatechartDelayedHostSendState> = when {
        event is StatechartDelayedHostSendEvent.Turn.Done -> TransitionResult.External(StatechartDelayedHostSendState.Cancelling, StatechartDelayedHostSendState.Armed)

        event is StatechartDelayedHostSendEvent.Error.Execution -> TransitionResult.External(StatechartDelayedHostSendState.Unserved, StatechartDelayedHostSendState.Armed)

        else -> TransitionResult.Ignored
    }

    private fun processCancelling(
        event: StatechartDelayedHostSendEvent
    ): TransitionResult<StatechartDelayedHostSendState> = when {
        event is StatechartDelayedHostSendEvent.Settle -> TransitionResult.External(StatechartDelayedHostSendState.CancelPending, StatechartDelayedHostSendState.Cancelling)

        event is StatechartDelayedHostSendEvent.Turn.Done -> TransitionResult.External(StatechartDelayedHostSendState.CancelLost, StatechartDelayedHostSendState.Cancelling)

        else -> TransitionResult.Ignored
    }

    private fun processCancelPending(
        event: StatechartDelayedHostSendEvent
    ): TransitionResult<StatechartDelayedHostSendState> = when {
        event is StatechartDelayedHostSendEvent.Turn.Done -> TransitionResult.External(StatechartDelayedHostSendState.CancelLost, StatechartDelayedHostSendState.CancelPending)

        event is StatechartDelayedHostSendEvent.Finish -> TransitionResult.External(StatechartDelayedHostSendState.Pass, StatechartDelayedHostSendState.CancelPending)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: StatechartDelayedHostSendEvent
    ): TransitionResult<StatechartDelayedHostSendState> = when {
        event is StatechartDelayedHostSendEvent.Turn.Done -> TransitionResult.External(StatechartDelayedHostSendState.TooEarly, StatechartDelayedHostSendState.Waiting)

        event is StatechartDelayedHostSendEvent.Probe -> TransitionResult.External(StatechartDelayedHostSendState.Armed, StatechartDelayedHostSendState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine
    override fun onEntry(state: StatechartDelayedHostSendState, pathChild: StatechartDelayedHostSendState?) {
        when (state) {
            is StatechartDelayedHostSendState.Armed -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:70 :: armed :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("armed")) return
            }
            is StatechartDelayedHostSendState.Cancelling -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:81 :: cancelling :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("cancelling")) return


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
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("cancelLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.CancelPending -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:90 :: cancelPending :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("cancelPending")) return


            cancelSend("h2")


            scheduleSend("__send_3", 200L, StatechartDelayedHostSendEvent.Finish)
            }
            is StatechartDelayedHostSendState.Pass -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:99 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.TooEarly -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:100 :: tooEarly :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("tooEarly")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.Unserved -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:102 :: unserved :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("unserved")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is StatechartDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:59 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return


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
                activeStateIds.remove("armed")
            }
            is StatechartDelayedHostSendState.Cancelling -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:81 :: cancelling :: _state_body
                activeStateIds.remove("cancelling")
            }
            is StatechartDelayedHostSendState.CancelLost -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:101 :: cancelLost :: _state_body
                activeStateIds.remove("cancelLost")
            }
            is StatechartDelayedHostSendState.CancelPending -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:90 :: cancelPending :: _state_body
                activeStateIds.remove("cancelPending")
            }
            is StatechartDelayedHostSendState.Pass -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:99 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is StatechartDelayedHostSendState.TooEarly -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:100 :: tooEarly :: _state_body
                activeStateIds.remove("tooEarly")
            }
            is StatechartDelayedHostSendState.Unserved -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:102 :: unserved :: _state_body
                activeStateIds.remove("unserved")
            }
            is StatechartDelayedHostSendState.Waiting -> {
                // SCE-MAP: statechart_delayed_host_send.scxml:59 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: statechart_delayed_host_send.scxml:55 :: _machine
    override fun executeTransitionActions(
        source: StatechartDelayedHostSendState,
        event: StatechartDelayedHostSendEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
