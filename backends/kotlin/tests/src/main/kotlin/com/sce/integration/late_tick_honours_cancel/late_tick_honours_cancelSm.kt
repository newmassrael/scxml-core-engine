// SCE-GENERATED — DO NOT EDIT
// source-hash: 830b289f80d91dc5d572815c7cc65d24d654428511fa200c1077c009c1ba9b91
// template-hash: 0df7c3dd89bf1ab35c62dca175cae2bb2e377b70fda63f4fb76009a06edcd3df
// generated-at: 0

// GENERATED CODE — DO NOT EDIT
// Source: integration_resources/late_tick_honours_cancel/late_tick_honours_cancel.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine

package com.sce.integration.late_tick_honours_cancel

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface LateTickHonoursCancelState : State {
    data object Active : LateTickHonoursCancelState
    data object CancelLost : LateTickHonoursCancelState
    data object Pass : LateTickHonoursCancelState
    data object Waiting : LateTickHonoursCancelState
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface LateTickHonoursCancelEvent : Event {
    sealed interface Error : LateTickHonoursCancelEvent {
        data object Execution : Error
    }
    data object Finish : LateTickHonoursCancelEvent
    data object Poke : LateTickHonoursCancelEvent
    data object Settle : LateTickHonoursCancelEvent
}
// --- State Machine (W3C SCXML) ---

class LateTickHonoursCancelStateMachine(
) : StateMachineEngine<LateTickHonoursCancelState, LateTickHonoursCancelEvent>() {

    override val initialState: LateTickHonoursCancelState = LateTickHonoursCancelState.Waiting

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = true



    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): LateTickHonoursCancelState? = when (stateId) {
        "active" -> LateTickHonoursCancelState.Active
        "cancelLost" -> LateTickHonoursCancelState.CancelLost
        "pass" -> LateTickHonoursCancelState.Pass
        "waiting" -> LateTickHonoursCancelState.Waiting
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: LateTickHonoursCancelState): String = when (state) {
        is LateTickHonoursCancelState.Active -> "active"
        is LateTickHonoursCancelState.CancelLost -> "cancelLost"
        is LateTickHonoursCancelState.Pass -> "pass"
        is LateTickHonoursCancelState.Waiting -> "waiting"
    }

    // W3C SCXML 3.4: Check if state is atomic (leaf — no children)
    override fun isAtomicState(state: LateTickHonoursCancelState): Boolean = when (state) {
        else -> true
    }


    // W3C SCXML 3.13: Document order for exit ordering
    override fun documentOrderOf(state: LateTickHonoursCancelState): Int = when (state) {
        is LateTickHonoursCancelState.Active -> 1
        is LateTickHonoursCancelState.CancelLost -> 3
        is LateTickHonoursCancelState.Pass -> 2
        is LateTickHonoursCancelState.Waiting -> 0
    }





    // Pure function: (State, Event) -> TransitionResult (W3C SCXML 3.12)
    override fun processEvent(
        state: LateTickHonoursCancelState,
        event: LateTickHonoursCancelEvent
    ): TransitionResult<LateTickHonoursCancelState> = when (state) {
        is LateTickHonoursCancelState.Active -> processActive(event)
        is LateTickHonoursCancelState.Waiting -> processWaiting(event)
        else -> TransitionResult.Ignored
    }


    // --- Per-State Event Handlers ---

    private fun processActive(
        event: LateTickHonoursCancelEvent
    ): TransitionResult<LateTickHonoursCancelState> = when {
        event is LateTickHonoursCancelEvent.Settle -> TransitionResult.External(LateTickHonoursCancelState.CancelLost, LateTickHonoursCancelState.Active)

        event is LateTickHonoursCancelEvent.Finish -> TransitionResult.External(LateTickHonoursCancelState.Pass, LateTickHonoursCancelState.Active)

        else -> TransitionResult.Ignored
    }

    private fun processWaiting(
        event: LateTickHonoursCancelEvent
    ): TransitionResult<LateTickHonoursCancelState> = when {
        event is LateTickHonoursCancelEvent.Poke -> TransitionResult.External(LateTickHonoursCancelState.Active, LateTickHonoursCancelState.Waiting)

        event is LateTickHonoursCancelEvent.Settle -> TransitionResult.External(LateTickHonoursCancelState.CancelLost, LateTickHonoursCancelState.Waiting)

        else -> TransitionResult.Ignored
    }



    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine
    override fun onEntry(state: LateTickHonoursCancelState, pathChild: LateTickHonoursCancelState?) {
        when (state) {
            is LateTickHonoursCancelState.Active -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:50 :: active :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("active")) return


            cancelSend("s1")


            scheduleSend("__send_1", 100L, LateTickHonoursCancelEvent.Finish)
            }
            is LateTickHonoursCancelState.CancelLost -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:59 :: cancelLost :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("cancelLost")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is LateTickHonoursCancelState.Pass -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:58 :: pass :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("pass")) return
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is LateTickHonoursCancelState.Waiting -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:42 :: waiting :: _state_body
                // W3C SCXML 3.8: Track active state, skip duplicate entry
                if (!activeStateIds.add("waiting")) return


            scheduleSend("s1", 200L, LateTickHonoursCancelEvent.Settle)


            scheduleSend("__send_0", 100L, LateTickHonoursCancelEvent.Poke)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine
    override fun onExit(state: LateTickHonoursCancelState) {
        when (state) {
            is LateTickHonoursCancelState.Active -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:50 :: active :: _state_body
                activeStateIds.remove("active")
            }
            is LateTickHonoursCancelState.CancelLost -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:59 :: cancelLost :: _state_body
                activeStateIds.remove("cancelLost")
            }
            is LateTickHonoursCancelState.Pass -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:58 :: pass :: _state_body
                activeStateIds.remove("pass")
            }
            is LateTickHonoursCancelState.Waiting -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:42 :: waiting :: _state_body
                activeStateIds.remove("waiting")
            }
        }
    }


    // Transition Actions (W3C SCXML 3.13)
    // SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine
    override fun executeTransitionActions(
        source: LateTickHonoursCancelState,
        event: LateTickHonoursCancelEvent?
    ) {
        when (source) {
        else -> {}
        }
    }
}
