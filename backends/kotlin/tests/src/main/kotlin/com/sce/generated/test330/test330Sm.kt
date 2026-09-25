// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/330/test330.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test330.scxml:5 :: _machine

package com.sce.generated.test330

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test330State : State {
    data object Fail : Test330State
    data object Pass : Test330State
    data object S0 : Test330State
    data object S1 : Test330State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test330Event : Event {
    sealed interface Error : Test330Event {
        data object Execution : Error
    }
    data object Foo : Test330Event
}
// --- State Machine (W3C SCXML) ---

class Test330StateMachine(
) : StateMachineEngine<Test330State, Test330Event>() {

    override val initialState: Test330State = Test330State.S0

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
    override fun isFinalState(state: Test330State): Boolean = when (state) {
        is Test330State.Fail, is Test330State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test330State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test330State, HistoryId>> =
            listOf(StateTarget(Test330State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test330State, HistoryId>(
            Test330State.S0,
            listOf(StateTarget(Test330State.S1)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test330State, HistoryId>(
            Test330State.S0,
            listOf(StateTarget(Test330State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 0, as the microstep reads it.
        val transitionS1At0 = EnabledTransition<Test330State, HistoryId>(
            Test330State.S1,
            listOf(StateTarget(Test330State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s1's transition 1, as the microstep reads it.
        val transitionS1At1 = EnabledTransition<Test330State, HistoryId>(
            Test330State.S1,
            listOf(StateTarget(Test330State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test330State? = when (stateId) {
        "fail" -> Test330State.Fail
        "pass" -> Test330State.Pass
        "s0" -> Test330State.S0
        "s1" -> Test330State.S1
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test330State): String = when (state) {
        is Test330State.Fail -> "fail"
        is Test330State.Pass -> "pass"
        is Test330State.S0 -> "s0"
        is Test330State.S1 -> "s1"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test330State): Int = when (state) {
        is Test330State.Fail -> 3
        is Test330State.Pass -> 2
        is Test330State.S0 -> 0
        is Test330State.S1 -> 1
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test330State,
        event: Test330Event?
    ): EnabledTransition<Test330State, HistoryId>? = when (state) {
        is Test330State.S0 -> when {
            event is Test330Event.Foo -> transitionS0At0
            event != null -> transitionS0At1
            else -> null
        }
        is Test330State.S1 -> when {
            event is Test330Event.Foo -> transitionS1At0
            event != null -> transitionS1At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun onEntry(state: Test330State, isDefaultEntry: Boolean) {
        when (state) {
            is Test330State.Fail -> {
                // SCE-MAP: test330.scxml:25 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.Pass -> {
                // SCE-MAP: test330.scxml:24 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test330State.S0 -> {
                // SCE-MAP: test330.scxml:7 :: s0 :: _state_body

            raiseInternal(Test330Event.Foo)
            }
            is Test330State.S1 -> {
                // SCE-MAP: test330.scxml:15 :: s1 :: _state_body


            send(Test330Event.Foo, EventMetadata.external(sendId = "__send_0", origin = scriptSessionId ?: ""))
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun onExit(state: Test330State) {
        when (state) {
            is Test330State.Fail -> {
                // SCE-MAP: test330.scxml:25 :: fail :: _state_body
            }
            is Test330State.Pass -> {
                // SCE-MAP: test330.scxml:24 :: pass :: _state_body
            }
            is Test330State.S0 -> {
                // SCE-MAP: test330.scxml:7 :: s0 :: _state_body
            }
            is Test330State.S1 -> {
                // SCE-MAP: test330.scxml:15 :: s1 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test330.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test330State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
