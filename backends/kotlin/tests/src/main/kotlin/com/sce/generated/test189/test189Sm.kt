// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/189/test189.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test189.scxml:6 :: _machine

package com.sce.generated.test189

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test189State : State {
    data object Fail : Test189State
    data object Pass : Test189State
    data object S0 : Test189State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test189Event : Event {
    sealed interface Error : Test189Event {
        data object Execution : Error
    }
    data object Event1 : Test189Event
    data object Event2 : Test189Event
}
// --- State Machine (W3C SCXML) ---

class Test189StateMachine(
) : StateMachineEngine<Test189State, Test189Event>() {

    override val initialState: Test189State = Test189State.S0

    // W3C SCXML 6.2: which entry point a host must drive this machine with in
    // the synchronous mode. The same verdict the generate manifest publishes
    // as `needs_event_scheduler`.
    override val needsEventScheduler: Boolean = false

    // --- Document structure (W3C SCXML 3.2-3.4, 3.10) ---
    //
    // What the runtime's Appendix D procedures (com.sce.runtime.Microstep)
    // read of this document. The tables are built once, in the companion
    // object below, because the structure is a fact about the document and
    // not about a run.

    // W3C SCXML 3.7: Check if state is a <final> element
    override fun isFinalState(state: Test189State): Boolean = when (state) {
        is Test189State.Fail, is Test189State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test189State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test189State, HistoryId>> =
            listOf(StateTarget(Test189State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test189State, HistoryId>(
            Test189State.S0,
            listOf(StateTarget(Test189State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test189State, HistoryId>(
            Test189State.S0,
            listOf(StateTarget(Test189State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test189State? = when (stateId) {
        "fail" -> Test189State.Fail
        "pass" -> Test189State.Pass
        "s0" -> Test189State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test189State): String = when (state) {
        is Test189State.Fail -> "fail"
        is Test189State.Pass -> "pass"
        is Test189State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test189State): Int = when (state) {
        is Test189State.Fail -> 2
        is Test189State.Pass -> 1
        is Test189State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test189State,
        event: Test189Event?
    ): EnabledTransition<Test189State, HistoryId>? = when (state) {
        is Test189State.S0 -> when {
            event is Test189Event.Event1 -> transitionS0At0
            event is Test189Event.Event2 -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun onEntry(state: Test189State, isDefaultEntry: Boolean) {
        when (state) {
            is Test189State.Fail -> {
                // SCE-MAP: test189.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.Pass -> {
                // SCE-MAP: test189.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test189State.S0 -> {
                // SCE-MAP: test189.scxml:9 :: s0 :: _state_body


            send(Test189Event.Event2, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            raiseInternal(Test189Event.Event1)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun onExit(state: Test189State) {
        when (state) {
            is Test189State.Fail -> {
                // SCE-MAP: test189.scxml:23 :: fail :: _state_body
            }
            is Test189State.Pass -> {
                // SCE-MAP: test189.scxml:22 :: pass :: _state_body
            }
            is Test189State.S0 -> {
                // SCE-MAP: test189.scxml:9 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test189.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test189State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
