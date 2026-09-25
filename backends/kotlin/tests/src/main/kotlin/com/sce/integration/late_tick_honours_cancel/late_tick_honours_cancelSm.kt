// SCE-GENERATED — DO NOT EDIT
// source-hash: 830b289f80d91dc5d572815c7cc65d24d654428511fa200c1077c009c1ba9b91

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

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: LateTickHonoursCancelState): Boolean = when (state) {
        is LateTickHonoursCancelState.CancelLost, is LateTickHonoursCancelState.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<LateTickHonoursCancelState, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<LateTickHonoursCancelState, HistoryId>> =
            listOf(StateTarget(LateTickHonoursCancelState.Waiting))

        // W3C SCXML 3.13: active's transition 0, as the microstep reads it.
        val transitionActiveAt0 = EnabledTransition<LateTickHonoursCancelState, HistoryId>(
            LateTickHonoursCancelState.Active,
            listOf(StateTarget(LateTickHonoursCancelState.CancelLost)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: active's transition 1, as the microstep reads it.
        val transitionActiveAt1 = EnabledTransition<LateTickHonoursCancelState, HistoryId>(
            LateTickHonoursCancelState.Active,
            listOf(StateTarget(LateTickHonoursCancelState.Pass)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 0, as the microstep reads it.
        val transitionWaitingAt0 = EnabledTransition<LateTickHonoursCancelState, HistoryId>(
            LateTickHonoursCancelState.Waiting,
            listOf(StateTarget(LateTickHonoursCancelState.Active)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: waiting's transition 1, as the microstep reads it.
        val transitionWaitingAt1 = EnabledTransition<LateTickHonoursCancelState, HistoryId>(
            LateTickHonoursCancelState.Waiting,
            listOf(StateTarget(LateTickHonoursCancelState.CancelLost)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

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

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: LateTickHonoursCancelState): Int = when (state) {
        is LateTickHonoursCancelState.Active -> 1
        is LateTickHonoursCancelState.CancelLost -> 3
        is LateTickHonoursCancelState.Pass -> 2
        is LateTickHonoursCancelState.Waiting -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: LateTickHonoursCancelState,
        event: LateTickHonoursCancelEvent?
    ): EnabledTransition<LateTickHonoursCancelState, HistoryId>? = when (state) {
        is LateTickHonoursCancelState.Active -> when {
            event is LateTickHonoursCancelEvent.Settle -> transitionActiveAt0
            event is LateTickHonoursCancelEvent.Finish -> transitionActiveAt1
            else -> null
        }
        is LateTickHonoursCancelState.Waiting -> when {
            event is LateTickHonoursCancelEvent.Poke -> transitionWaitingAt0
            event is LateTickHonoursCancelEvent.Settle -> transitionWaitingAt1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine
    override fun onEntry(state: LateTickHonoursCancelState, isDefaultEntry: Boolean) {
        when (state) {
            is LateTickHonoursCancelState.Active -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:50 :: active :: _state_body


            cancelSend("s1")


            scheduleSend("__send_1", 100L, LateTickHonoursCancelEvent.Finish)
            }
            is LateTickHonoursCancelState.CancelLost -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:59 :: cancelLost :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is LateTickHonoursCancelState.Pass -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:58 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is LateTickHonoursCancelState.Waiting -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:42 :: waiting :: _state_body


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
            }
            is LateTickHonoursCancelState.CancelLost -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:59 :: cancelLost :: _state_body
            }
            is LateTickHonoursCancelState.Pass -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:58 :: pass :: _state_body
            }
            is LateTickHonoursCancelState.Waiting -> {
                // SCE-MAP: late_tick_honours_cancel.scxml:42 :: waiting :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: late_tick_honours_cancel.scxml:39 :: _machine
    override fun executeTransitionContent(source: LateTickHonoursCancelState, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
