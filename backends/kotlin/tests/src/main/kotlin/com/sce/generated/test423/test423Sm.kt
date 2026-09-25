// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/423/test423.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test423.scxml:4 :: _machine

package com.sce.generated.test423

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test423State : State {
    data object Fail : Test423State
    data object Pass : Test423State
    data object S0 : Test423State
    data object S1 : Test423State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test423Event : Event {
    sealed interface Error : Test423Event {
        data object Execution : Error
    }
    data object ExternalEvent1 : Test423Event
    data object ExternalEvent2 : Test423Event
    data object InternalEvent : Test423Event
}
// --- State Machine (W3C SCXML) ---

class Test423StateMachine(
) : StateMachineEngine<Test423State, Test423Event>() {

    override val initialState: Test423State = Test423State.S0

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
    override fun isFinalState(state: Test423State): Boolean = when (state) {
        is Test423State.Fail, is Test423State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test423State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test423State, HistoryId>> =
            listOf(StateTarget(Test423State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test423State, HistoryId>(
            Test423State.S0,
            listOf(StateTarget(Test423State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test423State, HistoryId>(
            Test423State.S0,
            listOf(StateTarget(Test423State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test423State, HistoryId>(
            Test423State.S1,
            listOf(StateTarget(Test423State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test423State, HistoryId>(
            Test423State.S1,
            listOf(StateTarget(Test423State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test423State? = when (stateId) {
        "fail" -> Test423State.Fail
        "pass" -> Test423State.Pass
        "s0" -> Test423State.S0
        "s1" -> Test423State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test423State): String = when (state) {
        is Test423State.Fail -> "fail"
        is Test423State.Pass -> "pass"
        is Test423State.S0 -> "s0"
        is Test423State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test423State): Int = when (state) {
        is Test423State.Fail -> 3
        is Test423State.Pass -> 2
        is Test423State.S0 -> 0
        is Test423State.S1 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test423State,
        event: Test423Event?
    ): EnabledTransition<Test423State, HistoryId>? = when (state) {
        is Test423State.S0 -> when {
            event is Test423Event.InternalEvent -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        is Test423State.S1 -> when {
            event is Test423Event.ExternalEvent2 -> transitionS1At0
            event is Test423Event.InternalEvent -> transitionS1At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun onEntry(state: Test423State, isDefaultEntry: Boolean) {
        when (state) {
            is Test423State.Fail -> {
                // SCE-MAP: test423.scxml:26 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.Pass -> {
                // SCE-MAP: test423.scxml:25 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test423State.S0 -> {
                // SCE-MAP: test423.scxml:7 :: s0 :: _state_body


            send(Test423Event.ExternalEvent1, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))


            scheduleSend("__send_1", 1000L, Test423Event.ExternalEvent2)

            raiseInternal(Test423Event.InternalEvent)
            }
            is Test423State.S1 -> {
                // SCE-MAP: test423.scxml:18 :: s1 :: _state_body
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun onExit(state: Test423State) {
        when (state) {
            is Test423State.Fail -> {
                // SCE-MAP: test423.scxml:26 :: fail :: _state_body
            }
            is Test423State.Pass -> {
                // SCE-MAP: test423.scxml:25 :: pass :: _state_body
            }
            is Test423State.S0 -> {
                // SCE-MAP: test423.scxml:7 :: s0 :: _state_body
            }
            is Test423State.S1 -> {
                // SCE-MAP: test423.scxml:18 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test423.scxml:4 :: _machine
    override fun executeTransitionContent(source: Test423State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
