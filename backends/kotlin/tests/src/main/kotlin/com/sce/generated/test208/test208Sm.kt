// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/208/test208.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test208.scxml:6 :: _machine

package com.sce.generated.test208

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test208State : State {
    data object Fail : Test208State
    data object Pass : Test208State
    data object S0 : Test208State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test208Event : Event {
    sealed interface Error : Test208Event {
        data object Execution : Error
    }
    data object Event1 : Test208Event
    data object Event2 : Test208Event
}
// --- State Machine (W3C SCXML) ---

class Test208StateMachine(
) : StateMachineEngine<Test208State, Test208Event>() {

    override val initialState: Test208State = Test208State.S0

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
    override fun isFinalState(state: Test208State): Boolean = when (state) {
        is Test208State.Fail, is Test208State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test208State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test208State, HistoryId>> =
            listOf(StateTarget(Test208State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test208State, HistoryId>(
            Test208State.S0,
            listOf(StateTarget(Test208State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test208State, HistoryId>(
            Test208State.S0,
            listOf(StateTarget(Test208State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test208State? = when (stateId) {
        "fail" -> Test208State.Fail
        "pass" -> Test208State.Pass
        "s0" -> Test208State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test208State): String = when (state) {
        is Test208State.Fail -> "fail"
        is Test208State.Pass -> "pass"
        is Test208State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test208State): Int = when (state) {
        is Test208State.Fail -> 2
        is Test208State.Pass -> 1
        is Test208State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test208State,
        event: Test208Event?
    ): EnabledTransition<Test208State, HistoryId>? = when (state) {
        is Test208State.S0 -> when {
            event is Test208Event.Event2 -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun onEntry(state: Test208State, isDefaultEntry: Boolean) {
        when (state) {
            is Test208State.Fail -> {
                // SCE-MAP: test208.scxml:23 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.Pass -> {
                // SCE-MAP: test208.scxml:22 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test208State.S0 -> {
                // SCE-MAP: test208.scxml:9 :: s0 :: _state_body


            scheduleSend("foo", 1000L, Test208Event.Event1)


            scheduleSend("__send_0", 1500L, Test208Event.Event2)


            cancelSend("foo")
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun onExit(state: Test208State) {
        when (state) {
            is Test208State.Fail -> {
                // SCE-MAP: test208.scxml:23 :: fail :: _state_body
            }
            is Test208State.Pass -> {
                // SCE-MAP: test208.scxml:22 :: pass :: _state_body
            }
            is Test208State.S0 -> {
                // SCE-MAP: test208.scxml:9 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test208.scxml:6 :: _machine
    override fun executeTransitionContent(source: Test208State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
