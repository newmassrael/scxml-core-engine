// SCE-GENERATED — DO NOT EDIT
// source-hash: b1edd275a200b2f8553040c83495e98b687c11a97259eaf4d60667291dcb916a

// GENERATED CODE — DO NOT EDIT
// Source: resources/396/test396.scxml
// Generator: SCE Kotlin Code Generator v1.0
// SCE-MAP: test396.scxml:5 :: _machine

package com.sce.generated.test396

import com.sce.runtime.*


// --- States (W3C SCXML 3.2) ---

sealed interface Test396State : State {
    data object Fail : Test396State
    data object Pass : Test396State
    data object S0 : Test396State
}

// --- Events (W3C SCXML 3.12.1) ---

sealed interface Test396Event : Event {
    data object Foo : Test396Event
}
// --- State Machine (W3C SCXML) ---

class Test396StateMachine(
) : StateMachineEngine<Test396State, Test396Event>() {

    override val initialState: Test396State = Test396State.S0

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
    override fun isFinalState(state: Test396State): Boolean = when (state) {
        is Test396State.Fail, is Test396State.Pass -> true
        else -> false
    }

    // W3C SCXML 3.2: the target of the document's own initial transition, as
    // written.
    override val documentInitialTargets: List<EntryTarget<Test396State, HistoryId>>
        get() = documentInitialTargetList

    private companion object {
        val documentInitialTargetList: List<EntryTarget<Test396State, HistoryId>> =
            listOf(StateTarget(Test396State.S0))

        // W3C SCXML 3.13: s0's transition 0, as the microstep reads it.
        val transitionS0At0 = EnabledTransition<Test396State, HistoryId>(
            Test396State.S0,
            listOf(StateTarget(Test396State.Pass)),
            0,
            hasActions = false,
            isInternal = false,
        )

        // W3C SCXML 3.13: s0's transition 1, as the microstep reads it.
        val transitionS0At1 = EnabledTransition<Test396State, HistoryId>(
            Test396State.S0,
            listOf(StateTarget(Test396State.Fail)),
            1,
            hasActions = false,
            isInternal = false,
        )
    }

    // W3C SCXML: Resolve state ID string to State object
    override fun resolveState(stateId: String): Test396State? = when (stateId) {
        "fail" -> Test396State.Fail
        "pass" -> Test396State.Pass
        "s0" -> Test396State.S0
        else -> null
    }

    // W3C SCXML: Get state ID string from State object
    override fun stateIdOf(state: Test396State): String = when (state) {
        is Test396State.Fail -> "fail"
        is Test396State.Pass -> "pass"
        is Test396State.S0 -> "s0"
    }

    // W3C SCXML 3.13: Document order — entry order, and in reverse exit order
    override fun documentOrderOf(state: Test396State): Int = when (state) {
        is Test396State.Fail -> 2
        is Test396State.Pass -> 1
        is Test396State.S0 -> 0
    }






    // W3C SCXML Appendix D selectTransitions, the half only this document can
    // answer: the first of `state`'s own transitions, in document order, that
    // `event` enables and whose guard holds; for `null`, its first eventless
    // transition whose guard holds. The runtime walks the atomic states and
    // their ancestors and keeps the ordered set.
    override fun firstEnabledTransition(
        state: Test396State,
        event: Test396Event?
    ): EnabledTransition<Test396State, HistoryId>? = when (state) {
        is Test396State.S0 -> when {
            event is Test396Event.Foo -> transitionS0At0
            event is Test396Event.Foo -> transitionS0At1
            else -> null
        }
        else -> null
    }


    // Entry Actions (W3C SCXML 3.8)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun onEntry(state: Test396State, isDefaultEntry: Boolean) {
        when (state) {
            is Test396State.Fail -> {
                // SCE-MAP: test396.scxml:19 :: fail :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.Pass -> {
                // SCE-MAP: test396.scxml:18 :: pass :: _state_body
                // W3C SCXML 3.7: Top-level final state reached
                markFinalStateReached()
            }
            is Test396State.S0 -> {
                // SCE-MAP: test396.scxml:7 :: s0 :: _state_body

            raiseInternal(Test396Event.Foo)
            }
        }
    }

    // Exit Actions (W3C SCXML 3.9)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun onExit(state: Test396State) {
        when (state) {
            is Test396State.Fail -> {
                // SCE-MAP: test396.scxml:19 :: fail :: _state_body
            }
            is Test396State.Pass -> {
                // SCE-MAP: test396.scxml:18 :: pass :: _state_body
            }
            is Test396State.S0 -> {
                // SCE-MAP: test396.scxml:7 :: s0 :: _state_body
            }
        }
    }


    // Transition Content (W3C SCXML 3.13)
    // SCE-MAP: test396.scxml:5 :: _machine
    override fun executeTransitionContent(source: Test396State, transitionIndex: Int) {
        when (source) {
        else -> {}
        }
    }
}
